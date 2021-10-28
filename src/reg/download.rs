use std::borrow::{Borrow, BorrowMut};
use std::fs::File;
use std::io::{Read, Write};
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

pub struct RegDownloader {
    url: String,
    username: String,
    password: String,
    client: Client,
    temp: Arc<Mutex<RegDownloaderTemp>>,
}

impl RegDownloader {
    pub fn new_reg_downloader(url: String, username: String, password: String, client: Client, file_path: &Path) -> Result<RegDownloader> {
        let temp = Arc::new(Mutex::new(RegDownloaderTemp {
            file_size: 0,
            curr_size: 0,
            file: File::create(file_path)?,
            done: false,
        }));
        Ok(RegDownloader {
            url,
            username,
            password,
            client,
            temp,
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
        let handle = thread::spawn::<_, Result<()>>(move || {
            let temp = arc.clone();
            let result = downloading(temp, reg_http_downloader);
            let mut download_temp = arc.lock().expect("lock failed");
            download_temp.done = true;
            result
        });
        Ok(handle)
    }

    pub fn download_temp(&self) -> Arc<Mutex<RegDownloaderTemp>> {
        self.temp.clone()
    }
}

fn downloading(temp: Arc<Mutex<RegDownloaderTemp>>, reg_http_downloader: RegHttpDownloader) -> Result<()> {
    let mut writer = RegDownloaderWriter {
        temp
    };
    let mut http_response = reg_http_downloader.do_request_raw()?;
    check_and_get_body(&http_response)?;
    let copy_size = std::io::copy(&mut http_response, &mut writer)?;
    writer.flush();
    Ok(())
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

fn check_and_get_body(response: &Response) -> Result<()> {
    let content_type = get_header(response.headers(), "content-type")
        .expect("content_type not found");
    if !content_type.contains("application/octet-stream") {
        return Err(Error::msg(format!("Not support the cotnent type:{}", content_type)));
    }
    Ok(())
}

pub struct RegDownloaderWriter {
    temp: Arc<Mutex<RegDownloaderTemp>>,
}


impl Write for RegDownloaderWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let download_temp = &mut self.temp.lock().unwrap();
        download_temp.curr_size = download_temp.curr_size + buf.len();
        download_temp.file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let download_temp = &mut self.temp.lock().unwrap();
        download_temp.file.flush()
    }
}

pub struct RegDownloaderTemp {
    file_size: usize,
    pub curr_size: usize,
    file: File,
    pub done: bool,
}