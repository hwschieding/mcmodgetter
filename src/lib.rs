use futures::future;

#[cfg(test)]
mod tests;
pub mod modrinth;
pub mod arguments;

static APP_USER_AGENT: &str = concat!(
    "hwschieding/",
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/hwschieding/mcmodgetter)"
);

pub fn create_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()
}

pub async  fn modrinth_download_from_id_list(
    conf: &arguments::Config,
    client: &reqwest::Client,
    ids: &Vec<String>,
    out_dir: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>>
{
    let query: modrinth::VersionQuery = modrinth::VersionQuery::build_query(
        conf.mcvs(), 
        &conf.loader_as_string()
    );
    let mut version_results  = Vec::new();
    for id in ids {
        println!("Getting ID '{id}'...");
        version_results.push(modrinth::get_top_version(&client, id, &query))
    }
    let versions = modrinth::collect_versions(
        future::join_all(version_results).await
    );
    let mut download_tasks = Vec::new();
    for v in &versions {
        if let Some(primary_idx) = modrinth::search_for_primary_file(&v.files()) {
            download_tasks.push(
                modrinth::download_file(client,
                    &v.files()[primary_idx],
                    out_dir.unwrap_or("mods/")
                )
            )
        }
    }
    let downloads = future::join_all(download_tasks).await;
    let download_errors: Vec<Box<dyn std::error::Error>> = downloads.into_iter()
        .filter_map(Result::err)
        .collect();
    for err in download_errors {
        println!("Download error: {err}")
    }
    Ok(())
}