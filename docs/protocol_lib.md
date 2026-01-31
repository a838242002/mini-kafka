# crates/protocol/src/lib.rs 詳細解釋

以下說明針對 `crates/protocol/src/lib.rs`，逐段解釋其角色、資料格式、與編解碼流程。

## 依賴與錯誤型別

- `bytes::{Buf, BufMut, Bytes, BytesMut}`：用來做二進位緩衝區的讀寫與複製。
- `thiserror::Error`：提供簡潔的錯誤定義與顯示。

### bytes crate 型別與 trait 說明

- `Buf`：讀取介面（trait）。提供 `remaining()`、`get_u8/get_u16/get_u32/get_i64` 等方法，從目前讀取位置往後「消耗」資料。
- `BufMut`：寫入介面（trait）。提供 `put_u8/put_u16/put_u32/put_i64`、`put_slice` 等方法，將資料追加到緩衝區。
- `Bytes`：不可變、可共享的 bytes 緩衝區。適合拿來做已完成的 payload 或 record 欄位；切片/複製成本低。
- `BytesMut`：可變的 bytes 緩衝區。適合編碼/組裝封包時使用，完成後可 `freeze()` 轉成 `Bytes`。

### `dyn` 與 `&mut dyn Buf`

- `dyn Trait` 表示 trait object（動態分派）。編譯期不知道實際型別，執行期透過 vtable 呼叫方法。
- `&mut dyn Buf` 表示「可變的 Buf trait object 參考」：函式可以接受任何實作了 `Buf` 的型別，且會消耗讀取位置。
- 這裡用 `dyn` 可以讓 `get_u8/get_u16/...` 等 helper 同時適用於 `Bytes`、`BytesMut`、`Cursor<Vec<u8>>` 等不同 buffer。

#### 動態分派 vs 泛型（簡短對照）

```
// 泛型：編譯期決定型別，靜態分派
fn read_u8<T: Buf>(b: &mut T) -> Result<u8, ProtoError> {
    need(b, 1)?;
    Ok(b.get_u8())
}

// trait object：執行期決定型別，動態分派
fn read_u8_dyn(b: &mut dyn Buf) -> Result<u8, ProtoError> {
    need(b, 1)?;
    Ok(b.get_u8())
}
```

- 泛型：零動態分派成本，但可能造成程式碼膨脹（monomorphization）。
- `dyn`：可以在執行期統一處理多種實作，代價是一次 vtable 呼叫。

`ProtoError` 定義了協定解析時可能遇到的錯誤：

- `Eof`：可讀資料不足（緩衝區剩餘長度不夠）。
- `InvalidApiKey(u8)`：未知的 API key。
- `StringTooLong`：字串長度超過 `u16::MAX` 可表示的範圍。

## 基礎讀寫工具（helpers）

這段是協定的原始讀寫工具，全部都針對 bytes 緩衝區操作。

- `need(b, n)`：檢查 `Buf` 剩餘長度是否 >= `n`，不足就回傳 `Eof`。
- `get_u8 / get_u16 / get_u32 / get_i64`：先檢查，再從 `Buf` 讀取對應型別。
- `put_str(out, s)`：
  - 先將字串長度寫入為 `u16`
  - 再寫入字串 bytes
  - 若字串長度超過 `u16::MAX`，回傳 `StringTooLong`
- `get_str(b)`：
  - 先讀取 `u16` 長度
  - 再依長度取出 bytes
  - 使用 `from_utf8_lossy` 轉為 `String`（遇到非法 UTF-8 會以替代字元處理）

## 協定領域模型（domain types）

這些 struct/enum 定義了邏輯層面的請求/回應結構。

### Record

- `Record { key: Bytes, value: Bytes }`
- 用 `Bytes` 表示不可變的二進位資料片段，方便複製與切片共享。

### Request

- `Produce(ProduceRequest)`：寫入資料
- `Fetch(FetchRequest)`：讀取資料

### ProduceRequest

- `topic: String`
- `partition: u16`
- `records: Vec<Record>`

### FetchRequest

- `topic: String`
- `partition: u16`
- `offset: i64`
- `max_bytes: u32`

### Response

- `Produce(ProduceResponse)`：寫入回應
- `Fetch(FetchResponse)`：讀取回應
- `Error { message: String }`：錯誤回應

### ProduceResponse

- `status: u8`：狀態碼
- `base_offset: i64`：第一筆 record 的 offset

### FetchResponse

- `status: u8`
- `items: Vec<(i64, Record)>`：每筆資料含 `(offset, Record)`

## 二進位協定：Request 解碼

`decode_request(payload: Bytes)` 根據第一個位元組（API key）決定解析方式。

### 共同格式

```
u8   api_key
...  依 api_key 分支解碼
```

### Produce (api_key = 1)

格式如下：

```
u8    api_key = 1
u16   topic_len
bytes topic (topic_len)
u16   partition
u16   record_count
loop record_count:
  u16   key_len
  bytes key (key_len)
  u32   value_len
  bytes value (value_len)
```

解析流程：

- 讀 `topic`（以 `get_str`）
- 讀 `partition`
- 讀 `record_count`
- 依 `record_count` 逐筆讀 key/value

### Fetch (api_key = 2)

格式如下：

```
u8   api_key = 2
u16  topic_len
bytes topic (topic_len)
u16  partition
i64  offset
u32  max_bytes
```

