use std::time::Duration;

use reqwest::Client;
use url::Url;

use super::parser;
use super::types::HaddockDoc;
use crate::backend::BackendError;
use crate::cache::DiskCache;

const USER_AGENT: &str = concat!("hoogle-tui/", env!("CARGO_PKG_VERSION"));

pub struct HaddockFetcher {
    client: Client,
    cache: DiskCache,
}

impl HaddockFetcher {
    pub fn new(cache: DiskCache, timeout_secs: u64) -> Result<Self, BackendError> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(timeout_secs))
            .read_timeout(Duration::from_secs(timeout_secs * 2))
            .gzip(true)
            .user_agent(USER_AGENT)
            .build()
            .map_err(|e| BackendError::SearchFailed {
                message: format!("failed to create HTTP client: {e}"),
            })?;

        Ok(Self { client, cache })
    }

    /// Fetch a Haddock HTML page, using cache if available.
    pub async fn fetch_html(&self, url: &Url) -> Result<String, BackendError> {
        let cache_key = url.as_str();

        // Check cache first
        if let Some(html) = self.cache.get_string(cache_key).await {
            tracing::debug!("cache hit for {url}");
            return Ok(html);
        }

        tracing::debug!("cache miss for {url}, fetching...");

        // Fetch from network
        match self.client.get(url.as_str()).send().await {
            Ok(response) => {
                if !response.status().is_success() {
                    return Err(BackendError::DocNotAvailable {
                        reason: format!("HTTP {}", response.status()),
                    });
                }

                let html = response.text().await.map_err(BackendError::Network)?;

                // Store in cache (best effort)
                if let Err(e) = self.cache.put_string(cache_key, &html).await {
                    tracing::warn!("failed to cache {url}: {e}");
                }

                Ok(html)
            }
            Err(e) => {
                // On network error, try stale cache
                if let Some(stale) = self.cache.get_stale(cache_key).await {
                    if let Ok(html) = String::from_utf8(stale) {
                        tracing::warn!("network error, using stale cache for {url}: {e}");
                        return Ok(html);
                    }
                }
                Err(BackendError::Network(e))
            }
        }
    }

    /// Fetch, parse, and return structured docs.
    pub async fn fetch_doc(&self, url: &Url) -> Result<HaddockDoc, BackendError> {
        let html = self.fetch_html(url).await?;
        parser::parse_haddock_html(&html, url)
            .map_err(|msg| BackendError::ParseError { message: msg })
    }

    /// Fetch the source code page for a declaration.
    pub async fn fetch_source(&self, source_url: &Url) -> Result<String, BackendError> {
        let html = self.fetch_html(source_url).await?;
        Ok(parser::parse_source_html(&html))
    }
}
