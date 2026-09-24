use std::{error, fmt, fs, intrinsics::write_bytes, io::Write, path::PathBuf};

use futures::{future::ok, io};
use reqwest::Response;
use serde::{Serialize, de::DeserializeOwned};

use crate::http_handler::DownloadError::BadVerify;

static DOWNLOAD_SIG: &str = "DOWNLOAD";

#[derive(Debug)]
pub enum DownloadError {
    BadRequest(reqwest::Error),
    BadFile(io::Error),
    BadVerify(String),
}

impl DownloadError
{
    fn err_str(s: &str) -> String
    {
        format!("[{}/ERROR] {}", DOWNLOAD_SIG, s)
    }
}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadRequest(err) => write!(
                f, "{}{}", Self::err_str("Bad request: "), err
            ),
            Self::BadFile(err) => write!(
                f, "{}{}", Self::err_str("Bad file: "), err
            ),
            Self::BadVerify(msg) => write!(
                f, "{}{}", Self::err_str("Bad verification: "), msg
            ),
        }
    }
}

impl error::Error for DownloadError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::BadRequest(err) => Some(err),
            Self::BadFile(err) => Some(err),
            _ => None
        }
    }
}

impl From<reqwest::Error> for DownloadError {
    fn from(value: reqwest::Error) -> Self {
        Self::BadRequest(value)
    }
}

impl From<std::io::Error> for DownloadError {
    fn from(value: std::io::Error) -> Self {
        Self::BadFile(value)
    }
}

struct Request<'a, T>
{
    client: &'a reqwest::Client,
    query: T
}

impl<'a, T> Request<'a, T>
where
    T: Serialize,
{
    pub fn build(client: &'a reqwest::Client, query: T) -> Request<'a, T>
    {
        Request { client, query }
    }

    async fn get_serialized_response(&self, url: &str) -> reqwest::Result<Response>
    {
        self.client.get(url)
            .query(&self.query)
            .send()
            .await
    }

    pub async fn retrieve_deserialized<U>(&self, url: &str) -> reqwest::Result<U>
    where
        U: DeserializeOwned
    {
        self.get_serialized_response(url).await?
            .json::<U>()
            .await
    }
}

pub struct Downloader<'a>
{
    client: &'a reqwest::Client
}
impl<'a> Downloader<'a>
{
    fn msg(s: &str) -> String
    {
        format!("[{}] {}", DOWNLOAD_SIG, s)
    }

    async fn retrieve_bytes(&self, url: &str) -> reqwest::Result<bytes::Bytes>
    {
        self.client.get(url)
            .send()
            .await?
            .bytes()
            .await
    }

    fn write_to_file(bytes: &[u8], out_dir: &PathBuf) -> io::Result<()>
    {
        fs::File::create(out_dir)?.write_all(bytes)?;
        println!("{}", Self::msg("Download successful"));
        Ok(())
    }

    pub async fn download(&self, url: &str, out_dir: &PathBuf) -> Result<(), DownloadError>
    {
        let bytes = self.retrieve_bytes(url).await?;
        
        Self::write_to_file(&bytes, out_dir)?;

        Ok(())
    }

    pub async fn verify_and_download<F>(&self, url: &str, out_dir: &PathBuf, verify: F) -> Result<(), DownloadError>
    where
        F: Fn(&[u8]) -> bool
    {
        let bytes = self.retrieve_bytes(url).await?;

        if verify(&bytes)
        {
            Ok(())
        }
        else {
            Err(BadVerify(String::from("Download could not be verified")))
        }
    }
}