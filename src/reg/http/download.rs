use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

use anyhow::{Error, Result};
use reqwest::blocking::{Client, Response};
use reqwest::Method;

use crate::progress::{Processor, ProcessorAsync, ProgressStatus};
use crate::reg::http::{do_request_raw, get_header, HttpAuth};
use crate::util::sha::Sha;

pub struct RegDownloader {
    url: String,
    auth: Option<HttpAuth>,
    client: Client,
    temp: Arc<Mutex<RegDownloaderTemp>>,
    file_path: String,
}

impl Processor<String> for RegDownloader {
    fn start(&self) -> Box<dyn ProcessorAsync<String>> {
        let arc = self.temp.clone();
        let reg_http_downloader = RegHttpDownloader {
            url: self.url.clone(),
            auth: self.auth.clone(),
            client: self.client.clone(),
        };
        let file_path_c = self.file_path.clone();
        let handle = thread::spawn::<_, Result<String>>(move || {
            let downloader = reg_http_downloader;
            let temp = arc.clone();
            let result = downloading(temp, file_path_c.as_str(), downloader);
            let mut download_temp = arc.lock().expect("lock failed");
            download_temp.done = true;
            if let Err(err) = &result {
                println!("{}\n{}", err, err.backtrace());
            }
            Ok(file_path_c)
        });
        Box::new(RegDownloadHandler {
            join: handle
        })
    }

    fn process_status(&self) -> Arc<Mutex<dyn ProgressStatus>> {
        self.temp.clone()
    }
}


pub struct RegDownloadHandler {
    join: JoinHandle<Result<String>>,
}

impl ProcessorAsync<String> for RegDownloadHandler {
    fn wait_result(self: Box<Self>) -> Result<String> {
        let result = self.join.join();
        result.unwrap()
    }
}

pub struct RegDownloadHandler2 {
    result: String,
}

impl ProcessorAsync<String> for RegDownloadHandler2 {
    fn wait_result(self: Box<Self>) -> Result<String> {
        Ok(self.result)
    }
}

impl RegDownloader {
    pub fn new_reg_downloader(
        url: String,
        auth: Option<HttpAuth>,
        client: Client,
        file_path: &str,
    ) -> Result<RegDownloader> {
        let temp = Arc::new(Mutex::new(RegDownloaderTemp {
            file_size: 0,
            curr_size: 0,
            done: false,
        }));
        Ok(RegDownloader {
            url,
            auth,
            client,
            temp,
            file_path: file_path.to_string(),
        })
    }
}

fn downloading(
    temp: Arc<Mutex<RegDownloaderTemp>>,
    file_path_str: &str,
    reg_http_downloader: RegHttpDownloader,
) -> Result<()> {
    //检查本地是否存在已有
    let file_path = Path::new(file_path_str);
    let parent_path = file_path.parent().expect("未找到目录");
    if !parent_path.exists() {
        std::fs::create_dir(parent_path)?
    }
    // 请求HTTP下载
    let mut http_response = reg_http_downloader.do_request_raw()?;
    check(&http_response)?;
    let file = File::create(file_path)?;
    let mut writer = RegDownloaderWriter { temp, file };
    let _copy_size = std::io::copy(&mut http_response, &mut writer)?;
    writer.flush()?;
    Ok(())
}

struct RegHttpDownloader {
    url: String,
    auth: Option<HttpAuth>,
    client: Client,
}

impl RegHttpDownloader {
    fn do_request_raw(&self) -> Result<Response> {
        let url = self.url.as_str();
        do_request_raw::<u8>(&self.client, url, Method::GET, &self.auth, &None, None)
    }
}

fn check(response: &Response) -> Result<()> {
    let headers = response.headers();
    let content_type = get_header(headers, "content-type").expect("content_type not found");
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
        DownloadFilenameType::Custom(custom) => custom.dir_path.join(&custom.file_name),
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

impl ProgressStatus for RegDownloaderTemp {
    fn full_size(&self) -> usize {
        self.file_size
    }

    fn now_size(&self) -> usize {
        self.curr_size
    }

    fn is_done(&self) -> bool {
        self.done
    }
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
