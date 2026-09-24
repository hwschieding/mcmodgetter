use std::{fmt, io, error};

static DOWNLOAD_SIG: &str = "DOWNLOAD";
static VERIFICATION_SIG: &str = "VERIFY";
static ERROR_SIG: &str = "ERROR";

#[derive(Debug)]
pub enum DownloadError {
    BadRequest(reqwest::Error),
    BadFile(io::Error),
    BadHash(String),
}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sig = format!("[{}/{}]", DOWNLOAD_SIG, ERROR_SIG);
        match self {
            Self::BadRequest(err) => write!(f, "{} Bad request: {}", sig, err),
            Self::BadFile(err) => write!(f, "{} Bad file: {}", sig, err),
            Self::BadHash(msg) => write!(f, "{} Bad hash: {}", sig, msg),
        }
    }
}

impl error::Error for DownloadError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::BadRequest(err) => Some(err),
            Self::BadFile(err) => Some(err),
            _ => None
        }
    }
}

impl From<reqwest::Error> for DownloadError {
    fn from(value: reqwest::Error) -> Self {
        Self::BadRequest(value)
    }
}

impl From<std::io::Error> for DownloadError {
    fn from(value: std::io::Error) -> Self {
        Self::BadFile(value)
    }
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

pub trait Item {
    fn build() -> impl std::future::Future<Output = Self> + Send;
    fn download() -> impl std::future::Future<Output = Result<(), DownloadError>> + Send;
    fn name() -> String;
}