use std::path::{Path};

pub enum AppMode<'a> {
    SingleId(String),
    IdFromFile(&'a Path),
    ClearMods,
    Help
}

pub enum Loader {
    Fabric,
    Neoforge,
    Forge
}

pub struct Options {
    verify: bool,
    skip_deps: bool,
}

impl Options {
    pub fn new() -> Self {
        let verify = false;
        let skip_deps = false;
        Options {verify, skip_deps}
    }
    pub fn set_verify(&mut self, new:bool) -> () {
        self.verify = new;
    }
    pub fn set_skip_deps(&mut self, new:bool) -> () {
        self.skip_deps = new;
    }
    pub fn get_verify(&self) -> bool {
        self.verify
    }
    pub fn get_skip_deps(&self) -> bool {
        self.skip_deps
    }
}

pub struct Config<'a> {
    mode: AppMode<'a>,
    ops: Options,
    mcvs: String,
    loader: Loader,
    out_dir: Option<&'a Path>,
}

impl<'a> Config<'a> {
    pub fn build_from_args(args: &'a Vec<String>) -> Result<Config<'a>, &'static str> {
        let mut mode: Result<AppMode, &'static str> = Err("No ID specified");
        let mut ops: Options = Options::new();
        let mut mcvs: Result<String, &'static str> = Err("No mc version specified");
        let mut loader: Loader = Loader::Fabric;
        let mut out_dir: Option<&Path> = None;
        let mut args_iter = args.iter();
        args_iter.next();
        while let Some(arg) = args_iter.next(){
            match arg.as_str() {
                "-id" => mode = Ok(get_id_mode(args_iter.next())?),
                "--readfile" => mode = Ok(get_file_mode(args_iter.next())?),
                "-mcv" => mcvs = Ok(get_mcvs(args_iter.next())?),
                "-l" => loader = get_loader(args_iter.next())?,
                "-o" => out_dir = Some(get_out_dir(args_iter.next())?),
                "clearmods" => mode = Ok(AppMode::ClearMods),
                "checkmods" => { ops.set_verify(true); },
                "--skipdeps" => { ops.set_skip_deps(true); }
                "-h" => mode = Ok(AppMode::Help),
                "--help" => mode = Ok(AppMode::Help),
                "-help" => mode = Ok(AppMode::Help),
                _ => println!("arg '{arg}' not recognized")
            }
        };
        let mode = mode?;
        let mcvs = match mode {
            AppMode::ClearMods => String::new(),
            AppMode::Help => String::new(),
            _ => mcvs?
        };
        Ok(Config { mode, ops, mcvs, loader, out_dir })
    }
    pub fn mode(&self) -> &AppMode<'a> {
        &self.mode
    }
    pub fn options(&self) -> &Options {
        &self.ops
    }
    pub fn mcvs(&self) -> &String {
        &self.mcvs
    }
    pub fn loader(&self) -> &Loader {
        &self.loader
    }
    pub fn out_dir(&self) -> &Option<&Path> {
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

fn get_out_dir(file: Option<&String>) -> Result<&Path, &'static str> {
    match file {
        Some(f) => Ok(Path::new(f)),
        None => Err("Invalid output directory")
    }
}