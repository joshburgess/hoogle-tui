use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use tokio::process::Command;
use url::Url;

use super::{parse, BackendError, HoogleBackend};
use crate::config::BackendConfig;
use crate::haddock::types::HaddockDoc;
use crate::models::SearchResult;

#[derive(Debug)]
pub struct LocalBackend {
    hoogle_path: PathBuf,
    database_path: Option<PathBuf>,
    timeout: Duration,
}

impl LocalBackend {
    pub fn new(config: &BackendConfig) -> Result<Self, BackendError> {
        let hoogle_path = if let Some(ref path) = config.hoogle_path {
            if path.exists() {
                path.clone()
            } else {
                return Err(BackendError::HoogleNotFound {
                    path: path.display().to_string(),
                });
            }
        } else {
            which::which("hoogle").map_err(|_| BackendError::HoogleNotFound {
                path: "hoogle".into(),
            })?
        };

        tracing::info!("using hoogle at {}", hoogle_path.display());

        Ok(Self {
            hoogle_path,
            database_path: config.database_path.clone(),
            timeout: Duration::from_secs(config.timeout_secs),
        })
    }
}

#[async_trait]
impl HoogleBackend for LocalBackend {
    async fn search(&self, query: &str, count: usize) -> Result<Vec<SearchResult>, BackendError> {
        let mut cmd = Command::new(&self.hoogle_path);
        cmd.arg("search")
            .arg(query)
            .arg(format!("--count={count}"))
            .arg("--json");

        if let Some(ref db) = self.database_path {
            cmd.arg(format!("--database={}", db.display()));
        }

        let output = tokio::time::timeout(self.timeout, cmd.output())
            .await
            .map_err(|_| BackendError::Timeout {
                seconds: self.timeout.as_secs(),
            })?
            .map_err(|e| BackendError::SearchFailed {
                message: format!("failed to execute hoogle: {e}"),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BackendError::SearchFailed {
                message: format!("hoogle exited with {}: {}", output.status, stderr.trim()),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let results = parse::parse_hoogle_output(&stdout)
            .map_err(|message| BackendError::ParseError { message })?;
        tracing::info!(
            "local search for {:?} returned {} results",
            query,
            results.len()
        );
        Ok(results)
    }

    async fn fetch_doc(&self, _url: &Url) -> Result<HaddockDoc, BackendError> {
        // Doc fetching is implemented in Phase 7 via the HaddockFetcher
        Err(BackendError::DocNotAvailable {
            reason: "doc fetching not yet implemented".into(),
        })
    }

    fn name(&self) -> &str {
        "local"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires hoogle to be installed
    async fn integration_local_search() {
        let config = BackendConfig::default();
        let backend = match LocalBackend::new(&config) {
            Ok(b) => b,
            Err(_) => {
                eprintln!("hoogle not found, skipping integration test");
                return;
            }
        };

        let results = backend.search("map", 5).await.unwrap();
        assert!(!results.is_empty());
        assert!(results.len() <= 5);

        // All results should have a name
        for r in &results {
            assert!(!r.name.is_empty(), "result name should not be empty");
        }
    }

    #[tokio::test]
    #[ignore] // Requires hoogle to be installed
    async fn integration_local_search_type_sig() {
        let config = BackendConfig::default();
        let backend = match LocalBackend::new(&config) {
            Ok(b) => b,
            Err(_) => return,
        };

        let results = backend.search("a -> a", 5).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn integration_local_search_empty() {
        let config = BackendConfig::default();
        let backend = match LocalBackend::new(&config) {
            Ok(b) => b,
            Err(_) => return,
        };

        let results = backend
            .search("xyzzy_nonexistent_function_42", 5)
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn hoogle_not_found() {
        let config = BackendConfig {
            hoogle_path: Some(PathBuf::from("/nonexistent/hoogle")),
            ..Default::default()
        };
        let err = LocalBackend::new(&config).unwrap_err();
        assert!(matches!(err, BackendError::HoogleNotFound { .. }));
    }
}
