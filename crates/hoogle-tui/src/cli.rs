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

    /// Run 'hoogle generate' to build/update the local database, then exit
    #[arg(long)]
    pub generate: bool,
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

    /// Run `hoogle generate` and return true if --generate was used.
    pub fn handle_generate(&self) -> bool {
        if !self.generate {
            return false;
        }

        eprintln!("Running hoogle generate...");
        eprintln!("This downloads and indexes Haskell package documentation.");
        eprintln!("It may take a few minutes on first run.\n");

        // Find hoogle binary
        let hoogle = which::which("hoogle").unwrap_or_else(|_| {
            eprintln!("Error: hoogle binary not found on PATH.");
            eprintln!("Install it with: cabal install hoogle");
            std::process::exit(1);
        });

        let status = std::process::Command::new(hoogle)
            .arg("generate")
            .status();

        match status {
            Ok(s) if s.success() => {
                eprintln!("\nHoogle database generated successfully.");
                eprintln!("You can now run: hoogle-tui");
            }
            Ok(s) => {
                eprintln!("\nhoogle generate exited with status: {s}");
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("\nFailed to run hoogle generate: {e}");
                std::process::exit(1);
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use hoogle_core::config::{BackendMode, Config};

    fn default_config() -> Config {
        Config::default()
    }

    fn parse(args: &[&str]) -> CliArgs {
        CliArgs::try_parse_from(args).expect("failed to parse args")
    }

    #[test]
    fn default_args() {
        let args = parse(&["hoogle-tui"]);
        assert_eq!(args.query, None);
        assert_eq!(args.backend, "auto");
        assert_eq!(args.theme, None);
        assert_eq!(args.database, None);
        assert_eq!(args.config, None);
        assert!(!args.no_cache);
        assert_eq!(args.max_results, None);
        assert_eq!(args.log_level, "warn");
        assert!(!args.generate);
    }

    #[test]
    fn query_positional() {
        let args = parse(&["hoogle-tui", "map"]);
        assert_eq!(args.query, Some("map".to_string()));
    }

    #[test]
    fn backend_local() {
        let args = parse(&["hoogle-tui", "--backend", "local"]);
        assert_eq!(args.backend, "local");
    }

    #[test]
    fn backend_short() {
        let args = parse(&["hoogle-tui", "-b", "web"]);
        assert_eq!(args.backend, "web");
    }

    #[test]
    fn theme_override() {
        let args = parse(&["hoogle-tui", "--theme", "nord"]);
        assert_eq!(args.theme, Some("nord".to_string()));
    }

    #[test]
    fn no_cache_flag() {
        let args = parse(&["hoogle-tui", "--no-cache"]);
        assert!(args.no_cache);
    }

    #[test]
    fn max_results_flag() {
        let args = parse(&["hoogle-tui", "--max-results", "25"]);
        assert_eq!(args.max_results, Some(25));
    }

    #[test]
    fn database_path() {
        let args = parse(&["hoogle-tui", "--database", "/tmp/hoogle.db"]);
        assert_eq!(args.database, Some(PathBuf::from("/tmp/hoogle.db")));
    }

    // Tests for apply_to_config

    #[test]
    fn apply_theme_override() {
        let args = parse(&["hoogle-tui", "--theme", "nord"]);
        let mut config = default_config();
        args.apply_to_config(&mut config);
        assert_eq!(config.theme, "nord");
    }

    #[test]
    fn apply_no_theme_leaves_default() {
        let args = parse(&["hoogle-tui"]);
        let mut config = default_config();
        let original_theme = config.theme.clone();
        args.apply_to_config(&mut config);
        assert_eq!(config.theme, original_theme);
    }

    #[test]
    fn apply_no_cache_disables_cache() {
        let args = parse(&["hoogle-tui", "--no-cache"]);
        let mut config = default_config();
        assert!(config.cache.enabled);
        args.apply_to_config(&mut config);
        assert!(!config.cache.enabled);
    }

    #[test]
    fn apply_without_no_cache_preserves_cache() {
        let args = parse(&["hoogle-tui"]);
        let mut config = default_config();
        args.apply_to_config(&mut config);
        assert!(config.cache.enabled);
    }

    #[test]
    fn apply_max_results() {
        let args = parse(&["hoogle-tui", "--max-results", "10"]);
        let mut config = default_config();
        args.apply_to_config(&mut config);
        assert_eq!(config.ui.max_results, 10);
    }

    #[test]
    fn apply_no_max_results_preserves_default() {
        let args = parse(&["hoogle-tui"]);
        let mut config = default_config();
        let original = config.ui.max_results;
        args.apply_to_config(&mut config);
        assert_eq!(config.ui.max_results, original);
    }

    #[test]
    fn apply_database_path() {
        let args = parse(&["hoogle-tui", "--database", "/foo/bar.hoo"]);
        let mut config = default_config();
        args.apply_to_config(&mut config);
        assert_eq!(
            config.backend.database_path,
            Some(PathBuf::from("/foo/bar.hoo"))
        );
    }

    #[test]
    fn apply_backend_local() {
        let args = parse(&["hoogle-tui", "--backend", "local"]);
        let mut config = default_config();
        args.apply_to_config(&mut config);
        assert_eq!(config.backend.mode, BackendMode::Local);
    }

    #[test]
    fn apply_backend_web() {
        let args = parse(&["hoogle-tui", "--backend", "web"]);
        let mut config = default_config();
        args.apply_to_config(&mut config);
        assert_eq!(config.backend.mode, BackendMode::Web);
    }

    #[test]
    fn apply_backend_auto_no_change() {
        let args = parse(&["hoogle-tui", "--backend", "auto"]);
        let mut config = default_config();
        args.apply_to_config(&mut config);
        // "auto" doesn't match "local" or "web", so mode stays default (Auto)
        assert_eq!(config.backend.mode, BackendMode::Auto);
    }

    #[test]
    fn apply_backend_unknown_no_change() {
        let args = parse(&["hoogle-tui", "--backend", "foobar"]);
        let mut config = default_config();
        let original = config.backend.mode;
        args.apply_to_config(&mut config);
        assert_eq!(config.backend.mode, original);
    }

    #[test]
    fn apply_multiple_overrides() {
        let args = parse(&[
            "hoogle-tui",
            "--theme",
            "gruvbox_dark",
            "--no-cache",
            "--max-results",
            "5",
            "--backend",
            "web",
            "--database",
            "/tmp/db",
        ]);
        let mut config = default_config();
        args.apply_to_config(&mut config);
        assert_eq!(config.theme, "gruvbox_dark");
        assert!(!config.cache.enabled);
        assert_eq!(config.ui.max_results, 5);
        assert_eq!(config.backend.mode, BackendMode::Web);
        assert_eq!(
            config.backend.database_path,
            Some(PathBuf::from("/tmp/db"))
        );
    }
}
