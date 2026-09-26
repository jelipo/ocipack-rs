use crate::container::BlobConfig;
use crate::container::http::{HttpAuth, do_request_raw_read};
use crate::progress::{CoreStatus, ProcessResult, ProgressStatus, ProgressStatusEnum, TransferJob};
use anyhow::{Result, anyhow};
use reqwest::Client;
use reqwest::Method;
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::fs::File;
use tokio::io::{AsyncRead, ReadBuf};
use tokio_util::io::InspectReader;

pub struct RegUploader {
    reg_uploader_enum: RegUploaderEnum,
    blob_config: Arc<BlobConfig>,
    temp: RegUploaderStatus,
}

struct RegUploaderCore {
    url: String,
    auth: HttpAuth,
    client: Client,
}

enum RegUploaderEnum {
    Finished { _file_size: u64, finished_reason: String },
    Run(RegUploaderCore),
}

#[derive(Clone)]
pub struct RegUploaderStatus {
    status_core: Arc<Mutex<RegUploaderStatusCore>>,
}

struct RegUploaderStatusCore {
    blob_config: Arc<BlobConfig>,
    file_size: u64,
    pub curr_size: u64,
}

impl RegUploader {
    /// 创建一个已经完成状态的Uploader
    pub fn new_finished_uploader(blob_config: BlobConfig, file_size: u64, finished_reason: String) -> RegUploader {
        let blob_config_arc = Arc::new(blob_config);
        let temp = RegUploaderStatus {
            status_core: Arc::new(Mutex::new(RegUploaderStatusCore {
                blob_config: blob_config_arc.clone(),
                file_size,
                curr_size: file_size,
            })),
        };
        RegUploader {
            reg_uploader_enum: RegUploaderEnum::Finished {
                _file_size: file_size,
                finished_reason,
            },
            blob_config: blob_config_arc,
            temp,
        }
    }

    pub fn new_uploader(url: String, auth: HttpAuth, client: Client, blob_config: BlobConfig, file_size: u64) -> RegUploader {
        let blob_config_arc = Arc::new(blob_config);
        let temp = RegUploaderStatus {
            status_core: Arc::new(Mutex::new(RegUploaderStatusCore {
                blob_config: blob_config_arc.clone(),
                file_size,
                curr_size: 0,
            })),
        };
        RegUploader {
            reg_uploader_enum: RegUploaderEnum::Run(RegUploaderCore { url, auth, client }),
            blob_config: blob_config_arc,
            temp,
        }
    }

    pub fn into_job(self) -> TransferJob<UploadResult> {
        let status = ProgressStatusEnum::RegUploaderStatus(self.temp.clone());
        TransferJob {
            status,
            future: Box::pin(self.run()),
        }
    }

    async fn run(self) -> Result<UploadResult> {
        let result_str = match self.reg_uploader_enum {
            RegUploaderEnum::Finished { finished_reason, .. } => finished_reason,
            RegUploaderEnum::Run(info) => {
                let uploader = RegHttpUploader {
                    url: info.url,
                    auth: info.auth,
                    client: info.client,
                };
                uploading(self.temp, &self.blob_config.file_path, uploader, self.blob_config.clone()).await?;
                "success".to_string()
            }
        };
        Ok(UploadResult { result_str })
    }
}

async fn uploading(
    status: RegUploaderStatus,
    file_path: &Path,
    reg_http_uploader: RegHttpUploader,
    blob_config: Arc<BlobConfig>,
) -> Result<()> {
    //检查本地是否存在已有
    let local_file = File::open(file_path).await?;
    let file_size = local_file.metadata().await?.len();
    let reader = RegUploaderReader::new(status, local_file);
    let response = do_request_raw_read::<RegUploaderReader>(
        &reg_http_uploader.client,
        reg_http_uploader.url.as_str(),
        Method::PUT,
        Some(&reg_http_uploader.auth),
        &[],
        Some(reader),
        file_size,
    )
    .await?;
    let short_hash = &blob_config.short_hash;
    if response.status().is_success() {
        let _body = response.text().await?;
        Ok(())
    } else {
        let status_code = response.status().as_u16();
        let response_string = response.text().await?;
        Err(anyhow!(
            "{} upload request failed. code: {}, body: {}",
            short_hash,
            status_code,
            response_string
        ))
    }
}

