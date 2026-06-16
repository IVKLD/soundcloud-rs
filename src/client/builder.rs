use crate::models::{client::Client, config::RetryConfig, error::Error};

#[derive(Debug, Default)]
pub struct ClientBuilder {
    retry_config: RetryConfig,
    client_id: Option<String>,
    proxy_url: Option<String>,
}

impl ClientBuilder {
    /// Create a new ClientBuilder with default retry configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum number of retry attempts.
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.retry_config.max_retries = max_retries;
        self
    }

    /// Enable or disable retrying on 401 Unauthorized responses.
    pub fn with_retry_on_401(mut self, retry_on_401: bool) -> Self {
        self.retry_config.retry_on_401 = retry_on_401;
        self
    }

    /// Set a pre-fetched client ID to skip automatic discovery.
    pub fn with_client_id(mut self, client_id: String) -> Self {
        self.client_id = Some(client_id);
        self
    }

    pub fn with_proxy(mut self, proxy_url: String) -> Self {
        self.proxy_url = Some(proxy_url);
        self
    }

    /// Build the Client with the configured settings.
    pub async fn build(self) -> Result<Client, Error> {
        Client::new_with_options(self.client_id, self.retry_config, self.proxy_url).await
    }
}
