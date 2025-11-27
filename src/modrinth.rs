use std::{fmt, fs};
use std::io::{Write};
use std::path::{self, PathBuf};
use futures::future;
use serde::{Serialize, Deserialize, Deserializer};
use serde::de::{Error};
use sha2::{Sha512, Digest};

use crate::arguments;

static MODRINTH_URL: &str = "https://api.modrinth.com";

#[derive(Debug)]
pub enum VersionError {
    BadRequest(reqwest::Error),
    NoVersion(&'static str),
}

impl fmt::Display for VersionError {
    fn fmt(&self, f:& mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::BadRequest(err) => write!(f, "Couldn't get versions: {}", err),
            Self::NoVersion(msg) => write!(f, "{}", msg)
        }
    }
}

impl std::error::Error for VersionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::BadRequest(err) => Some(err),
            _ => None
        }
    }
}

impl From<reqwest::Error> for VersionError {
    fn from(value: reqwest::Error) -> Self {
        Self::BadRequest(value)
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
    files: Vec<ModrinthFile>
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
}

impl Clone for Version {
    fn clone(&self) -> Self {
        Version {
            id: self.id.clone(),
            project_id: self.project_id.clone(),
            name: self.name.clone(),
            version_number: self.version_number.clone(),
            files: self.files.clone()
        }
    }
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

pub async fn get_top_version(
    client: & reqwest::Client,
    project_id: &str,
    query: &VersionQuery
) -> Result<Version, VersionError>
{
    let response = get_version(client, project_id, query).await?;
    match response.get(0).cloned() {
        Some(v) => Ok(v),
        None => {
            eprintln!("No suitable version for id {project_id}");
            Err(VersionError::NoVersion("No version available"))
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
) -> Result<(), Box<dyn std::error::Error>> 
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

pub fn collect_versions(results: Vec<Result<Version, VersionError>>) -> Vec<Version> {
    let mut out: Vec<Version> = Vec::new();
    for res in results {
        match res {
            Ok(v) => out.push(v),
            Err(e) => eprintln!("Could not retrieve version: {e}")
        }       
    }
    out
}

pub fn verify_file(mfile: &ModrinthFile, out_dir: &PathBuf) -> bool{
    let file_path = out_dir.join(mfile.filename());
    if !path::Path::exists(&file_path) {
        return false;
    };
    match fs::read(&file_path) {
        Ok(bytes) => {
            let file_hash = Sha512::digest(bytes);
            mfile.hashes.sha512[..] == file_hash[..]
        },
        Err(_) => false
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
) -> Vec<Result<(), Box<dyn std::error::Error>>>
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

async fn download_from_id_list<'a>(
    conf: &arguments::Config<'a>,
    client: &reqwest::Client,
    ids: &Vec<String>,
    out_dir: &PathBuf
) -> Result<(), Box<dyn std::error::Error>>
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
    let download_errors: Vec<Box<dyn std::error::Error>> = downloads.into_iter()
        .filter_map(Result::err)
        .collect();
    for err in download_errors {
        println!("Download error: {err}")
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
        if verify_file(&f, out_dir){
            println!("Successfully verified file {}", f.filename());
        } else {
            println!("Unable to verify file {}", f.filename());
            bad_results += 1;
        }
    };
    if bad_results > 0 {
        println!("\n{} out of {} files were unable to be verified", bad_results, mod_files.len());
    } else {
        println!("\nAll files verified successfully");
    };
    ()
}


async fn download_from_id<'a>(
    conf: &arguments::Config<'a>,
    client: &reqwest::Client,
    id: &str,
    out_dir: &PathBuf
) -> Result<(), Box<dyn std::error::Error>>
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
        if verify_file(&f, &out_dir) {
            println!("Successfully verified file {}", f.filename());
        }
        else {
            println!("Unable to verify file {}", f.filename());
        }
    };
    ()
}

pub async fn handle_list_input<'a>(
    conf: &arguments::Config<'a>,
    client: &reqwest::Client,
    id_list: &Vec<String>,
    out_dir: &PathBuf
) -> Result<(), Box<dyn std::error::Error>> {
    if conf.verify() {
            verify_ids_from_list(
                conf,
                client,
                id_list,
                out_dir
            ).await;
        } else {
            download_from_id_list(
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
) -> Result<(), Box<dyn std::error::Error>> {
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