use std::{collections::HashMap, path::{PathBuf}};

use serde::{Deserialize, Deserializer, Serialize};
use sha2::{digest::Output, Digest, Sha256};

use crate::items::{self, ItemError};
use crate::http_handler::{self, HttpRequest};

static HANGAR_URL: &'static str = "https://hangar.papermc.io/api/v1";
static HANGAR_SIG: &'static str = "HANGAR";
static HANGAR_ITEM_ID: &'static str = "HA";

pub struct HangarItem{
    id: String,
    version_title: String,
    version_id: String,
    downloadable: HangarFile,
    dependencies: Vec<RequiredDependency>,
    downloaded: bool,
}
impl items::Item for HangarItem
{
    fn filename(&self) -> &PathBuf {
        self.downloadable.filename()
    }
    fn version_id(&self) -> &String {
        &self.version_id
    }
    fn id(&self) -> &String {
        &self.id
    }
    fn item_id(&self) -> &'static str {
        HANGAR_ITEM_ID
    }
    fn downloaded(&self) -> bool {
        self.downloaded
    }
    async fn download<'a>(
        &mut self,
        downloader: &http_handler::Downloader<'a>,
    ) -> ()
    {
        match self.downloadable.download(downloader).await
        {
            Ok(_) => self.downloaded = true,
            Err(err) => println!("{err}")
        }
    }
}

impl HangarItem
{
    async fn build_from_version(v: HangarVersion) -> Result<Self, ItemError>
    {
        todo!()
    }

    async fn build_from_id<'a>(
        v_requester: &http_handler::QueryRequest<'a, VersionQuery>,
        project_id: &str
    ) -> Result<Self, ItemError>
    {
        let url  = format!("{}/projects/{}/versions", HANGAR_URL, project_id);
        let mut response = v_requester
            .retrieve_deserialized::<HangarVersionResponse>(&url)
            .await?
            .result
        ;
        if response.len() == 0
        {
            return Err(ItemError::NoVersion(format!("No version available for id '{project_id}'")));
        }

        Self::build_from_version(response.swap_remove(0)).await
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HangarVersionResponse
{
    result: Vec<HangarVersion>
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HangarVersion
{
    id: u32,
    project_id: u32,
    name: String,
    downloads: HashMap<String, HangarFile>,
    #[serde(deserialize_with="deserialize_req_deps")]
    plugin_dependencies: HashMap<String, Vec<RequiredDependency>>

}

fn deserialize_req_deps<'de, D>(
    deserializer: D
) -> Result<HashMap<String, Vec<RequiredDependency>>, D::Error> 
    where D: Deserializer<'de>
{   
    let deps: HashMap<String, Vec<HangarDependency>> = Deserialize::deserialize(deserializer)?;
    let mut res = HashMap::<String, Vec<RequiredDependency>>::new();
    for (key, value) in deps
    {
        let req_deps = value
            .into_iter()
            .filter_map(|d| {
                if d.required { Some(RequiredDependency::from_dep(d)) }
                else { None } 
            })
            .collect()
        ;
        res.insert(key, req_deps);
    }

    Ok(res)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all="camelCase")]
struct HangarFile
{
    download_url: String,
    file_info: HangarFileInfo,
}

impl HangarFile
{
    pub async fn download<'a>(
        &self,
        downloader: &http_handler::Downloader<'a>,
    ) -> Result<(), http_handler::DownloadError>
    {
        downloader.verify_and_download(
            &self.download_url, 
            &self.file_info.name,
            |b| { self.file_info.verify_hash(&Sha256::digest(b))}
        ).await
    }
    fn filename(&self) -> &PathBuf
    {
        self.file_info.name()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all="camelCase")]
struct HangarFileInfo
{
    #[serde(deserialize_with="http_handler::deserialize_hex_str_to_bytes")]
    sha256_hash: Vec<u8>,
    name: PathBuf,
}
impl HangarFileInfo
{
    fn name(&self) -> &PathBuf
    {
        &self.name
    }
    fn verify_hash(&self, other: &Output<Sha256>) -> bool
    {
        self.sha256_hash.as_slice() == other.as_slice()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all="camelCase")]
struct HangarDependency
{
    name: String,
    project_id: u32,
    required: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all="camelCase")]
struct RequiredDependency
{
    name: String,
    project_id: u32
}

impl RequiredDependency
{
    fn from_dep(d: HangarDependency) -> Self
    {
        RequiredDependency {
            name: d.name,
            project_id: d.project_id,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all="camelCase")]
struct VersionQuery
{
    platform: String,
    platform_version: String,
    limit: u32,
}