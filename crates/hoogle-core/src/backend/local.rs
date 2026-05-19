use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use tokio::process::Command;

use super::{parse, BackendError, HoogleBackend};
use crate::config::BackendConfig;
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

fn build_search_args(
    query: &str,
    offset: usize,
    count: usize,
    database_path: Option<&PathBuf>,
) -> Vec<String> {
    let fetch_count = offset.saturating_add(count);
    let mut args = vec![
        "search".to_string(),
        query.to_string(),
        format!("--count={fetch_count}"),
        "--json".to_string(),
    ];

    if let Some(db) = database_path {
        args.push(format!("--database={}", db.display()));
    }

    args
}

fn slice_results_for_offset(
    results: impl IntoIterator<Item = SearchResult>,
    offset: usize,
    count: usize,
) -> Vec<SearchResult> {
    results.into_iter().skip(offset).take(count).collect()
}

#[async_trait]
impl HoogleBackend for LocalBackend {
    async fn search(
        &self,
        query: &str,
        offset: usize,
        count: usize,
    ) -> Result<Vec<SearchResult>, BackendError> {
        let mut cmd = Command::new(&self.hoogle_path);
        for arg in build_search_args(query, offset, count, self.database_path.as_ref()) {
            cmd.arg(arg);
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
        let results = slice_results_for_offset(results, offset, count);
        tracing::info!(
            "local search for {:?} returned {} results",
            query,
            results.len()
        );
        Ok(results)
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

        let results = backend.search("map", 0, 5).await.unwrap();
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

        let results = backend.search("a -> a", 0, 5).await.unwrap();
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
            .search("xyzzy_nonexistent_function_42", 0, 5)
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

    #[test]
    fn build_search_args_use_offset_adjusted_count() {
        assert_eq!(
            build_search_args("map", 10, 5, None),
            vec!["search", "map", "--count=15", "--json"]
        );
    }

    #[test]
    fn build_search_args_preserve_type_signature_query() {
        assert_eq!(
            build_search_args("a -> b", 0, 20, None),
            vec!["search", "a -> b", "--count=20", "--json"]
        );
    }

    #[test]
    fn build_search_args_include_database_path() {
        let database_path = PathBuf::from("/tmp/hoogle/default.hoo");

        assert_eq!(
            build_search_args("foldMap", 2, 3, Some(&database_path)),
            vec![
                "search",
                "foldMap",
                "--count=5",
                "--json",
                "--database=/tmp/hoogle/default.hoo"
            ]
        );
    }

    #[test]
    fn slices_results_for_cli_offset() {
        let results = (0..5)
            .map(|i| SearchResult {
                name: format!("result_{i}"),
                module: None,
                package: None,
                signature: None,
                doc_url: None,
                short_doc: None,
                result_kind: crate::models::ResultKind::Function,
            })
            .collect::<Vec<_>>();

        let page = slice_results_for_offset(results, 2, 2);
        assert_eq!(
            page.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
            vec!["result_2", "result_3"]
        );
    }

    #[test]
    fn slices_empty_page_after_available_results() {
        let results = (0..2)
            .map(|i| SearchResult {
                name: format!("result_{i}"),
                module: None,
                package: None,
                signature: None,
                doc_url: None,
                short_doc: None,
                result_kind: crate::models::ResultKind::Function,
            })
            .collect::<Vec<_>>();

        assert!(slice_results_for_offset(results, 10, 5).is_empty());
    }
}
