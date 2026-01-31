use bytes::{Buf, BufMut, Bytes, BytesMut};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtoError {
    #[error("unexpected EOF")]
    Eof,
    #[error("invalid api key: {0}")]
    InvalidApiKey(u8),
    #[error("string too long")]
    StringTooLong,
}

// ---------- helpers: read primitives ----------
fn need(b: &dyn Buf, n: usize) -> Result<(), ProtoError> {
    if b.remaining() < n {
        Err(ProtoError::Eof)
    } else {
        Ok(())
    }
}
fn get_u8(b: &mut dyn Buf) -> Result<u8, ProtoError> {
    need(b, 1)?;
    Ok(b.get_u8())
}
fn get_u16(b: &mut dyn Buf) -> Result<u16, ProtoError> {
    need(b, 2)?;
    Ok(b.get_u16())
}
fn get_u32(b: &mut dyn Buf) -> Result<u32, ProtoError> {
    need(b, 4)?;
    Ok(b.get_u32())
}
fn get_i64(b: &mut dyn Buf) -> Result<i64, ProtoError> {
    need(b, 8)?;
    Ok(b.get_i64())
}

fn put_str(out: &mut BytesMut, s: &str) -> Result<(), ProtoError> {
    let bytes = s.as_bytes();
    if bytes.len() > u16::MAX as usize {
        return Err(ProtoError::StringTooLong);
    }
    out.put_u16(bytes.len() as u16);
    out.put_slice(bytes);
    Ok(())
}

fn get_str(b: &mut dyn Buf) -> Result<String, ProtoError> {
    let len = get_u16(b)? as usize;
    need(b, len)?;
    let mut v = vec![0u8; len];
    b.copy_to_slice(&mut v);
    Ok(String::from_utf8_lossy(&v).to_string())
}

// ---------- domain types ----------
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

// ---------- decode / encode ----------
pub fn decode_request(payload: Bytes) -> Result<Request, ProtoError> {
    let mut b = payload;
    let api = get_u8(&mut b)?;
    match api {
        1 => {
            let topic = get_str(&mut b)?;
            let partition = get_u16(&mut b)?;
            let count = get_u16(&mut b)? as usize;

            let mut records = Vec::with_capacity(count);
            for _ in 0..count {
                let klen = get_u16(&mut b)? as usize;
                need(&b, klen)?;
                let key = b.copy_to_bytes(klen);

                let vlen = get_u32(&mut b)? as usize;
                need(&b, vlen)?;
                let value = b.copy_to_bytes(vlen);

                records.push(Record { key, value });
            }

            Ok(Request::Produce(ProduceRequest {
                topic,
                partition,
                records,
            }))
        }
        2 => {
            let topic = get_str(&mut b)?;
            let partition = get_u16(&mut b)?;
            let offset = get_i64(&mut b)?;
            let max_bytes = get_u32(&mut b)?;
            Ok(Request::Fetch(FetchRequest {
                topic,
                partition,
                offset,
                max_bytes,
            }))
        }
        x => Err(ProtoError::InvalidApiKey(x)),
    }
}

pub fn encode_response(resp: Response) -> Result<Bytes, ProtoError> {
    let mut out = BytesMut::with_capacity(256);

    match resp {
        Response::Produce(r) => {
            out.put_u8(1);
            out.put_u8(r.status);
            out.put_i64(r.base_offset);
        }
        Response::Fetch(r) => {
            out.put_u8(2);
            out.put_u8(r.status);
            out.put_u16(r.items.len() as u16);
            for (offset, rec) in r.items {
                out.put_i64(offset);

                out.put_u16(rec.key.len() as u16);
                out.put_slice(&rec.key);

                out.put_u32(rec.value.len() as u32);
                out.put_slice(&rec.value);
            }
        }
        Response::Error { message } => {
            out.put_u8(255);
            put_str(&mut out, &message)?;
        }
    }

    Ok(out.freeze())
}
