use regex::Regex;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use tokio::sync::RwLock;

use crate::{
    constants::{SOUNDCLOUD_API_URL, SOUNDCLOUD_URL},
    models::{client::Client, config::RetryConfig, error::Error},
};

impl Client {
    pub async fn new() -> Result<Self, Error> {
        Self::with_retry_config(RetryConfig::default()).await
    }

    pub async fn with_retry_config(retry_config: RetryConfig) -> Result<Self, Error> {
        let http_client = reqwest::Client::new();
        let client_id = Self::get_client_id(&http_client).await?;
        Ok(Self {
            client_id: RwLock::new(client_id),
            retry_config,
            http_client,
        })
    }

    pub async fn refresh_client_id(&self) -> Result<(), Error> {
        let new_client_id = Self::get_client_id(&self.http_client).await?;
        *self.client_id.write().await = new_client_id;
        Ok(())
    }

    pub async fn get_client_id_value(&self) -> String {
        self.client_id.read().await.clone()
    }

    pub async fn get_json<R: DeserializeOwned, Q: Serialize>(
        &self,
        base_url: &str,
        path: Option<&str>,
        query: Option<&Q>,
        client_id: &str,
    ) -> Result<(R, u16), Error> {
        let url = match path {
            Some(path) => format!(
                "{}/{}",
                base_url.trim_end_matches('/'),
                path.trim_start_matches('/')
            ),
            None => base_url.to_string(),
        };

        let mut request = self.http_client.get(&url);

        if let Some(q) = query {
            request = request.query(q);
        }
        request = request.query(&[("client_id", client_id)]);

        let response = request.send().await.map_err(Error::from)?;

        let status = response.status().as_u16();

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(Error::new(format!("HTTP {}: {}", status, text)));
        }

        let body = response.json::<R>().await.map_err(Error::from)?;

        Ok((body, status))
    }

    pub async fn get<Q: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        query: Option<&Q>,
    ) -> Result<R, Error> {
        let mut retries = 0;
        let max_retries = self.retry_config.max_retries;

        loop {
            let client_id = self.client_id.read().await.clone();
            let result = self
                .get_json(SOUNDCLOUD_API_URL, Some(path), query, &client_id)
                .await;

            match result {
                Ok((body, _status)) => {
                    return Ok(body);
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    // Check if we got a 401 and should retry
                    if error_msg.contains("401")
                        && self.retry_config.retry_on_401
                        && retries < max_retries
                    {
                        retries += 1;
                        self.refresh_client_id().await?;
                        continue;
                    }
                    // For non-401 errors or if we've exhausted retries, return the error
                    return Err(e);
                }
            }
        }
    }

    async fn get_script_urls(client: &reqwest::Client) -> Result<Vec<String>, Error> {
        let response = client.get(SOUNDCLOUD_URL).send().await?;
        let text = response.text().await?;
        let re = Regex::new(r#"https?://[^\s"]+\.js"#).expect("Failed to find script URLs");
        let urls: Vec<String> = re
            .find_iter(&text)
            .map(|mat| mat.as_str().to_string())
            .filter(|url| url.contains("sndcdn.com"))
            .collect();
        Ok(urls)
    }

    async fn find_client_id(client: reqwest::Client, url: String) -> Result<Option<String>, Error> {
        let response = client.get(url).send().await?;
        let text = response.text().await?;
        let re = Regex::new(r#"client_id[:=]"?(\w{32})"#).expect("Failed to find client ID");
        if let Some(cap) = re.captures_iter(&text).next() {
            return Ok(Some(cap[1].to_string()));
        }
        Ok(None)
    }

    async fn get_client_id(client: &reqwest::Client) -> Result<String, Error> {
        let script_urls = Self::get_script_urls(client).await?;
        let mut set = tokio::task::JoinSet::new();
        for url in script_urls {
            set.spawn(Self::find_client_id(client.clone(), url));
        }
        while let Some(res) = set.join_next().await {
            if let Ok(Ok(Some(client_id))) = res {
                set.abort_all();
                return Ok(client_id);
            }
        }
        Err(Error::new("Client ID not found"))
    }

    /// Health check endpoint that checks if the current client_id is valid
    /// Returns true if the API responds successfully (2xx), false otherwise
    pub async fn health_check(&self) -> bool {
        #[derive(serde::Serialize)]
        struct ResolveQuery {
            url: String,
        }

        self.get::<ResolveQuery, Value>(
            "resolve",
            Some(&ResolveQuery {
                url: "https://soundcloud.com/soundcloud".to_string(),
            }),
        )
        .await
        .is_ok()
    }
}
