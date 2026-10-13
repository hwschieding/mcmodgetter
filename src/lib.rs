use std::{env, io, error, fs};
use std::path::{Path, PathBuf};

use crate::arguments::{DownloadSettings, ReadModsSettings};

#[cfg(test)]
mod tests;
pub mod modrinth;
pub mod hangar;
pub mod arguments;
pub mod file_parse;
pub mod items;
pub mod http_handler;

/*  
    This user agent is only to be used for mcmodgetter and projects affiliated
    with me (https://github.com/hwschieding) and mcmodgetter
    (https://github.com/hwschieding/mcmodgetter).

    I ask as a basic courtesy that this user agent not be reused or misused in
    other projects, including forks and custom builds of mcmodgetter. As stated
    in the README, forks/custom builds/redistributions are not affiliated with
    or endorsed by me or mcmodgetter.
*/
static APP_USER_AGENT: &'static str = concat!(
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
    conf: &arguments::Config<'a, &ReadModsSettings>,
    client: &reqwest::Client,
) -> Result<(), Box<dyn std::error::Error>>
{
    let file = match conf.settings().supplier()
    {
        arguments::IdSupplier::File(f) => f,
        _ => { println!("Couldn't get filename"); return Ok(()) }
    };

    match conf.settings().api()
    {
        arguments::Api::Modrinth => modrinth_read_mods(client, file).await?,
        arguments::Api::Hangar => hangar_read_mods(client, file).await?,
    }
    Ok(())
}

async fn modrinth_read_mods<'a>(
    client: &reqwest::Client,
    file: &PathBuf,
) -> io::Result<()>
{
    Ok(modrinth::list_projects(client, &get_ids(file)?.modrinth()).await)
}

async fn hangar_read_mods<'a>(
    client: &reqwest::Client,
    file: &PathBuf,
) -> io::Result<()>
{
    Ok(hangar::list_projects(client, &get_ids(file)?.hangar()).await)
}

pub async fn operation_download<'a>(
    conf: &arguments::Config<'a, &DownloadSettings>,
    client: &reqwest::Client,
    out_dir: PathBuf
) -> Result<(), Box<dyn error::Error>>
{
    match conf.settings().platform().api()
    {
        arguments::Api::Hangar => hangar_download(conf, client, out_dir).await,
        arguments::Api::Modrinth => modrinth_download(conf, client, out_dir).await,
    }
}

async fn modrinth_download<'a>(
    conf: &arguments::Config<'a, &DownloadSettings>,
    client: &reqwest::Client,
    out_dir: PathBuf
) -> Result<(), Box<dyn error::Error>>
{
    match conf.settings().supplier()
    {
        arguments::IdSupplier::File(file) => 
        {
            modrinth::download_from_id_list(conf, client, &get_ids(file)?.modrinth(), out_dir).await
        },
        arguments::IdSupplier::Id(id) =>
        {
            modrinth::download_from_id(conf, client, id, out_dir).await
        }
    }
}

async fn hangar_download<'a>(
    conf: &arguments::Config<'a, &DownloadSettings>,
    client: &reqwest::Client,
    out_dir: PathBuf
) -> Result<(), Box<dyn error::Error>>
{
    match conf.settings().supplier()
    {
        arguments::IdSupplier::File(file) => 
        {
            hangar::download_from_id_list(conf, client, &get_ids(file)?.hangar(), out_dir).await
        },
        arguments::IdSupplier::Id(id) =>
            hangar::download_from_id(conf, client, id, out_dir).await
    }
}

pub async fn operation_verify<'a>(
    conf: &arguments::Config<'a, &DownloadSettings>,
    client: &reqwest::Client,
    out_dir: PathBuf
) -> Result<(), Box<dyn error::Error>>
{
    match conf.settings().platform().api()
    {
        arguments::Api::Modrinth => modrinth_verify(conf, client, out_dir).await,
        arguments::Api::Hangar => hangar_verify(conf, client, out_dir).await,
    }
}

async fn modrinth_verify<'a>(
    conf: &arguments::Config<'a, &DownloadSettings>,
    client: &reqwest::Client,
    out_dir: PathBuf
) -> Result<(), Box<dyn error::Error>>
{
    if let arguments::IdSupplier::File(file) = conf.settings().supplier()
    {
        modrinth::verify_ids_from_list(conf, client, &get_ids(file)?.modrinth(), out_dir).await
    }
    else if let arguments::IdSupplier::Id(id) = conf.settings().supplier()
    {
        modrinth::verify_id(conf, client, id, out_dir).await
    }
    else { Ok(()) }
}

