# crates/net 與 async/Future 語法整理

## crates/net 架構
- `crates/net/src/lib.rs` 是唯一原始碼檔案。
- 對外 API：`pub async fn serve_hello(addr: &str) -> std::io::Result<()>`。
- 內部私有函式：`async fn handle_conn(sock: TcpStream) -> std::io::Result<()>`。
- 功能：建立 TCP 伺服器、接受連線、回傳 greeting 並 echo。

## `move` 的用法與原理
- `move` 會把外部變數的所有權移進 closure（包含 `async move { ... }`）。
- 目的：讓被 spawn 的任務擁有資料，避免引用到已結束的外部作用域。
- 沒有 `move` 時，closure 可能只借用外部變數，無法滿足 `'static` 需求。

範例：
```rust
tokio::spawn(async move {
    // sock 被移入這個任務
    if let Err(e) = handle_conn(sock).await {
        eprint!("conn error: {}", e);
    }
});
```

## `async` 與 `Future`
- `async fn` 呼叫後回傳 `Future`，不是立即執行完成。
- `Future` 是可被反覆 `poll` 的狀態機：
  - `Poll::Ready(val)` 表示完成。
  - `Poll::Pending` 表示未完成，稍後再試。
- `await` 會暫停當前 async 任務並把控制權交回 runtime，不會阻塞執行緒。

## `await` 與同步阻塞的差異
- 阻塞 I/O：卡住整條執行緒。
- `await`：只暫停當前 Future，執行緒可去跑其他任務。

## `.await?` 的意思
- `await` 取得結果，`?` 在 `Err` 時直接回傳。
- 等價：
```rust
let v = match foo().await {
    Ok(v) => v,
    Err(e) => return Err(e),
};
```

## `serve_hello` 的 `poll` 擬碼（概念）
```rust
State::Bind
  -> poll(bind)
     - Pending: return Pending
     - Ready(Ok(listener)) -> State::Accept
     - Ready(Err(e)) -> return Err

State::Accept
  -> poll(accept)
     - Pending: return Pending
     - Ready(Ok((sock, peer))) -> spawn(handle_conn); stay in Accept
     - Ready(Err(e)) -> return Err
```

## `handle_conn` 的 `poll` 擬碼（概念）
```rust
State::WriteGreeting
  -> poll(write_all("hello"))
     - Pending
     - Ready -> State::Read

State::Read
  -> poll(read)
     - Pending
     - Ready(0) -> Done
     - Ready(n) -> State::WriteEchoPrefix

State::WriteEchoPrefix
  -> poll(write_all("echo: "))
     - Pending
     - Ready -> State::WriteEchoBody

State::WriteEchoBody
  -> poll(write_all(buf[..n]))
     - Pending
     - Ready -> State::Read
```

## `serve_hello` 與 `handle_conn` 的互動圖
```
serve_hello
  bind -> accept loop
     |-- spawn(handle_conn)

handle_conn
  write greeting -> read -> write echo -> read -> ...
```
