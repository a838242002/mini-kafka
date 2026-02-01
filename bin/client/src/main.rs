use bytes::{Buf, BufMut, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut sock = TcpStream::connect("127.0.0.1:9092").await?;

    // --- Produce request (api_key=1) ---
    let produce_payload = build_produce("test", 0, vec![("k1", "v1"), ("k2", "v2")]);
    write_frame(&mut sock, &produce_payload).await?;
    let resp1 = read_frame(&mut sock).await?;
    println!("produce resp bytes ={}", hex_preview(&resp1));

    // --- Fetch request (api_key=2) ---
    let fetch_payload = build_fetch("test", 0, 0, 1024 * 1024);
    write_frame(&mut sock, &fetch_payload).await?;
    let resp2 = read_frame(&mut sock).await?;
    println!("fetch resp bytes len={}", hex_preview(&resp2));

    Ok(())
}

fn build_produce(topic: &str, partition: u16, kvs: Vec<(&str, &str)>) -> bytes::Bytes {
    let mut out = BytesMut::with_capacity(256);
    common::write_api_key(&mut out, 1);
    common::write_topic(&mut out, topic);
    common::write_partition(&mut out, partition);
    common::write_record_count(&mut out, kvs.len() as u16);
    for (k, v) in kvs {
        common::write_record(&mut out, k, v);
    }
    out.freeze()
}

fn build_fetch(topic: &str, partition: u16, offset: i64, max_bytes: u32) -> bytes::Bytes {
    let mut out = BytesMut::with_capacity(64);
    common::write_api_key(&mut out, 2);
    common::write_topic(&mut out, topic);
    common::write_partition(&mut out, partition);
    common::write_offset(&mut out, offset);
    common::write_max_bytes(&mut out, max_bytes);
    out.freeze()
}

async fn write_frame(sock: &mut TcpStream, payload: &bytes::Bytes) -> std::io::Result<()> {
    let mut frame = BytesMut::with_capacity(4 + payload.len());
    frame.put_u32(payload.len() as u32);
    frame.put_slice(payload);
    sock.write_all(&frame).await
}

async fn read_frame(sock: &mut TcpStream) -> std::io::Result<bytes::Bytes> {
    let mut len_buf = [0u8; 4];
    sock.read_exact(&mut len_buf).await?;
    let mut cur = std::io::Cursor::new(len_buf);
    let len = cur.get_u32() as usize;

    let mut payload = vec![0u8; len];
    sock.read_exact(&mut payload).await?;
    Ok(payload.into())
}

fn hex_preview(b: &bytes::Bytes) -> String {
    let n = b.len().min(64);
    let mut s = String::new();
    for byte in &b[..n] {
        s.push_str(&format!("{:02x} ", byte));
    }

    if b.len() > n {
        s.push_str("...");
    }

    s
}
