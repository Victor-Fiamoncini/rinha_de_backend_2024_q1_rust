use serde::Serialize;
use std::{
    error::Error,
    io::{self, Write},
    iter, u64,
};

const PAGE_SIZE: usize = 1024 * 4;
const ROW_SIZE: usize = 256;

struct Page {
    data: Vec<u8>,
}

enum DatabaseError {
    Io(io::Error),
    Serialize(Box<dyn Error>),
}

impl Page {
    pub fn new() -> Self {
        Self {
            data: Vec::with_capacity(PAGE_SIZE),
        }
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

    pub fn rows(&self) -> impl Iterator<Item = Vec<u8>> + '_ {
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

            cursor += 1;

            Some(row[8..8 + header].to_vec())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_into_page() {
        let mut page = Page::new();

        page.insert(String::from("Rinha"));
        page.insert(2024 as usize);

        let mut rows = page.rows();

        assert_eq!(
            "Rinha",
            bitcode::deserialize::<String>(&rows.next().unwrap()).unwrap()
        );
        assert_eq!(
            2024,
            bitcode::deserialize::<u64>(&rows.next().unwrap()).unwrap()
        );
    }
}
