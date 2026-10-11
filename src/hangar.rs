use std::{collections::HashMap, path::{PathBuf}};

use serde::{Deserialize, Deserializer, Serialize};
use sha2::{digest::Output, Digest, Sha256};

use crate::{arguments, items::{self, Dependency, DependencyRequester, ItemError}};
use crate::http_handler::{self, HttpRequest};

static HANGAR_URL: &'static str = "https://hangar.papermc.io/api/v1";
static HANGAR_SIG: &'static str = "HANGAR";
static HANGAR_ITEM_ID: &'static str = "HA";

fn hangar_msg(msg: &str) -> String
{
    format!("[{}] {}", HANGAR_SIG, msg)
}

pub struct HangarItem{
    id: String,
    version_title: String,
    version_id: String,
    platform: arguments::Platform,
    downloadable: HangarFile,
    dependencies: Vec<HangarReqDep>,
    downloaded: bool,
}
impl items::Item for HangarItem
{
    type Dep = HangarReqDep;
    type VersionQuery = HangarVerQuery;

    fn version_title(&self) -> &String {
        &self.version_title
    }
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
    async fn get_filtered_deps<'a, F>(
        &self,
        requester: &items::DependencyRequester<'a, HangarVerQuery>,
        verify: F,
    ) -> Vec<Self>
    where
        F: Fn(&HangarReqDep) -> bool
    {
        let mut res = Vec::<Self>::new();
        for dep in self.dependencies.iter()
            .filter(|d| { verify(d) })
            .collect::<Vec<_>>()
        {
            match Self::try_dep(requester, dep, &self.platform).await
            {
                Some(d) => res.push(d),
                None => {
                    println!("{}", hangar_msg(&format!(
                        "Couldn't acquire dependency for id '{}'", self.id
                    )))
                }
            }
        }
        
        res
    }
}

impl HangarItem
{
    async fn build_from_version(
        mut v: HangarVersion,
        platform: &arguments::Platform
    ) -> Result<Self, ItemError>
    {
        let upper_platform_str = &platform.to_str().to_uppercase();

        let downloadable = v.downloads
            .remove(upper_platform_str)
            .ok_or(ItemError::NoFile(format!(
                "Unable to acquire version file for id '{}'", v.id
            )))?
        ;
        let dependencies = match v.plugin_dependencies
            .remove(upper_platform_str)
        {
            Some(d) => d,
            None => Vec::new()
        };

        Ok(Self {
            id: v.project_id.to_string(),
            version_title: v.name,
            version_id: v.id.to_string(),
            platform: platform.clone(),
            downloadable,
            dependencies,
            downloaded: false
        })
    }

    async fn build_from_id<'a>(
        v_requester: &http_handler::QueryRequest<'a, HangarVerQuery>,
        project_id: &str,
        platform: &arguments::Platform
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

        Self::build_from_version(response.swap_remove(0), platform).await
    }

    async fn try_dep<'a>(
        requester: &DependencyRequester<'a, HangarVerQuery>,
        dep: &HangarReqDep,
        platform: &arguments::Platform
    ) -> Option<Self>
    {
        let result = match dep.project_id()
        {
            Some(id) => {
                Self::build_from_id(
                    requester.project_id_requester(),
                    id,
                    platform
                ).await
            }
            None => { return None }
        };

        match result
        {
            Ok(i) => Some(i),
            Err(_) => None
        }
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
    plugin_dependencies: HashMap<String, Vec<HangarReqDep>>

}

fn deserialize_req_deps<'de, D>(
    deserializer: D
) -> Result<HashMap<String, Vec<HangarReqDep>>, D::Error> 
    where D: Deserializer<'de>
{   
    let deps: HashMap<String, Vec<HangarDependency>> = Deserialize::deserialize(deserializer)?;
    let mut res = HashMap::<String, Vec<HangarReqDep>>::new();
    for (key, value) in deps
    {
        let req_deps = value
            .into_iter()
            .filter_map(|d| {
                if d.required { Some(HangarReqDep::from_dep(d)) }
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
    project_id: u32,
    required: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct HangarReqDep
{
    project_id: String
}

impl HangarReqDep
{
    fn from_dep(d: HangarDependency) -> Self
    {
        HangarReqDep {
            project_id: d.project_id.to_string(),
        }
    }
}

impl items::Dependency for HangarReqDep
{
    fn project_id(&self) -> Option<&String> {
        Some(&self.project_id)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all="camelCase")]
pub struct HangarVerQuery
{
    platform: String,
    platform_version: String,
    limit: u32,
}

impl http_handler::Query for HangarVerQuery {}