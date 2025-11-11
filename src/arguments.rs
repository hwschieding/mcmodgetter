use std::path::{Path, PathBuf};

pub enum AppMode<'a> {
    SingleId(String),
    IdFromFile(&'a Path),
}

pub enum Loader {
    Fabric,
    Neoforge,
    Forge
}

pub struct Config<'a> {
    mode: AppMode<'a>,
    mcvs: String,
    loader: Loader,
    out_dir: Option<PathBuf>,
}

impl<'a> Config<'a> {
    pub fn build_from_args(args: &'a Vec<String>) -> Result<Config<'a>, &'static str> {
        let mut mode: Result<AppMode, &'static str> = Err("No ID specified");
        let mut mcvs: Result<String, &'static str> = Err("No mc version specified");
        let mut loader: Loader = Loader::Fabric;
        let mut out_dir: Option<PathBuf> = None;
        let mut args_iter = args.iter();
        args_iter.next();
        while let Some(arg) = args_iter.next(){
            match arg.as_str() {
                "-id" => mode = Ok(get_id_mode(args_iter.next())?),
                "--readfile" => mode = Ok(get_file_mode(args_iter.next())?),
                "-mcv" => mcvs = Ok(get_mcvs(args_iter.next())?),
                "-l" => loader = get_loader(args_iter.next())?,
                "-o" => out_dir = Some(get_out_dir(args_iter.next())?),
                _ => println!("arg '{arg}' not recognized")
            }
        };
        let mode = mode?;
        let mcvs = mcvs?;
        Ok(Config { mode, mcvs, loader, out_dir })
    }
    pub fn mode(&self) -> &AppMode<'a> {
        &self.mode
    }
    pub fn mcvs(&self) -> &String {
        &self.mcvs
    }
    pub fn loader(&self) -> &Loader {
        &self.loader
    }
    pub fn out_dir(&self) -> &Option<PathBuf> {
        &self.out_dir
    }
    pub fn loader_as_str(&self) -> &str {
        match self.loader {
            Loader::Fabric => "fabric",
            Loader::Neoforge => "neoforge",
            Loader::Forge => "forge"
        }
    }
    pub fn loader_as_string(&self) -> String {
        match self.loader {
            Loader::Fabric => String::from("fabric"),
            Loader::Neoforge => String::from("neoforge"),
            Loader::Forge => String::from("forge")
        }
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

fn get_id_mode<'a>(id: Option<&'a String>) -> Result<AppMode<'a>, &'static str> {
    match id {
        Some(v) => Ok(AppMode::SingleId(v.to_string())),
        None => Err("Invalid ID")
    }
}

fn get_file_mode<'a>(file: Option<&'a String>) -> Result<AppMode<'a>, &'static str> {
    match file {
        Some(v) => Ok(AppMode::IdFromFile(&Path::new(v))),
        None => Err("Invalid filename")
    }
}

fn get_out_dir(file: Option<&String>) -> Result<PathBuf, &'static str> {
    match file {
        Some(f) => Ok(PathBuf::from(f)),
        None => Err("Invalid output directory")
    }
}