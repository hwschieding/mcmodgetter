use std::sync::Arc;
use std::{collections::HashMap, error, path::PathBuf};

use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use futures::future;
use tokio::{sync::Semaphore, task::JoinSet};

use crate::http_handler::{self, HttpRequest, BasicRequest, QueryRequest, HashCheck, Query};
use crate::arguments::{self, DownloadSettings};
use crate::items::{self, Dependency, DependencyHandler, DependencyRequester, Item, ItemError};
use crate::file_parse::TrackerFile;

static HANGAR_URL: &'static str = "https://hangar.papermc.io/api/v1";
static HANGAR_SIG: &'static str = "HANGAR";
static HANGAR_ITEM_ID: &'static str = "HA";

fn hangar_msg(msg: &str) -> String
{
    format!("[{}] {}", HANGAR_SIG, msg)
}

#[derive(Debug)]
pub struct HangarItem{
    id: String,
    version_title: String,
    version_id: String,
    platform: arguments::Platform,
    downloadable: HangarFile,
    dependencies: Vec<HangarReqDep>,
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
    async fn download(
        &self,
        downloader: &http_handler::Downloader,
    ) -> Result<(), http_handler::DownloadError>
    {
        self.downloadable.download(downloader).await
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

        let downloadable = HangarFile::from_response(v.downloads
            .remove(upper_platform_str)
            .ok_or(ItemError::NoFile(format!(
                "Unable to acquire version file for id '{}'", v.id
            )))?
        )?;
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
        })
    }

    async fn build_from_id(
        v_requester: &http_handler::QueryRequest<HangarVerQuery>,
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
    downloads: HashMap<String, HangarFileResponse>,
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
struct HangarFileResponse
{
    download_url: Option<String>,
    file_info: Option<HangarFileInfo>,
}

impl HangarFileResponse
{
    
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
    // fn verify_hash(&self, other: &Output<Sha256>) -> bool
    // {
    //     self.sha256_hash.as_slice() == other.as_slice()
    // }
    fn get_hash_checker<'a>(&'a self) -> HangarHashCheck<'a>
    {
        HangarHashCheck::new(&self.sha256_hash)
    }
}

struct HangarHashCheck<'a>
{
    sha512: &'a Vec<u8>,
    hasher: Sha256,
}

impl<'a> HangarHashCheck<'a>
{
    fn new(sha: &'a Vec<u8>) -> Self
    {
        Self {
            sha512: sha,
            hasher: Sha256::new()
        }
    }
}

impl<'a> HashCheck for HangarHashCheck<'a>
{
    fn add_chunk(&mut self, b: &bytes::Bytes) {
        self.hasher.update(b);
    }
    fn compare(self) -> bool
    {
        self.hasher.finalize().as_slice() == self.sha512.as_slice()
    }
}

#[derive(Debug)]
struct HangarFile
{
    url: String,
    info: HangarFileInfo,
}

impl HangarFile
{
    pub fn from_response(response: HangarFileResponse) -> Result<Self, ItemError>
    {
        let url = response.download_url.ok_or(ItemError::NoFile(
            String::from("No direct download link available")
        ))?;
        let info = response.file_info.ok_or(ItemError::NoFile(
            String::from("File information could not be acquired")
        ))?;

        Ok(Self { url, info })
    }
    pub async fn download(
        &self,
        downloader: &http_handler::Downloader,
    ) -> Result<(), http_handler::DownloadError>
    {
        downloader.verify_and_download(
            &self.url,
            &self.info.name(),
            self.info.get_hash_checker(),
        ).await
    }
    fn filename(&self) -> &PathBuf
    {
        self.info.name()
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
    platform_version: Option<String>,
    limit: u32,
    channel: Option<String>,
}

impl HangarVerQuery
{
    fn get_version(v_list: &String) -> Option<String>
    {
        match v_list.split(',').next()
        {
            Some(v) => Some(String::from(v)),
            None => None,
        }
    }

    fn get_plaform(p: &arguments::Platform) -> String
    {
        p.to_string()
    }
}

impl http_handler::Query for HangarVerQuery
{
    fn build_query(
        user_version: &String,
        user_platforms: &Vec<arguments::Platform>
    ) -> Option<Self>
    {
        let first_platform = match user_platforms.get(0)
        {
            Some(p) => p,
            None => {return None},
        };

        Some(Self{
            platform: Self::get_plaform(first_platform),
            platform_version: Self::get_version(user_version),
            limit: 1,
            channel: None,
        })
    }
    fn set_channel(&mut self, new: http_handler::Channels) {
        self.channel = match new
        {
            http_handler::Channels::None => None,
            http_handler::Channels::Release => Some(String::from("release")),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all="camelCase")]
struct ProjectNamespace
{
    owner: String,
    slug: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all="camelCase")]
struct HangarProject
{
    id: u32,
    name: String,
    description: String,
    namespace: ProjectNamespace,
}

impl HangarProject
{
    async fn build(
        requester: &BasicRequest,
        id: &str,
    ) -> reqwest::Result<Self>
    {
        let url = format!("{}/projects/{}", HANGAR_URL, id);
        requester.retrieve_deserialized::<Self>(&url).await
    }

