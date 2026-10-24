use std::sync::Arc;
use std::{error};
use std::path::{PathBuf};
use futures::future;
use serde::{Serialize, Deserialize, Deserializer};
use sha2::{Sha512, Digest};
use tokio::{sync::Semaphore, task::JoinSet};

use crate::arguments::{DownloadSettings};
use crate::file_parse::TrackerFile;
use crate::http_handler::{Downloader, HashCheck, Query, QueryRequest};
use crate::items::{ItemError, DependencyHandler, DependencyRequester};
use crate::{arguments, http_handler::{self, HttpRequest}, items::{self, Item}};

static MODRINTH_URL: &'static str = "https://api.modrinth.com/v2";
static MODRINTH_SIG: &'static str = "MODRINTH";

static MODRINTH_ITEM_ID: &'static str = "MR";

fn modrinth_msg(s: String) -> ()
{
    println!("[{}] {}", MODRINTH_SIG, s)
}



pub struct ModrinthItem
{
    id: String,
    version_title: String,
    version_id: String,
    downloadable: ModrinthFile,
    dependencies: Vec<ModrinthReqDep>,
}
impl Item for ModrinthItem
{
    type Dep = ModrinthReqDep;
    type VersionQuery = ModrinthVerQuery;

    async fn download<'b>(
        &self,
        downloader: &'b Downloader,
    ) -> Result<(), http_handler::DownloadError>
    {
        self.downloadable.download(downloader).await
    }
    fn id(&self) -> &String
    {
        &self.id
    }
    fn version_id(&self) -> &String
    {
        &self.version_id
    }
    fn version_title(&self) -> &String
    {
        &self.version_title
    }
    fn filename(&self) -> &PathBuf
    {
        self.downloadable.filename()
    }
    fn item_id(&self) -> &'static str {
        MODRINTH_ITEM_ID
    }
    async fn get_filtered_deps<'a, F>(
        &self,
        requester: &DependencyRequester<'a, ModrinthVerQuery>,
        verify: F,
    ) -> Vec<Self>
    where
        F: Fn(&ModrinthReqDep) -> bool
    {
        let mut res = Vec::<Self>::new();

        for dep in self.dependencies
            .iter()
            .filter(|d| verify(*d))
            .collect::<Vec<_>>()
        {
            match Self::try_dep(requester, dep).await
            {
                Some(m) => { res.push(m); },
                None => { modrinth_msg(format!("Couldn't acquire dependency for id '{}'", self.id)); }
            }
        }

        res
    }
}
impl ModrinthItem
{
    fn build_from_version(
        mut version: ModrinthVersion
    ) -> Result<Self, ItemError>
    {
        if version.files.len() == 0
        {
            return Err(ItemError::NoFile(String::from("No files for this version")));
        }

        let downloadable: ModrinthFile = match version.get_primary_file()
        {
            Some(idx) => version.files.swap_remove(idx),
            None => version.files.swap_remove(0)
        };

        Ok(ModrinthItem {
            id: version.project_id,
            version_title: version.name,
            version_id: version.id,
            downloadable,
            dependencies: version.dependencies,
        })
    }
    
