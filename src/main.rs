use std::{env, process};
use std::error::Error;

use mcmodgetter::{create_client, modrinth, modrinth_download_from_id_list};
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
async fn run(conf: Config) -> Result<(), Box<dyn Error>> {
    if let AppMode::IdFromFile(filename) = conf.mode() {
        println!("Starting...");
        let client = create_client()?;
        let ids = vec![String::from("P7dR8mSH"), String::from("AANobbMI"), String::from("9s6osm5g")];
        modrinth_download_from_id_list(&conf, &client, &ids, None).await?;
    }
    Ok(())
}