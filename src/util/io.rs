use std::borrow::Cow;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

const BUF_SIZE: usize = 8 * 1024;

pub struct TeeReader {
    reader: Box<dyn AsyncRead + Unpin>,
    writer: Box<dyn AsyncWrite + Unpin>,
    write_buf: Vec<u8>,
    buf_start: usize,
    buf_end: usize, // 读缓冲区位置
}

impl TeeReader {
    pub fn new(reader: Box<dyn AsyncRead + Unpin>, writer: Box<dyn AsyncWrite + Unpin>) -> Self {
        TeeReader {
            reader,
            writer,
            write_buf: vec![0; BUF_SIZE],
            buf_start: 0,
            buf_end: 0,
        }
    }
}

impl AsyncRead for TeeReader {
    fn poll_read<'a>(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        if this.buf_start == 0 && this.buf_end == 0 {
            // 说明buf是空的，开始读取
            // 先从 reader 读取数据到 buf
            let buf_free_size = buf.remaining();
            let max_read_size = buf_free_size.min(BUF_SIZE);
            // 缓冲区已满
            if max_read_size == 0 {
                return Poll::Ready(Ok(()));
            }
            let buf_filled_len = buf.filled().len();
            match Pin::new(&mut this.reader).poll_read(cx, buf)? {
                Poll::Ready(()) => {
                    let filled = buf.filled();
                    if filled.is_empty() || buf_filled_len == filled.len() {
                        return Poll::Ready(Ok(()));
                    }
                    let read_share = &filled[buf_filled_len..];
                    // 读取到数据，尝试直接写入 writer
                    let un_write_share = match Pin::new(&mut this.writer).poll_write(cx, read_share)? {
                        Poll::Ready(write_size) => &read_share[write_size..],
                        Poll::Pending => read_share,
                    };
                    if !un_write_share.is_empty() {
                        // 说明没有全部写入，需要缓存剩余部分
                        this.write_buf.reserve(un_write_share.len());
                        this.write_buf[..un_write_share.len()].copy_from_slice(un_write_share);
                        this.buf_end = un_write_share.len();
                    }
                }
                Poll::Pending => return Poll::Pending,
            }
        }
        if this.buf_start != 0 || this.buf_end != 0 {
            // 写入 write_buf 到 writer
            let write_buf = &this.write_buf[this.buf_start..this.buf_end];
            return match Pin::new(&mut this.writer).poll_write(cx, &write_buf)? {
                Poll::Ready(size) => {
                    this.buf_start += size;
                    if this.buf_start >= this.buf_end {
                        this.buf_start = 0;
                        this.buf_end = 0;
                    }
                    Poll::Ready(Ok(()))
                }
                Poll::Pending => Poll::Pending,
            };
        }
        Poll::Ready(Ok(()))
    }
}
