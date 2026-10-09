use std::path::{PathBuf};

macro_rules! missing_arg {
    ($name:literal) => {
        concat!("Missing ", $name, " argument")
    };
}

static PLATFORM_MISSING: &'static str = missing_arg!("platform");
static VERSION_MISSING: &'static str = missing_arg!("version");
static FILE_MISSING: &'static str = missing_arg!("filename");
static ID_MISSING: &'static str = missing_arg!("id");
static OUTPUT_MISSING: &'static str = missing_arg!("output file");

static PLATFORM_FABRIC: &'static str = "fabric";
static PLATFORM_FORGE: &'static str = "forge";
static PLATFORM_NEOFORGE: &'static str = "neoforge";
static PLATFORM_PAPER: &'static str = "paper";
static PLATFORM_VELOCITY: &'static str = "velocity";

#[derive(Debug)]
pub enum AppMode {
    Download(DownloadSettings),
    CheckMods(DownloadSettings),
    ClearMods,
    ReadMods(ReadModsSettings),
    Help
}

#[derive(Debug)]
pub struct DownloadSettings
{
    supplier: IdSupplier,
    platform: UserPlatform,
    version_search: String,
}

impl DownloadSettings
{
    pub fn supplier(&self) -> &IdSupplier
    {
        &self.supplier
    }
    pub fn platform(&self) -> &UserPlatform
    {
        &self.platform
    }
    pub fn version_search(&self) -> &String
    {
        &self.version_search
    }
}

#[derive(Debug)]
pub struct ReadModsSettings
{
    supplier: IdSupplier
}

impl ReadModsSettings
{
    pub fn supplier(&self) -> &IdSupplier
    {
        &self.supplier
    }
}
pub trait Platform
{
    fn from_str(plat: &str) -> Option<Self> where Self: Sized;
    fn as_str(&self) -> &'static str;
    fn as_string(&self) -> String
    {
        String::from(self.as_str())
    }
    fn get_default() -> Self where Self: Sized;
}

#[derive(Debug)]
pub enum UserPlatform {
    Modrinth(ModLoader),
    Hangar(PluginPlatform),
}

