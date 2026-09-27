use std::{io, error, fs};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;
pub mod modrinth;
pub mod arguments;
pub mod file_parse;
pub mod items;
pub mod http_handler;

static DEFAULT_OUT_DIR: &'static str = "mods";

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

pub async fn verify<'a>(
    conf: &arguments::Config<'a>,
    client: &reqwest::Client,
    out_dir: PathBuf
) -> Result<(), Box<dyn error::Error>>
{
    if let Some(file) = conf.options().get_file()
    {
        let ids = get_ids(file)?;
        if let Some(modrinth_ids) = ids.modrinth()
        {
            modrinth::verify_ids_from_list(conf, client, modrinth_ids, out_dir).await?;
        }

        return Ok(())
    }

    if let Some(id) = conf.options().get_id()
    {
        modrinth::verify_id(conf, client, id, out_dir).await?;

        return Ok(())
    }

    Ok(())
}

pub async fn id_from_file<'a>(
    conf: &arguments::Config<'a>,
    client: &reqwest::Client,
    out_dir: PathBuf
) -> Result<(), Box<dyn error::Error>>
{
    let ids = match conf.options().get_file()
    {
        Some(filename) => get_ids(filename)?,
        None => {
            println!("Couldn't get filename");
            return Ok(())
        }
    };

    if let Some(modrinth_ids) = ids.modrinth() {
        println!("Handling modrinth ids...");
        modrinth::download_from_id_list(conf, client, modrinth_ids, out_dir).await?;
    };
    
    Ok(())
}

pub async fn single_id<'a>(
    conf: &arguments::Config<'a>,
    client: &reqwest::Client,
    out_dir: PathBuf
) -> Result<(), Box<dyn std::error::Error>>
{
    if let Some(id) = conf.options().get_id() {
        modrinth::download_from_id(conf, client, id, out_dir).await?;
    }
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
  clearmods: Removes tracked files in specified mod folder (use -o if necessary)
  readmods: Query names and descriptions for project ids in the specified file (use -file)
  *Include at exactly one of these when running mcmodgetter.

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

async fn clear_dir(out_dir: &PathBuf) -> Result<(), Box<dyn error::Error>>{
    println!("[REMOVAL] Clearing folder {}...", out_dir.display());

    let mut tracker = file_parse::TrackerFile::build(out_dir)?;

    tracker.wipe_all_entries().await;
    tracker.write_csv();

    Ok(())
}