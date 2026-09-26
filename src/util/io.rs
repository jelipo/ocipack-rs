use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

const BUF_SIZE: usize = 8 * 1024;

/// Copies each read chunk to `writer` before returning it to the caller.
/// Pending writes and cancelled reads retain the chunk in the internal buffer.
pub struct TeeReader<R, W> {
    reader: R,
    writer: W,
    buffer: [u8; BUF_SIZE],
    buffered: usize,
    written: usize,
    delivered: usize,
    eof: bool,
}

impl<R, W> TeeReader<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader,
            writer,
            buffer: [0; BUF_SIZE],
            buffered: 0,
            written: 0,
            delivered: 0,
            eof: false,
        }
    }
}

impl<R: AsyncRead + Unpin, W: AsyncWrite + Unpin> AsyncRead for TeeReader<R, W> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, out: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if out.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }

        if this.buffered == 0 && !this.eof {
            let mut read_buf = ReadBuf::new(&mut this.buffer);
            match Pin::new(&mut this.reader).poll_read(cx, &mut read_buf) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                Poll::Ready(Ok(())) => {
                    this.buffered = read_buf.filled().len();
                    this.eof = this.buffered == 0;
                }
            }
        }

        if this.eof {
            return Pin::new(&mut this.writer).poll_flush(cx);
        }

        while this.written < this.buffered {
            match Pin::new(&mut this.writer).poll_write(cx, &this.buffer[this.written..this.buffered]) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                Poll::Ready(Ok(0)) => return Poll::Ready(Err(io::ErrorKind::WriteZero.into())),
                Poll::Ready(Ok(count)) => this.written += count,
            }
        }

        let count = out.remaining().min(this.buffered - this.delivered);
        out.put_slice(&this.buffer[this.delivered..this.delivered + count]);
        this.delivered += count;
        if this.delivered == this.buffered {
            this.buffered = 0;
            this.written = 0;
            this.delivered = 0;
        }
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::poll_fn;
    use std::io::Cursor;
    use tokio::io::AsyncReadExt;

    #[derive(Default)]
    struct ShortWriter {
        data: Vec<u8>,
        pending_once: bool,
        flushes: usize,
    }

    impl AsyncWrite for ShortWriter {
        fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, bytes: &[u8]) -> Poll<io::Result<usize>> {
            if !self.pending_once {
                self.pending_once = true;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            let count = bytes.len().min(3);
            self.data.extend_from_slice(&bytes[..count]);
            Poll::Ready(Ok(count))
        }

        fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            self.flushes += 1;
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    #[tokio::test]
    async fn mirrors_all_bytes_across_pending_and_short_writes() -> io::Result<()> {
        let input: Vec<u8> = (0..BUF_SIZE + 17).map(|index| (index % 251) as u8).collect();
        let mut tee = TeeReader::new(Cursor::new(input.clone()), ShortWriter::default());

        // Drop the first read after the mirror returns Pending.
        poll_fn(|cx| {
            let mut bytes = [0; 5];
            let mut out = ReadBuf::new(&mut bytes);
            assert!(Pin::new(&mut tee).poll_read(cx, &mut out).is_pending());
            Poll::Ready(())
        })
        .await;

        let mut received = Vec::new();
        let mut small = [0; 5];
        loop {
            let count = tee.read(&mut small).await?;
            if count == 0 {
                break;
            }
            received.extend_from_slice(&small[..count]);
        }
        assert_eq!(received, input);
        assert_eq!(tee.writer.data, input);
        assert!(tee.writer.flushes > 0);
        Ok(())
    }
}
