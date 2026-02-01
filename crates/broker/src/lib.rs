use std::{collections::HashMap, path::PathBuf};

use protocol::{Request, Response};
use storage::PartitionLog;
use tokio::sync::Mutex;

pub struct Broker {
    data_dir: PathBuf,
    partitions: Mutex<HashMap<(String, u16), PartitionLog>>,
}

impl Broker {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            partitions: Mutex::new(HashMap::new()),
        }
    }

    async fn get_or_open(&self, topic: &str, partition: u16) -> Result<PartitionLog, String> {
        let mut map = self.partitions.lock().await;
        if let Some(log) = map.remove(&(topic.to_string(), partition)) {
            return Ok(log);
        }

        PartitionLog::open(&self.data_dir, topic, partition).map_err(|e| e.to_string())
    }

    async fn put_back(&self, topic: &str, partition: u16, log: PartitionLog) {
        let mut map = self.partitions.lock().await;
        map.insert((topic.to_string(), partition), log);
    }

    pub async fn handle(&self, req: Request) -> Response {
        match req {
            Request::Produce(r) => {
                let mut log = match self.get_or_open(&r.topic, r.partition).await {
                    Ok(x) => x,
                    Err(e) => return Response::Error { message: e },
                };

                let resp = match log.append(&r.records) {
                    Ok(base) => Response::Produce(protocol::ProduceResponse {
                        status: 0,
                        base_offset: base,
                    }),
                    Err(e) => Response::Error {
                        message: format!("append error: {e}"),
                    },
                };

                self.put_back(&r.topic, r.partition, log).await;

                resp
            }

            Request::Fetch(r) => {
                let log = match self.get_or_open(&r.topic, r.partition).await {
                    Ok(x) => x,
                    Err(e) => return Response::Error { message: e },
                };

                let resp = match log.fetch(r.offset, r.max_bytes) {
                    Ok(items) => Response::Fetch(protocol::FetchResponse { status: 0, items }),
                    Err(e) => Response::Error {
                        message: format!("fetch error: {e}"),
                    },
                };

                self.put_back(&r.topic, r.partition, log).await;

                resp
            }
        }
    }
}
