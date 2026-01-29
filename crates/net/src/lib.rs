use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

pub async fn serve_hello(addr: &str) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("Server listening on {}", addr);

    loop {
        let (sock, peer) = listener.accept().await?;
        println!("Accepted connection from {}", peer);

        tokio::spawn(async move {
            if let Err(e) = handle_conn(sock).await {
                eprint!("conn error: {}", e);
            }
        });
    }
}

async fn handle_conn(mut sock: TcpStream) -> std::io::Result<()> {
    sock.write_all(b"mini-kafka-rs: hello!\n").await?;

    let mut buf = [0u8; 1024];
    loop {
        let n = sock.read(&mut buf).await?;
        if n == 0 {
            // client closed
            return Ok(());
        }
        sock.write_all(b"echo: ").await?;
        sock.write_all(&buf[..n]).await?;
    }
}