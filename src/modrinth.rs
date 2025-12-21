use std::pin::Pin;
use std::{fmt, fs, error};
use std::collections::HashSet;
use std::io::{self, Write};
use std::path::{self, PathBuf};
use futures::future;
use serde::{Serialize, Deserialize, Deserializer};
use serde::de::{Error};
use sha2::digest::generic_array::{ArrayLength, GenericArray};
use sha2::{Sha512, Digest};

use crate::arguments;

static MODRINTH_URL: &str = "https://api.modrinth.com";

#[derive(Debug)]
pub enum ModError {
    NoFile(String),
    BadRequest(reqwest::Error),
    NoVersion(String),
    NoDependency(String),
}

impl fmt::Display for ModError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoFile(msg) => write!(f, "[MODRINTH/ERROR] No file: {}", msg),
            Self::BadRequest(err) => write!(f, "[MODRINTH/ERROR] Bad request: {}", err),
            Self::NoVersion(msg) => write!(f, "[MODRINTH/ERROR] No version: {}", msg),
            Self::NoDependency(msg) => write!(f, "[MODRINTH/ERROR] No dependency: {}", msg)
        }
    }
}

impl error::Error for ModError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::BadRequest(err) => Some(err),
            _ => None
        }
    }
}

impl From<reqwest::Error> for ModError {
    fn from(value: reqwest::Error) -> Self {
        Self::BadRequest(value)
    }
}

#[derive(Debug)]
pub enum DownloadError {
    BadRequest(reqwest::Error),
    BadFile(io::Error),
    BadHash(String),
}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadRequest(err) => write!(f, "[MODRINTH/DOWNLOAD/ERROR] Bad request: {}", err),
            Self::BadFile(err) => write!(f, "[MODRINTH/DOWNLOAD/ERROR] Bad file: {}", err),
            Self::BadHash(msg) => write!(f, "[MODRINTH/DOWNLOAD/ERROR] Bad hash: {}", msg),
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

pub struct Mod {
    title: String,
    project_id: String,
    version_name: String,
    version_id: String,
    file: ModrinthFile,
    dependencies: Vec<RequiredDependency>,
}

