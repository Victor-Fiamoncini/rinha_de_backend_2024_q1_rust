use serde::{de::DeserializeOwned, Serialize};
use std::{
    error::Error,
    fs::{File, OpenOptions},
    io::{self, Read, Seek, Write},
    iter,
    marker::PhantomData,
    path::Path,
    u64,
};

const PAGE_SIZE: usize = 1024 * 4; // 4 kilo-bytes

pub enum DatabaseError {
    Io(io::Error),
    Serialize(Box<dyn Error>),
}

struct Page<const ROW_SIZE: usize = 64> {
    data: Vec<u8>,
}

impl<const ROW_SIZE: usize> Page<ROW_SIZE> {
    pub fn new() -> Self {
        Self {
            data: Vec::with_capacity(PAGE_SIZE),
        }
    }

    pub fn from_bytes(data: Vec<u8>) -> Result<Self, &'static str> {
        if data.len() != PAGE_SIZE {
            return Err("Invalid data size");
        }

        Ok(Self { data })
    }

    pub fn insert<S: Serialize>(&mut self, row: S) -> Result<(), DatabaseError> {
        let serialized =
            bitcode::serialize(&row).map_err(|err| DatabaseError::Serialize(Box::new(err)))?;

        let header = serialized.len() as u64;
        let header = header.to_be_bytes();

        self.data
            .write(&header)
            .map_err(|err| DatabaseError::Io(err))?;
        self.data
            .write(&serialized)
            .map_err(|err| DatabaseError::Io(err))?;
        self.data
            .write(&vec![0; ROW_SIZE - (serialized.len() + header.len())])
            .map_err(|err| DatabaseError::Io(err))?;

        Ok(())
    }

    pub fn rows(&self) -> impl Iterator<Item = &[u8]> + '_ {
        let mut cursor = 0;

        iter::from_fn(move || {
            let offset = cursor * ROW_SIZE;

            if offset + ROW_SIZE > self.data.len() {
                return None;
            }

            let row = &self.data[offset..offset + ROW_SIZE];
            let header = {
                let mut buffer = [0; 8];

                buffer.copy_from_slice(&row[0..8]);

                u64::from_be_bytes(buffer) as usize
            };

            if header == 0 {
                return None;
            }

            cursor += 1;

            Some(&row[8..8 + header])
        })
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn available_rows(&self) -> usize {
        (PAGE_SIZE - self.data.len()) / ROW_SIZE
    }
}

impl<const ROW_SIZE: usize> AsRef<[u8]> for Page<ROW_SIZE> {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

pub struct Database<T, const ROW_SIZE: usize = 64> {
    current_page: Page<ROW_SIZE>,
    reader: File,
    writer: File,
    data: PhantomData<T>,
}

impl<const ROW_SIZE: usize, T: Serialize + DeserializeOwned> Database<T, ROW_SIZE> {
    pub fn from_path(path: impl AsRef<Path>) -> io::Result<Self> {
        let file = OpenOptions::new().write(true).create(true).open(&path)?;

        Ok(Self {
            current_page: Page::new(),
            reader: File::open(&path)?,
            writer: file,
            data: PhantomData,
        })
    }

    pub fn insert(&mut self, row: T) -> Result<(), DatabaseError> {
        self.current_page.insert(row);

        self.writer.write_all(
            &[
                self.current_page.as_ref(),
                &vec![0; PAGE_SIZE - self.current_page.len()],
            ]
            .concat(),
        );

        if self.current_page.available_rows() == 0 {
            self.current_page = Page::new();
        } else {
            self.writer.seek(io::SeekFrom::End(-(PAGE_SIZE as i64)));
        }

        Ok(())
    }

    fn pages(&mut self) -> impl Iterator<Item = Page> + '_ {
        let mut cursor = 0;

        iter::from_fn(move || {
            let offset = (cursor * PAGE_SIZE) as u64;

            if self.reader.seek(io::SeekFrom::Start(offset)).is_err() {
                return None;
            }

            let mut buffer = vec![0; PAGE_SIZE];

            cursor += 1;

            match self.reader.read_exact(&mut buffer) {
                Ok(()) => Some(Page::from_bytes(buffer).unwrap()),
                Err(_) => None,
            }
        })
    }

    pub fn rows(&mut self) -> impl Iterator<Item = T> + '_ {
        self.pages().flat_map(|page| {
            page.rows()
                .filter_map(|row| bitcode::deserialize(row).ok())
                .collect::<Vec<_>>()
        })
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_insert_into_page() {
        let mut page = Page::<1024>::new();

        assert_eq!(4, page.available_rows());
        page.insert(String::from("Rinha")).ok();
        assert_eq!(3, page.available_rows());
        page.insert(String::from("de")).ok();
        assert_eq!(2, page.available_rows());
        page.insert(String::from("Backend")).ok();
        assert_eq!(1, page.available_rows());
        page.insert(2024 as usize).ok();
        assert_eq!(0, page.available_rows());

        let mut rows = page.rows();

        assert_eq!(
            "Rinha",
            bitcode::deserialize::<String>(&rows.next().unwrap()).unwrap()
        );
        assert_eq!(
            "de",
            bitcode::deserialize::<String>(&rows.next().unwrap()).unwrap()
        );
        assert_eq!(
            "Backend",
            bitcode::deserialize::<String>(&rows.next().unwrap()).unwrap()
        );
        assert_eq!(
            2024,
            bitcode::deserialize::<u64>(&rows.next().unwrap()).unwrap()
        );
        assert!(rows.next().is_none());
    }

    #[test]
    fn test_insert_into_database() {
        let tmp = tempdir().unwrap();
        let mut database =
            Database::<(i64, String), 1024>::from_path(tmp.path().join("test.db")).unwrap();

        database.insert((50, String::from("Primeira"))).ok();
        database.insert((-20, String::from("Segunda"))).ok();

        let mut rows = database.rows();

        assert_eq!((50, String::from("Primeira")), rows.next().unwrap());
        assert_eq!((-20, String::from("Segunda")), rows.next().unwrap());
        assert!(rows.next().is_none());
    }
}
