use std::fs::{self, DirEntry};
use std::io;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;
pub mod modrinth;
pub mod arguments;
pub mod mmg_parse;

const DEFAULT_OUT_DIR: &str = "mods";
const APP_USER_AGENT: &str = concat!(
    "hwschieding/",
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/hwschieding/mcmodgetter)"
);

pub async fn id_from_file<'a>(
    conf: &arguments::Config<'a>,
    client: &reqwest::Client,
    filename: &Path,
    out_dir: &PathBuf
) -> Result<(), Box<dyn std::error::Error>>
{
    let ids: mmg_parse::FileIDs;
    if let Some(ext) = filename.extension() && ext == mmg_parse::FILE_EXT {
        println!("Parsing .mmg file!");
        ids = mmg_parse::parse_ids(filename)?;
    } else {
        println!("Parsing some other filetype");
        ids = mmg_parse::parse_ids_txt(filename)?;
    }
    if let Some(modrinth_ids) = ids.modrinth() {
        modrinth::handle_list_input(conf, client, modrinth_ids, out_dir).await?;
    };
    if let Some(curse_ids) = ids.curseforge() {
        for id in curse_ids {
            println!("Curseforge id '{id}'");
        }
    }
    Ok(())
}

pub async fn single_id<'a>(
    conf: &arguments::Config<'a>,
    client: &reqwest::Client,
    id: &String,
    out_dir: &PathBuf
) -> Result<(), Box<dyn std::error::Error>>
{
    modrinth::handle_single_input(conf, client, id, out_dir).await?;
    Ok(())
}

pub fn clear_mods(
    out_dir: &PathBuf
) -> Result<(), Box<dyn std::error::Error>>
{
    println!("Delete everything in directory {}? (y/n)",
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

pub fn get_out_dir(conf_dir: &Option<&Path>) -> Result<PathBuf, io::Error> {
    let path = conf_dir.unwrap_or(Path::new(DEFAULT_OUT_DIR));
    fs::create_dir_all(path)?;
    Ok(PathBuf::from(path))
}

fn remove_entry(entry: &DirEntry) -> io::Result<()> {
    let path = entry.path();
    if path.is_dir() {
        fs::remove_dir_all(&path)?;
    } else if path.is_file() {
        fs::remove_file(&path)?;
    }
    println!("Removed entry {}", &path.display());
    Ok(())
}

fn clear_dir(out_dir: &PathBuf) -> io::Result<()>{
    println!("Clearing folder {}...", out_dir.display());
    for entry in fs::read_dir(out_dir)? {
        match entry {
            Ok(v) => {
                if let Err(e) = remove_entry(&v) {
                    println!("Could not remove entry {} because {e}",
                        &v.path().display()
                    );
                }
            },
            Err(e) => {
                println!("Could not resolve dir entry: {e}");
            }
        }
    }
    Ok(())
}