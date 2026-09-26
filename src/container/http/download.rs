use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::container::BlobConfig;
use crate::container::http::{HttpAuth, do_request_raw, get_header};
use crate::progress::{CoreStatus, ProcessResult, ProgressStatus, ProgressStatusEnum, TransferJob};
use anyhow::{Result, anyhow};
use reqwest::Method;
use reqwest::{Client, Response};
use sha2::{Digest, Sha256};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;

pub struct RegDownloader {
    finished: bool,
    url: String,
    auth: Option<HttpAuth>,
    client: Option<Client>,
    temp: RegDownloaderStatus,
    blob_down_config: Arc<BlobConfig>,
}

impl RegDownloader {
    pub fn new_reg(
        url: String,
        auth: Option<HttpAuth>,
        client: Client,
        blob_down_config: BlobConfig,
        layer_size: Option<u64>,
    ) -> Result<RegDownloader> {
        let blob_down_config_arc = Arc::new(blob_down_config);
        let temp = RegDownloaderStatus {
            status_core: Arc::new(Mutex::new(RegDownloaderStatusCore {
                blob_config: blob_down_config_arc.clone(),
                file_size: layer_size.unwrap_or(0),
                curr_size: 0,
            })),
        };
        Ok(RegDownloader {
            finished: false,
            url,
            auth,
            client: Some(client),
            temp,
            blob_down_config: blob_down_config_arc,
        })
    }

    pub fn new_finished(blob_down_config: BlobConfig, file_size: u64) -> Result<RegDownloader> {
        let blob_down_config_arc = Arc::new(blob_down_config);
        let temp = RegDownloaderStatus {
            status_core: Arc::new(Mutex::new(RegDownloaderStatusCore {
                blob_config: blob_down_config_arc.clone(),
                file_size,
                curr_size: file_size,
            })),
        };
        Ok(RegDownloader {
            finished: true,
            url: String::default(),
            auth: None,
            client: None,
            temp,
            blob_down_config: blob_down_config_arc,
        })
    }

    pub fn into_job(self) -> TransferJob<DownloadResult> {
        let status = ProgressStatusEnum::RegDownloaderStatus(self.temp.clone());
        TransferJob {
            status,
            future: Box::pin(self.run()),
        }
    }

    async fn run(self) -> Result<DownloadResult> {
        let blob_config = self.blob_down_config;
        let file_path = blob_config.file_path.clone();
        if self.finished {
            let size = fs::metadata(&file_path).await?.len();
            return Ok(DownloadResult {
                file_path: Some(file_path),
                _file_size: size,
                blob_config,
                local_existed: true,
                result_str: "local exists".to_string(),
            });
        }
        let downloader = RegHttpDownloader {
            url: self.url,
            auth: self.auth,
            client: self.client.ok_or_else(|| anyhow!("download client not found"))?,
        };
        let size = downloading(self.temp, &file_path, downloader, &blob_config.reg_digest.sha256).await?;
        Ok(DownloadResult {
            file_path: Some(file_path),
            _file_size: size,
            blob_config,
            local_existed: false,
            result_str: "complete".to_string(),
        })
    }
}

async fn downloading(
    status: RegDownloaderStatus,
    file_path: &Path,
    reg_http_downloader: RegHttpDownloader,
    expected_sha256: &str,
) -> Result<u64> {
    let parent_path = file_path.parent().ok_or_else(|| anyhow!("download path has no parent"))?;
    fs::create_dir_all(parent_path).await?;
    let mut http_response = reg_http_downloader.do_request_raw().await?;
    http_response.error_for_status_ref()?;
    check(&http_response)?;
    if let Some(len) = http_response.content_length() {
        let mut status_core = status.status_core.lock().expect("lock failed");
        status_core.file_size = len;
    }
    let temp_path = tempfile::Builder::new().tempfile_in(parent_path)?.into_temp_path();
    let mut file = File::create(&temp_path).await?;
    let mut total = 0;
    let mut hasher = Sha256::new();
    while let Some(chunk) = http_response.chunk().await? {
        file.write_all(&chunk).await?;
        hasher.update(&chunk);
        total += chunk.len() as u64;
        status.status_core.lock().expect("lock failed").curr_size = total;
    }
    file.flush().await?;
    drop(file);
    let actual_sha256 = hex::encode(hasher.finalize());
    if actual_sha256 != expected_sha256 {
        return Err(anyhow!(
            "download digest mismatch: expected sha256:{expected_sha256}, got sha256:{actual_sha256}"
        ));
    }
    temp_path.persist(file_path)?;
    Ok(total)
}

