use std::{path::{Path, PathBuf}, io, fmt, error};
use futures::future;

use crate::{http_handler, file_parse::{TrackerFile}};

static VERIFICATION_SIG: &'static str = "VERIFY";
static ERROR_SIG: &'static str = "ERROR";

pub enum VerificationResult {
    Ok(String),
    Err(String)
}

impl VerificationResult {
    pub fn print(&self, platform_sig: Option<&'static str>) -> () {
        let sig = platform_sig.unwrap_or("");
        match self {
            Self::Ok(v) => {
                println!("[{}/{}] {v}", sig, VERIFICATION_SIG)
            }
            Self::Err(e) => {
                println!("[{}/{}/{}] {e}", sig, VERIFICATION_SIG, ERROR_SIG)
            }
        }
    }
    pub fn is_ok(&self) -> bool {
        if let Self::Ok(_) = self {
            true
        } else {
            false
        }
    }
}

#[derive(Debug)]
pub enum ItemError
{
    BadRequest(reqwest::Error),
    NoVersion(String),
    NoFile(String),
}

impl fmt::Display for ItemError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self
        {
            Self::BadRequest(err) => write!(
                f, "Bad request: {}", err
            ),
            Self::NoVersion(msg) => write!(
                f, "No version: {}", msg
            ),
            Self::NoFile(msg) => write!(
                f, "No file: {}", msg
            )
        }
    }
}

impl error::Error for ItemError
{
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self
        {
            Self::BadRequest(err) => Some(err),
            _ => None
        }
    }
}

impl From<reqwest::Error> for ItemError
{
    fn from(value: reqwest::Error) -> Self {
        Self::BadRequest(value)
    }
}

pub trait Item {
    fn download<'a>(
        &mut self,
        downloader: &http_handler::Downloader<'a>,
    ) -> impl std::future::Future<Output = ()> + Send;
    fn id(&self) -> &String;
    fn version_id(&self) -> &String;
    fn filename(&self) -> &PathBuf;
    fn downloaded(&self) -> bool;

    fn item_id(&self) -> &'static str;
}

pub fn filter_items_by_tracker<T>(
    items: Vec<T>,
    tracker: &TrackerFile
) -> Vec<T>
where
    T: Item
{
    let start_len = items.len();

    let res: Vec<T> = items
        .into_iter()
        .filter(|item| !tracker.matches_entry(item))
        .collect()
    ;

    let new_len = res.len();

    println!("\n{} items are available for download\n{} are already present\n",
        new_len,
        start_len - new_len
    );

    res
}

pub async fn download_items<'a, T>(
    downloader: &http_handler::Downloader<'a>,
    items: &mut Vec<T>,
) -> ()
where
    T: Item
{
    let mut download_futures = Vec::new();

    for m in items
    {
        download_futures.push(m.download(downloader));
    }

    future::join_all(download_futures).await;
    ()
}

pub fn ask_user_to_download(out_dir: &Path) -> io::Result<bool>
{
    println!("Download all items to directory '{}'? (y/n)", out_dir.display());
    let mut user_ans = String::new();
    io::stdin().read_line(&mut user_ans)?;

    Ok(user_ans.trim().to_lowercase() == "y")
}
