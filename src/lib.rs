use futures::future;
use std::fs::File;
use std::io;
use std::io::BufRead;

use crate::modrinth::ModrinthFile;

#[cfg(test)]
mod tests;
pub mod modrinth;
pub mod arguments;

static APP_USER_AGENT: &str = concat!(
    "hwschieding/",
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/hwschieding/mcmodgetter)"
);

pub fn create_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()
}

async fn collect_modrinth_downloads(
    client: &reqwest::Client,
    files: &Vec<Option<ModrinthFile>>,
    out_dir: &str
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

pub async  fn modrinth_download_from_id_list(
    conf: &arguments::Config,
    client: &reqwest::Client,
    ids: &Vec<String>,
    out_dir: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>>
{
    let query: modrinth::VersionQuery = modrinth::VersionQuery::build_query(
        conf.mcvs(), 
        &conf.loader_as_string()
    );
    let mut file_results  = Vec::new();
    for id in ids {
        println!("Getting ID '{id}'...");
        file_results.push(
            modrinth::get_file_direct(client, id, &query)
        )
    }
    let mod_files = future::join_all(file_results).await;
    let downloads = collect_modrinth_downloads(
        client,
        &mod_files,
        out_dir.unwrap_or("mods/")
    ).await;
    let download_errors: Vec<Box<dyn std::error::Error>> = downloads.into_iter()
        .filter_map(Result::err)
        .collect();
    for err in download_errors {
        println!("Download error: {err}")
    }
    Ok(())
}

pub fn vec_from_lines(filename: &String) -> io::Result<Vec<String>> {
    let mut out = Vec::new();
    let f_in = File::open(filename)?;
    for reader_line in io::BufReader::new(f_in).lines() {
        if let Ok(line) = reader_line {
            out.push(line)
        }
    }
    Ok(out)
}