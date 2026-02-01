use bytes::{BufMut, Bytes, BytesMut};

pub mod error;
pub mod types;
use types::*;

use crate::error::ProtoError;

// ---------- decode / encode ----------
pub fn decode_request(payload: Bytes) -> Result<Request, ProtoError> {
    let mut b = payload;
    let api = common::read_api_key(&mut b)?;
    match ApiKey::try_from(api) {
        Ok(ApiKey::Produce) => decode_produce_request(b),
        Ok(ApiKey::Fetch) => decode_fetch_request(b),
        Err(x) => Err(ProtoError::InvalidApiKey(x)),
    }
}

fn decode_produce_request(payload: Bytes) -> Result<Request, ProtoError> {
    let mut p = payload;
    let topic = common::read_topic(&mut p)?;
    let partition = common::read_partition(&mut p)?;
    let count = common::read_record_count(&mut p)? as usize;

    let mut records = Vec::with_capacity(count);
    for _ in 0..count {
        let (key, value) = common::read_record(&mut p)?;

        records.push(Record { key, value });
    }

    Ok(Request::Produce(ProduceRequest {
        topic,
        partition,
        records,
    }))
}

fn decode_fetch_request(payload: Bytes) -> Result<Request, ProtoError> {
    let mut p = payload;
    let topic = common::read_topic(&mut p)?;
    let partition = common::read_partition(&mut p)?;
    let offset = common::read_offset(&mut p)?;
    let max_bytes = common::read_max_bytes(&mut p)?;
    Ok(Request::Fetch(FetchRequest {
        topic,
        partition,
        offset,
        max_bytes,
    }))
}

pub fn encode_response(resp: Response) -> Result<Bytes, ProtoError> {
    let mut out = BytesMut::with_capacity(256);

    match resp {
        Response::Produce(r) => {
            common::write_api_key(&mut out, 1);
            common::write_status(&mut out, r.status);
            out.put_i64(r.base_offset);
        }
        Response::Fetch(r) => {
            common::write_api_key(&mut out, 2);
            common::write_status(&mut out, r.status);
            common::write_record_count(&mut out, r.items.len() as u16);
            for (offset, rec) in r.items {
                common::write_offset(&mut out, offset);
                common::write_record_bytes(&mut out, &rec.key, &rec.value);
            }
        }
        Response::Error { message } => {
            out.put_u8(255);
            common::write_str(&mut out, &message)?;
        }
    }

    Ok(out.freeze())
}
