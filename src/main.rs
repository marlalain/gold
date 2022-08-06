use tokio::io;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> io::Result<()> {
    // For some reason, 'localhost' is not the same as '127.0.0.1'.
    // Using 'localhost' gives UB (kinda?).
    let server_addr = "127.0.0.1:5445";
    let listener = TcpListener::bind(server_addr).await?;
    println!("Listening on '{}'...", server_addr);

    loop {
        let (mut stream, socket_addr) = listener.accept().await?;
        println!("User connected: '{:?}'", socket_addr);

        stream.write_all(b"hello world").await?;
    }
}
