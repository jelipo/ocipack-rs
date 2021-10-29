use std::borrow::{Borrow, BorrowMut};
use std::fs::File;
use std::io::{Read, Write};
use std::ops::Index;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::{JoinHandle, spawn};

use anyhow::{Error, Result};
use bytes::{Buf, Bytes};
use reqwest::blocking::{Client, Response};
use reqwest::header::HeaderMap;
use reqwest::Method;

use crate::reg::client::RegistryHttpClient;
use crate::reg::get_header;
use crate::util::sha::{file_sha256, Sha, ShaType};

pub struct RegDownloader {
    url: String,
    username: String,
    password: String,
    client: Client,
    temp: Arc<Mutex<RegDownloaderTemp>>,
    filename_type: DownloadFilenameType,
}

impl RegDownloader {
    pub fn new_reg_downloader(url: String, username: String, password: String, client: Client, filename_type: DownloadFilenameType) -> Result<RegDownloader> {
        let temp = Arc::new(Mutex::new(RegDownloaderTemp {
            file_size: 0,
            curr_size: 0,
            done: false,
        }));
        Ok(RegDownloader {
            url,
            username,
            password,
            client,
            temp,
            filename_type,
        })
    }

    pub fn start(&mut self) -> Result<JoinHandle<Result<()>>> {
        let arc = self.temp.clone();
        let reg_http_downloader = RegHttpDownloader {
            url: self.url.clone(),
            username: self.username.clone(),
            password: self.password.clone(),
            client: self.client.clone(),
        };
        let filename_type = self.filename_type.clone();
        let handle = thread::spawn::<_, Result<()>>(move || {
            let temp = arc.clone();
            let result = downloading(temp, reg_http_downloader, filename_type);
            let mut download_temp = arc.lock().expect("lock failed");
            download_temp.done = true;
            if let Err(err) = &result {
                println!("{}\n{}", err, err.backtrace());
            }
            result
        });
        Ok(handle)
    }

    pub fn download_temp(&self) -> Arc<Mutex<RegDownloaderTemp>> {
        self.temp.clone()
    }
}

fn downloading(temp: Arc<Mutex<RegDownloaderTemp>>, reg_http_downloader: RegHttpDownloader, filename_type: DownloadFilenameType) -> Result<()> {
    //检查本地是否存在已有
    let file_path = get_filepath(&reg_http_downloader.url, &filename_type)?;
    let parent_path = file_path.parent().expect("未找到目录");
    if !parent_path.exists() { std::fs::create_dir(parent_path)? }
    if file_path.exists() {
        if check_exists_file_legal(&file_path, &filename_type)? {
            return Ok(());
        } else {
            std::fs::remove_file(&file_path);
        }
    }
    // 请求HTTP
    let mut http_response = reg_http_downloader.do_request_raw()?;
    check(&http_response)?;
    let file = File::create(file_path)?;
    let mut writer = RegDownloaderWriter {
        temp,
        file,
    };
    let copy_size = std::io::copy(&mut http_response, &mut writer)?;
    writer.flush();
    Ok(())
}

fn check_exists_file_legal(exists_file_path: &Path, filename_type: &DownloadFilenameType) -> Result<bool> {
    match filename_type {
        DownloadFilenameType::Auto(_) => Ok(false),
        DownloadFilenameType::Custom(custom) => match &custom.sha {
            None => Ok(false),
            Some(shas) => {
                return match shas.sha_type {
                    ShaType::Sha256 => {
                        let sha256 = file_sha256(&exists_file_path)?;
                        if shas.sha_str == sha256 {
                            Ok(true)
                        } else {
                            Ok(false)
                        }
                    }
                    ShaType::Sha128 => Ok(false)
                };
            }
        }
    }
}

struct RegHttpDownloader {
    url: String,
    username: String,
    password: String,
    client: Client,
}

impl RegHttpDownloader {
    fn do_request_raw(&self) -> Result<Response> {
        let url = self.url.as_str();
        let mut builder = self.client.request(Method::GET, url)
            .basic_auth(&self.username, Some(&self.password));
        let request = builder.build()?;
        let http_response = self.client.execute(request)?;
        Ok(http_response)
    }
}

fn check(response: &Response) -> Result<()> {
    let headers = response.headers();
    let content_type = get_header(headers, "content-type")
        .expect("content_type not found");
    if !content_type.contains("application/octet-stream") {
        return Err(Error::msg(format!("Not support the content type:{}", content_type)));
    }
    Ok(())
}

fn get_filepath(url: &str, filename_type: &DownloadFilenameType) -> Result<Box<Path>> {
    let path_buf = match filename_type {
        DownloadFilenameType::Auto(auto) => {
            let ri = url.rfind("/").expect("URL error");
            auto.dir_path.join(&url[ri..])
        }
        DownloadFilenameType::Custom(custom) => {
            custom.dir_path.join(&custom.file_name)
        }
    };
    Ok(path_buf.into_boxed_path())
}

pub struct RegDownloaderWriter {
    temp: Arc<Mutex<RegDownloaderTemp>>,
    file: File,
}


impl Write for RegDownloaderWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let download_temp = &mut self.temp.lock().unwrap();
        download_temp.curr_size = download_temp.curr_size + buf.len();
        self.file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

pub struct RegDownloaderTemp {
    file_size: usize,
    pub curr_size: usize,
    pub done: bool,
}

#[derive(Clone)]
pub enum DownloadFilenameType {
    Auto(AutoDownloadFileName),
    Custom(CustomDownloadFileName),
}

#[derive(Clone)]
pub struct AutoDownloadFileName {
    dir_path: Box<Path>,
}

#[derive(Clone)]
pub struct CustomDownloadFileName {
    pub dir_path: Box<Path>,
    pub file_name: String,
    pub sha: Option<Sha>,
}