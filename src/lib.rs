use std::fs::{self, DirEntry};
use std::{fmt, io, error};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;
pub mod modrinth;
pub mod arguments;
pub mod file_parse;

const DEFAULT_OUT_DIR: &str = "mods";
const APP_USER_AGENT: &str = concat!(
    "hwschieding/",
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/hwschieding/mcmodgetter)"
);

fn get_ids<'a>(f: &Path) -> io::Result<file_parse::FileIDs> {
    println!("Parsing file '{}'...", f.display());
    return file_parse::parse_ids(f)
}

pub async fn read_mods<'a>(
    conf: &arguments::Config<'a>,
    client: &reqwest::Client,
) -> Result<(), Box<dyn std::error::Error>>
{
    if let Some(filename) = conf.options().get_file() {
        let ids = get_ids(filename)?;
        if let Some(modrinth_ids) = ids.modrinth() {
            modrinth::list_projects(client, modrinth_ids).await;
        }
    } else {
        println!("Couldn't get filename");
    }
    Ok(())
}

pub async fn id_from_file<'a>(
    conf: &arguments::Config<'a>,
    client: &reqwest::Client,
    out_dir: &PathBuf
) -> Result<(), Box<dyn std::error::Error>>
{
    if let Some(filename) = conf.options().get_file() {
        let ids = get_ids(filename)?;

        if let Some(modrinth_ids) = ids.modrinth() {
            println!("Handling modrinth ids...");
            modrinth::handle_list_input(conf, client, modrinth_ids, out_dir).await?;
        };
        if let Some(curse_ids) = ids.curseforge() {
            for id in curse_ids {
                println!("Curseforge id '{id}'");
            }
        }
    } else {
        println!("Couldn't get filename");
    }
    Ok(())
}

pub async fn single_id<'a>(
    conf: &arguments::Config<'a>,
    client: &reqwest::Client,
    out_dir: &PathBuf
) -> Result<(), Box<dyn std::error::Error>>
{
    if let Some(id) = conf.options().get_id() {
        modrinth::handle_single_input(conf, client, id, out_dir).await?;
    }
    Ok(())
}

pub fn clear_mods(
    out_dir: &PathBuf
) -> Result<(), Box<dyn std::error::Error>>
{
    if !out_dir.is_dir() {
        let out_dir_name = out_dir.display();
        println!("'{out_dir_name}' does not exist/isn't a directory.");
        println!("clearmods can only be performed on existing directories.");
        return Ok(())
    }
    println!("Delete all '.jar' files in directory {}? (y/n)",
        &out_dir.display()
    );
    let mut user_ans = String::new();
    io::stdin().read_line(&mut user_ans)?;
    if user_ans.trim().to_lowercase() == "y" {
        clear_dir(&out_dir)?;
    }
    Ok(())
}

pub fn create_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()
}

pub fn get_out_dir(conf_dir: &Option<&Path>) -> PathBuf {
    let path = conf_dir.unwrap_or(Path::new(DEFAULT_OUT_DIR));
    PathBuf::from(path)
}

pub fn create_out_dir(dir_path: &PathBuf) -> Result<(), io::Error> {
    fs::create_dir_all(dir_path)?;
    Ok(())
}

pub fn help() -> () {
    println!(
        "COMMANDS:
  download: Downloads specifed mods from modrinth (use -id or -file, -mcv required)
  checkmods: Verifies mods in mod folder against specified options
  clearmods: Removes all .jar files in specified mod folder (use -o)
  readmods: List all ids in the modlist file with a matching project title (use -file)

  OPTIONS:
  -id <string>: Specifies single modrinth ID to download
  -file <filename>: Specifies filename of modrinth IDs to download

  -mcv <minecraft version>: Specifies MC version to query for mods
  -l <mod loader> [DEFAULT=fabric]: Specifies mod loader to query for (fabric, forge, etc)
  *To query for multiple versions/loaders, separate by commas(,) with no spaces

  -o <folder> [DEFAULT=mods]: Specifies output folder for mods relative to local directory

  --skipdeps: Skip searching for and downloading mod dependencies
  
  -h, --help, -help: Show this help prompt"
    )
}

#[derive(Debug)]
enum RemovalError {
    BadExtensionForFile(String),
    FileError(io::Error)
}

impl fmt::Display for RemovalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadExtensionForFile(file_name) => write!(f, "[REMOVAL/ERROR] Unexpected extension for '{file_name}'"),
            Self::FileError(err) => write!(f, "[REMOVAL/ERROR] Could not remove file: {err}")
        }
    }
}

impl error::Error for RemovalError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::FileError(e) => Some(e),
            _ => None
        }
    }
}

impl From<io::Error> for RemovalError {
    fn from(value: io::Error) -> Self {
        Self::FileError(value)
    }    
}

fn remove_jar(entry: &DirEntry) -> Result<(), RemovalError> {
    let path = entry.path();
    if let Some(ext) = path.extension() && ext == "jar"{
        fs::remove_file(&path)?;
        println!("[REMOVAL] Removed entry {}", &path.display());
        Ok(())
    } else {
        Err(RemovalError::BadExtensionForFile(path.display().to_string()))
    }
}

fn clear_dir(out_dir: &PathBuf) -> io::Result<()>{
    println!("[REMOVAL] Clearing folder {}...", out_dir.display());
    let entries = fs::read_dir(out_dir)?
    .into_iter()
    .filter_map(|ent_res| {
        match ent_res {
            Ok(de) => Some(de),
            Err(err) => {
                println!("[REMOVAL/ERROR] Could not resolve dir entry: {err}");
                None
            }
        }
    })
    .collect::<Vec<DirEntry>>();

    for entry in entries {
        if let Err(e) = remove_jar(&entry) {
            println!("{e}");
        }
    }
    Ok(())
}