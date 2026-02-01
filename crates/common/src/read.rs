use bytes::{Buf, Bytes};

use crate::error::IoError;

pub fn read_api_key(buf: &mut dyn Buf) -> Result<u8, IoError> {
    read_u8(buf)
}

pub fn read_topic(buf: &mut dyn Buf) -> Result<String, IoError> {
    read_str(buf)
}

pub fn read_partition(buf: &mut dyn Buf) -> Result<u16, IoError> {
    read_u16(buf)
}

pub fn read_record_count(buf: &mut dyn Buf) -> Result<u16, IoError> {
    read_u16(buf)
}

pub fn read_record(buf: &mut dyn Buf) -> Result<(Bytes, Bytes), IoError> {
    let key = read_key(buf)?;
    let value = read_value(buf)?;
    Ok((key, value))
}

pub fn read_key(buf: &mut dyn Buf) -> Result<Bytes, IoError> {
    let klen = read_u16(buf)? as usize;
    ensure_remaining(buf, klen)?;
    Ok(buf.copy_to_bytes(klen))
}

pub fn read_value(buf: &mut dyn Buf) -> Result<Bytes, IoError> {
    let vlen = read_u32(buf)? as usize;
    ensure_remaining(buf, vlen)?;
    Ok(buf.copy_to_bytes(vlen))
}

pub fn read_offset(buf: &mut dyn Buf) -> Result<i64, IoError> {
    read_i64(buf)
}

pub fn read_max_bytes(buf: &mut dyn Buf) -> Result<u32, IoError> {
    read_u32(buf)
}

pub fn ensure_remaining(buf: &dyn Buf, n: usize) -> Result<(), IoError> {
    if buf.remaining() < n {
        Err(IoError::Eof)
    } else {
        Ok(())
    }
}

pub fn read_u8(buf: &mut dyn Buf) -> Result<u8, IoError> {
    ensure_remaining(buf, 1)?;
    Ok(buf.get_u8())
}

pub fn read_u16(buf: &mut dyn Buf) -> Result<u16, IoError> {
    ensure_remaining(buf, 2)?;
    Ok(buf.get_u16())
}

pub fn read_u32(buf: &mut dyn Buf) -> Result<u32, IoError> {
    ensure_remaining(buf, 4)?;
    Ok(buf.get_u32())
}

pub fn read_i64(buf: &mut dyn Buf) -> Result<i64, IoError> {
    ensure_remaining(buf, 8)?;
    Ok(buf.get_i64())
}

pub fn read_str(buf: &mut dyn Buf) -> Result<String, IoError> {
    let len = read_u16(buf)? as usize;
    ensure_remaining(buf, len)?;
    let mut v = vec![0u8; len];
    buf.copy_to_slice(&mut v);
    Ok(String::from_utf8_lossy(&v).to_string())
}

pub fn read_bytes(buf: &mut dyn Buf) -> Result<Vec<u8>, IoError> {
    let len = read_u32(buf)? as usize;
    ensure_remaining(buf, len)?;
    let mut v = vec![0u8; len];
    buf.copy_to_slice(&mut v);
    Ok(v)
}
