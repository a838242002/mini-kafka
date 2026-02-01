use bytes::Bytes;

// ---------- domain types ----------
#[derive(Debug)]
pub enum ApiKey {
    Produce = 1,
    Fetch = 2,
}

impl TryFrom<u8> for ApiKey {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(ApiKey::Produce),
            2 => Ok(ApiKey::Fetch),
            x => Err(x),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Record {
    pub key: Bytes,
    pub value: Bytes,
}

#[derive(Debug)]
pub enum Request {
    Produce(ProduceRequest),
    Fetch(FetchRequest),
}

#[derive(Debug)]
pub struct ProduceRequest {
    pub topic: String,
    pub partition: u16,
    pub records: Vec<Record>,
}

#[derive(Debug)]
pub struct FetchRequest {
    pub topic: String,
    pub partition: u16,
    pub offset: i64,
    pub max_bytes: u32,
}

#[derive(Debug)]
pub enum Response {
    Produce(ProduceResponse),
    Fetch(FetchResponse),
    Error { message: String },
}

#[derive(Debug)]
pub struct ProduceResponse {
    pub status: u8,
    pub base_offset: i64,
}

#[derive(Debug)]
pub struct FetchResponse {
    pub status: u8,
    pub items: Vec<(i64, Record)>,
}
