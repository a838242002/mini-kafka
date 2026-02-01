use bytes::{Buf, Bytes, BytesMut};

use common::{IoError, read_api_key, read_partition, read_record, read_record_count, read_str, read_topic, write_api_key, write_partition, write_record_bytes, write_record_count, write_topic};

#[test]
fn round_trip_basic_fields() {
    let mut out = BytesMut::with_capacity(128);
    write_api_key(&mut out, 1);
    write_topic(&mut out, "topic");
    write_partition(&mut out, 3);
    write_record_count(&mut out, 2);

    let mut buf = Bytes::from(out.freeze());
    assert_eq!(read_api_key(&mut buf).unwrap(), 1);
    assert_eq!(read_topic(&mut buf).unwrap(), "topic");
    assert_eq!(read_partition(&mut buf).unwrap(), 3);
    assert_eq!(read_record_count(&mut buf).unwrap(), 2);
    assert_eq!(buf.remaining(), 0);
}

#[test]
fn round_trip_record_bytes() {
    let mut out = BytesMut::with_capacity(64);
    let key = Bytes::from_static(b"k");
    let value = Bytes::from_static(b"val");
    write_record_bytes(&mut out, &key, &value);

    let mut buf = Bytes::from(out.freeze());
    let (read_key, read_value) = read_record(&mut buf).unwrap();
    assert_eq!(read_key, key);
    assert_eq!(read_value, value);
    assert_eq!(buf.remaining(), 0);
}

#[test]
fn read_str_returns_eof_on_short_buffer() {
    let mut buf = Bytes::from_static(&[0x00]);
    let err = read_str(&mut buf).unwrap_err();
    match err {
        IoError::Eof => {}
        _ => panic!("expected IoError::Eof"),
    }
}
