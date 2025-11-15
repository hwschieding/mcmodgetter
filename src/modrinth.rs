use std::{fmt, fs};
use std::io::{Write};
use std::path::PathBuf;
use futures::future;
use serde::{Serialize, Deserialize};

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
    primary: bool
}

impl ModrinthFile {
    pub fn new(user_url: &str, user_filename: &str)-> ModrinthFile {
        let url = String::from(user_url);
        let filename = String::from(user_filename);
        let primary = true;
        ModrinthFile { url, filename, primary }
    }
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
            primary: self.primary
        }
    }
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
            params.next().expect("param shouldn't be empty"),
        );
        while let Some(prm) = params.next() {
            res = format!("{},\"{}\"",
                res,
                prm,
            );
        }
        format!("{}{}", res, "]")
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

pub async fn download_file(
    client: &reqwest::Client,
    f_in: &ModrinthFile,
    out_dir: &PathBuf
) -> Result<(), Box<dyn std::error::Error>> 
{
    let res = client.get(f_in.url())
        .send()
        .await?
        .bytes()
        .await?;
    let mut f_out = fs::File::create(
        out_dir.join(f_in.filename())
    )?;
    f_out.write_all(&res)?;
    println!("Successfully downloaded {}", f_in.filename());
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