impl Mod {
    pub fn title(&self) -> &String {
        &self.title
    }
    pub fn version_name(&self) -> &String {
        &self.version_name
    }
    pub fn filename(&self) -> &String {
        self.file.filename()
    }
    pub fn dependencies(&self) -> &Vec<RequiredDependency> {
        &self.dependencies
    }
    fn build(
        proj: Project,
        ver: Version,
        primary_file_idx: usize,
    ) -> Self {
        println!("[MODRINTH] Found mod '{}' for id '{}'", proj.get_title(), proj.get_id());
        Mod { 
            title: proj.get_title().clone(),
            project_id: proj.get_id().clone(),
            version_name: ver.name().clone(),
            version_id: ver.id().clone(),
            file: ver.files()[primary_file_idx].clone(),
            dependencies: ver.dependencies().clone()
        }
    }
    pub async fn build_from_project_id(
        client: &reqwest::Client,
        project_id: String,
        query: &VersionQuery
    ) -> Result<Self, ModError> {
        println!("[MODRINTH] Searching for project id '{}'", project_id);
        let proj = get_project(client, &project_id).await?;
        let top_version = get_top_version(client, &project_id, query).await?;
        let primary_file_idx = search_for_primary_file(top_version.files())
        .ok_or(ModError::NoFile(
            format!("Couldn't find file for project {}", proj.get_title())
        ))?;
        Ok(Self::build(proj, top_version, primary_file_idx))
    }
    pub async fn build_from_version_id(
        client: & reqwest::Client,
        version_id: String,
    ) -> Result<Self, ModError> {
        println!("[MODRINTH] Searching for version id '{}'", version_id);
        let ver = get_version_from_version_id(client, &version_id).await?;
        let proj = get_project(client, &ver.project_id()).await?;
        let primary_file_idx = search_for_primary_file(ver.files())
        .ok_or(ModError::NoFile(
            format!("Couldn't find file for project {}", proj.get_title())
        ))?;
        Ok(Self::build(proj, ver, primary_file_idx))
    }
    pub async fn build_from_version(
        client: &reqwest::Client,
        ver: Version
    ) -> Result<Self, ModError> {
        println!("[MODRINTH] Using version id '{}'", ver.id());
        let proj = get_project(client, ver.project_id()).await?;
        let primary_file_idx = search_for_primary_file(ver.files())
        .ok_or(ModError::NoFile(
            format!("Couldn't find file for project {}", proj.get_title())
        ))?;
        Ok(Self::build(proj, ver, primary_file_idx))
    }
    pub fn verify_against(&self, file_path: &PathBuf) -> FileVerification {
        if !path::Path::exists(&file_path) {
            return FileVerification::NotExists
        }
        match fs::read(&file_path) {
            Ok(bytes) => {
                if self.file.hashes.check512(&Sha512::digest(bytes)) {
                    FileVerification::Ok
                } else {
                    FileVerification::BadHash
                }
            }
            Err(_) => FileVerification::BadFile
        }
    }
    async fn check_dep_against(
        dep_ver: &Version,
        check_against: &Option<HashSet<&String>>,
    ) -> bool {
        if let Some(check) = check_against 
        && check.contains(dep_ver.project_id()) { false }
        else { true }
    }
    pub async fn get_dependencies(
        &self,
        client: &reqwest::Client,
        query: &VersionQuery,
        check_against: Option<&Vec<Mod>>
    ) -> Vec<Self> {
        let mut out: Vec<Self> = Vec::new();
        let mut check_set: Option<HashSet<&String>> = None;
        if let Some(c) = check_against {
            check_set = Some(
                c.iter()
                .map(|x| {
                    &x.project_id
                })
                .collect()
            );
        }
        for dep in self.dependencies() {
            let dep_ver = dep.resolve_to_version(client, query).await;
            if let Ok(ver) = dep_ver
            && Self::check_dep_against(&ver, &check_set).await
            && let Ok(m) = Mod::build_from_version(client, ver).await {
                out.push(m);
            };
        }
        out
    }
    pub async fn download(
        &self,
        client: &reqwest::Client,
        out_dir: &PathBuf
    ) -> Result<(), DownloadError> {
        let file_path = out_dir.join(self.filename());
        match self.verify_against(&file_path){
            FileVerification::Ok => {
                println!("[MODRINTH/DOWNLOAD] {} already present. Skipping download...", self.title());
                return Ok(());
            }
            FileVerification::BadHash => {
                println!("[MODRINTH/DOWNLOAD/WARNING] File present for {}, but hashes do not match. Continuing with download...", self.title());
            }
            FileVerification::BadFile => {
                println!("[MODRINTH/DOWNLOAD/WARNING] File present for {}, but something is wrong. Continuing with download...", self.title());
            }
            FileVerification::NotExists => {
                println!("[MODRINTH/DOWNLOAD] Downloading file {} for {}", self.file.filename(), self.title());
            }
        }
        let res = client.get(self.file.url())
            .send()
            .await?
            .bytes()
            .await?;
        if self.file.hashes.check512(&Sha512::digest(&res)) {
            println!("[MODRINTH/DOWNLOAD] Hashes match. Writing to file...");
            let mut f_out = fs::File::create(
                file_path
            )?;
            f_out.write_all(&res)?;
            println!("[MODRINTH/DOWNLOAD] Successfully downloaded {}", self.file.filename());
        } else {
            DownloadError::BadHash(
                format!("Hashes do not match for file '{}'. Skipping download...",
                    self.file.filename()
                )
            );
        }
        Ok(())
    }
}

impl PartialEq for Mod {
    fn eq(&self, other: &Self) -> bool {
        self.project_id == other.project_id
    }
}

impl PartialEq<String> for Mod {
    fn eq(&self, other: &String) -> bool {
        &self.project_id == other
    }
}