    pub async fn build_from_id<'a>(
        version_requester: &http_handler::QueryRequest<ModrinthVerQuery>,
        project_id: &str
    ) -> Result<Self, ItemError>
    {
        let url = format!("{}/project/{}/version", MODRINTH_URL, project_id);

        let mut versions = version_requester
            .retrieve_deserialized::<Vec<ModrinthVersion>>(&url)
            .await?;

        if versions.len() == 0
        {
            return Err(ItemError::NoVersion(format!("No version for id '{}'", project_id)));
        }

        let selected_version = versions.swap_remove(0);

        Self::build_from_version(selected_version)
    }

    async fn build_from_version_id<'a>(
        direct_requester: &http_handler::BasicRequest,
        version_id: &str
    ) -> Result<Self, ItemError>
    {
        let url = format!("{}/version/{}", MODRINTH_URL, version_id);
        Self::build_from_version(
            direct_requester.retrieve_deserialized::<ModrinthVersion>(&url).await?
        )
    }

    async fn try_dep<'a>(
        requester: &DependencyRequester<'a, ModrinthVerQuery>,
        dep: &ModrinthReqDep
    ) -> Option<Self>
    {
        if let Some(id) = &dep.version_id
            && let Ok(m) = Self::build_from_version_id(
                requester.version_id_requester(),
                id
            ).await
        {
            Some(m)
        }
        else if let Some(id) = &dep.project_id
            && let Ok(m) = Self::build_from_id(
                requester.project_id_requester(),
                id
            ).await
        {
            Some(m)
        }
        else
        {
            None
        }
    }

    async fn _get_dependencies<'a>(
        &self,
        requester: &DependencyRequester<'a, ModrinthVerQuery>
    ) -> Vec<Self>
    {
        self.get_filtered_deps(requester, |_| { true }).await
    }
}

#[derive(Deserialize)]
struct ModrinthVersion
{
    name: String,
    id: String,
    project_id: String,
    files: Vec<ModrinthFile>,
    #[serde(deserialize_with = "deserialize_only_required_deps")]
    dependencies: Vec<ModrinthReqDep>,
}

impl ModrinthVersion
{
    fn get_primary_file(&self) -> Option<usize>
    {
        self.files.iter().position(|f| f.primary)
    }
}

impl Clone for ModrinthVersion
{
    fn clone(&self) -> Self {
        ModrinthVersion
        {
            name: self.name.clone(),
            id: self.id.clone(),
            project_id: self.project_id.clone(),
            files: self.files.clone(),
            dependencies: self.dependencies.clone()
        }
    }
}

#[derive(Deserialize)]
pub struct ModrinthFile {
    url: String,
    filename: PathBuf,
    primary: bool,
    hashes: ModrinthFileHash,
}
impl ModrinthFile {
    pub fn url(&self) -> &String {
        &self.url
    }
    pub fn filename(&self) -> &PathBuf {
        &self.filename
    }
    pub fn primary(&self) -> &bool {
        &self.primary
    }

    pub async fn download<'a>(
        &self,
        downloader: &http_handler::Downloader,
    ) -> Result<(), http_handler::DownloadError>
    {
        downloader.verify_and_download(
            &self.url,
            &self.filename,
            self.hashes.get_hash_checker(),
        ).await
    }
}

impl Clone for ModrinthFile {
    fn clone(&self) -> Self {
        ModrinthFile {
            url: self.url.clone(),
            filename: self.filename.clone(),
            primary: self.primary,
            hashes: self.hashes.clone()
        }
    }
}

#[derive(Deserialize)]
struct ModrinthFileHash {
    #[serde(deserialize_with = "http_handler::deserialize_hex_str_to_bytes")]
    sha512: Vec<u8>
}

impl ModrinthFileHash {
    // pub fn check512(&self, other_hash: &Output<Sha512>) -> bool {
    //     self.sha512.as_slice() == other_hash.as_slice()
    // }
    pub fn get_hash_checker<'a>(&'a self) -> ModrinthHashCheck<'a>
    {
        ModrinthHashCheck::new(&self.sha512)
    }
}

impl Clone for ModrinthFileHash {
    fn clone(&self) -> Self {
        ModrinthFileHash {
            sha512: self.sha512.clone()
        }
    }
}

struct ModrinthHashCheck<'a>
{
    sha512: &'a Vec<u8>,
    hasher: Sha512,
}

