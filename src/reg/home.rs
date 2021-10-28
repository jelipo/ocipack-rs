use std::path::Path;

pub struct HomeDir {
    pub cache: CacheDir,
}

pub struct CacheDir {
    pub blobs: BlobsDir,
}

pub struct BlobsDir {
    path: Box<Path>,
}