use std::path::Path;

pub struct HomeDir {
    pub cache: CacheDir,
}

impl HomeDir {
    pub fn new_home_dir(cache_dir_path: &Path) -> HomeDir {
        let blob_cache_dir_path = &cache_dir_path.join("blobs");
        HomeDir {
            cache: CacheDir {
                blobs: BlobsDir {
                    config_path: blob_cache_dir_path.join("config").into_boxed_path(),
                    layers_path: blob_cache_dir_path.join("layers").into_boxed_path(),
                },
            },
        }
    }
}

pub struct CacheDir {
    pub blobs: BlobsDir,
}

pub struct BlobsDir {
    pub config_path: Box<Path>,
    pub layers_path: Box<Path>,
}
