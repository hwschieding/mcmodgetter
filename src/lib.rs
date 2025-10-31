use futures::future;
use serde::Deserialize;

static APP_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
);
static MODRINTH_URL: &str = "https://api.modrinth.com";

#[derive(Deserialize)]
pub struct Project {
    id: String,
    title: String,
    description: String,
}

impl Project {
    pub fn get_id(&self) -> &String {
        &self.id
    }
    pub fn get_title(&self) -> &String {
        &self.title
    }
    pub fn get_desc(&self) -> &String {
        &self.description
    }
}

pub fn create_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()
}

pub async fn get_project(client: &reqwest::Client, id: &str) -> Result<Project, reqwest::Error> {
    let url = format!("{}{}{}", MODRINTH_URL, "/v2/project/", id);
    let response = client.get(url).send().await?;
    response.json::<Project>().await
}

pub async fn get_projects_from_list(
    client: &reqwest::Client,
    ids: &Vec<String>
) -> Vec<Result<Project, reqwest::Error>>
{
    let mut responses = Vec::new();
    for id in ids {
        responses.push(get_project(client, id));
    }
    future::join_all(responses).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn get_single_project() {
        let client = create_client().expect("Client should be created");
        let project = get_project(&client, "AANobbMI").await.expect("should exist");
        assert_eq!(project.title, "Sodium");
        assert_eq!(project.id, "AANobbMI")
    }

    #[tokio::test]
    async fn get_list_of_projects() {
        let client = create_client().expect("Client should be created");
        let ids_vec = vec![String::from("P7dR8mSH"), String::from("AANobbMI"), String::from("9s6osm5g")];
        let project_vec = get_projects_from_list(&client, &ids_vec).await;
        assert_eq!(project_vec[0].as_ref().expect("Should exist").title, "Fabric API");
        assert_eq!(project_vec[1].as_ref().expect("Should exist").title, "Sodium")
    }
}