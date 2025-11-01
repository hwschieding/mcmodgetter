use std::fmt;
use futures::future;
use serde::Deserialize;

#[cfg(test)]
mod tests;

static APP_USER_AGENT: &str = concat!(
    "hwschieding/",
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/hwschieding/mcmodgetter)"
);
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
    files: Vec<File>
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
pub struct File {
    url: String,
    filename: String,
    primary: bool
}

impl Clone for File {
    fn clone(&self) -> Self {
        File {
            url: self.url.clone(),
            filename: self.filename.clone(),
            primary: self.primary
        }
    }
}

pub fn create_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()
}

pub async fn get_project(client: &reqwest::Client, id: &str) -> Result<Project, reqwest::Error> {
    let url = format!("{}{}{}", MODRINTH_URL, "/v2/project/", id);
    let response = client.get(url).send().await?;
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

pub async fn get_version(client: &reqwest::Client, project_id: &str) -> Result<Vec<Version>, reqwest::Error> {
    let url = format!("{}{}{}{}", MODRINTH_URL, "/v2/project/", project_id, "/version");
    let response = client.get(url).send().await?;
    response.json::<Vec<Version>>().await
}

pub async fn get_top_version(client: & reqwest::Client, project_id: &str) -> Result<Version, VersionError> {
    let response = get_version(client, project_id).await?;
    match response.get(0).cloned() {
        Some(v) => Ok(v),
        None => Err(VersionError::NoVersion("No version available"))
    }
}

pub async fn search_for_primary_file(files: &Vec<File>) -> Option<usize> {
    if files.len() == 0 {
        return None; // If there are no files
    }
    for (i, file) in files.iter().enumerate() {
        if file.primary {
            return Some(i);
        }
    }
    Some(0) // If no file is marked primary, return 1st file
}