impl<'a> ModrinthHashCheck<'a>
{
    fn new(sha: &'a Vec<u8>) -> Self
    {
        Self {
            sha512: sha,
            hasher: Sha512::new()
        }
    }
}

impl<'a> HashCheck for ModrinthHashCheck<'a>
{
    fn add_chunk(&mut self, b: &bytes::Bytes) {
        self.hasher.update(b);
    }
    fn compare(self) -> bool
    {
        self.hasher.finalize().as_slice() == self.sha512.as_slice()
    }
}

#[derive(Serialize)]
pub struct ModrinthVerQuery {
    game_versions: String,
    loaders: String,
    include_changelog: bool,
    limit: u32,
    version_type: Option<String>,
}

impl ModrinthVerQuery {
    pub fn mcvs(&self) -> &str {
        &self.game_versions.as_str()
    }
    pub fn loader(&self) -> &str {
        &self.loaders.as_str()
    }
}

impl Query for ModrinthVerQuery {
    fn build_query(
        user_mcvs: &String,
        user_loader: &Vec<arguments::Platform>
    ) -> Option<Self> {
        let game_versions= Self::build_version_array(user_mcvs);
        let loaders= Self::build_platform_array(user_loader);
        Some(Self {
            game_versions,
            loaders,
            include_changelog: false,
            limit: 1,
            version_type: None,
        })
    }
    fn set_channel(&mut self, new: http_handler::Channels) {
        self.version_type = match new
        {
            http_handler::Channels::None => None,
            http_handler::Channels::Release => Some(String::from("release")),
        }
    }
}

#[derive(Deserialize)]
pub struct ModrinthDependency {
    version_id: Option<String>,
    project_id: Option<String>,
    dependency_type: String
}

pub struct ModrinthReqDep {
    version_id: Option<String>,
    project_id: Option<String>,
}

impl ModrinthReqDep {
    pub fn from_dep(dep: ModrinthDependency) -> Self {
        ModrinthReqDep {
            version_id: dep.version_id,
            project_id: dep.project_id
        }
    }
    pub fn version_id(&self) -> &Option<String> {
        &self.version_id
    }
}

impl items::Dependency for ModrinthReqDep
{
    fn project_id(&self) -> Option<&String> {
        match &self.project_id
        {
            Some(v) => Some(v),
            None => None
        }
    }
}

impl Clone for ModrinthReqDep {
    fn clone(&self) -> Self {
        ModrinthReqDep {
            version_id: self.version_id.clone(),
            project_id: self.project_id.clone(),
        }
    }
}

fn deserialize_only_required_deps<'de, D>(
    deserializer: D
) -> Result<Vec<ModrinthReqDep>, D::Error> 
    where D: Deserializer<'de>
{
    let deps: Vec<ModrinthDependency> = Deserialize::deserialize(deserializer)?;
    Ok (deps.into_iter()
        .filter_map(|d|
            if d.dependency_type == "required" {
                Some(ModrinthReqDep::from_dep(d))
            } else {
                None
            }
        )
        .collect()
    )
}

async fn collect_mods<'a>(
    requester: Arc<http_handler::QueryRequest<ModrinthVerQuery>>,
    ids: Vec<String>,
) -> Vec<ModrinthItem>
{
    let mut set = JoinSet::new();
    let semaphore = Arc::new(Semaphore::new(8));

    for id in ids
    {
        let sem_clone = Arc::clone(&semaphore);
        let req_clone = Arc::clone(&requester);
        set.spawn(async move 
        {
            let _permit = match sem_clone.acquire().await
            {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Async HTTP request failed: {e}");
                    return None;
                }
            };
            match ModrinthItem::build_from_id(&req_clone, &id).await
            {
                Ok(i) => {
                    modrinth_msg(format!("Found '{}'", i.version_title()));
                    Some(i)
                },
                Err(e) => { eprintln!("{e}"); None }
            }
        });
    }
    let mut items = Vec::<ModrinthItem>::new();
    while let Some(i_res) = set.join_next().await
    {
        if let Ok(Some(i)) = i_res
        {
            items.push(i);
        }
    }
    items
    // let mut mods = Vec::new();
    // for id in ids {
    //     mods.push(ModrinthItem::build_from_id(requester, id));
    // }
    // future::join_all(mods)
    // .await
    // .into_iter()
    // .filter_map(|m_res| {
    //     match m_res {
    //         Err(e) => {
    //             println!("{e}");
    //             None
    //         }
    //         Ok(m) => {
    //             modrinth_msg(format!("Found '{}'", m.version_title()));
    //             Some(m)
    //         }
    //     }
    // })
    // .collect()
}

