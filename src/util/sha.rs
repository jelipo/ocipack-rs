use std::fs::{File, read};
use std::io::{Read, Write};
use std::path::Path;

use anyhow::Result;
use bytes::Bytes;
use sha2::{Digest, Sha256};
use sha2::digest::DynDigest;

pub fn sha256(bytes: &Bytes) -> String {
    let mut hasher = Sha256::new();
    DynDigest::update(&mut hasher, bytes.as_ref());
    let sha256 = &hasher.finalize()[..];
    hex::encode(sha256)
}

/// 计算文件的sha256值,并返回Hex
pub fn file_sha256(file_path: &Path) -> Result<String> {
    let mut file = File::open(file_path)?;
    let mut sha256 = Sha256::new();
    let _i = std::io::copy(&mut file, &mut sha256)?;
    let sha256 = &sha256.finalize()[..];
    Ok(hex::encode(sha256))
}

pub struct Sha256Reader<R: Read> {
    read: R,
    hasher: Sha256,
}

impl<R: Read> Sha256Reader<R> {
    pub fn new(read: R) -> Sha256Reader<R> {
        Sha256Reader {
            read,
            hasher: Sha256::new(),
        }
    }

    pub fn sha256(self) -> Result<String> {
        let sha256_bytes = &self.hasher.finalize()[..];
        Ok(hex::encode(sha256_bytes))
    }
}

impl<R: Read> Read for Sha256Reader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read_size = self.read.read(buf)?;
        let _write_size = self.hasher.write(&buf[..read_size])?;
        Ok(read_size)
    }
}


pub struct Sha256Writer<W: Write> {
    write: W,
    hasher: Sha256,
}

impl<W: Write> Write for Sha256Writer<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let _hasher_write_size = self.hasher.write(buf)?;
        self.write.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.hasher.flush()?;
        self.write.flush()
    }
}

impl<W: Write> Sha256Writer<W> {
    pub fn new(write: W) -> Sha256Writer<W> {
        Sha256Writer {
            write,
            hasher: Sha256::new(),
        }
    }

    pub fn sha256(self) -> Result<String> {
        let sha256_bytes = &self.hasher.finalize()[..];
        Ok(hex::encode(sha256_bytes))
    }
}