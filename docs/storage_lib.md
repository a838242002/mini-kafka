# crates/storage/src/lib.rs 詳細解釋

以下說明針對 `crates/storage/src/lib.rs`，聚焦在目前的 `PartitionLog` 結構與其行為。

## PartitionLog 概覽

`PartitionLog` 是單一 topic-partition 的 append-only log 檔案抽象。它負責：

- 打開或建立對應的 `.log` 檔案
- 掃描既有檔案內容，建立 offset -> 檔案位置的索引
- 計算下一筆可用的 offset

### 資料欄位

`PartitionLog` 目前包含：

- `path: PathBuf`：log 檔案的完整路徑（例如 `<dir>/<topic>-<partition>.log`）。
- `file: File`：用於 append 的檔案 handle（read + append）。
- `next_offset: i64`：下一筆可寫入的 offset（由掃描結果決定）。
- `index: BTreeMap<i64, u64>`：offset -> 檔案位置（byte offset）的索引。

### 檔案格式

註解標示的格式如下：

```
[offset:i64][klen:u16][key bytes][vlen:u32][value bytes]
```

每筆 record 順序寫入，沒有額外的長度前綴或校驗。

## open()

`PartitionLog::open(dir, topic, partition)` 的流程：

1. `create_dir_all(dir)`：確保目錄存在
2. 建立檔名 `<topic>-<partition>.log` 並以 `create + read + append` 打開
3. 另外用 `read` 打開一次進行掃描
4. 透過 `scan_build_index` 建索引與 `next_offset`
5. 將 append 檔案指標移到檔尾（`SeekFrom::End(0)`）

結果回傳已就緒的 `PartitionLog`。

### 更細節的重點

- 使用兩個 `File` handle：一個專門掃描、另一個保留做 append，避免掃描後還要重設同一個 handle 的游標位置。
- `OpenOptions` 的 `.append(true)` 會讓寫入永遠在檔尾（即使手動 seek 也無法在中間寫入）。
- 先掃描再 `SeekFrom::End(0)`，確保後續 append 不會覆蓋既有資料。

## scan_build_index()

`scan_build_index(f)` 會完整掃描檔案內容，建立索引與下一筆 offset：

- 先把整個檔案讀入記憶體 `buf`
- 用 `Cursor` 逐筆解析
- 每次迴圈：
  - 記錄目前位置 `pos`
  - 讀取 `offset`、`key` 長度與內容、`value` 長度與內容
  - 將 `offset -> pos` 放入 `index`
  - 將 `next_offset` 設為 `offset + 1`
- 任一段不足長度就回傳 `StorageError::Corrupted`

### 更細節的重點

- `pos` 是該筆 record 的起始 byte 位置，用於之後快速 seek 到該 offset。
- 讀取流程完全依照檔案格式順序推進，沒有額外的 record header 或 checksum。
- `next_offset` 以「最後一筆 offset + 1」推進；如果檔案是空的，就保持 0。
- 整個檔案一次載入到 `buf`，簡化解析但可能在大檔案時耗記憶體。

### 檔案損毀（Corrupted）判斷例子

`scan_build_index` 在每次讀取固定長度欄位前，都會檢查剩餘長度是否足夠：

- 剩餘不足 8 bytes：無法讀 `offset:i64`
- 讀完 `offset` 後，剩餘不足 2 bytes：無法讀 `klen:u16`
- 讀完 `klen` 後，剩餘不足 `klen`：key 資料不完整
- 讀完 key 後，剩餘不足 4 bytes：無法讀 `vlen:u32`
- 讀完 `vlen` 後，剩餘不足 `vlen`：value 資料不完整

只要任一檢查失敗，就回傳 `StorageError::Corrupted`，表示檔案可能被截斷或內容不完整。

### Byte offset 示意圖（單筆 record）

```
pos -> [offset:i64][klen:u16][key bytes][vlen:u32][value bytes]
       |<---8--->|<--2-->|<-klen->|<--4-->|<-vlen-->|
```

- `pos` 是這筆 record 在檔案中的起始 byte 位置。
- `index` 會把 `offset -> pos` 存起來，方便之後 seek 到該筆資料起點。

### 多筆 record 串接示意

```
pos0 -> [rec0 ...][rec1 ...][rec2 ...] ...
          ^          ^          ^
          |          |          |
       index[0]   index[1]   index[2]
```

- 每一筆 record 緊接在上一筆後面，沒有額外的分隔符。
- `scan_build_index` 會逐筆計算起始位置，建立 offset 索引。

### 邊界與限制

- 索引只在記憶體中，重新開啟會重新掃描。
- 使用 `BTreeMap` 方便依 offset 有序查詢。
- 目前尚未提供寫入/讀取 API，僅建立結構與索引掃描。

## fetch()

`PartitionLog::fetch(offset, max_bytes)` 會從指定的 `offset` 開始讀取資料，回傳多筆 `(offset, Record)`：

- 先用 `index.range(offset..).next()` 找到第一個 `>= offset` 的記錄位置並 `seek` 過去。
- 用 `max_bytes` 當作讀取上限，逐筆解析 `[offset][klen][key][vlen][value]`。
- 若剩餘 `max_bytes` 不足以完整讀完下一段，就停止，避免回傳半筆 record。

### `key.into()` / `value.into()` 的意義

`Record` 的欄位型別不是固定的 `Vec<u8>`，所以在建立 `Record` 時會用 `into()` 進行型別轉換：

```rust
Record {
    key: key.into(),
    value: value.into(),
}
```

- `into()` 會消耗 `Vec<u8>`，轉成 `Record` 欄位宣告的型別（例如 `Bytes` 或 `Vec<u8>`）。
- 若有 `From<Vec<u8>>` 的實作，這個轉換通常是零拷貝或最少拷貝的。
