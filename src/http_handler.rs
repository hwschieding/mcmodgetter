use std::{error, fmt, path::{Path, PathBuf}};

use tokio::{fs, io::{self, AsyncWriteExt}};
use reqwest::Response;
use serde::{Deserialize, Deserializer, Serialize, de::{DeserializeOwned, Error}};
use futures_util::StreamExt;

use crate::{arguments};

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

pub trait HashCheck {
    fn add_chunk(&mut self, b: &bytes::Bytes);
    fn compare(self) -> bool;
}

pub struct BasicRequest
{
    client: reqwest::Client
}

impl BasicRequest
{
    pub fn build(client: reqwest::Client) -> BasicRequest
    {
        BasicRequest { client }
    }
}

impl HttpRequest for BasicRequest
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
pub enum Channels
{
    None,
    Release,
}

pub trait Query : Sized
{
    fn build_query(user_version: &String, user_platforms: &Vec<arguments::Platform>) -> Option<Self>;
    fn set_channel(&mut self, new: Channels);
    fn build_version_array(user_params: &String) -> String {
        let mut params = user_params.split(",");
        let mut res = QArrayBuilder::new_from(
            params.next().unwrap_or("")
        );

        while let Some(prm) = params.next() {
            res.push(prm);
        }

        res.build()
    }
    fn build_platform_array(user_platforms: &Vec<arguments::Platform>) -> String
    {
        let mut platforms = user_platforms.iter();
        let mut res = QArrayBuilder::new_from(match platforms.next()
            {
                Some(l) => l.to_str(),
                None => "",
            }
        );

        while let Some(l) = platforms.next()
        {
            res.push(l.to_str());
        }
        
        res.build()
    }
}

struct QArrayBuilder
{
    arr: String
}

impl QArrayBuilder
{
    pub fn new_from(start: &str) -> Self
    {
        QArrayBuilder { arr: format!("[\"{}\"", start) }
    }
    pub fn push(&mut self, elem: &str) -> ()
    {
        let parts = [",\"", elem, "\""];
        self.arr.push_str(&parts.concat());
    }
    pub fn build(mut self) -> String
    {
        self.arr.push_str("]");
        return self.arr
    }
}

pub struct QueryRequest<T>
{
    client: reqwest::Client,
    query: T,
    extra_queries: Vec<QueryParam>,
}

impl<T> QueryRequest<T>
where T: Serialize + Query
{
    pub fn build(client: reqwest::Client, query: T) -> QueryRequest<T>
    {
        QueryRequest { client, query , extra_queries: Vec::new() }
    }
    pub fn add_query(&mut self, q: QueryParam) -> ()
    {
        self.extra_queries.push(q);
    }
}

impl<T> HttpRequest for QueryRequest<T>
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

pub struct Downloader
{
    client: reqwest::Client,
    output_directory: PathBuf,
}
impl Downloader
{
    pub fn build(client: reqwest::Client, out_dir: PathBuf) -> Self
    {
        Downloader { client, output_directory: (out_dir) }
    }

    fn msg(s: &str) -> String
    {
        format!("[{}] {}", DOWNLOAD_SIG, s)
    }
    async fn try_stream_download<T>(
        url_response: reqwest::Response,
        file: &mut fs::File,
        checker: &mut T
    ) -> Result<(), DownloadError>
    where
        T: HashCheck
    {
        let mut stream = url_response.bytes_stream();
        while let Some(chunk) = stream.next().await
        {
            let chunk = chunk?;
            checker.add_chunk(&chunk);
            file.write_all(&chunk).await?;
        }
        Ok(())
    }

    pub async fn verify_and_download(
        &self,
        url: &str,
        filename: &Path,
        mut checker: impl HashCheck,
    ) -> Result<(), DownloadError>
    {

        let res = self.client
            .get(url)
            .send()
            .await?
            .error_for_status()?
        ;

        let dest_path = try_path_join(&self.output_directory, filename)?;
        let temp_path = dest_path.with_added_extension(".tmp");

        let mut file = fs::File::create(&temp_path).await?;

        if let Err(e) = Self::try_stream_download(res, &mut file, &mut checker).await
        {
            fs::remove_file(&temp_path).await?;
            return Err(e)
        }

        file.flush().await?;
        drop(file);

        if checker.compare()
        {
            fs::rename(temp_path, &dest_path).await?;
            println!("{} '{}'", Self::msg("Successfully downloaded"), dest_path.display());
            Ok(())
        }
        else
        {
            fs::remove_file(temp_path).await?;
            Err(DownloadError::BadVerify(
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

fn try_path_join(base: &PathBuf, new_comp: &Path) -> Result<PathBuf, DownloadError>
{    
    if new_comp.is_relative()
    {
        Ok(base.join(new_comp))
    }
    else
    {
        Err(DownloadError::Unknown(
            format!("Filename '{}' is unsafe", new_comp.display())
        ))
    }
}