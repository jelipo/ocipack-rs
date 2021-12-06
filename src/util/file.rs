use std::fs::File;
use std::path::{Path, PathBuf};

use anyhow::Result;

pub fn remove(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        std::fs::remove_dir(path)?
    } else if path.is_file() {
        std::fs::remove_file(path)?
    }
    Ok(())
}

pub trait PathExt {
    fn remove(&self) -> Result<()>;
}

impl PathExt for Path {
    fn remove(&self) -> Result<()> {
        remove(self)
    }
}

impl PathExt for PathBuf {
    fn remove(&self) -> Result<()> {
        remove(self)
    }
}
