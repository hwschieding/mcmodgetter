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
    pub fn mode(&self) -> &AppMode {
        &self.mode
    }
    pub fn mcvs(&self) -> &String {
        &self.mcvs
    }
    pub fn loader(&self) -> &Loader {
        &self.loader
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