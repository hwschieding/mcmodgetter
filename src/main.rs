use std::{env, process};
use std::error::Error;

use mcmodgetter::{create_client,
    get_out_dir,
    modrinth_download_from_id,
    modrinth_download_from_id_list,
    vec_from_lines};
use mcmodgetter::arguments::{Config, AppMode};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let conf = Config::build_from_args(&args).unwrap_or_else(|e| {
        eprintln!("{e}");
        process::exit(1);
    });
    if let Err(e) = run(conf).await {
        eprintln!("{e}");
        process::exit(1);
    }
}
async fn run<'a>(conf: Config<'a>) -> Result<(), Box<dyn Error>> {
    println!("Starting...");
    let client = create_client()?;
    let out_dir = get_out_dir(&conf.out_dir())?;
    if let AppMode::IdFromFile(filename) = conf.mode() {
        let ids = vec_from_lines(filename)?;
        modrinth_download_from_id_list(&conf, &client, &ids, &out_dir).await?;
    }
    if let AppMode::SingleId(id) = conf.mode() {
        modrinth_download_from_id(&conf, &client, id, &out_dir).await?;
    }
    Ok(())
}