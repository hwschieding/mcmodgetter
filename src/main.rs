use std::{env, process};
use std::error::Error;

use mcmodgetter::{create_client, modrinth::get_project};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let conf = Config::build_from_args(&args).unwrap_or_else(|e| {
        eprintln!("{e}");
        process::exit(1);
    });
    if let Err(e) = run(conf).await {
        eprintln!("{e}");
        process::exit(1);
    }
}

async fn run(conf: Config) -> Result<(), Box<dyn Error>> {
    if let AppMode::SingleId(id) = conf.mode {
        println!("Creating client...");
        let client = create_client()?;
        println!("Getting project...");
        let proj = get_project(&client, &id).await?;
        println!("id={}\ntitle={}\ndesc={}", proj.get_id(), proj.get_title(), proj.get_desc());
    }
    Ok(())
}

pub enum AppMode {
    SingleId(String),
    IdFromFile(String),
}

pub enum Loader {
    Fabric,
    Neoforge,
    Forge
}

pub struct Config {
    mode: AppMode,
    mcvs: String,
    loader: Loader,
}

impl Config {
    pub fn build_from_args(args: &Vec<String>) -> Result<Config, &'static str> {
        let mut mode: Result<AppMode, &'static str> = Err("No ID specified");
        let mut mcvs: Result<String, &'static str> = Err("No mc version specified");
        let mut loader: Loader = Loader::Fabric;
        let mut args_iter = args.iter();
        args_iter.next();
        while let Some(arg) = args_iter.next(){
            match arg.as_str() {
                "-id" => mode = Ok(get_id_mode(args_iter.next())?),
                "--readfile" => mode = Ok(get_file_mode(args_iter.next())?),
                "-mcv" => mcvs = Ok(get_mcvs(args_iter.next())?),
                "-l" => loader = get_loader(args_iter.next())?,
                _ => println!("arg '{arg}' not recognized")
            }
        };
        let mode = mode?;
        let mcvs = mcvs?;
        Ok(Config { mode, mcvs, loader })
    }
}

fn get_mcvs(mcvs: Option<&String>) -> Result<String, &'static str> {
    match mcvs {
        Some(v) => Ok(v.to_string()),
        None => Err("Invalid mcv")
    }
}

fn get_loader(loader: Option<&String>) -> Result<Loader, &'static str> {
    match loader {
        Some(v) => { match v.as_str() {
            "fabric" => Ok(Loader::Fabric),
            "neoforge" => Ok(Loader::Neoforge),
            "forge" => Ok(Loader::Forge),
            _ => Err("Invalid loader")
        }},
        None => Err("Invalid loader")
    }
}

fn get_id_mode(id: Option<&String>) -> Result<AppMode, &'static str> {
    match id {
        Some(v) => Ok(AppMode::SingleId(v.to_string())),
        None => Err("Invalid ID")
    }
}

fn get_file_mode(file: Option<&String>) -> Result<AppMode, &'static str> {
    match file {
        Some(v) => Ok(AppMode::IdFromFile(v.to_string())),
        None => Err("Invalid filename")
    }
}