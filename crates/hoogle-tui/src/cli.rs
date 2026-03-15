use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "hoogle-tui", about = "Terminal UI for Hoogle", version)]
pub struct CliArgs {
    /// Initial search query
    pub query: Option<String>,

    /// Backend: auto, local, web
    #[arg(short, long, default_value = "auto")]
    pub backend: String,

    /// Path to hoogle database
    #[arg(short, long)]
    pub database: Option<PathBuf>,

    /// Color theme name
    #[arg(short, long)]
    pub theme: Option<String>,

    /// Config file path
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Disable caching
    #[arg(long)]
    pub no_cache: bool,

    /// Max results
    #[arg(long)]
    pub max_results: Option<usize>,

    /// Log level (error, warn, info, debug, trace)
    #[arg(long, default_value = "warn")]
    pub log_level: String,

    /// Generate shell completions and exit
    #[arg(long, value_name = "SHELL")]
    pub completions: Option<Shell>,
}

impl CliArgs {
    /// Apply CLI overrides to a config.
    pub fn apply_to_config(&self, config: &mut hoogle_core::config::Config) {
        if let Some(ref theme) = self.theme {
            config.theme = theme.clone();
        }
        if self.no_cache {
            config.cache.enabled = false;
        }
        if let Some(max) = self.max_results {
            config.ui.max_results = max;
        }
        if let Some(ref db) = self.database {
            config.backend.database_path = Some(db.clone());
        }
        match self.backend.as_str() {
            "local" => config.backend.mode = hoogle_core::config::BackendMode::Local,
            "web" => config.backend.mode = hoogle_core::config::BackendMode::Web,
            _ => {}
        }
    }

    /// Generate shell completions to stdout and return true if --completions was used.
    pub fn handle_completions(&self) -> bool {
        if let Some(shell) = self.completions {
            let mut cmd = Self::command();
            clap_complete::generate(shell, &mut cmd, "hoogle-tui", &mut std::io::stdout());
            true
        } else {
            false
        }
    }
}