pub async fn build_modlist_from_ids<'a>(
    conf: &arguments::Config<'a, &DownloadSettings>,
    client: reqwest::Client,
    ids: Vec<String>,
) -> Result<Vec<ModrinthItem>, Box<dyn error::Error>>
{
    let mut query = ModrinthVerQuery::build_query(
        conf.settings().version_search(),
        conf.settings().platform().get(),
    ).ok_or("Unable to build query")?;

    if conf.opts().release_only()
    {
        query.set_channel(http_handler::Channels::Release);
    }

    let pid_requester = Arc::new(QueryRequest::<ModrinthVerQuery>::build(
        client.clone(),
        query
    ));

    // Get modlist
    let mut items = collect_mods(Arc::clone(&pid_requester), ids).await;

    if conf.opts().skip_deps()
    {
        return Ok(items)
    }

    // Get dependencies
    let vid_requester = http_handler::BasicRequest::build(client.clone());
    let dep_requester = DependencyRequester::new(
        &pid_requester,
        &vid_requester
    );
    let mut dep_handler = DependencyHandler::build(&mut items);
    dep_handler.acquire_all_dependencies(&dep_requester).await;

    Ok(items)
}

pub async fn download_from_id_list<'a>(
    conf: &arguments::Config<'a, &DownloadSettings>,
    client: reqwest::Client,
    ids: Vec<String>,
    out_dir: PathBuf
) -> Result<(), Box<dyn error::Error>>
{
    let mut items = build_modlist_from_ids(conf, client.clone(), ids).await?;

    let mut tracker: TrackerFile = TrackerFile::build(&out_dir)?;
    items = items::filter_items_by_tracker(items, &tracker);

    if !items::ask_user_to_download(&out_dir)?
    {
        return Ok(())
    }

    modrinth_msg(String::from("Downloading..."));

    // Download
    let downloader = Arc::new(http_handler::Downloader::build(client, out_dir));
    let successful = items::download_items(downloader, items).await?;

    tracker.update(successful);
    tracker.write_csv();

    modrinth_msg(String::from("Download finished"));

    Ok(())
}

pub async fn verify_ids_from_list<'a>(
    conf: &arguments::Config<'a, &DownloadSettings>,
    client: reqwest::Client,
    ids: Vec<String>,
    out_dir: PathBuf
) -> Result<(), Box<dyn error::Error>> {
    let items = build_modlist_from_ids(conf, client, ids).await?;

    let tracker: TrackerFile = TrackerFile::build(&out_dir)?;

    items::filter_items_by_tracker(items, &tracker);

    Ok(())
}

#[derive(Deserialize)]
struct ModrinthProject
{
    id: String,
    title: String,
    description: String,
    slug: Option<String>,
}
impl ModrinthProject
{
    async fn build(
        requester: &http_handler::BasicRequest,
        id: &str,
    ) -> reqwest::Result<Self>
    {
        let url = format!("{}/project/{}", MODRINTH_URL, id);
        requester.retrieve_deserialized::<ModrinthProject>(&url).await
    }

    fn list_info(&self)
    {

        println!("\n{} ({}) -> {}\n{}",
            self.id,
            match &self.slug {
                Some(s) => s,
                None => "no slug"
            },
            self.title,
            self.description,
        );
    }
}

pub async fn list_projects(
    client: reqwest::Client,
    id_list: &Vec<String>,
) -> () {
    let pid_requester = http_handler::BasicRequest::build(client.clone());

    let items: Vec<_> = id_list
        .iter()
        .map(|id| {
            ModrinthProject::build(&pid_requester, id)
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