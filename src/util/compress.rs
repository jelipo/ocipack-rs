use std::io;
use std::io::{Read, Write};

use anyhow::Result;
use async_compression::tokio::write::{GzipDecoder, GzipEncoder};
use async_compression::tokio::write::{ZstdDecoder, ZstdEncoder};
use async_compression::Level::{Default, Fastest};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use zstd::{stream, DEFAULT_COMPRESSION_LEVEL};

use crate::container::CompressType;

#[deprecated]
pub fn uncompress<R: Read, W: Write>(compress_type: CompressType, tar_input: &mut R, output_writer: &mut W) -> Result<()> {
    match compress_type {
        CompressType::Tar => io::copy(tar_input, output_writer).map(|_| ())?,
        CompressType::Tgz => uncompress_gz(tar_input, output_writer)?,
        CompressType::Zstd => stream::copy_decode(tar_input, output_writer)?,
    };
    Ok(())
}

pub async fn async_uncompress<R, W>(compress_type: CompressType, tar_input: &mut R, output_writer: &mut W) -> Result<()>
where
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    match compress_type {
        CompressType::Tar => tokio::io::copy(tar_input, output_writer).await.map(|_| ())?,
        CompressType::Tgz => async_uncompress_gz(tar_input, output_writer).await?,
        CompressType::Zstd => tokio::io::copy(tar_input, &mut ZstdDecoder::new(output_writer)).await.map(|_| ())?,
    };
    Ok(())
}

#[deprecated]
pub fn compress<R: Read, W: ?Sized + Write>(compress_type: CompressType, tar_input_reader: &mut R, output_writer: &mut W) -> Result<()> {
    match compress_type {
        CompressType::Tar => io::copy(tar_input_reader, output_writer).map(|_| ())?,
        CompressType::Tgz => compress_gz(tar_input_reader, output_writer)?,
        CompressType::Zstd => stream::copy_encode(tar_input_reader, output_writer, DEFAULT_COMPRESSION_LEVEL)?,
    }
    Ok(())
}

pub async fn async_compress<R, W>(compress_type: CompressType, reader: &mut R, writer: &mut W) -> Result<()>
where
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    match compress_type {
        CompressType::Tar => tokio::io::copy(reader, writer).await.map(|_| ())?,
        CompressType::Tgz => async_compress_gz(reader, writer).await?,
        CompressType::Zstd => tokio::io::copy(reader, &mut ZstdEncoder::with_quality(writer, Default)).await.map(|_| ())?,
    }
    Ok(())
}

#[deprecated]
pub fn uncompress_gz<R: Read, W: ?Sized + Write>(input: R, output_writer: &mut W) -> Result<()> {
    let mut decoder = GzDecoder::new(input);

    let mut buffer = vec![0u8; 1024 * 4].into_boxed_slice();
    loop {
        let read_size = decoder.read(&mut buffer)?;
        if read_size == 0 {
            break;
        }
        output_writer.write_all(&buffer[..read_size])?;
    }
    output_writer.flush()?;
    Ok(())
}

pub async fn async_uncompress_gz<R, W>(input: &mut R, output_writer: &mut W) -> Result<()>
where
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    let mut decoder = GzipDecoder::new(output_writer);
    let x = tokio::io::copy(input, &mut decoder).await?;
    Ok(())
}

#[deprecated]
pub fn compress_gz<R: Read, W: ?Sized + Write>(tar_input_reader: &mut R, output_writer: &mut W) -> Result<()> {
    let mut encoder = GzEncoder::new(output_writer, Compression::fast());
    let _ = io::copy(tar_input_reader, &mut encoder)?;
    Ok(())
}

pub async fn async_compress_gz<R, W>(reader: &mut R, writer: &mut W) -> Result<()>
where
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    let mut encoder = GzipEncoder::with_quality(writer, Fastest);
    let _ = tokio::io::copy(reader, &mut encoder).await?;
    Ok(())
}
