use futures::future;
use std::fs::{self, DirEntry, File};
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

use crate::modrinth::{
    ModrinthFile,
    VersionQuery,
    download_file,
    get_file_direct
};

#[cfg(test)]
mod tests;
pub mod modrinth;
pub mod arguments;

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
    let ids = vec_from_lines(filename)?;
    if conf.verify() {
        modrinth_verify_ids_from_list(
            &conf,
            &client,
            &ids,
            &out_dir
        ).await;
    } else {
        modrinth_download_from_id_list(
            &conf,
            &client,
            &ids,
            &out_dir
        ).await?;
    };
    Ok(())
}

pub async fn single_id<'a>(
    conf: &arguments::Config<'a>,
    client: &reqwest::Client,
    id: &String,
    out_dir: &PathBuf
) -> Result<(), Box<dyn std::error::Error>>
{
    if conf.verify() {
        modrinth_verify_id(
            &conf,
            &client,
            &id,
            &out_dir
        ).await;
    } else {
        modrinth_download_from_id(&conf,
            &client,
            id,
            &out_dir
        ).await?;
    };
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

async fn modrinth_download_from_id<'a>(
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

async fn collect_modrinth_files(
    client: &reqwest::Client,
    ids: &Vec<String>,
    query: &modrinth::VersionQuery
) -> Vec<Option<ModrinthFile>>
{
    let mut file_results  = Vec::new();
    for id in ids {
        println!("Getting ID '{id}'...");
        file_results.push(
            modrinth::get_file_direct(client, id, &query)
        )
    }
    future::join_all(file_results).await
}

async fn collect_modrinth_downloads(
    client: &reqwest::Client,
    files: &Vec<Option<ModrinthFile>>,
    out_dir: &PathBuf
) -> Vec<Result<(), Box<dyn std::error::Error>>>
{
    let mut download_tasks = Vec::new();
    for file in files {
        if let Some(f) = file {
            download_tasks.push(
                modrinth::download_file(client, f, out_dir)
            )
        }
    }
    future::join_all(download_tasks).await
}

async fn modrinth_download_from_id_list<'a>(
    conf: &arguments::Config<'a>,
    client: &reqwest::Client,
    ids: &Vec<String>,
    out_dir: &PathBuf
) -> Result<(), Box<dyn std::error::Error>>
{
    let query: modrinth::VersionQuery = modrinth::VersionQuery::build_query(
        conf.mcvs(), 
        &conf.loader_as_string()
    );
    let mod_files = collect_modrinth_files(client, ids, &query).await;
    let downloads = collect_modrinth_downloads(
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

async fn modrinth_verify_id<'a>(
    conf: &arguments::Config<'a>,
    client: &reqwest::Client,
    id: &String,
    out_dir: &PathBuf
) -> () {
    let query: modrinth::VersionQuery = modrinth::VersionQuery::build_query(
        &conf.mcvs(),
        &conf.loader_as_string()
    );
    if let Some(f) = get_file_direct(&client, &id, &query).await {
        if modrinth::verify_file(&f, &out_dir) {
            println!("Successfully verified file {}", f.filename());
        }
        else {
            println!("Unable to verify file {}", f.filename());
        }
    };
    ()
}

async fn modrinth_verify_ids_from_list<'a>(
    conf: &arguments::Config<'a>,
    client: &reqwest::Client,
    ids: &Vec<String>,
    out_dir: &PathBuf
) -> () {
    let query: modrinth::VersionQuery = modrinth::VersionQuery::build_query(
        &conf.mcvs(),
        &conf.loader_as_string()
    );
    let mod_files: Vec<ModrinthFile> = collect_modrinth_files(client, ids, &query)
        .await
        .into_iter()
        .filter_map(|f| f)
        .collect();
    let mut bad_results: u32 = 0;
    for f in &mod_files {
        if modrinth::verify_file(&f, out_dir){
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

fn vec_from_lines(filename: &Path) -> io::Result<Vec<String>> {
    let mut out = Vec::new();
    let f_in = File::open(filename)?;
    for reader_line in io::BufReader::new(f_in).lines() {
        if let Ok(line) = reader_line {
            out.push(line)
        }
    }
    Ok(out)
}