pub async fn resolve_dependencies(
    client: &reqwest::Client,
    query: &VersionQuery,
    mods: &mut Vec<Mod>,
) -> Pin<Box<()>>
{
    println!("Func called");
    let mut deps_to_search: Vec<&RequiredDependency> = Vec::new();
    let mut new_deps: u16 = 0;
    for value in &mut *mods {
        deps_to_search.extend(value.dependencies());
    }
    let dep_versions= future::join_all(
        deps_to_search.iter()
        .map(|&x| {
            x.resolve_to_version(client, query)
        })
    ).await;
    for ver_res in dep_versions {
        if let Ok(ver) = ver_res
        && !mods.iter().any(|m| m == ver.project_id()) {
            if let Ok(m) = Mod::build_from_version(client, ver).await {
                mods.push(m);
                new_deps += 1;
            }
        }
    };
    if new_deps > 0 {
        Box::pin(resolve_dependencies(client, query, mods)).await
    } else {
        println!("No deps found");
        Box::pin(())
    }
}

#[derive(Deserialize)]
pub struct Project {
    id: String,
    title: String,
    description: String,
}

impl Project {
    pub fn get_id(&self) -> &String {
        &self.id
    }
    pub fn get_title(&self) -> &String {
        &self.title
    }
    pub fn get_desc(&self) -> &String {
        &self.description
    }
}

#[derive(Deserialize)]
pub struct Version {
    id: String,
    project_id: String,
    name: String,
    version_number: String,
    files: Vec<ModrinthFile>,
    #[serde(deserialize_with = "deserialize_only_required_deps")]
    dependencies: Vec<RequiredDependency>
}

impl Version {
    pub fn id(&self) -> &String{
        &self.id
    }
    pub fn project_id(&self) -> &String {
        &self.project_id
    }
    pub fn name(&self) -> &String {
        &self.name
    }
    pub fn version_number(&self) -> &String {
        &self.version_number
    }
    pub fn files(&self) -> &Vec<ModrinthFile> {
        &self.files
    }
    pub fn dependencies(&self) -> &Vec<RequiredDependency> {
        &self.dependencies
    }
}

