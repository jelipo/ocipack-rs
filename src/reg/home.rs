use std::path::Path;

pub struct HomeDir {
    pub cache: CacheDir,
}

impl HomeDir {
    pub fn new_home_dir(cache_dir_path: &Path) -> HomeDir {
        let blob_cache_dir_path = cache_dir_path.join("blobs");
        HomeDir {
            cache: CacheDir {
                blobs: BlobsDir {
                    path: blob_cache_dir_path.into_boxed_path()
                }
            }
        }
    }
}

pub struct CacheDir {
    pub blobs: BlobsDir,
}

pub struct BlobsDir {
    pub path: Box<Path>,
}