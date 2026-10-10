use std::{error, fmt};
use std::collections::HashSet;
use std::path::{PathBuf};
use futures::future;
use serde::{Serialize, Deserialize, Deserializer};
use serde::de::{Error};
use sha2::digest::Output;
use sha2::{Sha512, Digest};

use crate::arguments::{DownloadSettings};
use crate::file_parse::TrackerFile;
use crate::http_handler::{Downloader};
use crate::{arguments, http_handler::{self, HttpRequest}, items};

static MODRINTH_URL: &'static str = "https://api.modrinth.com/v2";
static MODRINTH_SIG: &'static str = "MODRINTH";

static MODRINTH_ITEM_ID: &'static str = "MR";

fn modrinth_msg(s: String) -> ()
{
    println!("[{}] {}", MODRINTH_SIG, s)
}

#[derive(Debug)]
pub enum ModrinthItemError
{
    BadRequest(reqwest::Error),
    NoVersion(String),
    NoFile(String),
}
impl ModrinthItemError
{
    fn err_str(s: &str) -> String
    {
        format!("[{}/ERROR] {}", MODRINTH_SIG, s)
    }
}

impl fmt::Display for ModrinthItemError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self
        {
            Self::BadRequest(err) => write!(
                f, "{}{}", Self::err_str("Bad request: "), err
            ),
            Self::NoVersion(msg) => write!(
                f, "{}{}", Self::err_str("No version: "), msg
            ),
            Self::NoFile(msg) => write!(
                f, "{}{}", Self::err_str("No file: "), msg
            )
        }
    }
}

impl error::Error for ModrinthItemError
{
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self
        {
            Self::BadRequest(err) => Some(err),
            _ => None
        }
    }
}

impl From<reqwest::Error> for ModrinthItemError
{
    fn from(value: reqwest::Error) -> Self {
        Self::BadRequest(value)
    }
}

pub struct ModrinthItem
{
    id: String,
    version_title: String,
    version_id: String,
    downloadable: ModrinthFile,
    dependencies: Vec<RequiredDependency>,
    downloaded: bool,
}
impl items::Item for ModrinthItem
{
    async fn download<'a>(
        &mut self,
        downloader: &Downloader<'a>,
    ) -> ()
    {
        match self.downloadable.download(downloader).await
        {
            Err(err) => println!("{}", err),
            Ok(_) => self.downloaded = true,
        }
    }
    fn id(&self) -> &String
    {
        &self.id
    }
    fn version_id(&self) -> &String
    {
        &self.version_id
    }
    fn downloaded(&self) -> bool
    {
        self.downloaded
    }
    fn filename(&self) -> &PathBuf
    {
        self.downloadable.filename()
    }
    fn item_id(&self) -> &'static str {
        MODRINTH_ITEM_ID
    }
}
impl ModrinthItem
{
    pub fn version_title(&self) -> &String
    {
        &self.version_title
    }

