use std::{env, process};
use std::error::Error;

use mcmodgetter::{
    arguments, clear_mods, create_client, create_out_dir, get_out_dir, help, operation_download, operation_verify, read_mods
};
use mcmodgetter::arguments::{RawConfig, AppMode};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let conf = build_args(&args)
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            help();
            process::exit(1);
        })
    ;
    if let Err(e) = run(conf).await {
        eprintln!("{e}");
        process::exit(1);
    }
}

fn build_args(args: &Vec<String>) -> Result<RawConfig, Box<dyn Error>>
{
    Ok(arguments::ConfigBuilder::new_from_args(args)?
        .build()?
    )
}

async fn run(raw_conf: RawConfig) -> Result<(), Box<dyn Error>> {
    // println!("Starting...");
    let client = create_client()?;
    let out_dir = get_out_dir(raw_conf.options().out_dir())?;
    match raw_conf.mode() {
        AppMode::Download(settings) => {
            create_out_dir(&out_dir)?;
            let conf = arguments::Config::build(&settings, raw_conf.options());
            operation_download(&conf, &client, out_dir).await?
        },
        AppMode::CheckMods(settings) => {
            create_out_dir(&out_dir)?;
            let conf = arguments::Config::build(&settings, raw_conf.options());
            operation_verify(&conf, &client, out_dir).await?;
        }
        AppMode::ReadMods(settings) => {
            let conf = arguments::Config::build(&settings, raw_conf.options());
            read_mods(&conf, &client).await?;
        }
        AppMode::ClearMods => {
            clear_mods(&out_dir).await?;
        },
        AppMode::Help => {
            help();
        }
    };
    Ok(())
}