use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use bytes::{Buf, BufMut, BytesMut};
use protocol::Record;

#[derive(thiserror::Error, Debug)]
pub enum StorageError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("corrupted log")]
    Corrupted,
}

// [offset:i64][klen:u16][key bytes][vlen:u32][value bytes]
#[derive(Debug)]
#[allow(dead_code)]
pub struct PartitionLog {
    path: PathBuf,
    file: File,
    next_offset: i64,
    index: BTreeMap<i64, u64>,
}

impl PartitionLog {
    pub fn open(dir: &Path, topic: &str, partition: u16) -> Result<Self, StorageError> {
        std::fs::create_dir_all(dir)?;
        let filename = format!("{topic}-{partition}.log");
        let path = dir.join(filename);

        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&path)?;

        let mut scan = OpenOptions::new().read(true).open(&path)?;
        let (index, next_offset) = Self::scan_build_index(&mut scan)?;

        let _ = file.seek(SeekFrom::End(0));

        Ok(Self {
            path,
            file,
            next_offset,
            index,
        })
    }

    fn scan_build_index(f: &mut File) -> Result<(BTreeMap<i64, u64>, i64), StorageError> {
        let mut index = BTreeMap::new();
        let _ = f.seek(SeekFrom::Start(0));

        let mut buf = Vec::new();
        let _ = f.read_to_end(&mut buf);

        let mut cur = std::io::Cursor::new(&buf);
        let mut next_offset = 0i64;

        while (cur.position() as usize) < buf.len() {
            let pos = cur.position() as u64;

            if buf.len() - (cur.position() as usize) < 8 {
                return Err(StorageError::Corrupted);
            }

            // read offset value
            let offset = cur.get_i64();
            if buf.len() - (cur.position() as usize) < 2 {
                return Err(StorageError::Corrupted);
            }

            // read key length
            let klen = cur.get_u16() as usize;
            if buf.len() - (cur.position() as usize) < klen {
                return Err(StorageError::Corrupted);
            }

            // skip key bytes
            cur.set_position(cur.position() + klen as u64);
            if buf.len() - (cur.position() as usize) < 4 {
                return Err(StorageError::Corrupted);
            }

            // read value length
            let vlen = cur.get_u32() as usize;
            if buf.len() - (cur.position() as usize) < vlen {
                return Err(StorageError::Corrupted);
            }

            // skip value bytes
            cur.set_position(cur.position() + vlen as u64);

            index.insert(offset, pos);
            next_offset = offset + 1;
        }

        Ok((index, next_offset))
    }

    pub fn append(&mut self, records: &[Record]) -> Result<i64, StorageError> {
        let base = self.next_offset;

        for rec in records {
            let offset = self.next_offset;
            let pos = self.file.stream_position()?;

            let mut out = BytesMut::with_capacity(8 + 2 + rec.key.len() + 4 + rec.value.len());
            out.put_i64(offset);
            out.put_u16(rec.key.len() as u16);
            out.put_slice(&rec.key);
            out.put_u32(rec.value.len() as u32);
            out.put_slice(&rec.value);

            self.file.write_all(&out)?;
            self.index.insert(offset, pos);

            self.next_offset += 1;
        }

        // flush to OS buffer
        self.file.flush()?;

        // commit to disk
        self.file.sync_data()?;

        Ok(base)
    }

    //// Fetch records starting from offset, up to max_bytes
    pub fn fetch(&self, offset: i64, max_bytes: u32) -> Result<Vec<(i64, Record)>, StorageError> {
        // Get current offset + position
        let start = match self.index.range(offset..).next() {
            Some((&off, &pos)) => (off, pos),
            None => return Ok(vec![]),
        };

        // Open file for reading, and seek to position
        let mut f = OpenOptions::new().read(true).open(&self.path)?;
        f.seek(SeekFrom::Start(start.1))?;

        let mut remaining = max_bytes as usize;
        let mut items = Vec::new();

        loop {
            if remaining < 8 + 2 + 4 {
                break;
            }

            // offset
            let mut off_bytes = [0u8; 8];
            if let Err(e) = f.read_exact(&mut off_bytes) {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    break;
                }

                return Err(e.into());
            }

            let mut cur = std::io::Cursor::new(off_bytes);
            let off = cur.get_i64();
            remaining = remaining.saturating_sub(8);

            // klen
            let mut klen_bytes = [0u8; 2];
            let _ = f.read_exact(&mut klen_bytes);
            let mut cur = std::io::Cursor::new(klen_bytes);
            let klen = cur.get_u16() as usize;
            remaining = remaining.saturating_sub(2);

            if remaining < klen {
                break;
            }

            // key
            let mut key = vec![0u8; klen];
            let _ = f.read_exact(&mut key);
            remaining = remaining.saturating_sub(klen);

            // vlen
            let mut vlen_bytes = [0u8; 4];
            let _ = f.read_exact(&mut vlen_bytes);
            let mut cur = std::io::Cursor::new(vlen_bytes);
            let vlen = cur.get_u32() as usize;
            remaining = remaining.saturating_sub(4);

            if remaining < vlen {
                break;
            }

            // value
            let mut value = vec![0u8; vlen];
            let _ = f.read_exact(&mut value);
            remaining = remaining.saturating_sub(vlen);

            if off < offset {
                continue;
            }

            items.push((
                off,
                Record {
                    key: key.into(),
                    value: value.into(),
                },
            ));
        }

        Ok(items)
    }
}