    fn build_from_version(
        mut version: ModrinthVersion
    ) -> Result<Self, ModrinthItemError>
    {
        if version.files.len() == 0
        {
            return Err(ModrinthItemError::NoFile(String::from("No files for this version")));
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
            downloaded: false,
        })
    }
    
    pub async fn build_from_id<'a>(
        version_requester: &http_handler::QueryRequest<'a, VersionQuery>,
        project_id: &str
    ) -> Result<Self, ModrinthItemError>
    {
        let url = format!("{}/project/{}/version", MODRINTH_URL, project_id);

        let mut versions = version_requester
            .retrieve_deserialized::<Vec<ModrinthVersion>>(&url)
            .await?;

        if versions.len() == 0
        {
            return Err(ModrinthItemError::NoVersion(format!("No version for id '{}'", project_id)));
        }

        let selected_version = versions.swap_remove(0);

        Self::build_from_version(selected_version)
    }

    async fn build_from_version_id<'a>(
        direct_requester: &http_handler::BasicRequest<'a>,
        version_id: &str
    ) -> Result<Self, ModrinthItemError>
    {
        let url = format!("{}/version/{}", MODRINTH_URL, version_id);
        Self::build_from_version(
            direct_requester.retrieve_deserialized::<ModrinthVersion>(&url).await?
        )
    }

    async fn try_dep<'a>(
        requester: &DependencyRequester<'a>,
        dep: &RequiredDependency
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
        requester: &DependencyRequester<'a>
    ) -> Vec<Self>
    {
        self.get_filtered_deps(requester, |_| { true }).await
    }

    async fn get_filtered_deps<'a, F>(
        &self,
        requester: &DependencyRequester<'a>,
        verify: F,
    ) -> Vec<Self>
    where F: Fn(&RequiredDependency) -> bool
    {
        let mut res = Vec::<Self>::new();

        for dep in self.dependencies
            .iter()
            .filter(|d| verify(d))
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

#[derive(Deserialize)]
struct ModrinthVersion
{
    name: String,
    id: String,
    project_id: String,
    files: Vec<ModrinthFile>,
    #[serde(deserialize_with = "deserialize_only_required_deps")]
    dependencies: Vec<RequiredDependency>,
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
        downloader: &http_handler::Downloader<'a>,
    ) -> Result<(), http_handler::DownloadError>
    {
        downloader.verify_and_download(
            &self.url,
            &self.filename,
            |b| self.hashes.check512(&Sha512::digest(b))
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
    #[serde(deserialize_with = "deserialize_hex_str_to_bytes")]
    sha512: Vec<u8>
}

impl ModrinthFileHash {
    pub fn check512(&self, other_hash: &Output<Sha512>) -> bool {
        self.sha512.as_slice() == other_hash.as_slice()
    }
}

impl Clone for ModrinthFileHash {
    fn clone(&self) -> Self {
        ModrinthFileHash {
            sha512: self.sha512.clone()
        }
    }
}
fn deserialize_hex_str_to_bytes<'de, D>(
    deserializer: D
) -> Result<Vec<u8>, D::Error>
    where D: Deserializer<'de>
{
    let hex_data: String = Deserialize::deserialize(deserializer)?;
    hex::decode(hex_data).map_err(D::Error::custom)
}

#[derive(Serialize)]
pub struct VersionQuery {
    game_versions: String,
    loaders: String,
    include_changelog: bool,
    limit: u32
}

impl VersionQuery {
    fn array_start_string(prm: &str) -> String
    {
        format!("[\"{}\"", prm)
    }
    fn next_param(s: &mut String, prm: &str) -> ()
    {
        s.push_str(&format!(",\"{}\"", prm));
    }
    fn build_version_array(user_params: &String) -> String {
        let mut params = user_params.split(",");
        let mut res: String = Self::array_start_string(
            params.next().unwrap_or("")
        );
        while let Some(prm) = params.next() {
            Self::next_param(&mut res, prm);
        }
        res.push(']');
        res
    }
    fn build_loader_array(user_loaders: &Vec<arguments::Platform>) -> String
    {
        let mut loaders = user_loaders.iter();
        let mut res = Self::array_start_string(match loaders.next()
            {
                Some(l) => l.to_str(),
                None => "",
            }
        );
        while let Some(l) = loaders.next()
        {
            Self::next_param(&mut res, l.to_str());
        }
        res.push(']');
        res
    }
    pub fn build_query(
        user_mcvs: &String,
        user_loader: &Vec<arguments::Platform>
    ) -> VersionQuery {
        let game_versions= Self::build_version_array(user_mcvs);
        let loaders= Self::build_loader_array(user_loader);
        VersionQuery {
            game_versions,
            loaders,
            include_changelog: false,
            limit: 1
        }
    }
    pub fn mcvs(&self) -> &str {
        &self.game_versions.as_str()
    }
    pub fn loader(&self) -> &str {
        &self.loaders.as_str()
    }
}

#[derive(Deserialize)]
pub struct Dependency {
    version_id: Option<String>,
    project_id: Option<String>,
    dependency_type: String
}

pub struct RequiredDependency {
    version_id: Option<String>,
    project_id: Option<String>,
}

impl RequiredDependency {
    pub fn from_dep(dep: Dependency) -> Self {
        RequiredDependency {
            version_id: dep.version_id,
            project_id: dep.project_id
        }
    }
    pub fn version_id(&self) -> &Option<String> {
        &self.version_id
    }
    pub fn project_id(&self) -> &Option<String> {
        &self.project_id
    }
}

impl Clone for RequiredDependency {
    fn clone(&self) -> Self {
        RequiredDependency {
            version_id: self.version_id.clone(),
            project_id: self.project_id.clone(),
        }
    }
}

fn deserialize_only_required_deps<'de, D>(
    deserializer: D
) -> Result<Vec<RequiredDependency>, D::Error> 
    where D: Deserializer<'de>
{
    let deps: Vec<Dependency> = Deserialize::deserialize(deserializer)?;
    Ok (deps.into_iter()
        .filter_map(|d|
            if d.dependency_type == "required" {
                Some(RequiredDependency::from_dep(d))
            } else {
                None
            }
        )
        .collect()
    )
}

struct DependencyRequester<'a>
{
    pid_req: &'a http_handler::QueryRequest<'a, VersionQuery>,
    vid_req: &'a http_handler::BasicRequest<'a>,
}
impl<'a> DependencyRequester<'a>
{
    pub fn new(
        pid_req: &'a http_handler::QueryRequest<'a, VersionQuery>,
        vid_req: &'a http_handler::BasicRequest<'a>
    ) -> DependencyRequester<'a>
    {
        DependencyRequester { pid_req, vid_req }
    }

    pub fn project_id_requester(
        &self
    ) -> &'a http_handler::QueryRequest<'a, VersionQuery>
    {
        self.pid_req
    }
    pub fn version_id_requester(&self) -> &'a http_handler::BasicRequest<'a>
    {
        self.vid_req
    }
}

