use bytes::{Buf, BufMut, BytesMut};
use protocol::{Request, Response, decode_request, encode_response};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

pub async fn serve_hello(addr: &str) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("Server listening on {}", addr);

    loop {
        let (sock, peer) = listener.accept().await?;
        println!("Accepted connection from {}", peer);

        tokio::spawn(async move {
            if let Err(e) = handle_conn(sock).await {
                eprint!("conn error: {}", e);
            }
        });
    }
}

async fn handle_conn(mut sock: TcpStream) -> std::io::Result<()> {
    loop {
        let payload = match read_frame(&mut sock).await {
            Ok(p) => p,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(e),
        };

        let resp = match decode_request(payload) {
            Ok(req) => handle_stub(req),
            Err(e) => Response::Error {
                message: format!("invalid request: {}", e),
            },
        };

        let out = encode_response(resp).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("encode error: {}", e),
            )
        })?;
        write_frame(&mut sock, &out).await?;
    }
}

fn handle_stub(req: Request) -> Response {
    match req {
        Request::Produce(_r) => Response::Produce(protocol::ProduceResponse {
            status: 0,
            base_offset: 0,
        }),
        Request::Fetch(_r) => Response::Fetch(protocol::FetchResponse {
            status: 0,
            items: vec![],
        }),
    }
}

/// Frame format: u32(len, bit-endian) + [len bytes of payload]
async fn read_frame(sock: &mut TcpStream) -> std::io::Result<bytes::Bytes> {
    let mut len_buf = [0u8; 4];
    sock.read_exact(&mut len_buf).await?;
    let mut cur = std::io::Cursor::new(len_buf);
    let len = cur.get_u32() as usize;

    const MAX_FRAME_SIZE: usize = 8 * 1024 * 1024; // 8 MB
    if len > MAX_FRAME_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("frame size {} exceeds maximum {}", len, MAX_FRAME_SIZE),
        ));
    }

    let mut payload = vec![0u8; len];
    sock.read_exact(&mut payload).await?;
    Ok(payload.into())
}

async fn write_frame(sock: &mut TcpStream, payload: &bytes::Bytes) -> std::io::Result<()> {
    let mut frame = BytesMut::with_capacity(4 + payload.len());
    frame.put_u32(payload.len() as u32);
    frame.put_slice(payload);
    sock.write_all(&frame).await
}
