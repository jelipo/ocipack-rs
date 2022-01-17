use std::fs::File;
use std::io;
use std::io::{Read, Write};


use anyhow::{Result};
use flate2::Compression;
use flate2::read::{GzDecoder};
use flate2::write::GzEncoder;
use zstd::Decoder;
use crate::reg::CompressType;

pub fn ungz<R: Read, W: ?Sized + Write>(input: R, output_writer: &mut W) -> Result<()> {
    let mut decoder = GzDecoder::new(input);

    let mut buffer = vec![0u8; 1024 * 4].into_boxed_slice();
    loop {
        let read_size = decoder.read(&mut buffer)?;
        if read_size == 0 { break; }
        let _write_size = output_writer.write(&buffer[..read_size])?;
    }
    output_writer.flush()?;
    Ok(())
}

pub fn uncompress<R: Read, W: Write>(compress_type: &CompressType, input: R, output_writer: &mut W) -> Result<()> {
    match compress_type {
        CompressType::TAR => {}
        CompressType::TGZ => {
            ungz(input, output_writer)?;
        }
        CompressType::ZSTD => {
            let mut decoder = Decoder::new(input)?;
            io::copy(&mut decoder, output_writer)?;
        }
    }
    Ok(())
}

pub fn gz_file<R: Read, W: ?Sized + Write>(input_reader: &mut R, output_writer: &mut W) -> Result<()> {
    let mut encoder = GzEncoder::new(output_writer, Compression::fast());
    let mut buffer = vec![0u8; 1024 * 4].into_boxed_slice();
    loop {
        let read_size = input_reader.read(&mut buffer)?;
        if read_size == 0 { break; }
        let _write_size = encoder.write(&buffer[..read_size])?;
    }
    Ok(())
}