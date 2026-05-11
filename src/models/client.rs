use serde::{Deserialize, Serialize};
use std::fmt;
use tokio::sync::RwLock;

use crate::models::config::RetryConfig;

#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug)]
pub struct Client {
    pub client_id: RwLock<String>,
    pub retry_config: RetryConfig,
    pub http_client: reqwest::Client,
}
impl Client {
    pub fn with_client_id(client_id: String, retry_config: RetryConfig) -> Self {
        Self {
            client_id: RwLock::new(client_id),
            retry_config,
            http_client: reqwest::Client::new(),
        }
    }
}