pub struct RegUploaderReader {
    inspect_reader: InspectReader<File, ReadProgress>,
}

type ReadProgress = Box<dyn Fn(&[u8]) + Send + Sync>;

impl AsyncRead for RegUploaderReader {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inspect_reader).poll_read(cx, buf)
    }
}

impl RegUploaderReader {
    fn new(status: RegUploaderStatus, file: File) -> Self {
        let callback: ReadProgress = Box::new(move |bytes: &[u8]| {
            if let Ok(mut guard) = status.status_core.lock() {
                guard.curr_size += bytes.len() as u64;
            }
        });
        RegUploaderReader {
            inspect_reader: InspectReader::new(file, callback),
        }
    }
}

impl ProgressStatus for RegUploaderStatus {
    fn status(&self) -> CoreStatus {
        let core = &self.status_core.lock().unwrap();
        CoreStatus {
            blob_config: core.blob_config.clone(),
            full_size: core.file_size,
            now_size: core.curr_size,
        }
    }
}

struct RegHttpUploader {
    url: String,
    auth: HttpAuth,
    client: Client,
}

pub struct UploadResult {
    pub result_str: String,
}

impl ProcessResult for UploadResult {
    fn finished_info(&self) -> &str {
        &self.result_str
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::RegDigest;
    use crate::progress::manager::ProcessorManager;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn upload_with_response(status: &'static str) -> Result<(Result<Vec<UploadResult>>, Vec<u8>)> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let server = tokio::spawn(async move {
            let (mut connection, _) = listener.accept().await?;
            let mut request = Vec::new();
            let mut buffer = [0; 4096];
            let header_end = loop {
                let count = connection.read(&mut buffer).await?;
                if count == 0 {
                    return Err(anyhow!("request ended before headers"));
                }
                request.extend_from_slice(&buffer[..count]);
                if let Some(index) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    break index + 4;
                }
            };
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let length: usize = headers
                .lines()
                .find_map(|line| line.to_ascii_lowercase().strip_prefix("content-length:").map(str::trim).map(str::to_string))
                .ok_or_else(|| anyhow!("missing content-length"))?
                .parse()?;
            while request.len() - header_end < length {
                let count = connection.read(&mut buffer).await?;
                if count == 0 {
                    return Err(anyhow!("request ended before body"));
                }
                request.extend_from_slice(&buffer[..count]);
            }
            let body = request[header_end..header_end + length].to_vec();
            connection.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).await?;
            connection.shutdown().await?;
            Ok::<_, anyhow::Error>(body)
        });

        let dir = tempfile::tempdir()?;
        let path = dir.path().join("blob");
        tokio::fs::write(&path, b"uploaded layer").await?;
        let blob = BlobConfig::new(
            path.into_boxed_path(),
            "blob".to_string(),
            RegDigest::new_with_sha256("a".repeat(64)),
        );
        let uploader = RegUploader::new_uploader(
            format!("http://{addr}/upload"),
            HttpAuth::BearerToken { token: "test".to_string() },
            Client::new(),
            blob,
            b"uploaded layer".len() as u64,
        );
        let manager = ProcessorManager::new_processor_manager(vec![uploader.into_job()]);
        let result = manager.wait_all_done().await;
        Ok((result, server.await??))
    }

    #[tokio::test]
    async fn upload_streams_file_and_propagates_http_errors() -> Result<()> {
        let (result, body) = upload_with_response("201 Created").await?;
        assert_eq!(result?.len(), 1);
        assert_eq!(body, b"uploaded layer");

        let (result, body) = upload_with_response("500 Internal Server Error").await?;
        assert!(result.is_err());
        assert_eq!(body, b"uploaded layer");
        Ok(())
    }
}
