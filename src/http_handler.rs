use std::{fs, path::PathBuf, io::Write};

use futures::io;
use reqwest::Response;
use serde::{Serialize, de::DeserializeOwned};

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
    client: &'a reqwest::Client,
    download_bytes: Option<bytes::Bytes>
}
impl<'a> Downloader<'a>
{
    pub fn download_bytes(&self) -> &Option<bytes::Bytes>
    {
        &self.download_bytes
    }
    pub async fn retrieve_bytes(&mut self, url: &str) -> reqwest::Result<()>
    {
        let bytes = self.client.get(url)
            .send()
            .await?
            .bytes()
            .await?;

        self.download_bytes = Some(bytes);
        Ok(())
    }

    pub async fn download(&self, out_dir: &PathBuf) -> io::Result<()>
    {
        match self.download_bytes
        {
            Some(ref b) => {
                fs::File::create(out_dir)?.write_all(b)?;
            }
            None => {
                println!("Bad download call, Downloader has no bytes");
            }
        }
        
        Ok(())
    }
}