impl UserPlatform {
    pub fn build_from_platform(plat: &str) -> Option<Self>
    {
        if let Some(l) = ModLoader::from_str(plat)
        {
            return Some(Self::Modrinth(l));
        };

        if let Some(p) = PluginPlatform::from_str(plat)
        {
            return Some(Self::Hangar(p));
        };

        None
    }
    pub fn get_default() -> Self
    {
        Self::Modrinth(ModLoader::get_default())
    }
    pub fn try_get_modrinth(&self) -> Option<&ModLoader>
    {
        match self
        {
            UserPlatform::Modrinth(m) => Some(m),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum ModLoader {
    Fabric,
    Neoforge,
    Forge,
}

impl Platform for ModLoader {
    fn from_str(loader: &str) -> Option<Self>
    {
        if loader == PLATFORM_FABRIC { Some(Self::Fabric) }
        else if loader == PLATFORM_FORGE { Some(Self::Forge) }
        else if loader == PLATFORM_NEOFORGE { Some(Self::Neoforge) }
        else { None }
    }

    fn as_str(&self) -> &'static str
    {
        match self
        {
            Self::Fabric => PLATFORM_FABRIC,
            Self::Neoforge => PLATFORM_NEOFORGE,
            Self::Forge => PLATFORM_FORGE,
        }
    }

    fn get_default() -> Self where Self: Sized {
        Self::Fabric
    }
}

#[derive(Debug)]
pub enum PluginPlatform {
    Paper,
    Velocity,
}

impl Platform for PluginPlatform
{
    fn from_str(plat: &str) -> Option<Self>
    {
        if plat == PLATFORM_PAPER { Some(Self::Paper) }
        else if plat == PLATFORM_VELOCITY { Some(Self::Velocity) }
        else { None }
    }

    fn as_str(&self) -> &'static str {
        match self
        {
            Self::Paper => PLATFORM_PAPER,
            Self::Velocity => PLATFORM_VELOCITY,
        }
    }

    fn get_default() -> Self where Self: Sized {
        Self::Paper
    }
}

pub struct OptionsBuilder
{
    out_dir: Option<PathBuf>,
    skip_deps: bool,
}

impl OptionsBuilder
{
    pub fn new() -> Self {
        let out_dir = None;
        let skip_deps = false;
        Self {out_dir, skip_deps}
    }
    pub fn set_skip_deps(&mut self, new: bool) -> () {
        self.skip_deps = new;
    }
    pub fn set_out_dir(&mut self, new: PathBuf) -> ()
    {
        self.out_dir = Some(new);
    }
    pub fn build(self) -> Options
    {
        Options
        {
            out_dir: self.out_dir,
            skip_deps: self.skip_deps
        }
    }
}

#[derive(Debug)]
pub enum IdSupplier 
{
    File(PathBuf),
    Id(String),
}

impl IdSupplier
{
    fn from_id_mode(id_mode: IdModeArg) -> Self
    {
        match id_mode
        {
            IdModeArg::File(f) => Self::File(PathBuf::from(f)),
            IdModeArg::Id(i) => Self::Id(i),
        }
    }

    fn _try_get_file(&self) -> Option<&PathBuf>
    {
        match self
        {
            IdSupplier::File(f) => Some(f),
            _ => None,
        }
    }
    fn _try_get_id(&self) -> Option<&String>
    {
        match self
        {
            IdSupplier::Id(i) => Some(i),
            _ => None,
        }
    }
}

pub struct Options {
    out_dir: Option<PathBuf>,
    skip_deps: bool,
}

impl Options {
    pub fn new() -> Self {
        let out_dir = None;
        let skip_deps = false;
        Options {out_dir, skip_deps}
    }
    pub fn out_dir(&self) -> Option<&PathBuf> {
        match &self.out_dir
        {
            Some(o) => Some(o),
            None => None
        }
    }
    pub fn skip_deps(&self) -> bool {
        self.skip_deps
    }
}

pub struct Config<'a, T>
{
    settings: &'a T,
    opts: &'a Options,
}

impl<'a, T> Config<'a, T>
{
    pub fn build(settings: &'a T, opts: &'a Options) -> Self
    {
        Self { settings, opts }
    }
    pub fn settings(&self) -> &'a T
    {
        self.settings
    }
    pub fn opts(&self) -> &'a Options
    {
        self.opts
    }
}

pub struct RawConfig {
    mode: AppMode,
    ops: Options,
}

impl RawConfig {
    pub fn mode(&self) -> &AppMode {
        &self.mode
    }
    pub fn options(&self) -> &Options {
        &self.ops
    }
}

enum ModeArg
{
    Download,
    ClearMods,
    ReadMods,
    CheckMods,
    Help,
}

enum IdModeArg
{
    File(String),
    Id(String),
}

pub struct ConfigBuilder
{
    mode: ModeArg,
    platform: Option<String>,
    platform_version: Option<String>,
    id_mode: Option<IdModeArg>,
    output: Option<String>,
    skip_deps: bool
}

impl ConfigBuilder
{
    pub fn new_from_args(args: &Vec<String>) -> Result<Self, &'static str>
    {
        let mut mode: Option<ModeArg> = None;
        let mut platform: Option<String> = None;
        let mut platform_version: Option<String> = None;
        let mut id_mode: Option<IdModeArg> = None;
        let mut output: Option<String> = None;
        let mut skip_deps: bool = false;

        let mut arg_iter = args.iter();
        arg_iter.next();
        while let Some(arg) = arg_iter.next()
        {
            match arg.as_str()
            {
                "download" => mode = Some(ModeArg::Download),
                "clearmods" => mode = Some(ModeArg::ClearMods),
                "checkmods" => mode = Some(ModeArg::CheckMods),
                "readmods" => mode = Some(ModeArg::ReadMods),
                "-l" => platform = Some(
                    try_get_arg(arg_iter.next(), PLATFORM_MISSING)?
                ),
                "-loader" => platform = Some(
                    try_get_arg(arg_iter.next(), PLATFORM_MISSING)?
                ),
                "-mcv" => platform_version = Some(
                    try_get_arg(arg_iter.next(), VERSION_MISSING)?
                ),
                "-file" => id_mode = Some(IdModeArg::File(
                    try_get_arg(arg_iter.next(), FILE_MISSING)?
                )),
                "-id" => id_mode = Some(IdModeArg::Id(
                    try_get_arg(arg_iter.next(), ID_MISSING)?
                )),
                "-o" => output = Some(
                    try_get_arg(arg_iter.next(), OUTPUT_MISSING)?
                ),
                "--skipdeps" => skip_deps = true,
                "-help" => mode = Some(ModeArg::Help),
                "--help" => mode = Some(ModeArg::Help),
                "-h" => mode = Some(ModeArg::Help),
                _ => println!("Unknown argument: {}", arg)
            }
        }
        
        let mode = mode.ok_or("No command specified")?;

        Ok(Self {mode, platform, platform_version, id_mode, output, skip_deps})
    }

    fn build_clearmods(self) -> Result<RawConfig, &'static str>
    {
        let mut opts = OptionsBuilder::new();
        if let Some(o) = self.output { opts.set_out_dir(PathBuf::from(o)); }
        let opts = opts.build();

        Ok(RawConfig { mode: AppMode::ClearMods, ops: opts })
    }

    fn build_readmods(self) -> Result<RawConfig, &'static str>
    {
        let supplier = IdSupplier::from_id_mode(
            self.id_mode.ok_or(FILE_MISSING)?
        );
        let settings = ReadModsSettings
        {
            supplier
        };

        let opts = OptionsBuilder::new().build();

        Ok(RawConfig { mode: AppMode::ReadMods(settings), ops: opts })
    }

    fn build_checkmods(self) -> Result<RawConfig, &'static str>
    {
        self.build_download()
    }

    fn build_download(self) -> Result<RawConfig, &'static str>
    {
        let supplier = IdSupplier::from_id_mode(
            self.id_mode.ok_or(ID_MISSING)?
        );
        let versions = self.platform_version.ok_or(
            VERSION_MISSING
        )?;
        let platform: UserPlatform = match self.platform
        {
            Some(p) => UserPlatform::build_from_platform(&p)
                .ok_or("Specified platform is not valid")?,
            None => UserPlatform::get_default()
        };

        let settings: DownloadSettings = DownloadSettings {
            supplier, platform, version_search: versions
        };

        let mut opts: OptionsBuilder = OptionsBuilder::new();
        if let Some(o) = self.output { opts.set_out_dir(PathBuf::from(o)); }
        opts.set_skip_deps(self.skip_deps);
        let opts = opts.build();

        Ok(RawConfig { mode: AppMode::Download(settings), ops: opts })
    }

    pub fn build<'a>(self) -> Result<RawConfig, &'static str>
    {
        match &self.mode
        {
            ModeArg::Download => self.build_download(),
            ModeArg::CheckMods => self.build_checkmods(),
            ModeArg::ClearMods => self.build_clearmods(),
            ModeArg::ReadMods => self.build_readmods(),
            _ => Err("")
        }
    }
}

fn try_get_arg(
    arg_opt: Option<&String>,
    msg: &'static str
) -> Result<String, &'static str>
{
    Ok(arg_opt.ok_or(msg)?.to_string())
}