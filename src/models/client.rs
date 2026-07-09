use std::{fmt, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::models::config::RetryConfig;

#[derive(Clone)]
pub struct ProxyUrlProvider(pub Arc<dyn Fn() -> Option<String> + Send + Sync>);

impl fmt::Debug for ProxyUrlProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ProxyUrlProvider")
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Identifier {
    Id(i64),
    Urn(String),
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Identifier::Id(id) => write!(f, "{id}"),
            Identifier::Urn(urn) => write!(f, "{urn}"),
        }
    }
}

/// SoundCloud API client
pub struct Client {
    pub client_id: RwLock<String>,
    pub retry_config: RetryConfig,
    pub http_client: RwLock<reqwest::Client>,
    pub proxy_url: Option<String>,
    pub proxy_provider: Option<ProxyUrlProvider>,
    pub active_proxy: RwLock<Option<String>>,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("client_id", &self.client_id)
            .field("retry_config", &self.retry_config)
            .field("http_client", &self.http_client)
            .field("proxy_url", &self.proxy_url)
            .field("proxy_provider", &self.proxy_provider)
            .field("active_proxy", &self.active_proxy)
            .finish()
    }
}
