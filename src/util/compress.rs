use anyhow::Result;
use async_compression::Level::{Default, Fastest};
use async_compression::tokio::write::{GzipDecoder, GzipEncoder};
use async_compression::tokio::write::{ZstdDecoder, ZstdEncoder};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

use crate::container::CompressType;

pub async fn async_uncompress<R, W>(compress_type: CompressType, tar_input: &mut R, output_writer: &mut W) -> Result<()>
where
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    match compress_type {
        CompressType::Tar => tokio::io::copy(tar_input, output_writer).await.map(|_| ())?,
        CompressType::Tgz => async_uncompress_gz(tar_input, output_writer).await?,
        CompressType::Zstd => {
            let mut decoder = ZstdDecoder::new(output_writer);
            tokio::io::copy(tar_input, &mut decoder).await?;
            decoder.shutdown().await?;
        }
    };
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
        CompressType::Zstd => {
            let mut encoder = ZstdEncoder::with_quality(writer, Default);
            tokio::io::copy(reader, &mut encoder).await?;
            encoder.shutdown().await?;
        }
    }
    Ok(())
}

pub async fn async_uncompress_gz<R, W>(input: &mut R, output_writer: &mut W) -> Result<()>
where
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    let mut decoder = GzipDecoder::new(output_writer);
    tokio::io::copy(input, &mut decoder).await?;
    decoder.shutdown().await?;
    Ok(())
}

pub async fn async_compress_gz<R, W>(reader: &mut R, writer: &mut W) -> Result<()>
where
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    let mut encoder = GzipEncoder::with_quality(writer, Fastest);
    tokio::io::copy(reader, &mut encoder).await?;
    encoder.shutdown().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[tokio::test]
    async fn compressed_layers_have_complete_gzip_and_zstd_frames() -> Result<()> {
        let input = b"layer content".repeat(4096);
        for kind in [CompressType::Tgz, CompressType::Zstd] {
            let mut compressed = Vec::new();
            async_compress(kind, &mut input.as_slice(), &mut compressed).await?;
            let decoded = match kind {
                CompressType::Tgz => {
                    let mut decoded = Vec::new();
                    flate2::read::GzDecoder::new(compressed.as_slice()).read_to_end(&mut decoded)?;
                    decoded
                }
                CompressType::Zstd => zstd::stream::decode_all(compressed.as_slice())?,
                CompressType::Tar => unreachable!(),
            };
            assert_eq!(decoded, input);

            let mut async_decoded = Vec::new();
            async_uncompress(kind, &mut compressed.as_slice(), &mut async_decoded).await?;
            assert_eq!(async_decoded, input);
        }
        Ok(())
    }
}
