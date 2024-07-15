use tokio::{
    io,
    net::{TcpListener, TcpStream},
};

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:9999").await?;

    let addresses = ["api01:3000", "api02:3000"];
    let mut counter = 0;

    while let Ok((mut downstream, _)) = listener.accept().await {
        counter += 1;

        let address = addresses[counter % addresses.len()];
        let mut upstream = TcpStream::connect(address).await?;

        tokio::spawn(async move {
            io::copy_bidirectional(&mut downstream, &mut upstream)
                .await
                .unwrap();
        });
    }

    Ok(())
}
