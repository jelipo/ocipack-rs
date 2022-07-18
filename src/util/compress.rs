use std::io;
use std::io::{Read, Write};

use anyhow::Result;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use zstd::{stream, DEFAULT_COMPRESSION_LEVEL};

use crate::reg::CompressType;

pub fn uncompress<R: Read, W: Write>(compress_type: CompressType, tar_input: &mut R, output_writer: &mut W) -> Result<()> {
    match compress_type {
        CompressType::Tar => io::copy(tar_input, output_writer).map(|_| ())?,
        CompressType::Tgz => uncompress_gz(tar_input, output_writer)?,
        CompressType::Zstd => stream::copy_decode(tar_input, output_writer)?,
    };
    Ok(())
}

pub fn compress<R: Read, W: ?Sized + Write>(
    compress_type: CompressType,
    tar_input_reader: &mut R,
    output_writer: &mut W,
) -> Result<()> {
    match compress_type {
        CompressType::Tar => io::copy(tar_input_reader, output_writer).map(|_| ())?,
        CompressType::Tgz => compress_gz(tar_input_reader, output_writer)?,
        CompressType::Zstd => stream::copy_encode(tar_input_reader, output_writer, DEFAULT_COMPRESSION_LEVEL)?,
    }
    Ok(())
}

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

pub fn compress_gz<R: Read, W: ?Sized + Write>(tar_input_reader: &mut R, output_writer: &mut W) -> Result<()> {
    let mut encoder = GzEncoder::new(output_writer, Compression::default());
    let _ = io::copy(tar_input_reader, &mut encoder)?;
    Ok(())
}
