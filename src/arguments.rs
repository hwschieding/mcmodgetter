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

const MODRINTH_PLATFORMS: &[&str] = &[
    PLATFORM_FABRIC,
    PLATFORM_FORGE,
    PLATFORM_NEOFORGE,
    PLATFORM_PAPER
];

const HANGAR_PLATFORMS: &[&str] = &[
    PLATFORM_PAPER,
    PLATFORM_VELOCITY,
];

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

#[derive(Debug)]
pub enum Api
{
    Modrinth,
    Hangar
}
impl Api
{
    fn default() -> Self
    {
        Self::Modrinth
    }
    fn get_valid_platform(&self, plat: &str) -> Option<Platform>
    {
        if self.is_valid_platform(plat)
        {
            Platform::from_str(plat)
        }
        else { None }
    }
    fn is_valid_platform(&self, plat: &str) -> bool
    {
        match self
        {
            Self::Modrinth => MODRINTH_PLATFORMS.contains(&plat),
            Self::Hangar => HANGAR_PLATFORMS.contains(&plat),
        }
    }
}

#[derive(Debug)]
pub enum Platform
{
    Fabric,
    Forge,
    Neoforge,
    Paper,
    Velocity,
}
impl Platform
{
    fn default() -> Self
    {
        Self::Fabric
    }
    fn from_str(s: &str) -> Option<Self>
    {
        if      s == PLATFORM_FABRIC   { Some(Self::Fabric) }
        else if s == PLATFORM_FORGE    { Some(Self::Forge) }
        else if s == PLATFORM_NEOFORGE { Some(Self::Neoforge) }
        else if s == PLATFORM_PAPER    { Some(Self::Paper) }
        else if s == PLATFORM_VELOCITY { Some(Self::Velocity) }
        else { None }
    }
    pub fn to_str(&self) -> &'static str
    {
        match self
        {
            Self::Fabric => PLATFORM_FABRIC,
            Self::Forge => PLATFORM_FORGE,
            Self::Paper => PLATFORM_PAPER,
            Self::Neoforge => PLATFORM_NEOFORGE,
            Self::Velocity => PLATFORM_VELOCITY,
        }
    }
    pub fn to_string(&self) -> String
    {
        String::from(self.to_str())
    }
}


#[derive(Debug)]
pub struct UserPlatform {
    api: Api,
    platforms: Vec<Platform>,
}
impl UserPlatform {
    fn build(api: Api, platform_arg: &str) -> Self
    {
        let mut platforms = Vec::<Platform>::new();
        for plat in platform_arg.split(',')
        {
            match api.get_valid_platform(plat)
            {
                Some(p) => platforms.push(p),
                None => println!("Unrecognized platform '{}'", plat),
            }
        }
        UserPlatform { api, platforms }
    }
    pub fn get(&self) -> &Vec<Platform>
    {
        &self.platforms
    }
    pub fn api(&self) -> &Api
    {
        &self.api
    }
}

pub struct OptionsBuilder
{
    out_dir: Option<PathBuf>,
    skip_deps: bool,
    release_only: bool,
}

impl OptionsBuilder
{
    pub fn new() -> Self {
        let out_dir = None;
        let skip_deps = false;
        let release_only = false;
        Self {out_dir, skip_deps, release_only}
    }
    pub fn set_skip_deps(&mut self, new: bool) -> ()
    {
        self.skip_deps = new;
    }
    pub fn set_out_dir(&mut self, new: PathBuf) -> ()
    {
        self.out_dir = Some(new);
    }
    pub fn set_release_only(&mut self, new: bool) -> ()
    {
        self.release_only = new;
    }
    pub fn build(self) -> Options
    {
        Options
        {
            out_dir: self.out_dir,
            skip_deps: self.skip_deps,
            release_only: self.release_only,
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
    release_only: bool,
}

impl Options {
    pub fn new() -> Self {
        let out_dir = None;
        let skip_deps = false;
        let release_only = false;
        Options {out_dir, skip_deps, release_only }
    }
    pub fn out_dir(&self) -> Option<&PathBuf> {
        match &self.out_dir
        {
            Some(o) => Some(o),
            None => None
        }
    }
    pub fn skip_deps(&self) -> bool
    {
        self.skip_deps
    }
    pub fn release_only(&self) -> bool
    {
        self.release_only
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
    platforms: Option<String>,
    platform_version: Option<String>,
    id_mode: Option<IdModeArg>,
    api: Option<Api>,
    output: Option<String>,
    skip_deps: bool,
    release_only: bool,
}

impl ConfigBuilder
{
    pub fn new_from_args(args: &Vec<String>) -> Result<Self, &'static str>
    {
        let mut mode: Option<ModeArg> = None;
        let mut platforms: Option<String> = None;
        let mut platform_version: Option<String> = None;
        let mut id_mode: Option<IdModeArg> = None;
        let mut api: Option<Api> = None;
        let mut output: Option<String> = None;
        let mut skip_deps: bool = false;
        let mut release_only: bool = false;

        let mut arg_iter = args.iter();
        arg_iter.next();
        while let Some(arg) = arg_iter.next()
        {
            match arg.to_lowercase().as_str()
            {
                "download" => mode = Some(ModeArg::Download),
                "clearmods" => mode = Some(ModeArg::ClearMods),
                "checkmods" => mode = Some(ModeArg::CheckMods),
                "readmods" => mode = Some(ModeArg::ReadMods),
                "-modrinth" => {
                    api = Some(Api::Modrinth);
                    platforms = Some(
                        try_get_arg(arg_iter.next(), PLATFORM_MISSING)?
                    )
                },
                "-hangar" => {
                    api = Some(Api::Hangar);
                    platforms = Some(
                        try_get_arg(arg_iter.next(), PLATFORM_MISSING)?
                    )
                }
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
                "--releaseonly" => release_only = true,
                "-help" => mode = Some(ModeArg::Help),
                "--help" => mode = Some(ModeArg::Help),
                "-h" => mode = Some(ModeArg::Help),
                _ => println!("Unknown argument: {}", arg)
            }
        }
        
        let mode = mode.ok_or("No command specified")?;

        Ok(Self {mode, platforms, platform_version, id_mode, api, output, skip_deps, release_only})
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
        let api = match self.api
        {
            Some(a) => a,
            None => Api::default(),
        };
        let platforms: UserPlatform = UserPlatform::build(
            api,
            &self.platforms.unwrap_or(Platform::default().to_string())
        );

        let settings: DownloadSettings = DownloadSettings {
            supplier, platform: platforms, version_search: versions
        };

        let mut opts: OptionsBuilder = OptionsBuilder::new();
        if let Some(o) = self.output { opts.set_out_dir(PathBuf::from(o)); }
        opts.set_skip_deps(self.skip_deps);
        opts.set_release_only(self.release_only);
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