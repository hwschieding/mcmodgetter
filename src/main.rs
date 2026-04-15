use std::{env, process};
use std::error::Error;

use mcmodgetter::{
    read_mods, clear_mods, create_client, create_out_dir, get_out_dir, help, id_from_file, single_id
};
use mcmodgetter::arguments::{Config, AppMode};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let conf = Config::build_from_args(&args)
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            help();
            process::exit(1);
        }
    );
    if let Err(e) = run(conf).await {
        eprintln!("{e}");
        process::exit(1);
    }
}

async fn run<'a>(conf: Config<'a>) -> Result<(), Box<dyn Error>> {
    // println!("Starting...");
    let client = create_client()?;
    let out_dir = get_out_dir(&conf.out_dir());
    match conf.mode() {
        AppMode::DownloadFile => {
            create_out_dir(&out_dir)?;
            id_from_file(
                &conf,
                &client,
                &out_dir
            ).await?;
        },
        AppMode::DownloadId => {
            create_out_dir(&out_dir)?;
            single_id(
                &conf,
                &client,
                &out_dir
            ).await?;
        },
        AppMode::ReadMods => {
            read_mods(&conf, &client).await?;
        }
        AppMode::ClearMods => {
            clear_mods(&out_dir)?;
        },
        AppMode::Help => {
            help();
        }
    };
    Ok(())
}