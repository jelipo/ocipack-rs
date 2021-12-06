use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::{Error, Result};
use flate2::read::GzDecoder;

pub fn ungzip_file<W: ?Sized + Write>(gzip_file: &File, output_writer: &mut W) -> Result<()> {
    let mut decoder = GzDecoder::new(gzip_file);
    let mut buffer = vec![0u8; 1024 * 4].into_boxed_slice();
    loop {
        let read_size = decoder.read(&mut buffer)?;
        if read_size == 0 { break; }
        let write_size = output_writer.write(&buffer[..read_size])?;
    }
    output_writer.flush()?;
    Ok(())
}