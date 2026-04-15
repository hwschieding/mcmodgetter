use std::{env, process};
use std::error::Error;

use mcmodgetter::{
    clear_mods, create_client, create_out_dir, get_out_dir, help, id_from_file, single_id
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
        AppMode::IdFromFile(filename) => {
            create_out_dir(&out_dir)?;
            id_from_file(
                &conf,
                &client,
                &filename, 
                &out_dir
            ).await?;
        },
        AppMode::SingleId(id) => {
            create_out_dir(&out_dir)?;
            single_id(
                &conf,
                &client,
                &id,
                &out_dir
            ).await?;
        },
        AppMode::ClearMods => {
            clear_mods(&out_dir)?;
            // if out_dir.try_exists()? && out_dir.is_dir() {
                
            // } else {
            //     let out_dir_name = out_dir.display();
            //     println!(
            //         "'{out_dir_name}' does not exist. clearmods can only be performed on existing directories."
            //     );
            // }
        },
        AppMode::Help => {
            help();
        }
    };
    Ok(())
}