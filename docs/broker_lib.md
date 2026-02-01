# crates/broker/src/lib.rs 詳細解釋

以下說明聚焦在 `crates/broker/src/lib.rs` 的結構與流程，包含共享狀態管理、請求處理與回應邏輯。

## Broker 結構

`Broker` 封裝了 broker 的核心狀態與操作，負責依 request 類型存取對應的 partition log。

### 欄位

- `data_dir: PathBuf`
  - 儲存 log 檔案的根目錄。
- `partitions: Mutex<HashMap<(String, u16), PartitionLog>>`
  - 以 `(topic, partition)` 為 key 的 in-memory cache。
  - 使用 `tokio::sync::Mutex` 保護，避免並發存取造成資料競爭。
  - value 為 `PartitionLog`，代表該 partition 的 log handle。

## new()

建立新的 `Broker`，初始化 `data_dir`，並建立空的 `HashMap`。

## get_or_open()

```rust
async fn get_or_open(&self, topic: &str, partition: u16) -> Result<PartitionLog, String>
```

此函式的目的：從 cache 取出指定 partition 的 log；若不存在則建立並回傳。

流程：

1. 取得 `partitions` 的 lock。
2. `map.remove(...)` 以 key 取出 `PartitionLog`。
   - 使用 `remove` 會把 log 的所有權移出 map。
   - 這樣做讓呼叫端能以可變方式操作 log，而不需要長時間持鎖。
3. 若 map 中沒有，則呼叫 `PartitionLog::open(...)` 開檔並回傳。

注意：

- `PartitionLog::open` 會在持鎖狀態下被呼叫，若要避免鎖內 I/O，可在找不到時先 `drop(map)` 再 open。
- 返回 `PartitionLog` 的所有權，呼叫端必須在使用完後放回 cache。

## put_back()

```rust
async fn put_back(&self, topic: &str, partition: u16, log: PartitionLog)
```

將 `PartitionLog` 放回 `partitions`：

1. 取得 lock
2. 以 `(topic, partition)` 為 key 插入

這和 `get_or_open` 的 `remove` 搭配，形成「取出 -> 使用 -> 放回」的生命週期。

## handle()

```rust
pub async fn handle(&self, req: Request) -> Response
```

核心請求處理流程，依 `Request` 類型分派：

### Request::Produce

1. 透過 `get_or_open` 取得 partition log。
2. 呼叫 `log.append(&r.records)` 將 records 寫入。
3. 成功時回傳 `Response::Produce { status: 0, base_offset }`。
4. 失敗時回傳 `Response::Error { message }`。
5. 將 log 放回 cache。

### Request::Fetch

1. 透過 `get_or_open` 取得 partition log。
2. 呼叫 `log.fetch(r.offset, r.max_bytes)` 讀取資料。
3. 成功時回傳 `Response::Fetch { status: 0, items }`。
4. 失敗時回傳 `Response::Error { message }`。
5. 將 log 放回 cache。

## 併發與鎖的設計重點

- `PartitionLog` 不存放在 `Arc<Mutex<_>>` 內，而是以「取出所有權、用完放回」的方式共享。
- 優點是避免長時間持鎖，`append`/`fetch` 期間不會鎖住整個 `HashMap`。
- 代價是操作期間該 partition 不在 map 中，其他請求會觸發 `open`，可能重複開檔。

若要避免重複開檔，可考慮：

- 使用 `Arc<Mutex<PartitionLog>>` 做細粒度鎖
- 或在 map 中存放 `Arc<PartitionLog>` 並使用內部同步
