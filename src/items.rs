use std::{
    collections::HashSet,
    error,
    fmt,
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};
use tokio::{sync::Semaphore, task::JoinSet};

use crate::{file_parse::TrackerFile, http_handler};

static VERIFICATION_SIG: &'static str = "VERIFY";
static ERROR_SIG: &'static str = "ERROR";

fn dep_msg(msg: &str) -> String
{
    format!("[DEPENDENCY] {}", msg)
}

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

pub trait Item : Sized
{
    type Dep: Dependency;
    type VersionQuery: http_handler::Query;

    // #[allow(async_fn_in_trait)]
    fn download(
        &self,
        downloader: &http_handler::Downloader,
    ) -> impl std::future::Future<Output = Result<(), http_handler::DownloadError>> + std::marker::Send;
    fn id(&self) -> &String;
    fn version_id(&self) -> &String;
    fn version_title(&self) -> &String;
    fn filename(&self) -> &PathBuf;
    // fn downloaded(&self) -> bool;

    fn item_id(&self) -> &'static str;

    #[allow(async_fn_in_trait)]
    async fn get_filtered_deps<'a, F>(
        &self,
        requester: &DependencyRequester<'a, Self::VersionQuery>,
        verify: F,
    ) -> Vec<Self>
    where
        F: Fn(&Self::Dep) -> bool
    ;
}

pub trait Dependency
{
    fn project_id(&self) -> Option<&String>;
}

pub struct DependencyRequester<'a, V>
where
    V: http_handler::Query
{
    pid_req: &'a http_handler::QueryRequest<V>,
    vid_req: &'a http_handler::BasicRequest,
}
impl<'a, V> DependencyRequester<'a, V>
where
    V: http_handler::Query
{
    pub fn new(
        pid_req: &'a http_handler::QueryRequest<V>,
        vid_req: &'a http_handler::BasicRequest
    ) -> DependencyRequester<'a, V>
    {
        DependencyRequester { pid_req, vid_req }
    }

    pub fn project_id_requester(
        &self
    ) -> &'a http_handler::QueryRequest<V>
    {
        self.pid_req
    }
    pub fn version_id_requester(&self) -> &'a http_handler::BasicRequest
    {
        self.vid_req
    }
}
pub struct DependencyHandler<'a, I>
    where I: Item
{
    modlist: &'a mut Vec<I>,
    dep_check_stack: Vec<usize>,
    present_ids: HashSet<String>,
}
impl<'a, I> DependencyHandler<'a, I>
where
    I: Item
{
    pub fn build(modlist: &'a mut Vec<I>) -> Self
    {
        let dep_check_stack: Vec<usize> = (0..modlist.len()).collect();
        let present_ids: HashSet<String> = modlist
            .iter()
            .map(|item| item.id().clone())
            .collect()
        ;
        DependencyHandler { modlist, dep_check_stack, present_ids }
    }

    fn is_dep_present(&self, dep: &I::Dep) -> bool
    {
        if let Some(pid) = dep.project_id()
            && self.present_ids.contains(pid)
        {
            return true
        }

        false
    }

    async fn try_get_deps(&mut self, requester: &DependencyRequester<'a, I::VersionQuery>, idx: &usize)
    {
        let deps = {
            match self.modlist.get(*idx)
            {
                Some(item) => item.get_filtered_deps(
                    requester,
                    |d| { !self.is_dep_present(d) }).await,
                None => { return () }
            }
        };

        for value in deps
        {
            if !self.present_ids.contains(value.id().as_str()){
                println!("{}", dep_msg(&format!("Found dependency: {}", value.version_title())));
                self.present_ids.insert(value.id().clone());

                self.dep_check_stack.push(self.modlist.len());
                self.modlist.push(value);
            }
        }
    }

    pub async fn acquire_all_dependencies(
        &mut self,
        requester: &DependencyRequester<'a, I::VersionQuery>
    ) -> ()
    {
        while let Some(idx) = self.dep_check_stack.pop()
        {
            self.try_get_deps(requester, &idx).await;
        }
    }
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

pub async fn download_items<T>(
    downloader: Arc<http_handler::Downloader>,
    items: Vec<T>,
) -> Result<Vec<T>, Box<dyn error::Error>>
where
    T: Send + 'static,
    T: Item
{
    let mut set = JoinSet::new();
    let semaphore = Arc::new(Semaphore::new(8));

    let start = Instant::now();
    for m in items
    {
        let sem_clone = Arc::clone(&semaphore);
        let dl_clone = Arc::clone(&downloader);

        set.spawn(async move {
            let _permit = match sem_clone.acquire().await
            {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Async download failed: {e}");
                    return None;
                }
            };

            match m.download(&dl_clone).await
            {
                Ok(_) => Some(m),
                Err(e) => {
                    eprintln!("{e}");
                    None
                }
            }
        });
    }

    let mut completed_downloads = Vec::new();
    while let Some(res) = set.join_next().await
    {
        if let Ok(Some(i)) = res
        {
            completed_downloads.push(i);
        }
    }

    let duration = start.elapsed();

    println!("Done in {}ms\n", duration.as_millis());

    Ok(completed_downloads)
}

pub fn ask_user_to_download(out_dir: &Path) -> io::Result<bool>
{
    println!("Download all items to directory '{}'? (y/n)", out_dir.display());
    let mut user_ans = String::new();
    io::stdin().read_line(&mut user_ans)?;

    Ok(user_ans.trim().to_lowercase() == "y")
}
