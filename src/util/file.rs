use std::path::{Path, PathBuf};

use anyhow::Result;

/// 删除文件/目录，当文件不存在时依然返回成功
pub fn remove(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        std::fs::remove_dir_all(path)?
    } else if path.is_file() {
        std::fs::remove_file(path)?
    }
    Ok(())
}

/// Path的扩展，删除文件或者目录
pub trait PathExt {
    fn clean_path(&self) -> Result<()>;
}

impl PathExt for Path {
    fn clean_path(&self) -> Result<()> {
        remove(self)
    }
}

impl PathExt for PathBuf {
    fn clean_path(&self) -> Result<()> {
        remove(self)
    }
}
