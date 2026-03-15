use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Mutex;
use tokio::time::Instant;
use url::Url;

use super::{parse, BackendError, HoogleBackend};
use crate::config::BackendConfig;
use crate::haddock::types::HaddockDoc;
use crate::models::SearchResult;

const USER_AGENT: &str = concat!("hoogle-tui/", env!("CARGO_PKG_VERSION"));
const MIN_REQUEST_INTERVAL: Duration = Duration::from_millis(200);
const MAX_RETRIES: u32 = 3;

#[derive(Debug)]
pub struct WebBackend {
    client: reqwest::Client,
    base_url: String,
    last_request: Mutex<Instant>,
}

impl WebBackend {
    pub fn new(config: &BackendConfig) -> Result<Self, BackendError> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(config.timeout_secs))
            .read_timeout(Duration::from_secs(config.timeout_secs * 2))
            .gzip(true)
            .user_agent(USER_AGENT)
            .build()
            .map_err(|e| BackendError::SearchFailed {
                message: format!("failed to create HTTP client: {e}"),
            })?;

        Ok(Self {
            client,
            base_url: config.web_url.trim_end_matches('/').to_string(),
            last_request: Mutex::new(Instant::now() - MIN_REQUEST_INTERVAL),
        })
    }

    /// Enforce minimum interval between requests.
    async fn rate_limit(&self) {
        let mut last = self.last_request.lock().await;
        let elapsed = last.elapsed();
        if elapsed < MIN_REQUEST_INTERVAL {
            tokio::time::sleep(MIN_REQUEST_INTERVAL - elapsed).await;
        }
        *last = Instant::now();
    }

    /// Execute a GET request with retry logic.
    async fn get_with_retry(&self, url: &str) -> Result<String, BackendError> {
        let mut last_err = None;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                let backoff = Duration::from_millis(500 * 2u64.pow(attempt - 1));
                tracing::debug!("retry attempt {attempt}, backing off {backoff:?}");
                tokio::time::sleep(backoff).await;
            }

            self.rate_limit().await;

            let response = match self.client.get(url).send().await {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!("request failed (attempt {attempt}): {e}");
                    last_err = Some(BackendError::Network(e));
                    if attempt == 0 {
                        continue; // retry once on network error
                    }
                    break;
                }
            };

            let status = response.status();

            if status.is_success() {
                return response.text().await.map_err(BackendError::Network);
            }

            if status.as_u16() == 429 {
                tracing::warn!("rate limited (429), attempt {attempt}");
                last_err = Some(BackendError::SearchFailed {
                    message: "rate limited by server".into(),
                });
                continue; // retry with exponential backoff
            }

            if status.is_server_error() {
                tracing::warn!("server error {status}, attempt {attempt}");
                last_err = Some(BackendError::SearchFailed {
                    message: format!("server error: {status}"),
                });
                if attempt == 0 {
                    continue; // retry once on 5xx
                }
                break;
            }

            // 4xx (not 429) — don't retry
            return Err(BackendError::SearchFailed {
                message: format!("HTTP {status}"),
            });
        }

        Err(last_err.unwrap_or(BackendError::SearchFailed {
            message: "request failed after retries".into(),
        }))
    }

    fn build_search_url(&self, query: &str, count: usize) -> String {
        let encoded = urlencoding::encode(query);
        format!(
            "{}?mode=json&hoogle={}&start=1&count={}",
            self.base_url, encoded, count
        )
    }
}

#[async_trait]
impl HoogleBackend for WebBackend {
    async fn search(&self, query: &str, count: usize) -> Result<Vec<SearchResult>, BackendError> {
        let url = self.build_search_url(query, count);
        tracing::debug!("web search: {url}");

        let body = self.get_with_retry(&url).await?;

        parse::parse_hoogle_output(&body).map_err(|message| BackendError::ParseError { message })
    }

    async fn fetch_doc(&self, _url: &Url) -> Result<HaddockDoc, BackendError> {
        // Doc fetching is implemented in Phase 7 via the HaddockFetcher
        Err(BackendError::DocNotAvailable {
            reason: "doc fetching not yet implemented".into(),
        })
    }

    fn name(&self) -> &str {
        "hoogle.haskell.org"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> BackendConfig {
        BackendConfig {
            web_url: "https://hoogle.haskell.org".into(),
            timeout_secs: 10,
            ..Default::default()
        }
    }

    #[test]
    fn build_search_url_basic() {
        let backend = WebBackend::new(&test_config()).unwrap();
        let url = backend.build_search_url("map", 10);
        assert_eq!(
            url,
            "https://hoogle.haskell.org?mode=json&hoogle=map&start=1&count=10"
        );
    }

    #[test]
    fn build_search_url_special_chars() {
        let backend = WebBackend::new(&test_config()).unwrap();
        let url = backend.build_search_url("a -> b", 5);
        assert!(url.contains("a%20-%3E%20b"));
        assert!(url.contains("count=5"));
    }

    #[test]
    fn build_search_url_type_signature() {
        let backend = WebBackend::new(&test_config()).unwrap();
        let url = backend.build_search_url("Ord k => k -> Map k v -> Maybe v", 20);
        assert!(url.contains("mode=json"));
        assert!(url.contains("count=20"));
    }

    #[test]
    fn build_search_url_unicode() {
        let backend = WebBackend::new(&test_config()).unwrap();
        let url = backend.build_search_url("α -> β", 5);
        assert!(url.contains("mode=json"));
    }

    #[test]
    fn build_search_url_empty() {
        let backend = WebBackend::new(&test_config()).unwrap();
        let url = backend.build_search_url("", 10);
        assert_eq!(
            url,
            "https://hoogle.haskell.org?mode=json&hoogle=&start=1&count=10"
        );
    }

    #[test]
    fn build_search_url_trailing_slash_stripped() {
        let config = BackendConfig {
            web_url: "https://hoogle.haskell.org/".into(),
            ..test_config()
        };
        let backend = WebBackend::new(&config).unwrap();
        let url = backend.build_search_url("map", 10);
        assert!(url.starts_with("https://hoogle.haskell.org?"));
    }

    #[tokio::test]
    #[ignore] // Hits the real Hoogle web API
    async fn integration_web_search() {
        let backend = WebBackend::new(&test_config()).unwrap();
        let results = backend.search("map", 5).await.unwrap();
        assert!(!results.is_empty());
        assert!(results.len() <= 5);
        for r in &results {
            assert!(!r.name.is_empty());
        }
    }

    #[tokio::test]
    #[ignore]
    async fn integration_web_search_type_sig() {
        let backend = WebBackend::new(&test_config()).unwrap();
        let results = backend.search("a -> a", 5).await.unwrap();
        assert!(!results.is_empty());
    }
}
