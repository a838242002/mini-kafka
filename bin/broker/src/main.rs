use std::{path::PathBuf, sync::Arc};

use broker::Broker;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    std::fs::create_dir_all("data").ok();
    let broker = Arc::new(Broker::new(PathBuf::from("data")));
    net::serve("127.0.0.1:9092", broker).await
}
