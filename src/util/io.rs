use std::io::{Read, Write};
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncReadExt};

pub const DEFAULT_BUF_SIZE: usize = if cfg!(target_os = "espidf") { 512 } else { 8 * 1024 };

pub async fn copy<R: AsyncRead + Unpin, W: Write>(mut async_read: R, mut write: W) -> anyhow::Result<usize> {
    let mut buf = [0u8; DEFAULT_BUF_SIZE];
    let mut total_read = 0;
    while let read_size = async_read.read(&mut buf).await? {
        if read_size == 0 { break; }
        write.write_all(&buf[0..read_size])?;
        total_read += read_size;
    }
    Ok(total_read)
}

