use std::{env, process};
use std::error::Error;
use std::io;

use mcmodgetter::{clear_dir, create_client, get_out_dir, modrinth_download_from_id, modrinth_download_from_id_list, modrinth_verify_id, modrinth_verify_ids_from_list, vec_from_lines
};
use mcmodgetter::arguments::{Config, AppMode};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let conf = Config::build_from_args(&args)
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            process::exit(1);
        }
    );
    if let Err(e) = run(conf).await {
        eprintln!("{e}");
        process::exit(1);
    }
}

async fn run<'a>(conf: Config<'a>) -> Result<(), Box<dyn Error>> {
    println!("Starting...");
    let client = create_client()?;
    let out_dir = get_out_dir(&conf.out_dir())?;
    match conf.mode() {
        AppMode::IdFromFile(filename) => {
            let ids = vec_from_lines(filename)?;
            if conf.verify() {
                modrinth_verify_ids_from_list(
                    &conf,
                    &client,
                    &ids,
                    &out_dir
                ).await;
            } else {
                modrinth_download_from_id_list(
                    &conf,
                    &client,
                    &ids,
                    &out_dir
                ).await?;
            };
        },
        AppMode::SingleId(id) => {
            if conf.verify() {
                modrinth_verify_id(
                    &conf,
                    &client,
                    &id,
                    &out_dir
                ).await;
            } else {
                modrinth_download_from_id(&conf,
                    &client,
                    id,
                    &out_dir
                ).await?;
            };
        },
        AppMode::ClearMods => {
            println!("Delete everything in directory {}? (y/n)",
                &out_dir.display()
            );
            let mut user_ans = String::new();
            io::stdin().read_line(&mut user_ans)?;
            if user_ans.trim().to_lowercase() == "y" {
                clear_dir(&out_dir)?;
            }
        }
    };
    Ok(())
}