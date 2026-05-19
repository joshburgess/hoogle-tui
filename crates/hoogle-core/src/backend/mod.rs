pub mod local;
mod parse;
pub mod web;

use crate::config::BackendConfig;
use crate::models::SearchResult;
use async_trait::async_trait;

pub use parse::parse_hoogle_json;

#[async_trait]
pub trait HoogleBackend: Send + Sync {
    /// Search for a query string, returning up to `count` results after `offset` matches.
    async fn search(
        &self,
        query: &str,
        offset: usize,
        count: usize,
    ) -> Result<Vec<SearchResult>, BackendError>;

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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::config::BackendMode;

    use super::*;

    fn config_with_mode(mode: BackendMode) -> BackendConfig {
        BackendConfig {
            mode,
            hoogle_path: Some(PathBuf::from("/nonexistent/hoogle")),
            web_url: "https://hoogle.haskell.org".into(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn create_backend_web_mode_uses_web_backend() {
        let backend = create_backend(&config_with_mode(BackendMode::Web))
            .await
            .unwrap();

        assert_eq!(backend.name(), "hoogle.haskell.org");
    }

    #[tokio::test]
    async fn create_backend_local_mode_reports_missing_hoogle() {
        let result = create_backend(&config_with_mode(BackendMode::Local)).await;

        assert!(matches!(result, Err(BackendError::HoogleNotFound { .. })));
    }

    #[tokio::test]
    async fn create_backend_auto_mode_falls_back_to_web_when_local_missing() {
        let backend = create_backend(&config_with_mode(BackendMode::Auto))
            .await
            .unwrap();

        assert_eq!(backend.name(), "hoogle.haskell.org");
    }
}
