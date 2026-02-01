#[cfg(test)]
mod tests {
    use bytes::{BufMut, BytesMut};
    use protocol::decode_request;
    use protocol::types::Request;

    #[test]
    fn decode_produce_request_of() {
        let mut p = BytesMut::new();
        p.put_u8(1);
        p.put_u16(4);
        p.put_slice(b"test");
        p.put_u16(0); // partition
        p.put_u16(2); // records_count

        // record1: k="k1", v="v1"
        p.put_u16(2);
        p.put_slice(b"k1");
        p.put_u32(2);
        p.put_slice(b"v1");

        // record2: k="", v="hello"
        p.put_u16(0);
        p.put_u32(5);
        p.put_slice(b"hello");

        let req = decode_request(p.freeze()).unwrap();
        match req {
            Request::Produce(r) => {
                assert_eq!(r.topic, "test");
                assert_eq!(r.partition, 0);
                assert_eq!(r.records.len(), 2);
                assert_eq!(&r.records[0].key[..], b"k1");
                assert_eq!(&r.records[0].value[..], b"v1");
                assert_eq!(&r.records[1].key[..], b"");
                assert_eq!(&r.records[1].value[..], b"hello");
            }
            _ => panic!("expected Produce request"),
        }
    }

    #[test]
    fn decode_fetch_request_ok() {
        let mut p = BytesMut::new();
        p.put_u8(2);
        p.put_u16(4);
        p.put_slice(b"test");
        p.put_u16(1);
        p.put_i64(10);
        p.put_u32(1024);

        let req = decode_request(p.freeze()).unwrap();
        match req {
            Request::Fetch(r) => {
                assert_eq!(r.topic, "test");
                assert_eq!(r.partition, 1);
                assert_eq!(r.offset, 10);
                assert_eq!(r.max_bytes, 1024);
            }
            _ => panic!("expected Fetch request"),
        }
    }
}
