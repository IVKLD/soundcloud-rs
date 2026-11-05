use crate::models::{
    client::{Client, ProxyUrlProvider},
    config::RetryConfig,
    error::Error,
};

/// Builder for configuring and initializing a [`Client`] instance.
///
/// # Examples
///
/// ### 1. Basic initialization with automatic Client ID discovery
/// ```no_run
/// use soundcloud_rs::ClientBuilder;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = ClientBuilder::new().build().await?;
///     let track = client.get_track(123456789).await?;
///     println!("Loaded track: {:?}", track.title);
///     Ok(())
/// }
/// ```
///
/// ### 2. Static proxy configuration
/// ```no_run
/// use soundcloud_rs::ClientBuilder;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = ClientBuilder::new()
///         .with_proxy("http://127.0.0.1:8080".to_string())
///         .with_max_retries(3)
///         .build()
///         .await?;
///     Ok(())
/// }
/// ```
///
/// ### 3. Dynamic (reactive) proxy provider
/// Useful for desktop apps or long-running workers where proxy settings can change at runtime:
/// ```no_run
/// use std::sync::{Arc, RwLock};
/// use soundcloud_rs::{ClientBuilder, ProxyUrlProvider};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let current_proxy = Arc::new(RwLock::new(Some("http://proxy1.local:8080".to_string())));
///     let proxy_state = current_proxy.clone();
///
///     let provider = ProxyUrlProvider(Arc::new(move || {
///         proxy_state.read().unwrap().clone()
///     }));
///
///     let client = ClientBuilder::new()
///         .with_proxy_provider(provider)
///         .build()
///         .await?;
///
///     // Later in your app: update proxy without restarting the client
///     *current_proxy.write().unwrap() = Some("socks5://proxy2.local:1080".to_string());
///     Ok(())
/// }
/// ```
#[derive(Debug, Default)]
pub struct ClientBuilder {
    retry_config: RetryConfig,
    client_id: Option<String>,
    proxy_url: Option<String>,
    proxy_provider: Option<ProxyUrlProvider>,
}

impl ClientBuilder {
    /// Create a new `ClientBuilder` with default retry and discovery settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum number of retry attempts for failed requests.
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.retry_config.max_retries = max_retries;
        self
    }

    /// Enable or disable automatic retry on HTTP `401 Unauthorized`.
    ///
    /// When enabled, encountering a `401` status triggers automatic re-discovery
    /// and refresh of the SoundCloud `client_id` before retrying the request.
    pub fn with_retry_on_401(mut self, retry_on_401: bool) -> Self {
        self.retry_config.retry_on_401 = retry_on_401;
        self
    }

    /// Set a pre-fetched Client ID to skip automatic web-scraping discovery.
    pub fn with_client_id(mut self, client_id: String) -> Self {
        self.client_id = Some(client_id);
        self
    }

    /// Set a static proxy URL for all requests made by the client.
    ///
    /// Supported protocols: `http://`, `https://`, and `socks5://`.
    ///
    /// Note: this sets a fixed proxy. If you need proxy rotation or runtime updates,
    /// consider using [`with_proxy_provider`](Self::with_proxy_provider) instead.
    pub fn with_proxy(mut self, proxy_url: String) -> Self {
        self.proxy_url = Some(proxy_url);
        self
    }

    /// Attach a dynamic, reactive proxy provider.
    ///
    /// The provider is a closure invoked prior to HTTP requests. If it returns a URL
    /// different from the one the underlying HTTP client is currently configured with,
    /// the client transparently rebuilds its internal connection pool on the fly
    /// without losing cached authentication state (`client_id`).
    ///
    /// # Example
    /// ```no_run
    /// use std::sync::Arc;
    /// use soundcloud_rs::{ClientBuilder, ProxyUrlProvider};
    ///
    /// let provider = ProxyUrlProvider(Arc::new(|| {
    ///     Some("http://127.0.0.1:9050".to_string())
    /// }));
    ///
    /// let builder = ClientBuilder::new().with_proxy_provider(provider);
    /// ```
    pub fn with_proxy_provider(mut self, provider: ProxyUrlProvider) -> Self {
        self.proxy_provider = Some(provider);
        self
    }

    /// Builds and asynchronously initializes the [`Client`].
    ///
    /// If no `client_id` was explicitly supplied, this method performs initial
    /// web discovery to fetch a valid `client_id` from SoundCloud assets.
    pub async fn build(self) -> Result<Client, Error> {
        let mut client =
            Client::new_with_options(self.client_id, self.retry_config, self.proxy_url).await?;
        if let Some(provider) = self.proxy_provider {
            client = client.with_proxy_provider(provider);
        }
        Ok(client)
    }
}
