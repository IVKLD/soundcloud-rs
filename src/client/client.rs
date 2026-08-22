use std::time::Duration;

use regex::Regex;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use tokio::sync::RwLock;

use crate::{
    constants::{SOUNDCLOUD_API_URL, SOUNDCLOUD_URL},
    models::{
        client::{Client, ProxyUrlProvider},
        config::RetryConfig,
        error::Error,
    },
};

impl Client {
    pub async fn new() -> Result<Self, Error> {
        Self::new_with_options(None, RetryConfig::default(), None).await
    }

    pub async fn new_with_options(
        client_id: Option<String>,
        retry_config: RetryConfig,
        proxy_url: Option<String>,
    ) -> Result<Self, Error> {
        let http_client = Self::build_http_client(proxy_url.as_deref())?;

        let client_id = match client_id {
            Some(id) => id,
            None => Self::get_client_id(&http_client).await?,
        };

        let active_proxy = proxy_url.clone();

        Ok(Self {
            client_id: RwLock::new(client_id),
            retry_config,
            http_client: RwLock::new(http_client),
            proxy_url,
            proxy_provider: None,
            active_proxy: RwLock::new(active_proxy),
        })
    }

    /// Attach a dynamic proxy provider. Each request will check if the provider
    /// returns a different URL than the one the current `http_client` was built
    /// with, and rebuild the client transparently if so.
    pub fn with_proxy_provider(mut self, provider: ProxyUrlProvider) -> Self {
        self.proxy_provider = Some(provider);
        self
    }

    fn build_http_client(proxy_url: Option<&str>) -> Result<reqwest::Client, Error> {
        let mut builder = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30));
        if let Some(proxy) = proxy_url {
            builder = builder.proxy(reqwest::Proxy::all(proxy)?);
        }
        builder.build().map_err(Error::from)
    }

    /// If a `proxy_provider` is set, check whether the proxy URL it returns
    /// differs from the one the current HTTP client was built with. If so,
    /// rebuild the HTTP client so the next request uses the updated proxy.
    pub(crate) async fn ensure_proxy_refreshed(&self) {
        let Some(ref provider) = self.proxy_provider else {
            return;
        };
        let desired = (provider.0)();
        let current = self.active_proxy.read().await.clone();
        if desired == current {
            return;
        }
        match Self::build_http_client(desired.as_deref()) {
            Ok(new_client) => {
                *self.http_client.write().await = new_client;
                *self.active_proxy.write().await = desired;
            }
            Err(e) => {
                tracing::warn!("Failed to rebuild HTTP client with new proxy: {e}");
            }
        }
    }

    pub async fn refresh_client_id(&self) -> Result<(), Error> {
        self.ensure_proxy_refreshed().await;
        let http = self.http_client.read().await;
        let new_client_id = Self::get_client_id(&http).await?;
        *self.client_id.write().await = new_client_id;
        Ok(())
    }

    pub async fn get_client_id_value(&self) -> String {
        self.client_id.read().await.clone()
    }

    /// Returns the currently-active proxy URL (may differ from `proxy_url`
    /// if a `proxy_provider` was attached and has returned a new value).
    pub async fn current_proxy_url(&self) -> Option<String> {
        self.active_proxy.read().await.clone()
    }

    pub async fn get_json<R: DeserializeOwned, Q: Serialize>(
        &self,
        base_url: &str,
        path: Option<&str>,
        query: Option<&Q>,
    ) -> Result<(R, u16), Error> {
        self.ensure_proxy_refreshed().await;

        let url = match path {
            Some(path) => format!(
                "{}/{}",
                base_url.trim_end_matches('/'),
                path.trim_start_matches('/')
            ),
            None => base_url.to_string(),
        };

        let http = self.http_client.read().await;
        let mut request = http.get(&url);

        if let Some(q) = query {
            request = request.query(q);
        }
        let client_id = self.client_id.read().await;
        request = request.query(&[("client_id", &*client_id)]);

        let response = request.send().await.map_err(Error::from)?;

        let status = response.status().as_u16();

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(Error::with_status(
                status,
                format!("HTTP {}: {}", status, text),
            ));
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
            let result = self.get_json(SOUNDCLOUD_API_URL, Some(path), query).await;

            match result {
                Ok((body, _status)) => {
                    return Ok(body);
                }
                Err(e) => {
                    if e.is_status(401) && self.retry_config.retry_on_401 && retries < max_retries {
                        retries += 1;
                        self.refresh_client_id().await?;
                        continue;
                    }
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

    pub async fn resolve_url(
        &self,
        url: impl AsRef<str>,
    ) -> Result<crate::models::response::ResolvedResource, Error> {
        self.get("resolve", Some(&[("url", url.as_ref())])).await
    }

    pub async fn health_check(&self) -> bool {
        self.get::<_, Value>(
            "resolve",
            Some(&[("url", "https://soundcloud.com/soundcloud")]),
        )
        .await
        .is_ok()
    }
}