struct DependencyHandler<'a>
{
    modlist: &'a mut Vec<ModrinthItem>,
    dep_check_stack: Vec<usize>,
    present_ids: HashSet<String>,
}
impl<'a> DependencyHandler<'a>
{
    pub fn build(modlist: &'a mut Vec<ModrinthItem>) -> Self
    {
        let dep_check_stack: Vec<usize> = (0..modlist.len()).collect();
        let present_ids: HashSet<String> = modlist
            .iter()
            .map(|item| item.id.clone())
            .collect()
        ;
        DependencyHandler { modlist, dep_check_stack, present_ids }
    }

    fn is_dep_present(&self, dep: &RequiredDependency) -> bool
    {
        if let Some(pid) = dep.project_id()
            && self.present_ids.contains(pid)
        {
            return true
        }

        false
    }

    async fn try_get_deps(&mut self, requester: &DependencyRequester<'a>, idx: &usize)
    {
        let deps = {
            match self.modlist.get(*idx)
            {
                Some(item) => item.get_filtered_deps(
                    requester,
                    |d| { !self.is_dep_present(d) }).await,
                None => { return () }
            }
        };

        for value in deps
        {
            if !self.present_ids.contains(&value.id){
                modrinth_msg(format!("Found dependency: {}", value.version_title()));
                self.present_ids.insert(value.id.clone());

                self.dep_check_stack.push(self.modlist.len());
                self.modlist.push(value);
            }
        }
    }

    pub async fn acquire_all_dependencies(
        &mut self,
        requester: &DependencyRequester<'a>
    ) -> ()
    {
        while let Some(idx) = self.dep_check_stack.pop()
        {
            self.try_get_deps(requester, &idx).await;
        }
    }
}

async fn collect_mods<'a>(
    requester: &http_handler::QueryRequest<'a, VersionQuery>,
    ids: &Vec<String>,
) -> Vec<ModrinthItem>
{
    let mut mods = Vec::new();
    for id in ids {
        mods.push(ModrinthItem::build_from_id(requester, id));
    }
    future::join_all(mods)
    .await
    .into_iter()
    .filter_map(|m_res| {
        match m_res {
            Err(e) => {
                println!("{e}");
                None
            }
            Ok(m) => {
                modrinth_msg(format!("Found '{}'", m.version_title()));
                Some(m)
            }
        }
    })
    .collect()
}

pub async fn build_modlist_from_ids<'a>(
    conf: &arguments::Config<'a, &DownloadSettings>,
    client: & reqwest::Client,
    ids: &Vec<String>,
) -> Result<Vec<ModrinthItem>, Box<dyn error::Error>>
{
    let query = VersionQuery::build_query(
        conf.settings().version_search(),
        conf.settings().platform().get(),
    );

    let mut pid_requester = http_handler::QueryRequest::<VersionQuery>::build(
        client,
        query
    );
    
    if conf.opts().release_only()
    {
        pid_requester.add_query(http_handler::QueryParam::new(
            "version_type",
            "release"
        ));
    }

    // Get modlist
    let mut items = collect_mods(&pid_requester, ids).await;

    if conf.opts().skip_deps()
    {
        return Ok(items)
    }

    // Get dependencies
    let vid_requester = http_handler::BasicRequest::build(client);
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
    client: & reqwest::Client,
    ids: &Vec<String>,
    out_dir: PathBuf
) -> Result<(), Box<dyn error::Error>>
{
    let mut items = build_modlist_from_ids(conf, client, ids).await?;

    let mut tracker: TrackerFile = TrackerFile::build(&out_dir)?;
    items = items::filter_items_by_tracker(items, &tracker);

    if !items::ask_user_to_download(&out_dir)?
    {
        return Ok(())
    }

    modrinth_msg(String::from("Downloading..."));

    // Download
    let downloader = http_handler::Downloader::build(client, out_dir);
    items::download_items(&downloader, &mut items).await;

    tracker.update(items);
    tracker.write_csv();

    modrinth_msg(String::from("Download finished"));

    Ok(())
}

pub async fn verify_ids_from_list<'a>(
    conf: &arguments::Config<'a, &DownloadSettings>,
    client: & reqwest::Client,
    ids: &Vec<String>,
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
}
impl ModrinthProject
{
    async fn build<'a>(
        requester: &http_handler::BasicRequest<'a>,
        id: &str,
    ) -> reqwest::Result<Self>
    {
        let url = format!("{}/project/{}", MODRINTH_URL, id);
        requester.retrieve_deserialized::<ModrinthProject>(&url).await
    }

    fn list_info(&self)
    {
        println!("\n{} -> {}\n{}",
            self.id,
            self.title,
            self.description
        );
    }
}
pub async fn list_projects(
    client: &reqwest::Client,
    id_list: &Vec<String>,
) -> () {
    let pid_requester = http_handler::BasicRequest::build(client);

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
    client: & reqwest::Client,
    id: &str,
    out_dir: PathBuf
) -> Result<(), Box<dyn error::Error>>
{   
    download_from_id_list(conf, client, &vec![id.to_string()], out_dir).await?;

    Ok(())
}

pub async fn verify_id<'a> (
    conf: &arguments::Config<'a, &DownloadSettings>,
    client: & reqwest::Client,
    id: &str,
    out_dir: PathBuf
) -> Result<(), Box<dyn error::Error>> {
    
    verify_ids_from_list(conf, client, &vec![id.to_string()], out_dir).await?;

    Ok(())
}