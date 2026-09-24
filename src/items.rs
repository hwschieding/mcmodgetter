use std::{fmt, io, error};
use crate::http_handler;

static VERIFICATION_SIG: &str = "VERIFY";
static ERROR_SIG: &str = "ERROR";

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
    fn build() -> impl std::future::Future<Output = Self> + Send;
    fn download() -> impl std::future::Future<Output = Result<(), http_handler::DownloadError>> + Send;
    fn name() -> String;
}