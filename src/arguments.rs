use std::path::{Path};

pub enum AppMode {
    DownloadId,
    DownloadFile,
    ClearMods,
    ReadMods,
    Help
}

pub enum Loader {
    Fabric,
    Neoforge,
    Forge
}

pub struct Options<'a> {
    file: Option<&'a Path>,
    id: Option<String>,
    verify: bool,
    skip_deps: bool,
}

impl <'a> Options<'a> {
    pub fn new() -> Self {
        let file = None;
        let id = None;
        let verify = false;
        let skip_deps = false;
        Options {file, id, verify, skip_deps}
    }
    pub fn set_file(&mut self, new:Option<&'a Path>) -> () {
        self.file = new;
    }
    pub fn set_id(&mut self, new:Option<String>) -> () {
        self.id = new;
    }
    pub fn has_id(&self) -> bool {
        self.id.is_some()
    }
    pub fn has_file(&self) -> bool {
        self.file.is_some()
    }
    pub fn set_verify(&mut self, new:bool) -> () {
        self.verify = new;
    }
    pub fn set_skip_deps(&mut self, new:bool) -> () {
        self.skip_deps = new;
    }
    pub fn get_file(&self) -> Option<&'a Path> {
        self.file
    }
    pub fn get_id(&self) -> &Option<String> {
        &self.id
    }
    pub fn get_verify(&self) -> bool {
        self.verify
    }
    pub fn get_skip_deps(&self) -> bool {
        self.skip_deps
    }
}

pub struct Config<'a> {
    mode: AppMode,
    ops: Options<'a>,
    mcvs: String,
    loader: Loader,
    out_dir: Option<&'a Path>,
}

impl<'a> Config<'a> {
    pub fn build_from_args(args: &'a Vec<String>) -> Result<Config<'a>, &'static str> {
        let mut is_download = false;
        let mut mode: Result<AppMode, &'static str> = Err("No ID specified");
        let mut ops: Options = Options::new();
        let mut mcvs: Result<String, &'static str> = Err("No mc version specified");
        let mut loader: Loader = Loader::Fabric;
        let mut out_dir: Option<&Path> = None;
        let mut args_iter = args.iter();
        args_iter.next();
        while let Some(arg) = args_iter.next(){
            match arg.as_str() {
                "download" => is_download = true,
                "clearmods" => mode = Ok(AppMode::ClearMods),
                "readmods" => mode = Ok(AppMode::ReadMods),
                "checkmods" => { ops.set_verify(true); },
                "-id" => ops.set_id(get_id(args_iter.next())?),
                "-file" => ops.set_file(get_file(args_iter.next())?),
                "-mcv" => mcvs = Ok(get_mcvs(args_iter.next())?),
                "-l" => loader = get_loader(args_iter.next())?,
                "-o" => out_dir = Some(get_out_dir(args_iter.next())?),
                "--skipdeps" => { ops.set_skip_deps(true); }
                "-h" => mode = Ok(AppMode::Help),
                "--help" => mode = Ok(AppMode::Help),
                "-help" => mode = Ok(AppMode::Help),
                _ => println!("arg '{arg}' not recognized")
            }
        };
        if is_download {
            if ops.has_id() {
                mode = Ok(AppMode::DownloadId)
            } else if ops.has_file() {
                mode = Ok(AppMode::DownloadFile)
            } else {
                return Err("'download' requires more arguments (-id or -file)")
            }
        }
        let mode = mode?;
        if matches!(mode, AppMode::ReadMods) && !ops.has_file() {
            return Err("'readmods' needs a file to be specified (-file)")
        }
        let mcvs = match mode {
            AppMode::ClearMods => String::new(),
            AppMode::ReadMods => String::new(),
            AppMode::Help => String::new(),
            _ => mcvs?
        };
        Ok(Config { mode, ops, mcvs, loader, out_dir })
    }
    pub fn mode(&self) -> &AppMode {
        &self.mode
    }
    pub fn options(&self) -> &Options<'a> {
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

fn get_id<'a>(id: Option<&'a String>) -> Result<Option<String>, &'static str> {
    match id {
        Some(v) => Ok(Some(v.to_string())),
        None => Err("Invalid ID")
    }
}

fn get_file<'a>(file: Option<&'a String>) -> Result<Option<&'a Path>, &'static str> {
    match file {
        Some(v) => Ok(Some(&Path::new(v))),
        None => Err("Invalid filename")
    }
}

fn get_out_dir(file: Option<&String>) -> Result<&Path, &'static str> {
    match file {
        Some(f) => Ok(Path::new(f)),
        None => Err("Invalid output directory")
    }
}