impl Clone for Version {
    fn clone(&self) -> Self {
        Version {
            id: self.id.clone(),
            project_id: self.project_id.clone(),
            name: self.name.clone(),
            version_number: self.version_number.clone(),
            files: self.files.clone(),
            dependencies: self.dependencies.clone()
        }
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
    pub async fn resolve_to_version(
        &self,
        client: &reqwest::Client,
        query: &VersionQuery
    ) -> Result<Version, ModError>{
        if let Some(v) = &self.version_id {
            return match get_version_from_version_id(client, v).await {
                Ok(v) => Ok(v),
                Err(e) => Err(ModError::BadRequest(e))
            }
        } else if let Some(p) = &self.project_id {
            return get_top_version(client, p, query).await
        } else {
            return Err(ModError::NoDependency("Could not resolve dependency".to_string()))
        }
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

#[derive(Deserialize)]
pub struct ModrinthFile {
    url: String,
    filename: String,
    primary: bool,
    hashes: ModrinthFileHash,
}

impl ModrinthFile {
    pub fn url(&self) -> &String {
        &self.url
    }
    pub fn filename(&self) -> &String {
        &self.filename
    }
    pub fn primary(&self) -> &bool {
        &self.primary
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
    pub fn check512<U>(&self, other_hash: &GenericArray<u8, U>) -> bool
        where U: ArrayLength<u8>
    {
        &self.sha512[..] == &other_hash[..]
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
    loaders: String
}

impl VersionQuery {
    fn build_param_array(user_params: &String) -> String {
        let mut params = user_params.split(",");
        let mut res: String = String::from("[");
        res = format!("{}\"{}\"",
            res,
            params.next().unwrap_or(""),
        );
        while let Some(prm) = params.next() {
            res = format!("{},\"{}\"",
                res,
                prm,
            );
        }
        format!("{}]", res)
    }
    pub fn build_query(user_mcvs: &String, user_loader: &String) -> VersionQuery {
        let game_versions= Self::build_param_array(user_mcvs);
        let loaders= Self::build_param_array(user_loader);
        VersionQuery { game_versions, loaders }
    }
    pub fn mcvs(&self) -> &str {
        &self.game_versions.as_str()
    }
    pub fn loader(&self) -> &str {
        &self.loaders.as_str()
    }
}

pub async fn get_project(
    client: &reqwest::Client,
    id: &str
) -> Result<Project, reqwest::Error>
{
    let url = format!("{}{}{}", MODRINTH_URL, "/v2/project/", id);
    let response = client.get(url)
        .send()
        .await?;
    response.json::<Project>().await
}

pub async fn get_projects_from_list(
    client: &reqwest::Client,
    ids: &Vec<String>
) -> Vec<Result<Project, reqwest::Error>>
{
    let mut responses = Vec::new();
    for id in ids {
        responses.push(get_project(client, id));
    }
    future::join_all(responses).await
}

pub async fn get_version(
    client: &reqwest::Client,
    project_id: &str,
    query: &VersionQuery
) -> Result<Vec<Version>, reqwest::Error>
{
    let url = format!("{}{}{}{}",
        MODRINTH_URL,
        "/v2/project/",
        project_id,
        "/version"
    );
    let response = client.get(url)
        .query(query)
        .send()
        .await?;
    response.json::<Vec<Version>>().await
}

pub async fn get_version_from_version_id(
    client: &reqwest::Client,
    id: &String
) -> Result<Version, reqwest::Error> {
    let url = format!("{}/v2/version/{}", MODRINTH_URL, id);
    let response = client.get(url)
        .send()
        .await?;
    response.json::<Version>().await
}

pub async fn get_top_version(
    client: & reqwest::Client,
    project_id: &str,
    query: &VersionQuery
) -> Result<Version, ModError>
{
    let response = get_version(client, project_id, query).await?;
    match response.get(0).cloned() {
        Some(v) => Ok(v),
        None => {
            Err(ModError::NoVersion(
                format!("No version found for id {project_id}")
            ))
        }
    }
}

pub fn search_for_primary_file(files: &Vec<ModrinthFile>) -> Option<usize> {
    if files.len() == 0 {
        return None; // If there are no files
    }
    for (i, file) in files.iter().enumerate() {
        if file.primary { return Some(i); }
    }
    Some(0) // If no file is marked primary, return 1st file
}


fn file_from_ver(v: &Version) -> Option<ModrinthFile>{
    let v_files = v.files();
    let primary_idx = search_for_primary_file(v_files);
    if let Some(idx) = primary_idx {
        Some(v_files[idx].clone())
    } else {
        None
    }
}

pub async fn get_file_direct(
    client: &reqwest::Client,
    project_id: &str,
    query: &VersionQuery,
) -> Option<ModrinthFile>
{
    match get_top_version(client, project_id, query).await {
        Ok(version) => {
            println!("({project_id}) Found suitable version: {} [{}]",
                version.name(),
                version.version_number()
            );
            file_from_ver(&version)
        }
        Err(e) => {
            println!("({project_id}) Failed to find suitable version: {e}");
            None
        }
    }
}

fn download_already_exists(file_path: &PathBuf, f_in: &ModrinthFile) -> bool {
    fn check_hash(bytes: &Vec<u8>, f_in: &ModrinthFile) -> bool {
        let file_hash = Sha512::digest(bytes);
        if &f_in.hashes.sha512[..] == &file_hash[..] {
            println!("File {} already here, skipping download...",
                f_in.filename()
            );
            return true;
        } else {
            println!("Filename {} already here, but hashes do not match. Redownloading...",
                f_in.filename()
            );
            return false;
        }
    }
    if !path::Path::exists(&file_path) {
        return false;
    }
    match fs::read(&file_path) {
        Ok(bytes) => {
            return check_hash(&bytes, &f_in);
        }
        Err(e) => {
            println!("Filename {} already found, but something went wrong ({e}). Redownloading...",
                f_in.filename()
            );
            return false;
        }
    }
}

pub async fn download_file(
    client: &reqwest::Client,
    f_in: &ModrinthFile,
    out_dir: &PathBuf
) -> Result<(), Box<dyn error::Error>> 
{
    let file_path = out_dir.join(f_in.filename());
    if download_already_exists(&file_path, &f_in) {
        return Ok(())
    }
    let res = client.get(f_in.url())
        .send()
        .await?
        .bytes()
        .await?;
    let file_hash = Sha512::digest(&res);
    if &f_in.hashes.sha512[..] == &file_hash[..] {
        println!("Hashes match. Downloading...");
        let mut f_out = fs::File::create(
            out_dir.join(f_in.filename())
        )?;
        f_out.write_all(&res)?;
        println!("Successfully downloaded {}", f_in.filename());
    } else {
        println!("WARNING: Hashes do not match for file '{}'. Skipping download.", f_in.filename())
    }
    Ok(())
}

pub fn collect_versions(results: Vec<Result<Version, ModError>>) -> Vec<Version> {
    let mut out: Vec<Version> = Vec::new();
    for res in results {
        match res {
            Ok(v) => out.push(v),
            Err(e) => eprintln!("Could not retrieve version: {e}")
        }       
    }
    out
}

pub enum FileVerification {
    Ok,
    NotExists,
    BadHash,
    BadFile
}

fn verify_file(mfile: &ModrinthFile, out_dir: &PathBuf) -> FileVerification{
    let file_path = out_dir.join(mfile.filename());
    if !path::Path::exists(&file_path) {
        return FileVerification::NotExists;
    };
    match fs::read(&file_path) {
        Ok(bytes) => {
            let file_hash = Sha512::digest(bytes);
            if mfile.hashes.sha512[..] == file_hash[..] {
                FileVerification::Ok
            } else {
                FileVerification::BadHash
            }
        },
        Err(_) => FileVerification::BadFile
    }
}

async fn collect_files(
    client: &reqwest::Client,
    ids: &Vec<String>,
    query: &VersionQuery
) -> Vec<Option<ModrinthFile>>
{
    let mut file_results  = Vec::new();
    for id in ids {
        println!("Getting ID '{id}'...");
        file_results.push(
            get_file_direct(client, id, &query)
        )
    }
    future::join_all(file_results).await
}

async fn collect_downloads(
    client: &reqwest::Client,
    files: &Vec<Option<ModrinthFile>>,
    out_dir: &PathBuf
) -> Vec<Result<(), Box<dyn error::Error>>>
{
    let mut download_tasks = Vec::new();
    for file in files {
        if let Some(f) = file {
            download_tasks.push(
                download_file(client, f, out_dir)
            )
        }
    }
    future::join_all(download_tasks).await
}

async fn collect_mods(
    client: & reqwest::Client,
    ids: &Vec<String>,
    query: &VersionQuery
) -> Vec<Mod>
{
    let mut mods = Vec::new();
    for id in ids {
        mods.push(Mod::build_from_project_id(client, id.to_string(), query));
    }
    future::join_all(mods)
    .await
    .into_iter()
    .filter_map(|m| {
        if let Err(e) = m {
            println!("{e}");
            return None
        } else {
            return m.ok()
        }
    })
    .collect()
}

async fn download_from_id_list<'a>(
    conf: &arguments::Config<'a>,
    client: &reqwest::Client,
    ids: &Vec<String>,
    out_dir: &PathBuf
) -> Result<(), Box<dyn error::Error>>
{
    let query: VersionQuery = VersionQuery::build_query(
        conf.mcvs(), 
        &conf.loader_as_string()
    );
    let mod_files = collect_files(client, ids, &query).await;
    let downloads = collect_downloads(
        client,
        &mod_files,
        &out_dir
    ).await;
    let download_errors: Vec<Box<dyn error::Error>> = downloads.into_iter()
        .filter_map(Result::err)
        .collect();
    for err in download_errors {
        println!("Download error: {err}")
    }
    Ok(())
}

async fn download_from_id_list2<'a>(
    conf: &arguments::Config<'a>,
    client: & reqwest::Client,
    ids: &Vec<String>,
    out_dir: &PathBuf
) -> Result<(), Box<dyn error::Error>>
{
    let query = VersionQuery::build_query(
        conf.mcvs(),
        &conf.loader_as_string()
    );
    let mut mods: Vec<Mod> = collect_mods(client, ids, &query).await;
    resolve_dependencies(client, &query, &mut mods).await;
    let mut download_tasks = Vec::new();
    for m in &mods {
        download_tasks.push(m.download(client, out_dir));
    }
    for e in future::join_all(download_tasks)
    .await
    .into_iter()
    .filter_map(Result::err)
    .collect::<Vec<DownloadError>>() {
        println!("{e}");
    }
    Ok(())
}

async fn verify_ids_from_list<'a>(
    conf: &arguments::Config<'a>,
    client: &reqwest::Client,
    ids: &Vec<String>,
    out_dir: &PathBuf
) -> () {
    let query: VersionQuery = VersionQuery::build_query(
        &conf.mcvs(),
        &conf.loader_as_string()
    );
    let mod_files: Vec<ModrinthFile> = collect_files(client, ids, &query)
        .await
        .into_iter()
        .filter_map(|f| f)
        .collect();
    let mut bad_results: u32 = 0;
    for f in &mod_files {
        match verify_file(&f, out_dir) {
            FileVerification::Ok => {
                println!("Successfully verified file {}", f.filename());
            },
            _ => {
                println!("Unable to verify file {}", f.filename());
                bad_results += 1;
            }
        }
    };
    if bad_results > 0 {
        println!("\n{} out of {} modrinth files were unable to be verified",
            bad_results,
            mod_files.len()
        );
    } else {
        println!("\nAll modrinth files verified successfully");
    };
    ()
}


async fn download_from_id<'a>(
    conf: &arguments::Config<'a>,
    client: &reqwest::Client,
    id: &str,
    out_dir: &PathBuf
) -> Result<(), Box<dyn error::Error>>
{
    let query = VersionQuery::build_query(
        conf.mcvs(),
        &conf.loader_as_string()
    );
    if let Some(file) = get_file_direct(client, id, &query).await {
        download_file(client, &file, out_dir).await?
    } else {
        println!("No file available for id {id}")
    };

    Ok(())
}

async fn verify_id<'a>(
    conf: &arguments::Config<'a>,
    client: &reqwest::Client,
    id: &str,
    out_dir: &PathBuf
) -> () {
    let query: VersionQuery = VersionQuery::build_query(
        &conf.mcvs(),
        &conf.loader_as_string()
    );
    if let Some(f) = get_file_direct(&client, &id, &query).await {
        match verify_file(&f, out_dir) {
            FileVerification::Ok => {
                println!("Successfully verified file {}", f.filename());
            },
            _ => {
                println!("Unable to verify file {}", f.filename());
            }
        }
    };
    ()
}

pub async fn handle_list_input<'a>(
    conf: &arguments::Config<'a>,
    client: &reqwest::Client,
    id_list: &Vec<String>,
    out_dir: &PathBuf
) -> Result<(), Box<dyn error::Error>> {
    if conf.verify() {
            verify_ids_from_list(
                conf,
                client,
                id_list,
                out_dir
            ).await;
        } else {
            download_from_id_list2(
                conf,
                client,
                id_list,
                out_dir
            ).await?;
        };
    Ok(())
}

pub async fn handle_single_input<'a>(
    conf: &arguments::Config<'a>,
    client: &reqwest::Client,
    id: &str,
    out_dir: &PathBuf
) -> Result<(), Box<dyn error::Error>> {
    if conf.verify() {
        verify_id(
            conf,
            client,
            id,
            out_dir
        ).await;
    } else {
        download_from_id(
            conf,
            client,
            id,
            out_dir
        ).await?;
    };
    Ok(())
}