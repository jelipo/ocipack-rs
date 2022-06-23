use std::io;
use std::io::{Read, Write};

use anyhow::Result;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;

use crate::reg::CompressType;

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

pub fn uncompress<R: Read, W: Write>(compress_type: &CompressType, mut input: R, output_writer: &mut W) -> Result<()> {
    match compress_type {
        CompressType::Tar => io::copy(&mut input, output_writer).map(|_| ())?,
        CompressType::Tgz => uncompress_gz(input, output_writer)?,
        CompressType::Zstd => zstd::stream::copy_decode(input, output_writer)?,
    };
    Ok(())
}

pub fn gz_file<R: Read, W: ?Sized + Write>(input_reader: &mut R, output_writer: &mut W) -> Result<()> {
    let mut encoder = GzEncoder::new(output_writer, Compression::fast());
    let mut buffer = vec![0u8; 1024 * 4];
    loop {
        let read_size = input_reader.read(&mut buffer)?;
        if read_size == 0 {
            break;
        }
        encoder.write_all(&buffer[..read_size])?;
    }
    Ok(())
}