struct RegHttpDownloader {
    url: String,
    auth: Option<HttpAuth>,
    client: Client,
}

impl RegHttpDownloader {
    async fn do_request_raw(&self) -> Result<Response> {
        let url = self.url.as_str();
        do_request_raw::<u8>(&self.client, url, Method::GET, self.auth.as_ref(), &[], None, None).await
    }
}

const OCTET_STREAM_TYPE: [&str; 2] = [
    "binary/octet-stream", // quay.io registry use this type
    "application/octet-stream",
];

fn check(response: &Response) -> Result<()> {
    let headers = response.headers();
    let content_type = get_header(headers, "content-type").ok_or_else(|| anyhow!("content-type not found"))?;
    if !OCTET_STREAM_TYPE.contains(&content_type.as_str()) {
        return Err(anyhow!("Not support the content type:{}", content_type));
    }
    Ok(())
}

#[derive(Clone)]
pub struct RegDownloaderStatus {
    status_core: Arc<Mutex<RegDownloaderStatusCore>>,
}

struct RegDownloaderStatusCore {
    blob_config: Arc<BlobConfig>,
    file_size: u64,
    pub curr_size: u64,
}

impl ProgressStatus for RegDownloaderStatus {
    fn status(&self) -> CoreStatus {
        let core = &self.status_core.lock().unwrap();
        CoreStatus {
            blob_config: core.blob_config.clone(),
            full_size: core.file_size,
            now_size: core.curr_size,
        }
    }
}

pub struct DownloadResult {
    pub file_path: Option<Box<Path>>,
    pub _file_size: u64,
    pub blob_config: Arc<BlobConfig>,
    pub local_existed: bool,
    pub result_str: String,
}

impl ProcessResult for DownloadResult {
    fn finished_info(&self) -> &str {
        &self.result_str
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::RegDigest;
    use crate::progress::manager::ProcessorManager;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpListener;

    async fn download_with_response(response: &'static [u8]) -> Result<(Result<Vec<DownloadResult>>, tempfile::TempDir)> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let server = tokio::spawn(async move {
            let (mut connection, _) = listener.accept().await?;
            connection.write_all(response).await?;
            connection.shutdown().await
        });
        let dir = tempfile::tempdir()?;
        let blob = BlobConfig::new(
            dir.path().join("blob").into_boxed_path(),
            "blob".to_string(),
            RegDigest::new_with_sha256(crate::util::sha::bytes_sha256(b"abc")),
        );
        let downloader = RegDownloader::new_reg(format!("http://{addr}/blob"), None, Client::new(), blob, Some(3))?;
        let manager = ProcessorManager::new_processor_manager(vec![downloader.into_job()]);
        let result = manager.wait_all_done().await;
        server.await??;
        Ok((result, dir))
    }

    #[tokio::test]
    async fn download_saves_body_and_propagates_http_errors() -> Result<()> {
        let (result, dir) = download_with_response(
            b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\nabc",
        )
        .await?;
        assert_eq!(result?.len(), 1);
        assert_eq!(fs::read(dir.path().join("blob")).await?, b"abc");

        let (result, dir) = download_with_response(
            b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\nabd",
        )
        .await?;
        assert!(result.is_err());
        assert!(!dir.path().join("blob").exists());

        let (result, dir) =
            download_with_response(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await?;
        assert!(result.is_err());
        assert!(!dir.path().join("blob").exists());
        Ok(())
    }
}
