use std::{env, process};
use std::error::Error;

use mcmodgetter::{create_client, modrinth_download_from_id_list, vec_from_lines};
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
    if let AppMode::IdFromFile(filename) = conf.mode() {
        println!("Starting...");
        let client = create_client()?;
        let ids = vec_from_lines(filename)?;
        modrinth_download_from_id_list(&conf, &client, &ids).await?;
    }
    Ok(())
}