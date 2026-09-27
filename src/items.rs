use std::{path};
use crate::http_handler;

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

pub trait Item {
    fn download<'a>(
        &mut self,
        downloader: &http_handler::Downloader<'a>,
    ) -> impl std::future::Future<Output = ()> + Send;
    fn id(&self) -> &String;
    fn version_id(&self) -> &String;
    fn filename(&self) -> &path::PathBuf;
    fn downloaded(&self) -> bool;

    fn item_id(&self) -> &'static str;
}