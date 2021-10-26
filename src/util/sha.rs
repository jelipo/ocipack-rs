use bytes::Bytes;
use sha2::digest::DynDigest;
use sha2::{Digest, Sha256};

pub fn sha256(bytes: &Bytes) -> String {
    let mut hasher = Sha256::new();
    DynDigest::update(&mut hasher, bytes.as_ref());
    let sha256 = &hasher.finalize()[..];
    hex::encode(sha256)
}