    fn list_info(&self) -> ()
    {
        println!(
            "\n{} ({}) -> {}, published by {}\n{}",
            self.id,
            self.namespace.slug,
            self.name,
            self.namespace.owner,
            self.description
        )
    }
}

async fn collect_plugins(
    requester: Arc<QueryRequest<HangarVerQuery>>,
    ids: Vec<String>,
    platform: Arc<arguments::Platform>,
) -> Vec<HangarItem>
{
    fn err_or_return(res: Result<HangarItem, ItemError>) -> Option<HangarItem>
    {
        match res
        {
            Ok(i) => {
                println!("{}", hangar_msg(&format!(
                    "Found version '{}'", i.version_title()
                )));
                Some(i)
            }
            Err(e) => { eprintln!("{e}"); None}
        }
    }

    let mut set = JoinSet::new();
    let semaphore = Arc::new(Semaphore::new(8));
    for id in ids
    {
        let sem_clone = Arc::clone(&semaphore);
        let req_clone = Arc::clone(&requester);
        let plat_clone = Arc::clone(&platform);

        set.spawn(async move
        {
            let _permit = match sem_clone.acquire().await
            {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Async HTTP request failed: {e}");
                    return None
                }
            };

            err_or_return(HangarItem::build_from_id(
                &req_clone,
                &id,
                &plat_clone
            ).await)
        });
    }

    let mut items = Vec::<HangarItem>::new();

    while let Some(i_res) = set.join_next().await
    {
        if let Ok(Some(i)) = i_res
        {
            items.push(i);
        }
    }

    items
    // let mut plugins = Vec::new();
    // for id in ids
    // {
    //     plugins.push(HangarItem::build_from_id(requester, id, platform))
    // }
    // future::join_all(plugins).await
    //     .into_iter()
    //     .filter_map(|result|
    //     {
    //         err_or_return(result)
    //     })
        // .collect::<Vec<_>>()
}

async fn plugin_list_from_ids<'a>(
    conf: &arguments::Config<'a, &DownloadSettings>,
    client: reqwest::Client,
    ids: Vec<String>,
) -> Result<Vec<HangarItem>, Box<dyn error::Error>>
{
    let platform = conf.settings()
        .platform()
        .try_get_top()
        .ok_or("Plugin platform must be specified for Hangar queries")?
        .clone()
    ;

    let mut query = HangarVerQuery::build_query(
        conf.settings().version_search(),
        &vec![platform.clone()]
    ).ok_or("Unable to build query")?;

    if conf.opts().release_only()
    {
        query.set_channel(http_handler::Channels::Release);
    }

    let pid_requester = Arc::new(QueryRequest::build(client.clone(), query));
    let platform = Arc::new(platform);
    let mut plugins = collect_plugins(Arc::clone(&pid_requester), ids, platform).await;

    if conf.opts().skip_deps()
    {
        return Ok(plugins)
    }

    let vid_requester = BasicRequest::build(client.clone());
    let dep_requester = DependencyRequester::new(
        &pid_requester,
        &vid_requester,
    );

    let mut dep_handler = DependencyHandler::build(&mut plugins);
    dep_handler.acquire_all_dependencies(&dep_requester).await;

    Ok(plugins)
}

pub async fn list_projects(
    client: reqwest::Client,
    id_list: &Vec<String>,
) -> () {
    let pid_requester = http_handler::BasicRequest::build(client.clone());

    let items: Vec<_> = id_list
        .iter()
        .map(|id| {
            HangarProject::build(&pid_requester, id)
        })
        .collect()
    ;
    
    for item_result in future::join_all(items).await
    {
        match item_result {
            Ok(item) => item.list_info(),
            Err(err) => println!("Couldn't retrieve ID: {err}")
        }
    };

    ()
}

pub async fn download_from_id_list<'a>(
    conf: &arguments::Config<'a, &DownloadSettings>,
    client: reqwest::Client,
    ids: Vec<String>,
    out_dir: PathBuf
) -> Result<(), Box<dyn error::Error>>
{
    let mut items = plugin_list_from_ids(conf, client.clone(), ids).await?;

    let mut tracker: TrackerFile = TrackerFile::build(&out_dir)?;
    items = items::filter_items_by_tracker(items, &tracker);

    if !items::ask_user_to_download(&out_dir)?
    {
        return Ok(())
    }

    println!("{}", hangar_msg("Downloading..."));

    // Download
    let downloader = Arc::new(http_handler::Downloader::build(client.clone(), out_dir));
    let successful = items::download_items(downloader, items).await?;

    tracker.update(successful);
    tracker.write_csv();

    println!("{}", hangar_msg("Download finished"));

    Ok(())
}

pub async fn verify_ids_from_list<'a>(
    conf: &arguments::Config<'a, &DownloadSettings>,
    client: reqwest::Client,
    ids: Vec<String>,
    out_dir: PathBuf
) -> Result<(), Box<dyn error::Error>> {
    let items = plugin_list_from_ids(conf, client, ids).await?;

    let tracker: TrackerFile = TrackerFile::build(&out_dir)?;

    items::filter_items_by_tracker(items, &tracker);

    Ok(())
}

pub async fn download_from_id<'a>(
    conf: &arguments::Config<'a, &DownloadSettings>,
    client: reqwest::Client,
    id: &str,
    out_dir: PathBuf
) -> Result<(), Box<dyn error::Error>>
{   
    download_from_id_list(conf, client, vec![id.to_string()], out_dir).await?;

    Ok(())
}

pub async fn verify_id<'a> (
    conf: &arguments::Config<'a, &DownloadSettings>,
    client: reqwest::Client,
    id: &str,
    out_dir: PathBuf
) -> Result<(), Box<dyn error::Error>> {
    
    verify_ids_from_list(conf, client, vec![id.to_string()], out_dir).await?;

    Ok(())
}