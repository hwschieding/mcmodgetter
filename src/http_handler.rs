use std::{error, fmt, fs, io::{self, Write}, path::{Path, PathBuf}};

use reqwest::Response;
use serde::{Deserialize, Deserializer, Serialize, de::{DeserializeOwned, Error}};

use crate::http_handler;

static DOWNLOAD_SIG: &'static str = "DOWNLOAD";

#[derive(Debug)]
pub enum DownloadError {
    BadRequest(reqwest::Error),
    BadFile(io::Error),
    BadVerify(String),
    Unknown(String),
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
            Self::Unknown(msg) => write!(
                f, "{}{}", Self::err_str("Something weird: "), msg
            )
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

pub trait HttpRequest
{
    #[allow(async_fn_in_trait)]
    async fn get_serialized_response(
        &self,
        url: &str
    ) -> reqwest::Result<Response>;

    #[allow(async_fn_in_trait)]
    async fn retrieve_deserialized<U>(
        &self,
        url: &str
    ) -> reqwest::Result<U>
    where
        U: DeserializeOwned
    {
        self.get_serialized_response(url)
            .await?
            .json::<U>()
            .await
    }
}

pub struct BasicRequest<'a>
{
    client: &'a reqwest::Client
}

impl<'a> BasicRequest<'a>
{
    pub fn build(client: &'a reqwest::Client) -> BasicRequest<'a>
    {
        BasicRequest { client }
    }
}

impl<'a> HttpRequest for BasicRequest<'a>
{
    async fn get_serialized_response(
        &self,
        url: &str
    ) -> reqwest::Result<Response>
    {
        self.client.get(url)
            .send()
            .await
    }
}

#[derive(Serialize)]
pub struct EmptyQuery{}

pub struct QueryRequest<'a, T>
{
    client: &'a reqwest::Client,
    query: T,
    extra_queries: Vec<QueryParam>,
}

impl<'a, T> QueryRequest<'a, T>
where T: Serialize
{
    pub fn build(client: &'a reqwest::Client, query: T) -> QueryRequest<'a, T>
    {
        QueryRequest { client, query , extra_queries: Vec::new() }
    }
    pub fn add_query(&mut self, q: QueryParam) -> ()
    {
        self.extra_queries.push(q);
    }
}

impl<'a, T> HttpRequest for QueryRequest<'a, T>
where
    T: Serialize,
{
    async fn get_serialized_response(
        &self,
        url: &str
    ) -> reqwest::Result<Response>
    {
        let mut builder = self.client.get(url)
            .query(&self.query);
        for prm in &self.extra_queries
        {
            builder = builder.query(&[prm.get()]);
        }

        // let request = builder.build()?;
        // println!("Final URL: {}", request.url());

        // self.client.execute(request).await
        builder.send().await
    }
}

pub struct QueryParam
{
    parameter: String,
    value: String,
}

impl QueryParam
{
    pub fn new(parameter: &str, value: &str) -> Self
    {
        QueryParam {
            parameter: String::from(parameter),
            value: String::from(value),
        }
    }
    fn get(&self) -> (&String, &String)
    {
        (&self.parameter, &self.value)
    }
}

pub struct Downloader<'a>
{
    client: &'a reqwest::Client,
    output_directory: PathBuf,
}
impl<'a> Downloader<'a>
{
    pub fn build(client: &'a reqwest::Client, out_dir: PathBuf) -> Self
    {
        Downloader { client, output_directory: (out_dir) }
    }

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

    fn write_to_file(&self, bytes: &[u8], filename: &Path) -> Result<(), DownloadError>
    {
        match try_path_join(&self.output_directory, filename)
        {
            Some(p) => {
                fs::File::create(&p)?.write_all(bytes)?;
                println!("{} '{}'", Self::msg("Successfully downloaded"), filename.display());
                Ok(())
            }
            None => Err(DownloadError::Unknown(format!("Filename '{}' unsafe", filename.display())))
        }
    }

    pub async fn download(&self, url: &str, filename: &Path) -> Result<(), DownloadError>
    {
        let bytes = self.retrieve_bytes(url).await?;
        
        self.write_to_file(&bytes, filename)?;

        Ok(())
    }

    pub async fn verify_and_download<F>(
        &self,
        url: &str,
        filename: &Path,
        verify: F
    ) -> Result<(), DownloadError>
    where
        F: Fn(&[u8]) -> bool
    {
        let bytes = self.retrieve_bytes(url).await?;

        if verify(&bytes)
        {
            self.write_to_file(&bytes, filename)?;
            Ok(())
        }
        else {
            Err(http_handler::DownloadError::BadVerify(
                String::from("Download could not be verified")
            ))
        }
    }
}

pub fn deserialize_hex_str_to_bytes<'de, D>(
    deserializer: D
) -> Result<Vec<u8>, D::Error>
    where D: Deserializer<'de>
{
    let hex_data: String = Deserialize::deserialize(deserializer)?;
    hex::decode(hex_data).map_err(D::Error::custom)
}

fn try_path_join(base: &PathBuf, new_comp: &Path) -> Option<PathBuf>
{
    
    if new_comp.is_relative() { Some(base.join(new_comp)) } else { None }
}