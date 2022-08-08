use std::io::{Read, Write};
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::AsyncRead;

pub fn uncompress<R: AsyncRead, W: Write>(async_read: R, write: W) {}

