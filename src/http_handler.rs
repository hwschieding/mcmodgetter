use serde::{Deserialize, Serialize, de::DeserializeOwned};

struct Request<'a, T>
{
    client: &'a reqwest::Client,
    query: T
}

impl<'a, T> Request<'a, T>
where
    T: Serialize,
{
    pub fn build(client: &'a reqwest::Client, query: T) -> Request<'a, T>
    {
        Request { client, query }
    }

    pub async fn send<U>(&self, url: &String) -> Result<U, reqwest::Error>
    where
        U: DeserializeOwned
    {
        let serialized_response = self.client.get(url)
            .query(&self.query)
            .send()
            .await?;

        serialized_response.json::<U>().await
    }
}