若 api_key 非 1 或 2，回傳 `InvalidApiKey`.

## 二進位協定：Response 編碼

`encode_response(resp: Response)` 會依回應型別寫入 bytes。

### ProduceResponse

```
u8   api_key = 1
u8   status
i64  base_offset
```

### FetchResponse

```
u8   api_key = 2
u8   status
u16  item_count
loop item_count:
  i64  offset
  u16  key_len
  bytes key
  u32  value_len
  bytes value
```

### Error Response

```
u8   api_key = 255
u16  message_len
bytes message
```

這裡使用 `put_str` 來寫入錯誤訊息。

## 重要細節與限制

- 所有數值型別都使用 `bytes` crate 的預設 byte order（Big Endian）。
- `topic` 與 `Error` 的訊息長度皆以 `u16` 表示，最大 65535 bytes。
- `Record.key` 長度用 `u16`，`Record.value` 長度用 `u32`。
- `get_str` 使用 `from_utf8_lossy`，遇到非法 UTF-8 不會報錯但會替代字元。
- `encode_response` 沒有檢查 key/value 長度是否超過 `u16/u32`，這由呼叫端保證。

## 欄位對照表（Fields Table）

以下整理 Request / Response 的欄位名稱、型別與位元組長度（固定長度以 bytes 表示，變動長度用「len + data」表示）。

### Produce Request (api_key = 1)

| 欄位 | 型別 | 長度 |
| --- | --- | --- |
| api_key | u8 | 1 |
| topic_len | u16 | 2 |
| topic | bytes | topic_len |
| partition | u16 | 2 |
| record_count | u16 | 2 |
| key_len | u16 | 2 |
| key | bytes | key_len |
| value_len | u32 | 4 |
| value | bytes | value_len |

### Fetch Request (api_key = 2)

| 欄位 | 型別 | 長度 |
| --- | --- | --- |
| api_key | u8 | 1 |
| topic_len | u16 | 2 |
| topic | bytes | topic_len |
| partition | u16 | 2 |
| offset | i64 | 8 |
| max_bytes | u32 | 4 |

### Produce Response (api_key = 1)

| 欄位 | 型別 | 長度 |
| --- | --- | --- |
| api_key | u8 | 1 |
| status | u8 | 1 |
| base_offset | i64 | 8 |

### Fetch Response (api_key = 2)

| 欄位 | 型別 | 長度 |
| --- | --- | --- |
| api_key | u8 | 1 |
| status | u8 | 1 |
| item_count | u16 | 2 |
| offset | i64 | 8 |
| key_len | u16 | 2 |
| key | bytes | key_len |
| value_len | u32 | 4 |
| value | bytes | value_len |

### Error Response (api_key = 255)

| 欄位 | 型別 | 長度 |
| --- | --- | --- |
| api_key | u8 | 1 |
| message_len | u16 | 2 |
| message | bytes | message_len |

## Rust 產生封包範例

以下範例示範如何建立一個 `Produce` 回應的 bytes（不含網路層封包長度）。

```rust
use bytes::{BufMut, Bytes, BytesMut};

fn encode_produce_response(status: u8, base_offset: i64) -> Bytes {
    let mut out = BytesMut::with_capacity(1 + 1 + 8);
    out.put_u8(1);          // api_key
    out.put_u8(status);     // status
    out.put_i64(base_offset);
    out.freeze()
}
```

以下是產生 `Error` 回應的範例：

```rust
use bytes::{BufMut, Bytes, BytesMut};

fn encode_error_response(message: &str) -> Bytes {
    let bytes = message.as_bytes();
    let mut out = BytesMut::with_capacity(1 + 2 + bytes.len());
    out.put_u8(255);                    // api_key
    out.put_u16(bytes.len() as u16);    // message_len
    out.put_slice(bytes);               // message
    out.freeze()
}
```

## 小結

這個 `protocol` crate 主要做兩件事：

1. 定義 Kafka-like 簡化協定的資料結構
2. 將結構與 bytes 緩衝區互相轉換（解碼 Request、編碼 Response）

它是 broker 與 client 溝通的核心約定，任何格式變更都必須同步更新編解碼邏輯。

## 範例封包（Hex）

以下範例皆以 Big Endian 表示，十六進位為逐 byte 顯示。

### 範例 1：Produce Request

假設：

- api_key = 1
- topic = "test" (len = 4)
- partition = 2
- record_count = 1
- record:
  - key = "k1" (len = 2)
  - value = "v123" (len = 4)

對應 bytes：

```
01                                  # api_key
00 04 74 65 73 74                   # topic_len=4, "test"
00 02                               # partition=2
00 01                               # record_count=1
00 02 6b 31                         # key_len=2, "k1"
00 00 00 04 76 31 32 33              # value_len=4, "v123"
```

### 範例 2：Fetch Response

假設：

- api_key = 2
- status = 0
- item_count = 1
- item:
  - offset = 42
  - key = "k"
  - value = "hello"

對應 bytes：

```
02                                  # api_key
00                                  # status
00 01                               # item_count=1
00 00 00 00 00 00 00 2a              # offset=42
00 01 6b                            # key_len=1, "k"
00 00 00 05 68 65 6c 6c 6f           # value_len=5, "hello"
```

### 範例 3：Error Response

假設：

- api_key = 255
- message = "bad"

對應 bytes：

```
ff                                  # api_key
00 03 62 61 64                      # message_len=3, "bad"
```
