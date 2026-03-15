pub mod local;
mod parse;
pub mod web;

use crate::config::BackendConfig;
use crate::haddock::types::HaddockDoc;
use crate::models::SearchResult;
use async_trait::async_trait;
use url::Url;

pub use parse::parse_hoogle_json;

#[async_trait]
pub trait HoogleBackend: Send + Sync {
    /// Search for a query string, returning up to `count` results.
    async fn search(&self, query: &str, count: usize) -> Result<Vec<SearchResult>, BackendError>;

    /// Fetch the Haddock documentation page at the given URL.
    async fn fetch_doc(&self, url: &Url) -> Result<HaddockDoc, BackendError>;

    /// Return the backend name for display.
    fn name(&self) -> &str;
}

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("hoogle binary not found at {path}")]
    HoogleNotFound { path: String },

    #[error("hoogle search failed: {message}")]
    SearchFailed { message: String },

    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("failed to parse hoogle output: {message}")]
    ParseError { message: String },

    #[error("timeout after {seconds}s")]
    Timeout { seconds: u64 },

    #[error("documentation not available: {reason}")]
    DocNotAvailable { reason: String },
}

/// Create a backend based on configuration.
pub async fn create_backend(
    config: &BackendConfig,
) -> Result<Box<dyn HoogleBackend>, BackendError> {
    match config.mode {
        crate::config::BackendMode::Local => {
            let backend = local::LocalBackend::new(config)?;
            Ok(Box::new(backend))
        }
        crate::config::BackendMode::Web => {
            let backend = web::WebBackend::new(config)?;
            tracing::info!("using web backend: {}", backend.name());
            Ok(Box::new(backend))
        }
        crate::config::BackendMode::Auto => match local::LocalBackend::new(config) {
            Ok(backend) => {
                tracing::info!("auto-detected local hoogle backend");
                Ok(Box::new(backend))
            }
            Err(e) => {
                tracing::info!("local hoogle not available ({e}), falling back to web");
                let backend = web::WebBackend::new(config)?;
                Ok(Box::new(backend))
            }
        },
    }
}