async fn hangar_verify<'a>(
    conf: &arguments::Config<'a, &DownloadSettings>,
    client: &reqwest::Client,
    out_dir: PathBuf
) -> Result<(), Box<dyn error::Error>>
{
    if let arguments::IdSupplier::File(file) = conf.settings().supplier()
    {
        hangar::verify_ids_from_list(conf, client, &get_ids(file)?.hangar(), out_dir).await
    }
    else if let arguments::IdSupplier::Id(id) = conf.settings().supplier()
    {
        hangar::verify_id(conf, client, id, out_dir).await
    }
    else { Ok(()) }

}

pub async fn modrinth_single_id<'a>(
    conf: &arguments::Config<'a, &DownloadSettings>,
    client: &reqwest::Client,
    id: &str,
    out_dir: PathBuf
) -> Result<(), Box<dyn std::error::Error>>
{
    modrinth::download_from_id(conf, client, id, out_dir).await?;
    Ok(())
}

pub async fn clear_mods(
    out_dir: &PathBuf
) -> Result<(), Box<dyn std::error::Error>>
{
    if !out_dir.is_dir() {
        let out_dir_name = out_dir.display();
        println!("'{out_dir_name}' does not exist/isn't a directory.");
        println!("clearmods can only be performed on existing directories.");
        return Ok(())
    }
    println!("Delete all tracked '.jar' files in directory {}? (y/n)",
        &out_dir.display()
    );
    let mut user_ans = String::new();
    io::stdin().read_line(&mut user_ans)?;
    if user_ans.trim().to_lowercase() == "y" {
        clear_dir(&out_dir).await?;
    }
    Ok(())
}

pub fn create_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()
}

pub fn get_out_dir<'a>(conf: &arguments::Config<'a, &DownloadSettings>) -> io::Result<PathBuf> {
    let dirname: &PathBuf = match conf.opts().out_dir()
    {
        Some(d) => d,
        None => &PathBuf::from(conf
            .settings()
            .platform()
            .out_dir_name()
        ),
    };

    Ok(
        env::current_dir()?.join(dirname)
    )
}

pub fn create_out_dir(dir_path: &PathBuf) -> Result<(), io::Error> {
    fs::create_dir_all(dir_path)?;
    Ok(())
}

pub fn help() -> () {
    println!(
        "{} v{}\n\nCOMMANDS:
  download: Downloads specifed items from a provider (use -id or -file, -v required)
  checkmods: Verifies items in mod folder against specified options
  clearmods: Removes tracked files in specified mod folder (-o required)
  readmods: Query names and descriptions for project ids in the specified file (use -file)
  *Include at exactly one of these when running mcmodgetter.

  For more information about setting up a modlist file, see
      https://github.com/hwschieding/mcmodgetter#setting-up-a-mod-list-file

  OPTIONS:
  -id <string>: Specifies single provider ID to download
  -file <filename>: Specifies filename of provider IDs to download

  -v <platform version>: Specifies platform version to query for items

  [-modrinth|-hangar] <platform>: Specify which provider to download from and a platform.
      Possible platforms are: fabric, forge, neoforge, paper, velocity

  *To query for multiple versions/platforms, separate by commas(,) with no spaces
  **Provider and plaform default to Modrinth and Fabric respectively

  -o <folder>: Specifies output folder for mods relative to local directory
  *Will default to [\"mods/\"|\"plugins/\"|\"items/\"] based on specified platforms.
  *Default will not apply for clearmods

  --skipdeps: Skip automatic dependency handling

  --releaseonly: Query for release builds only
  
  -h, --help, -help: Show this help prompt",
    env!("CARGO_PKG_NAME"),
    env!("CARGO_PKG_VERSION")
    )
}

async fn clear_dir(out_dir: &PathBuf) -> Result<(), Box<dyn error::Error>>{
    println!("[REMOVAL] Clearing folder {}...", out_dir.display());

    let mut tracker = file_parse::TrackerFile::build(out_dir)?;

    tracker.wipe_all_entries().await;
    tracker.write_csv();

    Ok(())
}