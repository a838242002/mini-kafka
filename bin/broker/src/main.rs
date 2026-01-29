#[tokio::main]
async fn main() -> std::io::Result<()> {
    net::serve_hello("127.0.0.1:9092").await
}