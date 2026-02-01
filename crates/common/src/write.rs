use bytes::{BufMut, Bytes, BytesMut};

use crate::error::IoError;

pub fn write_str(out: &mut BytesMut, s: &str) -> Result<(), IoError> {
    let bytes = s.as_bytes();
    if bytes.len() > u16::MAX as usize {
        return Err(IoError::StringTooLong);
    }
    out.put_u16(bytes.len() as u16);
    out.put_slice(bytes);
    Ok(())
}

pub fn write_api_key(buf: &mut BytesMut, key: u8) {
    buf.put_u8(key);
}

pub fn write_topic(buf: &mut BytesMut, topic: &str) {
    buf.put_u16(topic.len() as u16);
    buf.put_slice(topic.as_bytes());
}

pub fn write_partition(buf: &mut BytesMut, partition: u16) {
    buf.put_u16(partition);
}

pub fn write_record_count(buf: &mut BytesMut, count: u16) {
    buf.put_u16(count);
}

pub fn write_record(buf: &mut BytesMut, key: &str, value: &str) {
    write_key(buf, key);
    write_value(buf, value);
}

pub fn write_record_bytes(buf: &mut BytesMut, key: &Bytes, value: &Bytes) {
    buf.put_u16(key.len() as u16);
    buf.put_slice(key);
    buf.put_u32(value.len() as u32);
    buf.put_slice(value);
}

pub fn write_key(buf: &mut BytesMut, key: &str) {
    buf.put_u16(key.len() as u16);
    buf.put_slice(key.as_bytes());
}

pub fn write_value(buf: &mut BytesMut, value: &str) {
    buf.put_u32(value.len() as u32);
    buf.put_slice(value.as_bytes());
}

pub fn write_offset(buf: &mut BytesMut, offset: i64) {
    buf.put_i64(offset);
}

pub fn write_max_bytes(buf: &mut BytesMut, max_bytes: u32) {
    buf.put_u32(max_bytes);
}

pub fn write_status(buf: &mut BytesMut, status: u8) {
    buf.put_u8(status);
}
