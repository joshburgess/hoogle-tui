# hoogle-tui — Complete Implementation Plan

## Project Overview

**hoogle-tui** is a terminal user interface for Haskell's Hoogle search engine, built in Rust using ratatui. It provides interactive search, Haskell syntax highlighting, and in-terminal Haddock documentation browsing. It aims to surpass existing tools (`hoogle` CLI and `bhoogle`) in usability, aesthetics, and feature depth.

**Repository structure:** Cargo workspace with three crates.

```
hoogle-tui/
├── Cargo.toml                  # workspace root
├── README.md
├── LICENSE
├── .github/
│   └── workflows/
│       └── ci.yml
├── config/
│   └── default.toml            # default configuration shipped with binary
├── themes/
│   ├── dracula.toml
│   ├── catppuccin_mocha.toml
│   ├── gruvbox_dark.toml
│   ├── solarized_dark.toml
│   ├── monokai.toml
│   └── nord.toml
├── crates/
│   ├── hoogle-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── backend/
│   │       │   ├── mod.rs       # HoogleBackend trait
│   │       │   ├── local.rs     # local hoogle CLI backend
│   │       │   └── web.rs       # hoogle.haskell.org API backend
│   │       ├── haddock/
│   │       │   ├── mod.rs
│   │       │   ├── fetcher.rs   # HTTP fetching + caching
│   │       │   ├── parser.rs    # HTML -> DocBlock AST
│   │       │   └── types.rs     # HaddockDoc, DocBlock, etc.
│   │       ├── models.rs        # SearchResult, PackageInfo, etc.
│   │       ├── cache.rs         # disk cache abstraction
│   │       └── config.rs        # configuration types and loading
│   ├── hoogle-syntax/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── tokenizer.rs     # type signature tokenizer
│   │       ├── haskell.rs       # full Haskell code highlighter
│   │       ├── tokens.rs        # token types
│   │       └── theme.rs         # theme types + built-in themes
│   └── hoogle-tui/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── app.rs           # App state + update loop
│           ├── event.rs         # event handling (keyboard, mouse, resize)
│           ├── cli.rs           # clap CLI argument parsing
│           ├── ui/
│           │   ├── mod.rs
│           │   ├── search_bar.rs
│           │   ├── result_list.rs
│           │   ├── preview_pane.rs
│           │   ├── doc_viewer.rs
│           │   ├── source_viewer.rs
│           │   ├── help_overlay.rs
│           │   ├── toc_popup.rs
│           │   ├── filter_popup.rs
│           │   ├── status_bar.rs
│           │   └── layout.rs    # split pane logic
│           ├── actions.rs       # action enum dispatched by keybinds
│           ├── keymap.rs        # keybind resolution
│           └── clipboard.rs     # clipboard integration
```

---

## Technology Choices & Dependency Manifest

These are the exact crate dependencies to use across the workspace. Do not substitute alternatives unless a crate is unmaintained or broken.

### Workspace-level (`Cargo.toml` at root)

```toml
[workspace]
resolver = "2"
members = ["crates/hoogle-core", "crates/hoogle-syntax", "crates/hoogle-tui"]

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### hoogle-core

```toml
[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
reqwest = { version = "0.12", features = ["json", "gzip"] }
scraper = "0.21"
url = { version = "2", features = ["serde"] }
dirs = "6"
toml = "0.8"
sha2 = "0.10"           # for cache key hashing
async-trait = "0.1"
```

### hoogle-syntax

```toml
[dependencies]
serde = { workspace = true }
unicode-width = "0.2"
```

No heavy dependencies — this crate must compile fast and be pure computation.

### hoogle-tui

```toml
[dependencies]
hoogle-core = { path = "../hoogle-core" }
hoogle-syntax = { path = "../hoogle-syntax" }
tokio = { workspace = true }
serde = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
ratatui = "0.29"
crossterm = "0.28"
clap = { version = "4", features = ["derive"] }
nucleo = "0.5"            # fuzzy matching
arboard = "3"             # clipboard
tui-textarea = "0.7"      # search bar editing
```

---

## Data Models Reference

All phases reference these types. Implement them in full during the phase that first requires them, then extend as needed.

### Core models (`hoogle-core/src/models.rs`)

```rust
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub name: String,
    pub module: Option<ModulePath>,
    pub package: Option<PackageInfo>,
    pub signature: Option<String>,
    pub doc_url: Option<Url>,
    pub short_doc: Option<String>,
    pub result_kind: ResultKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModulePath(pub Vec<String>); // e.g. ["Data", "Map", "Strict"]

impl ModulePath {
    pub fn as_dotted(&self) -> String {
        self.0.join(".")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResultKind {
    Function,
    TypeAlias,
    DataType,
    Newtype,
    Class,
    Module,
    Package,
}
```

### Syntax tokens (`hoogle-syntax/src/tokens.rs`)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Keyword(String),       // forall, where, type, data, class, newtype, family, instance
    TypeConstructor(String), // Map, Maybe, Int, IO — starts uppercase
    TypeVariable(String),  // a, b, k, m — starts lowercase, not a keyword
    Operator(String),      // ->, =>, ::, .., ~, @, !, %1 ->
    QualifiedName(String), // Data.Map.Map (the whole thing)
    Punctuation(char),     // ( ) [ ] , { }
    StringLiteral(String), // "..." in type-level strings
    NumericLiteral(String),// 3 in type-level nats
    Whitespace(usize),     // number of spaces
    Comment(String),       // -- line comments in code blocks
    Pragma(String),        // {-# ... #-} in code blocks
    Unknown(String),       // fallback
}
```

### Haddock types (`hoogle-core/src/haddock/types.rs`)

```rust
use url::Url;

#[derive(Debug, Clone)]
pub struct HaddockDoc {
    pub module: String,
    pub package: String,
    pub description: Vec<DocBlock>,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone)]
pub struct Declaration {
    pub name: String,
    pub signature: Option<String>,
    pub doc: Vec<DocBlock>,
    pub since: Option<String>,
    pub source_url: Option<Url>,
    pub anchor: Option<String>,
}

#[derive(Debug, Clone)]
pub enum DocBlock {
    Paragraph(Vec<Inline>),
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    UnorderedList(Vec<Vec<Inline>>),
    OrderedList(Vec<Vec<Inline>>),
    Header {
        level: u8,
        content: Vec<Inline>,
    },
    HorizontalRule,
    Note(Vec<Inline>),       // Haddock @since, warnings, etc.
}

#[derive(Debug, Clone)]
pub enum Inline {
    Text(String),
    Code(String),
    Link { text: String, url: Url },
    ModuleLink(String),
    Emphasis(String),
    Bold(String),
    Math(String),            // Haddock LaTeX fragments
}
```

### Configuration (`hoogle-core/src/config.rs`)

```rust
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub backend: BackendConfig,
    pub ui: UiConfig,
    pub theme: String,             // theme name: "dracula", "catppuccin_mocha", etc.
    pub cache: CacheConfig,
    pub keybinds: KeybindOverrides,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BackendConfig {
    pub mode: BackendMode,         // "auto", "local", "web"
    pub hoogle_path: Option<PathBuf>,  // path to hoogle binary
    pub database_path: Option<PathBuf>,
    pub web_url: String,           // default: "https://hoogle.haskell.org"
    pub timeout_secs: u64,         // default: 5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub max_results: usize,        // default: 50
    pub preview_enabled: bool,     // default: true
    pub layout: LayoutMode,        // "auto", "vertical", "horizontal"
    pub mouse_enabled: bool,       // default: true
    pub debounce_ms: u64,          // default: 150
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CacheConfig {
    pub enabled: bool,
    pub dir: Option<PathBuf>,      // default: ~/.cache/hoogle-tui/
    pub ttl_hours: u64,            // default: 168 (7 days)
    pub max_size_mb: u64,          // default: 500
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BackendMode { Auto, Local, Web }

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LayoutMode { Auto, Vertical, Horizontal }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeybindOverrides {
    // maps action name -> key string, e.g. "scroll_down" -> "j"
    pub overrides: std::collections::HashMap<String, String>,
}

impl Default for Config { /* sensible defaults */ }
impl Default for BackendConfig { /* ... */ }
impl Default for UiConfig { /* ... */ }
impl Default for CacheConfig { /* ... */ }
```

---

## Default Keybindings Reference

Implement these as the default keybind map. All must be overridable via config.

```
── Global ──────────────────────────────────────────
Esc             Back to previous mode / cancel / quit from top level
q               Quit (when not in search bar)
?               Toggle help overlay
Ctrl-l          Force redraw
Ctrl-c          Quit immediately

── Search Bar ──────────────────────────────────────
<typing>        Live search (debounced)
Enter           Move focus to results list
Ctrl-r          Open search history popup
Ctrl-u          Clear search bar
Ctrl-w          Delete word backward
Esc             Clear search / quit if empty

── Result List ─────────────────────────────────────
j / Down        Move selection down
k / Up          Move selection up
g               Jump to first result
G               Jump to last result
Enter           Open full Haddock docs for selected result
Tab             Toggle preview pane on/off
Space           Scroll preview pane down (if visible)
/               Focus search bar
f               Open filter popup (by ResultKind)
s               Sort popup (relevance / package / module)
y               Yank type signature to clipboard
Y               Yank qualified import statement to clipboard
Ctrl-y          Yank Hackage URL to clipboard

── Doc Viewer ──────────────────────────────────────
j / Down        Scroll down one line
k / Up          Scroll up one line
d / Ctrl-d      Scroll down half page
u / Ctrl-u      Scroll up half page
f / Ctrl-f      Scroll down full page
b / Ctrl-b      Scroll up full page
g               Jump to top
G               Jump to bottom
o               Open table of contents popup
Enter           Follow highlighted link
Backspace       Navigate back in doc history
Tab             Cycle through focusable links
n               Next declaration
p               Previous declaration
s               View source code for current declaration
/               Search within document
Esc             Return to result list

── Source Viewer ────────────────────────────────────
j / k           Scroll
g / G           Top / bottom
Esc             Return to doc viewer
y               Yank source to clipboard

── Popups (TOC, Filter, History, Help) ─────────────
j / k           Navigate items
Enter           Select
Esc             Close popup
<typing>        Filter items (in TOC and history popups)
```

---

## Phase 1: Project Scaffolding & Workspace Setup

### Goal
Create the full Cargo workspace structure, CI pipeline, configuration system, and a compiling (but empty) TUI shell. After this phase, `cargo run` launches a blank ratatui app that handles terminal setup/teardown cleanly.

### Tasks

**1.1 — Initialize the workspace**

Create the directory structure exactly as specified in the "Repository structure" section above. Create all `Cargo.toml` files with the exact dependencies listed in the "Technology Choices" section. Create placeholder `lib.rs` and `mod.rs` files so the workspace compiles. The root `Cargo.toml` should define the workspace. Each subcrate should reference workspace dependencies where indicated.

Ensure `cargo build` succeeds with zero errors and zero warnings. Ensure `cargo clippy` passes clean. Ensure `cargo test` runs (even if there are no tests yet).

**1.2 — CI configuration**

Create `.github/workflows/ci.yml` with the following jobs:
- `check`: runs `cargo fmt --all -- --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`
- Run on: push to `main`, all pull requests
- Matrix: test on `ubuntu-latest` and `macos-latest`
- Cache `~/.cargo` and `target/` directories

**1.3 — Implement core data models**

In `hoogle-core/src/models.rs`, implement all the types from the "Core models" section above. Include `Display` implementations for `ModulePath` (dotted format), `PackageInfo` ("name-version" format), and `ResultKind`. Add `#[cfg(test)]` module with basic unit tests for Display impls and serialization round-trips.

In `hoogle-syntax/src/tokens.rs`, implement the `Token` enum from the "Syntax tokens" section. No highlighting logic yet, just the types.

**1.4 — Configuration system**

Implement `hoogle-core/src/config.rs` with all types from the "Configuration" section above. Implement `Default` for every config struct with sensible defaults. Implement a `Config::load()` function that:
1. Checks for `--config` CLI arg path
2. Falls back to `~/.config/hoogle-tui/config.toml`
3. Falls back to compiled-in defaults if no file found
4. Merges: file config overrides defaults, CLI args override file config

Write a `config/default.toml` file at the repo root that documents every option with comments. This file is the reference configuration.

Write unit tests that verify: default config is valid, TOML deserialization works for a sample config, missing fields fall back to defaults.

**1.5 — CLI argument parsing**

In `hoogle-tui/src/cli.rs`, implement a `clap` derive-based argument parser:

```rust
#[derive(Parser, Debug)]
#[command(name = "hoogle-tui", about = "Terminal UI for Hoogle")]
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
}
```

**1.6 — TUI shell with clean terminal management**

In `hoogle-tui/src/main.rs`, implement:
1. Parse CLI args
2. Load config
3. Initialize tracing subscriber (log to `~/.local/share/hoogle-tui/hoogle-tui.log`, not stderr)
4. Enter alternate screen, enable raw mode (crossterm)
5. Create ratatui `Terminal` with `CrosstermBackend`
6. Run the app event loop
7. On exit (or panic!): restore terminal — disable raw mode, leave alternate screen. **Use a panic hook** (`std::panic::set_hook`) that restores the terminal before printing the panic message, so the user's terminal is never left in a broken state.

In `hoogle-tui/src/app.rs`, implement the `App` struct:

```rust
pub struct App {
    pub mode: AppMode,
    pub should_quit: bool,
    pub config: Config,
    // These will be populated in later phases:
    // pub search_state: SearchState,
    // pub result_state: ResultState,
    // pub doc_state: DocState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Search,
    Results,
    DocView,
    SourceView,
    Help,
}
```

Implement `App::new(config)`, `App::tick()` (called on each event loop iteration), and `App::draw(frame: &mut Frame)` (renders a placeholder "hoogle-tui" centered text).

In `hoogle-tui/src/event.rs`, implement an async event loop:
- Use `crossterm::event::EventStream` (requires `crossterm` `event-stream` feature)
- Or use a channel-based approach: spawn a thread that reads `crossterm::event::read()` and sends to an `mpsc` channel, main loop selects between event channel and async task results
- Handle `KeyEvent`, `Resize`, `Mouse` (if enabled in config)
- Map raw key events to `Action` enum (defined in `actions.rs`)

In `hoogle-tui/src/actions.rs`, define the action enum:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Quit,
    Back,
    FocusSearch,
    FocusResults,
    MoveUp,
    MoveDown,
    MoveToTop,
    MoveToBottom,
    Select,          // Enter
    TogglePreview,
    ToggleHelp,
    ScrollDown,
    ScrollUp,
    ScrollHalfDown,
    ScrollHalfUp,
    ScrollPageDown,
    ScrollPageUp,
    NextDeclaration,
    PrevDeclaration,
    OpenTOC,
    FollowLink,
    NavBack,
    OpenFilter,
    OpenSort,
    YankSignature,
    YankImport,
    YankUrl,
    ViewSource,
    SearchInDoc,
    SearchHistory,
    ClearSearch,
    Redraw,
    Tick,            // periodic tick for animations/debounce
    None,
}
```

In `hoogle-tui/src/keymap.rs`, implement a `Keymap` struct that maps `(AppMode, KeyEvent) -> Action`. Load default keybindings. Apply overrides from config. The mapping should be a `HashMap<(AppMode, KeyEvent), Action>`.

### Verification

After completing Phase 1:
- `cargo build --workspace` succeeds with no warnings
- `cargo clippy --workspace -- -D warnings` is clean
- `cargo test --workspace` passes
- `cargo run` launches a blank TUI, shows centered text, quits on `q` or `Esc`
- The terminal is always cleanly restored, even on panic (test by adding a temporary `panic!()` in the draw function)
- Config loading works: create a `~/.config/hoogle-tui/config.toml` with one changed value and verify it's picked up

---

## Phase 2: Search Backend — Local Hoogle CLI

### Goal
Implement the local Hoogle backend that shells out to the `hoogle` command-line tool, parses its JSON output, and returns structured `SearchResult` values.

### Tasks

**2.1 — Backend trait**

In `hoogle-core/src/backend/mod.rs`, define:

```rust
use crate::models::SearchResult;
use crate::haddock::types::HaddockDoc;
use async_trait::async_trait;
use url::Url;

#[async_trait]
pub trait HoogleBackend: Send + Sync {
    /// Search for a query string, returning up to `count` results.
    async fn search(&self, query: &str, count: usize) -> Result<Vec<SearchResult>, BackendError>;

    /// Fetch the Haddock documentation page at the given URL.
    async fn fetch_doc(&self, url: &Url) -> Result<HaddockDoc, BackendError>;

    /// Return the backend name for display (e.g., "local", "hoogle.haskell.org")
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
```

**2.2 — Local backend implementation**

In `hoogle-core/src/backend/local.rs`, implement `LocalBackend`:

```rust
pub struct LocalBackend {
    hoogle_path: PathBuf,
    database_path: Option<PathBuf>,
    timeout: Duration,
}
```

The `search` method should:
1. Spawn `hoogle search <query> --count=<count> --json` as an async process using `tokio::process::Command`
2. If `database_path` is set, add `--database=<path>`
3. Apply timeout using `tokio::time::timeout`
4. Read stdout, parse as JSON
5. Map JSON entries to `SearchResult`

Hoogle's JSON output format (one JSON object per line, or a JSON array depending on version):

```json
{
  "url": "https://hackage.haskell.org/package/containers/docs/Data-Map-Strict.html#v:lookup",
  "module": { "name": "Data.Map.Strict", "url": "..." },
  "package": { "name": "containers", "url": "..." },
  "item": "<span class=name>lookup</span> :: <a>Ord</a> k =&gt; k -&gt; <a>Map</a> k a -&gt; <a>Maybe</a> a",
  "type": "",
  "docs": "O(log n). Look up the value at a key in the map.\n..."
}
```

Write a robust parser that:
- Handles both JSON-per-line and JSON-array formats (try array first, fall back to line-by-line)
- Strips HTML tags from the `item` field to extract the clean name and signature
- Parses the `module.name` string into a `ModulePath`
- Extracts package name and version from the package URL or name field
- Determines `ResultKind` by inspecting the `item` HTML (look for keywords: `data`, `type`, `newtype`, `class`, `module`)
- Extracts the first paragraph from `docs` as `short_doc`
- Handles missing/null fields gracefully

**2.3 — Backend auto-detection**

In `hoogle-core/src/backend/mod.rs`, add a factory function:

```rust
pub async fn create_backend(config: &BackendConfig) -> Result<Box<dyn HoogleBackend>, BackendError> {
    match config.mode {
        BackendMode::Local => { /* create LocalBackend */ },
        BackendMode::Web => { /* placeholder, Phase 3 */ },
        BackendMode::Auto => {
            // Try local first: check if hoogle binary exists
            // If yes, use local. If no, fall back to web.
        }
    }
}
```

For auto-detection:
1. Check `config.hoogle_path` if set, or look for `hoogle` on `$PATH` using `which::which` (add `which` crate)
2. Verify it works by running `hoogle --version`
3. If found and working, use `LocalBackend`
4. Otherwise, fall back to `WebBackend` (or error if `WebBackend` not yet implemented)

**2.4 — Unit and integration tests**

Write tests in `hoogle-core/src/backend/local.rs`:
- Unit test: parsing Hoogle JSON output (use fixture strings of real Hoogle output)
- Unit test: HTML tag stripping from `item` field
- Unit test: `ResultKind` detection from various item patterns
- Integration test (behind `#[cfg(test)]` and `#[ignore]` attribute for CI): actually run `hoogle search "map" --json --count=5` and verify parsing succeeds

Create a `crates/hoogle-core/tests/fixtures/` directory with sample Hoogle JSON outputs for:
- A function result (`lookup`)
- A type result (`Map`)
- A class result (`Monad`)
- A module result
- A result with complex signature (multi-line, constraints, forall)
- A result with no documentation

### Verification

After completing Phase 2:
- `cargo test --workspace` passes, including the JSON parsing tests
- The `LocalBackend::search()` method works against a real Hoogle installation (manual test)
- Edge cases are handled: empty results, malformed JSON, hoogle not found, timeout

---

## Phase 3: Search Backend — Web API

### Goal
Implement the Hoogle web API backend as an alternative to the local CLI backend. This allows the tool to work without a local Hoogle installation.

### Tasks

**3.1 — Web backend implementation**

In `hoogle-core/src/backend/web.rs`, implement `WebBackend`:

```rust
pub struct WebBackend {
    client: reqwest::Client,
    base_url: String,   // default: "https://hoogle.haskell.org"
    timeout: Duration,
}
```

The `search` method should:
1. Build URL: `{base_url}?mode=json&hoogle={url_encoded_query}&start=1&count={count}`
2. Send GET request with appropriate User-Agent header: `hoogle-tui/{version}`
3. Parse JSON response
4. Map to `Vec<SearchResult>`

The web API returns JSON in the same format as the CLI `--json` flag. Reuse the same parsing logic from Phase 2. Extract the shared parsing code into a `parse_hoogle_json(value: &serde_json::Value) -> Result<SearchResult>` function in `hoogle-core/src/backend/mod.rs` so both backends use it.

**3.2 — HTTP client configuration**

Configure the `reqwest::Client` with:
- Connection timeout: 5 seconds (from config)
- Read timeout: 10 seconds
- Gzip decompression enabled
- Connection pooling (default in reqwest)
- User-Agent header

**3.3 — Rate limiting and retry**

Implement a simple rate limiter for the web backend:
- Minimum 200ms between requests (to be respectful to the public Hoogle server)
- On HTTP 429 (Too Many Requests): back off exponentially (1s, 2s, 4s), max 3 retries
- On HTTP 5xx: retry once after 1s
- On network error: retry once after 500ms

Use a `tokio::time::Instant` to track the last request time and `tokio::time::sleep` to enforce spacing.

**3.4 — Update auto-detection**

Update the `create_backend` factory in `hoogle-core/src/backend/mod.rs` to fall back to `WebBackend` when `LocalBackend` is unavailable. Log the decision at `info` level.

**3.5 — Tests**

- Unit test: URL construction for various queries (special characters, unicode, empty string)
- Unit test: rate limiter logic (mock time)
- Integration test (`#[ignore]`): hit the real Hoogle web API with a simple query and verify parsing

### Verification

After completing Phase 3:
- Both backends implement the same trait and return compatible results
- The web backend works without Hoogle installed locally
- Auto-detection correctly chooses local when available, web otherwise
- Rate limiting prevents hammering the server

---

## Phase 4: Syntax Highlighting Engine

### Goal
Build a tokenizer and theme system in `hoogle-syntax` that can highlight Haskell type signatures and (later) full Haskell code blocks. After this phase, type signatures can be converted into colored `ratatui::text::Line` values.

### Tasks

**4.1 — Type signature tokenizer**

In `hoogle-syntax/src/tokenizer.rs`, implement a hand-written lexer function:

```rust
pub fn tokenize_signature(input: &str) -> Vec<Token>
```

The tokenizer must handle all of the following correctly:

1. **Type constructors**: identifiers starting with uppercase (`Map`, `Maybe`, `IO`, `Int`, `String`, `ByteString`)
2. **Type variables**: identifiers starting with lowercase that are NOT keywords (`a`, `b`, `k`, `m`, `f`, `xs`)
3. **Keywords**: `forall`, `where`, `type`, `data`, `class`, `newtype`, `family`, `instance`, `deriving`, `infixl`, `infixr`, `infix`
4. **Operators**: `->`, `=>`, `::`, `..`, `~`, `@`, `!`, `*`, `#`
5. **Linear types**: `%1 ->`, `%m ->`
6. **Qualified names**: `Data.Map.Map`, `Control.Monad.IO.Class.MonadIO` — tokenize as a single `QualifiedName` token
7. **Operator types in parens**: `(+++)`, `(>>>=)`, `(.|.)` — the parens are punctuation, the inner part is a name
8. **Type-level strings**: `"hello"` in kind signatures
9. **Type-level numbers**: `3` in `Nat` kind
10. **Promoted constructors**: `'True`, `'Just`
11. **Kind signatures**: `Type`, `Constraint`, `Type -> Type`
12. **Parentheses, brackets, commas, braces**: each is a `Punctuation` token
13. **Whitespace**: collapse to `Whitespace(n)` tokens
14. **Unboxed tuples**: `(# #)` — handle the `#` inside parens

The lexer should be single-pass, character-by-character, using a state machine or simple lookahead. It does NOT need to be a full parser — it's a lexer that assigns token types based on lexical rules.

Edge cases to handle:
- The identifier `a` is a type variable, not a keyword
- The identifier `class` is a keyword even mid-signature (in `class Monad m where`)
- `->` should be recognized even without surrounding spaces
- Nested parentheses: `(a -> (b -> c)) -> d`
- Empty input returns empty vec
- Unicode identifiers: Haskell allows unicode in identifiers, handle gracefully (treat as `Unknown` if truly unusual)

**4.2 — Haskell code block highlighter (simpler version)**

In `hoogle-syntax/src/haskell.rs`, implement:

```rust
pub fn tokenize_haskell(input: &str) -> Vec<Vec<Token>>  // one Vec<Token> per line
```

This handles full Haskell code (as found in Haddock code examples). It's a superset of the signature tokenizer, additionally handling:
- `-- line comments`
- `{- block comments -}`
- `{-# LANGUAGE ... #-}` pragmas
- String literals with escapes
- Character literals
- Numeric literals (decimal, hex, octal, float)
- All Haskell keywords: `module`, `import`, `qualified`, `as`, `hiding`, `do`, `let`, `in`, `case`, `of`, `if`, `then`, `else`, `where`, `deriving`, `instance`, etc.
- Operator definitions: symbols like `>>=`, `<$>`, `<*>`
- Pattern matching syntax

This does NOT need to be a full parser. A lexer-level highlighter that assigns reasonable token types based on lexical context is sufficient. When in doubt, emit `Unknown`.

**4.3 — Theme system**

In `hoogle-syntax/src/theme.rs`, implement:

```rust
use ratatui::style::{Color, Modifier, Style};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub styles: HashMap<SemanticToken, Style>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticToken {
    TypeConstructor,
    TypeVariable,
    Keyword,
    Operator,
    Punctuation,
    QualifiedName,
    StringLiteral,
    NumericLiteral,
    Comment,
    Pragma,
    ModuleName,    // in UI: module path display
    PackageName,   // in UI: package name display
    DocText,       // documentation prose
    DocCode,       // inline code in docs
    DocHeading,    // heading in docs
    DocLink,       // hyperlink in docs
    SearchInput,   // search bar text
    StatusBar,     // status bar background + text
    Selected,      // selected item in list
    Cursor,        // cursor line highlight
    Border,        // widget borders
    Error,         // error messages
    Spinner,       // loading indicator
}
```

Implement the `to_ratatui_style(&self, token: SemanticToken) -> Style` method that returns the ratatui `Style` for a given semantic token.

Create built-in themes by implementing `Theme::dracula()`, `Theme::catppuccin_mocha()`, `Theme::gruvbox_dark()`, `Theme::solarized_dark()`, `Theme::monokai()`, `Theme::nord()`. Each should define colors for every `SemanticToken`. Use the actual color palettes from each theme's specification. Also implement `Theme::default()` which returns Dracula.

Implement `Theme::from_toml(path: &Path) -> Result<Theme>` so users can define custom themes in TOML files. The TOML format should mirror the `SemanticToken` enum:

```toml
name = "my_theme"

[styles]
type_constructor = { fg = "#bd93f9", modifiers = ["bold"] }
type_variable = { fg = "#f8f8f2", modifiers = ["italic"] }
keyword = { fg = "#ff79c6", modifiers = ["bold"] }
# ... etc
```

Write theme TOML files for each built-in theme in the `themes/` directory.

**4.4 — Integration: Token → ratatui Span conversion**

In `hoogle-syntax/src/lib.rs`, implement a public function:

```rust
pub fn highlight_signature(sig: &str, theme: &Theme) -> ratatui::text::Line<'static>
```

And:

```rust
pub fn highlight_code(code: &str, theme: &Theme) -> Vec<ratatui::text::Line<'static>>
```

These tokenize the input, map each token to its `SemanticToken`, look up the style from the theme, and produce styled `Span` values grouped into `Line` values.

**4.5 — Tests**

Extensive tests for the tokenizer:
- Simple signatures: `Int -> Int`, `a -> b -> c`
- Constrained: `Ord k => k -> Map k a -> Maybe a`
- Multi-constraint: `(Monad m, MonadIO m) => m a -> IO a`
- Forall: `forall a b. (a -> b) -> [a] -> [b]`
- Qualified: `Data.Map.Strict.Map k v`
- Kind signature: `(Type -> Type) -> Constraint`
- Operator type: `(++) :: [a] -> [a] -> [a]`
- Edge cases: empty string, single identifier, only punctuation
- Snapshot tests: tokenize known signatures and compare against expected token sequences

Tests for themes:
- All built-in themes define a style for every `SemanticToken`
- TOML round-trip: serialize a theme to TOML, deserialize, verify equality
- Custom theme loading from file

### Verification

After completing Phase 4:
- `highlight_signature("Ord k => k -> Map k a -> Maybe a", &Theme::dracula())` returns a `Line` with correctly colored spans
- All 6 built-in themes load and cover all semantic tokens
- The tokenizer handles 20+ different signature patterns without panicking
- `cargo test` has comprehensive tokenizer coverage

---

## Phase 5: Interactive Search UI

### Goal
Build the complete search experience: search bar with live input, debounced async search, result list with syntax-highlighted signatures, and keyboard navigation. After this phase, the tool is functional for basic Hoogle searching — already better than raw `hoogle` CLI.

### Tasks

**5.1 — App state expansion**

Expand `App` in `hoogle-tui/src/app.rs`:

```rust
pub struct App {
    pub mode: AppMode,
    pub should_quit: bool,
    pub config: Config,
    pub theme: Theme,
    pub backend: Box<dyn HoogleBackend>,
    pub search: SearchState,
    pub results: ResultState,
    pub status: StatusMessage,
}

pub struct SearchState {
    pub textarea: tui_textarea::TextArea<'static>,
    pub query: String,              // current query text
    pub last_searched: String,      // last query actually sent to backend
    pub debounce_timer: Option<tokio::time::Instant>,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
}

pub struct ResultState {
    pub items: Vec<SearchResult>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub loading: bool,
    pub filter: Option<ResultKind>,
    pub highlighted_lines: Vec<HighlightedResult>, // pre-rendered
}

pub struct HighlightedResult {
    pub name_line: Line<'static>,
    pub sig_line: Line<'static>,
    pub doc_line: Option<Line<'static>>,
    pub module_line: Line<'static>,
}

pub enum StatusMessage {
    None,
    Info(String),
    Error(String),
    Loading(String),
}
```

**5.2 — Search bar widget**

In `hoogle-tui/src/ui/search_bar.rs`, implement a widget that renders:

```
╭─ Search ──────────────────────────────────────────╮
│ 🔍 map :: Ord k => k -> _                        │
╰───────────────────────────────────────────────────╯
```

Use `tui-textarea` for the actual text input handling (cursor movement, insertion, deletion, word-level operations). Wrap it in a `Block` with a title. Show a search icon or `>` prompt. When the search bar is focused, the border should use the `Selected` theme color. When not focused, use `Border` color.

**5.3 — Result list widget**

In `hoogle-tui/src/ui/result_list.rs`, implement a widget that renders each result as 2-3 lines:

```
  ┃ Data.Map.Strict                    containers-0.6.7
  ┃   lookup :: Ord k => k -> Map k a -> Maybe a
  ┃   O(log n). Look up the value at a key in the map.
  ┃─────────────────────────────────────────────────────
  ┃ Data.Map.Lazy                      containers-0.6.7
  ┃   lookup :: Ord k => k -> Map k a -> Maybe a
  ┃   O(log n). Look up the value at a key in the map.
```

The selected item should have a highlighted background (full width). The module name should use `ModuleName` style, package name should use `PackageName` style (right-aligned on the same line). The type signature should be syntax-highlighted using `highlight_signature()`. The doc preview should use `DocText` style, truncated to one line with `…` if too long.

Implement scrolling: when `selected` goes below the visible area, scroll down. When it goes above, scroll up. Keep 3 lines of context above/below the selection when scrolling (like vim's `scrolloff`).

Calculate the visible window based on the widget height and the number of lines per result (3 lines + 1 separator = 4 lines per result typically).

**5.4 — Status bar widget**

In `hoogle-tui/src/ui/status_bar.rs`, render a bottom status bar:

```
 local │ 42 results │ j/k:navigate  Enter:open  /:search  ?:help  q:quit
```

Left side: backend name, result count, current filter if active.
Right side: contextual key hints (change based on current `AppMode`).

For loading state, show a spinner animation (cycle through `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` on each tick).

**5.5 — Layout manager**

In `hoogle-tui/src/ui/layout.rs`, compute the layout:

```rust
pub fn compute_layout(area: Rect, config: &UiConfig, mode: AppMode) -> AppLayout {
    // Search bar: always 3 rows at the top
    // Status bar: always 1 row at the bottom
    // Remaining space: result list (and later, preview pane)
}

pub struct AppLayout {
    pub search_bar: Rect,
    pub main_area: Rect,  // results (and preview, in later phases)
    pub status_bar: Rect,
}
```

**5.6 — Debounced async search**

In the event loop, implement debounced searching:

1. On each keystroke in the search bar, reset the debounce timer to `now + config.ui.debounce_ms`
2. On each tick (run ticks at ~30ms intervals), check if the debounce timer has elapsed
3. If elapsed AND the query text differs from `last_searched`, fire an async search
4. When firing: set `results.loading = true`, spawn a `tokio::spawn` task that calls `backend.search()`
5. When the task completes, send the results back via a channel (use `tokio::sync::mpsc`)
6. In the event loop, check the results channel on each tick. If results arrived, update `results.items`, pre-render highlighted lines, set `loading = false`
7. If a new search fires while the previous is in-flight, the old results are simply replaced when the new ones arrive. Use a generation counter: each search increments a counter, and results are only accepted if their generation matches the current generation.

**5.7 — Keyboard navigation for results**

In the event handler, when `mode == AppMode::Results`:
- `j` / `Down`: increment `selected`, clamped to `items.len() - 1`
- `k` / `Up`: decrement `selected`, clamped to 0
- `g`: set `selected = 0`
- `G`: set `selected = items.len() - 1`
- `/`: switch to `AppMode::Search`, focus textarea
- `Enter`: switch to `AppMode::DocView` (implemented in Phase 7)
- `y`: copy signature to clipboard
- `Y`: copy import statement to clipboard
- `Ctrl-y`: copy URL to clipboard

**5.8 — Clipboard integration**

In `hoogle-tui/src/clipboard.rs`, implement clipboard operations using the `arboard` crate. The functions should:
- `copy_to_clipboard(text: &str) -> Result<()>`: copy text
- Show a status message "Copied to clipboard" for 2 seconds on success
- Show "Clipboard unavailable" on failure (e.g., no display server on headless Linux)
- Never panic on clipboard failure

For the "yank import" action, generate the import statement:
- If the result has a module, generate `import ModuleName (resultName)`
- If it's a qualified-heavy module like `Data.Map.Strict`, also offer `import qualified Data.Map.Strict as Map`

**5.9 — Initial query from CLI**

If the user passed an initial query via CLI args (`hoogle-tui "map"`), populate the search bar with it and immediately trigger a search (skip debounce).

**5.10 — Tests**

- Test that `HighlightedResult` generation works for various result types
- Test debounce logic with mocked time
- Test selection clamping at boundaries
- Snapshot test: render the result list to a `TestBackend` and verify the output buffer

### Verification

After completing Phase 5:
- Launch with `cargo run`, type a query, see syntax-highlighted results appear after debounce
- Navigate with j/k, selection visually highlights
- `/` returns to search, `g`/`G` jump to top/bottom
- Status bar shows result count and key hints
- `y` copies the signature (test by pasting)
- Works with both local and web backends
- Typing fast doesn't cause flickering or duplicate results (debounce works)
- Empty query shows empty state (not an error)

---

## Phase 6: Preview Pane & Result Filtering

### Goal
Add a split-pane preview of the short documentation for the selected result, plus result filtering by kind. After this phase, users can browse results and see docs without leaving the result list.

### Tasks

**6.1 — Split layout**

Update `compute_layout` to handle the preview pane:

When preview is enabled and terminal is wide enough (>= 120 cols for vertical, always for horizontal):
- **Vertical split**: results left (55% width), preview right (45% width)
- **Horizontal split**: results top (50% height), preview bottom (50% height)
- **Auto mode**: pick based on terminal width

```rust
pub struct AppLayout {
    pub search_bar: Rect,
    pub result_list: Rect,
    pub preview_pane: Option<Rect>,  // None if preview disabled or no space
    pub status_bar: Rect,
}
```

**6.2 — Preview pane widget**

In `hoogle-tui/src/ui/preview_pane.rs`, render a bordered pane showing:
- Title bar: module name and package
- The full type signature (syntax-highlighted, word-wrapped)
- A separator line
- The short documentation text (word-wrapped, basic formatting)
- If the doc contains code examples (lines starting with `>>> ` or indented by 4+ spaces), syntax-highlight them

The preview pane updates whenever the selection changes in the result list. The content is derived from the `SearchResult` fields already fetched (no additional network request).

**6.3 — Lazy doc preview fetching (optional enhancement)**

If the `short_doc` from the search result is too brief, optionally fetch the full first section of the Haddock doc in the background. This is a low-priority enhancement — the basic preview from search results is usually sufficient. If implemented:
- Fetch only when user hovers on a result for >500ms
- Cache the fetched preview
- Show a subtle loading indicator in the preview pane

**6.4 — Filter popup**

In `hoogle-tui/src/ui/filter_popup.rs`, render a centered popup when the user presses `f`:

```
╭─ Filter Results ─╮
│ ○ All             │
│ ● Functions       │
│ ○ Types           │
│ ○ Classes         │
│ ○ Modules         │
╰──────────────────╯
```

Navigate with `j`/`k`, select with `Enter`, close with `Esc`. When a filter is active, the result list only shows matching items. The status bar shows the active filter. The filter is client-side (filter the existing results, don't re-search).

**6.5 — Sort popup**

Similar to filter, pressing `s` opens a sort popup:

```
╭─ Sort Results ──╮
│ ● Relevance      │
│ ○ Package        │
│ ○ Module         │
│ ○ Name           │
╰─────────────────╯
```

Sorting is also client-side on the existing results.

**6.6 — Fuzzy re-filtering**

Integrate `nucleo` for a secondary fuzzy filter within results. When in result list mode, typing (without `/` prefix) applies a fuzzy filter overlay that narrows results based on the typed characters matching against name, module, and package. Show the fuzzy filter text in the status bar. This is distinct from the search bar — it filters already-fetched results client-side.

Implementation: when the user presses a letter key in result mode and it's not a keybinding, enter "fuzzy filter" sub-mode. Show a small input at the top of the result list: `Filter: map str`. Clear with `Esc` (returns to unfiltered view). Use `nucleo` matcher against `result.name + " " + result.module + " " + result.package`.

**6.7 — Tests**

- Test split layout calculation for various terminal sizes
- Test filter logic: apply each filter kind, verify correct results shown
- Test sort logic: verify ordering
- Snapshot test: render the preview pane with a known result

### Verification

After completing Phase 6:
- Preview pane appears and shows the doc for the selected result
- Preview updates as you navigate up/down
- `Tab` toggles preview on/off
- `f` opens filter popup, filtering works correctly
- `s` opens sort popup, sorting works correctly
- Layout adapts to terminal width
- Works correctly with 0 results, 1 result, and many results

---

## Phase 7: Haddock Document Fetching & Parsing

### Goal
Implement the HTML fetching, caching, and parsing pipeline that converts Haddock documentation pages into the structured `HaddockDoc` AST. This is the data layer for the doc viewer — no rendering yet.

### Tasks

**7.1 — Disk cache**

In `hoogle-core/src/cache.rs`, implement a file-based cache:

```rust
pub struct DiskCache {
    base_dir: PathBuf,    // ~/.cache/hoogle-tui/
    ttl: Duration,
    max_size_bytes: u64,
}

impl DiskCache {
    pub async fn get(&self, key: &str) -> Option<Vec<u8>>;
    pub async fn put(&self, key: &str, data: &[u8]) -> Result<()>;
    pub async fn invalidate(&self, key: &str) -> Result<()>;
    pub async fn clear(&self) -> Result<()>;
    pub async fn prune(&self) -> Result<()>; // evict entries over TTL or if total size > max
}
```

Cache key: SHA-256 hash of the URL. Store as files in `base_dir/ab/abcdef1234...` (first 2 chars as subdirectory to avoid huge flat directory). Store metadata (URL, fetch time, content length) in a `.meta` sidecar file.

**7.2 — Haddock HTML fetcher**

In `hoogle-core/src/haddock/fetcher.rs`:

```rust
pub struct HaddockFetcher {
    client: reqwest::Client,
    cache: DiskCache,
}

impl HaddockFetcher {
    /// Fetch a Haddock HTML page, using cache if available.
    pub async fn fetch(&self, url: &Url) -> Result<String, BackendError>;

    /// Fetch, parse, and return structured docs.
    pub async fn fetch_doc(&self, url: &Url) -> Result<HaddockDoc, BackendError>;

    /// Fetch the source code page for a declaration.
    pub async fn fetch_source(&self, source_url: &Url) -> Result<String, BackendError>;
}
```

The `fetch` method:
1. Check cache first
2. If cache miss or expired, fetch via HTTP
3. On success, store in cache
4. On network error with cached version available, return cached (stale) version with a warning
5. Set User-Agent, accept gzip

**7.3 — Haddock HTML parser**

In `hoogle-core/src/haddock/parser.rs`, implement the main parser:

```rust
pub fn parse_haddock_html(html: &str, url: &Url) -> Result<HaddockDoc, ParseError>
```

This is the most complex piece of the project. Haddock generates HTML with a specific structure. Use the `scraper` crate with CSS selectors.

**Module page structure to parse:**

The parser must handle the following Haddock HTML structure:

1. **Module header**: `#module-header` div — extract module name
2. **Description section**: `#description` div — the module's prose documentation
3. **Synopsis section**: `#synopsis` — list of all exported declarations (brief)
4. **Interface section**: the main content, containing all declarations

**Declarations**: Each declaration is typically in a structure like:
```html
<div class="top">
  <p class="src">
    <a id="v:lookup" class="def">lookup</a> :: Ord k => k -> Map k a -> Maybe a
    <a href="src/Data-Map-Internal.html#lookup" class="link">Source</a>
  </p>
  <div class="doc">
    <p>O(log n). Look up the value at a key in the map.</p>
    <pre>>>> lookup 1 (fromList [(1, 'a')])</pre>
    <pre>Just 'a'</pre>
  </div>
</div>
```

The parser should extract:
- Declaration name and anchor from `a.def`
- Type signature from `p.src` text (strip the source link)
- Source URL from `a.link[href]`
- Documentation from `div.doc`
- "Since" annotations from `<p class="since">`

**DocBlock parsing rules:**

For the documentation content (`div.doc` children), map HTML elements to `DocBlock`:

| HTML | DocBlock |
|---|---|
| `<p>` | `Paragraph(parse_inlines(children))` |
| `<pre>` | `CodeBlock { code: text_content }` |
| `<ul>` | `UnorderedList(items)` |
| `<ol>` | `OrderedList(items)` |
| `<h1>`..`<h6>` | `Header { level, content }` |
| `<hr>` | `HorizontalRule` |
| `<table>` | Flatten to paragraph (tables are rare in Haddock) |
| `<div class="warning">` | `Note(content)` |

**Inline parsing rules:**

| HTML | Inline |
|---|---|
| text node | `Text(text)` |
| `<code>` / `<tt>` | `Code(text)` |
| `<a href="...">` | `Link { text, url }` or `ModuleLink(name)` if href matches module pattern |
| `<em>` / `<i>` | `Emphasis(text)` |
| `<strong>` / `<b>` | `Bold(text)` |

**Resolving relative URLs:**

Many links in Haddock are relative. Resolve them against the page URL using `url::Url::join()`. Module links typically look like `Module-Name.html` — detect these and convert to `ModuleLink`.

**Robustness:**

- The parser should never panic. All errors should be `Result` or fallback to `Unknown` content.
- If a declaration can't be fully parsed, include it with whatever was extracted (partial is better than skipping).
- Handle variations across Haddock versions (the HTML structure has changed over the years). Test against old and new Hackage packages.
- Handle empty modules, modules with only re-exports, modules with no documentation.

**7.4 — Tests**

This phase needs the most thorough testing:

Create `crates/hoogle-core/tests/fixtures/haddock/` with saved HTML files from real Hackage pages:
- `Data.Map.Strict.html` — a large, well-documented module
- `Data.Maybe.html` — a small, simple module
- `Control.Monad.html` — has class instances, complex signatures
- `GHC.Generics.html` — complex types, associated types
- `Data.Aeson.html` — third-party package, different Haddock style
- `Prelude.html` — huge module, re-exports

For each fixture, write tests that verify:
- Correct number of declarations parsed
- A specific declaration's name, signature, and doc are correct
- Links are correctly resolved
- Code blocks are extracted
- "Since" annotations are found

Also test edge cases:
- Invalid HTML (parser should not panic)
- Empty HTML
- HTML with no declarations
- HTML from very old Haddock versions (Haskell Platform era)

Write cache tests:
- Put and get round-trip
- TTL expiration
- Cache miss returns None
- Pruning removes old entries

### Verification

After completing Phase 7:
- `parse_haddock_html()` correctly parses all fixture files
- The cache works: first fetch is slow, second is instant
- Error handling: bad URLs return proper errors, not panics
- The fetcher falls back to cached content on network failure

---

## Phase 8: Haddock Document Viewer

### Goal
Build the terminal renderer that takes a `HaddockDoc` AST and renders it beautifully in a ratatui widget. This is the killer feature — reading Haddock docs in the terminal.

### Tasks

**8.1 — Doc viewer widget**

In `hoogle-tui/src/ui/doc_viewer.rs`, implement a widget that renders a full `HaddockDoc`:

The widget receives a `HaddockDoc` and renders it into a scrollable area. Pre-render the entire document into a `Vec<Line>` (styled ratatui lines) so scrolling is just adjusting an offset into this vec.

**Rendering rules for each DocBlock:**

**Paragraph:**
- Word-wrap the inline content to fit the available width minus 2 (for padding)
- Respect `Inline` styles: `Code` gets `DocCode` theme style, `Emphasis` gets italic, `Bold` gets bold, `Link` gets `DocLink` style + underline
- Add 1 blank line after each paragraph

**CodeBlock:**
- Render inside a bordered box using unicode box-drawing characters (`┌─┐│└─┘`)
- Use a slightly different background if the terminal supports it (or just use the `DocCode` style)
- Syntax-highlight the code using `hoogle_syntax::highlight_code()`
- Lines starting with `>>>` should highlight the `>>>` as a prompt marker (use `Comment` style for `>>>`), and the rest as Haskell code
- Lines NOT starting with `>>>` after a `>>>` line are output — render in plain `DocText` style
- Add 1 blank line before and after the code block

**Headers:**
- `Header { level: 1 }`: bold, `DocHeading` style, underlined with `━` characters
- `Header { level: 2 }`: bold, `DocHeading` style, underlined with `─` characters
- `Header { level: 3+ }`: bold, `DocHeading` style, no underline
- Add 1 blank line before and after

**Lists:**
- Unordered: indent 2 spaces, prefix with `• `
- Ordered: indent 2 spaces, prefix with `1. `, `2. `, etc.
- Items may be multi-line; subsequent lines indent to align with the first character after the bullet
- Add 1 blank line after the entire list

**Declaration rendering:**

At the top of the doc view (or when jumping to a declaration), render:

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  lookup :: Ord k => k -> Map k a -> Maybe a
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

The signature is syntax-highlighted. The box uses `Operator` style for the `━` characters.

After the signature, render the declaration's `doc` blocks using the rules above.

If `since` is present, render it as a right-aligned badge: `[since containers-0.5.0]` in dim style.

**8.2 — Scroll state**

```rust
pub struct DocViewState {
    pub doc: Option<HaddockDoc>,
    pub rendered_lines: Vec<Line<'static>>,
    pub scroll_offset: usize,
    pub total_lines: usize,
    pub viewport_height: usize,
    pub declaration_offsets: Vec<(String, usize)>, // (name, line_offset) for TOC
    pub focused_link: Option<usize>,   // index into links vec
    pub links: Vec<(usize, Url)>,      // (line_offset, target URL)
    pub nav_stack: Vec<Url>,           // back navigation history
    pub search_query: Option<String>,
    pub search_matches: Vec<usize>,    // line offsets of matches
    pub current_match: Option<usize>,
}
```

**8.3 — Scrolling**

Implement all scroll keybindings:
- `j`/`Down`: scroll 1 line
- `k`/`Up`: scroll 1 line up
- `d`/`Ctrl-d`: scroll half viewport
- `u`/`Ctrl-u`: scroll up half viewport
- `f`/`Ctrl-f`: scroll full viewport
- `b`/`Ctrl-b`: scroll up full viewport
- `g`: scroll to top
- `G`: scroll to bottom

Show a scroll indicator on the right edge: a thin scrollbar using `█` and `░` characters showing position within the document.

**8.4 — Declaration navigation**

- `n`: jump to the next declaration (find next entry in `declaration_offsets` after current `scroll_offset`)
- `p`: jump to the previous declaration

When entering the doc viewer from a search result, jump directly to the matching declaration (match by name and anchor).

**8.5 — Table of contents popup**

In `hoogle-tui/src/ui/toc_popup.rs`, when the user presses `o`, show a popup listing all declarations:

```
╭─ Table of Contents ────────────────────────────╮
│ 🔍 Filter...                                   │
│                                                 │
│   lookup       :: Ord k => k -> Map k a -> ...  │
│   insert       :: Ord k => k -> a -> Map k ...  │
│ > delete       :: Ord k => k -> Map k a -> ...  │
│   member       :: Ord k => k -> Map k a -> ...  │
│   findWithDef  :: Ord k => a -> k -> Map k ...  │
│   ...                                           │
╰─────────────────────────────────────────────────╯
```

Features:
- Scrollable list of all declarations with truncated signatures
- `j`/`k` to navigate, `Enter` to jump to that declaration, `Esc` to close
- Typing filters the list (fuzzy match on declaration name)
- Selected item highlighted

**8.6 — Link following**

In the doc view, links are rendered with underline + `DocLink` style. Implement link navigation:
- `Tab`: cycle focus to the next link on screen (highlight it with a distinct style, e.g., reverse video)
- `Enter` when a link is focused: follow the link
  - If it's a `ModuleLink`: fetch that module's Haddock page, push current URL onto `nav_stack`, display new doc
  - If it's an external URL: show a status message with the URL (don't open browser from TUI by default)
- `Backspace`: pop `nav_stack`, go back to previous doc

**8.7 — In-document search**

When the user presses `/` in doc view:
- Show a search input at the bottom of the doc view (above status bar)
- As the user types, highlight all matches in the rendered lines (use reverse video or a highlight color)
- Jump to the first match
- `n` cycles to the next match, `N` to the previous
- `Esc` closes search, clears highlights
- `Enter` closes search, keeps position

Implement highlighting by re-rendering affected lines with match spans styled differently.

**8.8 — Async doc loading**

When the user presses `Enter` on a search result:
1. Switch to `AppMode::DocView`
2. Show a loading state ("Loading documentation...")
3. Spawn async task: `backend.fetch_doc(url)` or `fetcher.fetch_doc(url)`
4. When loaded, render the doc and display it
5. On error, show error message in the doc view area and allow the user to go back

**8.9 — Tests**

- Snapshot test: render a known `HaddockDoc` to a `TestBackend`, verify the output
- Test scroll math: ensure scroll stays in bounds for docs of various lengths
- Test declaration offset calculation
- Test TOC generation
- Test link extraction and focusing
- Test back navigation: follow 3 links, press back 3 times, verify you're at the original doc

### Verification

After completing Phase 8:
- Press `Enter` on a search result → see beautifully rendered Haddock docs
- Code examples are syntax-highlighted inside bordered boxes
- `>>>` GHCi examples are distinctly styled
- Scrolling is smooth and correct
- `o` shows TOC, jumping works
- `n`/`p` navigate between declarations
- Link following works (at least for module links)
- Back navigation works
- In-doc search highlights matches and cycles through them
- Loading state shows while fetching
- Errors display gracefully

---

## Phase 9: Source Code Viewer

### Goal
Allow users to view the Haskell source code of any declaration, with full syntax highlighting.

### Tasks

**9.1 — Source HTML parser**

Hackage serves source code as HTML at URLs like:
`https://hackage.haskell.org/package/containers-0.6.7/docs/src/Data.Map.Internal.html#lookup`

In `hoogle-core/src/haddock/parser.rs`, add:

```rust
pub fn parse_source_html(html: &str) -> Result<String, ParseError>
```

This extracts the raw Haskell source text from the HTML. The source pages wrap code in `<pre>` tags with hyperlinked identifiers. Strip all HTML tags and extract plain text. Preserve line numbers if present.

**9.2 — Source viewer widget**

In `hoogle-tui/src/ui/source_viewer.rs`, implement a scrollable code viewer:

- Full Haskell syntax highlighting using `hoogle_syntax::highlight_code()`
- Line numbers in a gutter (dim style, right-aligned)
- When entered from a declaration, scroll to the relevant line (use the `#anchor` from the source URL to find the line)
- Same scrolling keybinds as doc viewer

**9.3 — Fetching integration**

When the user presses `s` in the doc viewer on a declaration that has a `source_url`:
1. Show loading state
2. Fetch the source page via `HaddockFetcher::fetch_source()`
3. Parse and render
4. On error, show message

If no `source_url` is available for the declaration, show "Source not available" in status bar.

**9.4 — Tests**

- Parse a real Hackage source HTML fixture
- Verify line number calculation
- Snapshot test of rendered source

### Verification

After completing Phase 9:
- Press `s` on a declaration → see its Haskell source code, syntax-highlighted
- Line numbers visible
- Scrolling works
- `Esc` returns to doc viewer
- Loading and error states work

---

## Phase 10: Search History & Bookmarks

### Goal
Add persistent search history and a bookmark system for saving frequently referenced results.

### Tasks

**10.1 — Persistent history**

Store search history in `~/.local/share/hoogle-tui/history.json`:

```rust
pub struct SearchHistory {
    entries: VecDeque<HistoryEntry>,
    max_size: usize,  // default 500
    path: PathBuf,
}

pub struct HistoryEntry {
    pub query: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub result_count: usize,
}
```

Add `chrono` with `serde` feature to dependencies.

Save on each successful search (deduplicate — if query already exists, move it to front and update timestamp). Load on startup. Save is async (don't block UI).

**10.2 — History popup**

On `Ctrl-r` in search mode, show a popup with search history:
- Most recent first
- Typing filters the list (fuzzy)
- `Enter` selects a history entry, populates search bar, and triggers search
- `Esc` closes
- `Ctrl-d` on a selected entry deletes it from history

**10.3 — Bookmarks**

Store bookmarks in `~/.local/share/hoogle-tui/bookmarks.json`:

```rust
pub struct Bookmark {
    pub name: String,
    pub module: Option<String>,
    pub package: Option<String>,
    pub signature: Option<String>,
    pub doc_url: Option<Url>,
    pub added: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
}
```

- `m` in result list: bookmark the selected result (show brief confirmation in status bar)
- `'` (single quote): open bookmarks popup
  - List of all bookmarks, sorted by most recent
  - Typing filters
  - `Enter` opens the doc for that bookmark
  - `d` deletes the selected bookmark
  - `Esc` closes

**10.4 — Tests**

- History: add entries, verify deduplication, verify max size, verify persistence round-trip
- Bookmarks: add, delete, persistence round-trip

### Verification

After completing Phase 10:
- Search for several things, quit, relaunch → `Ctrl-r` shows previous searches
- Bookmark a result, quit, relaunch → `'` shows the bookmark
- History and bookmark filtering works
- Deletion works
- Files are created in the correct XDG locations

---

## Phase 11: Help Overlay & Mouse Support

### Goal
Add a comprehensive help overlay showing all keybindings, and optional mouse support.

### Tasks

**11.1 — Help overlay**

In `hoogle-tui/src/ui/help_overlay.rs`, render a full-screen (or near-full-screen) overlay when `?` is pressed:

```
╭─ hoogle-tui Help ──────────────────────────────────────────╮
│                                                             │
│  Search                                                     │
│  ────────────────────────────────────────                   │
│  <typing>        Search Hoogle                              │
│  Enter           Focus results                              │
│  Ctrl-r          Search history                             │
│  Ctrl-u          Clear search                               │
│                                                             │
│  Results                                                    │
│  ────────────────────────────────────────                   │
│  j / k           Navigate up/down                           │
│  Enter           Open documentation                         │
│  Tab             Toggle preview                             │
│  ...                                                        │
│                                                             │
│  Press ? or Esc to close                                    │
╰─────────────────────────────────────────────────────────────╯
```

The help overlay should:
- Be scrollable if it's taller than the terminal
- Be dynamically generated from the actual keymap (so custom keybinds show correctly)
- Organized by mode (Search, Results, Doc Viewer, Source Viewer, Global)
- Rendered semi-transparently over the existing UI (use a dimmed background)

**11.2 — Mouse support**

If `config.ui.mouse_enabled` is true (default), enable mouse capture with crossterm and handle:
- **Click on result**: select that result
- **Click on search bar**: focus search bar
- **Scroll wheel in results**: scroll the result list
- **Scroll wheel in doc viewer**: scroll the document
- **Click on link in doc viewer**: follow the link (calculate which link was clicked based on position)
- **Click on preview pane**: focus preview pane for scrolling

Mouse support should be entirely optional and never required.

**11.3 — Tests**

- Verify help overlay generates correct content from the keymap
- Verify mouse click position mapping to result index

### Verification

After completing Phase 11:
- `?` shows a complete, correctly-formatted help overlay
- Mouse clicking on results works
- Scroll wheel works in all panes
- Mouse can be disabled in config

---

## Phase 12: Performance Optimization & Edge Cases

### Goal
Profile and optimize the entire application. Handle all edge cases gracefully. Ensure the tool works across diverse terminal emulators and environments.

### Tasks

**12.1 — Performance profiling**

Use `cargo flamegraph` to profile:
- Startup time (target: <100ms to first paint)
- Search result rendering (target: <10ms per frame with 50 results)
- Haddock doc rendering (target: <50ms for a large module like `Data.Map.Strict`)
- Scrolling smoothness (target: <16ms per frame, i.e., 60fps)

**12.2 — Optimizations to implement**

- **Virtualized result rendering**: only render results visible in the viewport, not all 50
- **Lazy highlighted line generation**: only syntax-highlight results as they scroll into view
- **Pre-rendered doc cache**: after parsing a `HaddockDoc`, cache the `Vec<Line>` in memory (keyed by URL)
- **String interning**: if profiling shows many small string allocations, consider interning module names and package names
- **Async doc pre-fetching**: when user hovers on a result for >300ms, start fetching the doc in the background so it loads instantly when they press Enter

**12.3 — Terminal compatibility**

Test and fix issues on:
- `alacritty` (modern, good baseline)
- `kitty` (common, supports extended features)
- `wezterm` (common on macOS)
- `iTerm2` (macOS)
- `Windows Terminal` (Windows)
- `tmux` / `screen` (multiplexers — test that colors and input work correctly)
- `linux` console (fallback — 16 colors, no unicode box drawing on some)

Implement detection and fallback:
- Check `COLORTERM` env var: if `truecolor` or `24bit`, use full RGB colors; otherwise, map theme colors to 256-color palette
- Check `TERM`: if it doesn't contain `256color`, fall back to 16-color approximations
- For box-drawing characters: always use Unicode (`─`, `│`, `┌`, etc.) — all modern terminals support these

**12.4 — Edge case handling**

- Very small terminal (< 80x24): show a "terminal too small" message instead of crashing
- Very large terminal (> 300 cols): content should still look good, not stretch absurdly
- Long type signatures that exceed terminal width: word-wrap at arrow boundaries (`->`, `=>`)
- Module names with many components: truncate with `…` in tight spaces (`D…M…Strict`)
- Haddock pages that fail to parse: show raw text fallback, log parsing errors
- Network goes down mid-use: show cached content, show "offline" indicator in status bar
- Hoogle database not generated: helpful error message with instructions to run `hoogle generate`
- Empty search results: show helpful message ("No results. Try a broader query or check your Hoogle database.")
- Unicode in search queries and results: handle correctly
- Rapid resize events: debounce re-layout to avoid flicker

**12.5 — Logging**

Ensure all significant events are logged at appropriate levels:
- `error`: backend failures, parse errors, cache corruption
- `warn`: fallback behaviors (stale cache, color degradation)
- `info`: backend selection, cache hits/misses, search timing
- `debug`: individual parse steps, keybind resolution
- `trace`: raw HTTP responses, event loop details

Logs go to file only (`~/.local/share/hoogle-tui/hoogle-tui.log`), never to terminal. Log rotation: keep last 5MB.

**12.6 — Tests**

- Benchmark: search + render 50 results, measure time
- Benchmark: render a large Haddock doc
- Test minimum terminal size handling
- Test color fallback logic

### Verification

After completing Phase 12:
- The app is snappy — no perceptible lag on any operation
- Works in all tested terminals
- Small terminals show a graceful error
- Network failures don't crash the app
- Logs are helpful for debugging

---

## Phase 13: Distribution & Documentation

### Goal
Prepare the project for public release. Write comprehensive documentation. Set up binary distribution.

### Tasks

**13.1 — README.md**

Write a detailed README with:
- Project description and motivation (why this exists, how it compares to `hoogle` CLI and `bhoogle`)
- Screenshot/GIF of the tool in action (create using `vhs` tape file or `asciinema`)
- Installation instructions (all methods)
- Quick start guide
- Feature list
- Configuration reference
- Keybinding reference
- Troubleshooting section
- Contributing guide
- License

**13.2 — Installation methods**

Set up the following distribution channels:

**Cargo:**
- Ensure `Cargo.toml` metadata is complete (description, license, repository, keywords, categories)
- `cargo publish` for all three crates (publish `hoogle-syntax` first, then `hoogle-core`, then `hoogle-tui`)

**GitHub Releases:**
- Create `.github/workflows/release.yml`:
  - Triggered on tag push (`v*`)
  - Build release binaries using `cross` for:
    - `x86_64-unknown-linux-gnu`
    - `x86_64-unknown-linux-musl` (static)
    - `aarch64-unknown-linux-gnu`
    - `x86_64-apple-darwin`
    - `aarch64-apple-darwin`
    - `x86_64-pc-windows-msvc`
  - Create GitHub Release with all binaries attached
  - Generate changelog from conventional commits

**Nix:**
- Create `flake.nix` with:
  - `packages.default` = the built binary
  - `devShells.default` = development shell with Rust toolchain
  - `overlays.default` = overlay for adding to nixpkgs

**Homebrew:**
- Create a `homebrew-tap` repository
- Write formula that downloads the release binary for macOS

**AUR (Arch Linux):**
- Create `PKGBUILD` for `hoogle-tui` and `hoogle-tui-bin` (source and binary packages)

**13.3 — Man page**

Generate a man page from the clap CLI definition using `clap_mangen`. Install it to the correct location. Include in release archives.

**13.4 — Shell completions**

Generate shell completions for bash, zsh, fish, and PowerShell using `clap_complete`. Include in release archives with installation instructions.

**13.5 — CHANGELOG.md**

Write a changelog for v0.1.0 covering all implemented features.

**13.6 — LICENSE**

Use MIT license (compatible with the Haskell ecosystem's common licensing).

**13.7 — VHS tape file**

Create a `demo.tape` file for [vhs](https://github.com/charmbracelet/vhs) that generates an animated GIF showing:
1. Launching the tool
2. Typing a search query
3. Navigating results
4. Opening documentation
5. Scrolling through docs
6. Following a link
7. Viewing source code

### Verification

After completing Phase 13:
- `cargo install hoogle-tui` works from a clean environment
- Release binaries work on all target platforms
- README is comprehensive and accurate
- Man page installs correctly
- Shell completions work
- Demo GIF looks good

---

## Appendix A: Theme Color Reference

These are the exact hex colors to use for each built-in theme's `SemanticToken` mappings.

### Dracula

```
TypeConstructor:  #bd93f9 (purple), bold
TypeVariable:     #f8f8f2 (foreground), italic
Keyword:          #ff79c6 (pink), bold
Operator:         #ff5555 (red)
Punctuation:      #f8f8f2 (foreground)
QualifiedName:    #8be9fd (cyan)
StringLiteral:    #f1fa8c (yellow)
NumericLiteral:   #bd93f9 (purple)
Comment:          #6272a4 (comment gray)
Pragma:           #6272a4, bold
ModuleName:       #8be9fd (cyan)
PackageName:      #6272a4, italic
DocText:          #f8f8f2
DocCode:          #f1fa8c
DocHeading:       #bd93f9, bold
DocLink:          #8be9fd, underline
SearchInput:      #f8f8f2
StatusBar:        bg=#44475a, fg=#f8f8f2
Selected:         bg=#44475a
Cursor:           bg=#6272a4
Border:           #6272a4
Error:            #ff5555
Spinner:          #bd93f9
```

### Catppuccin Mocha

```
TypeConstructor:  #cba6f7 (mauve), bold
TypeVariable:     #cdd6f4 (text), italic
Keyword:          #f38ba8 (red), bold
Operator:         #fab387 (peach)
Punctuation:      #cdd6f4
QualifiedName:    #89dceb (sky)
StringLiteral:    #a6e3a1 (green)
NumericLiteral:   #fab387 (peach)
Comment:          #6c7086 (overlay0)
Pragma:           #6c7086, bold
ModuleName:       #89dceb
PackageName:      #6c7086, italic
DocText:          #cdd6f4
DocCode:          #a6e3a1
DocHeading:       #cba6f7, bold
DocLink:          #89b4fa (blue), underline
SearchInput:      #cdd6f4
StatusBar:        bg=#313244, fg=#cdd6f4
Selected:         bg=#313244
Cursor:           bg=#45475a
Border:           #6c7086
Error:            #f38ba8
Spinner:          #cba6f7
```

### Gruvbox Dark

```
TypeConstructor:  #d3869b (purple), bold
TypeVariable:     #ebdbb2 (fg), italic
Keyword:          #fb4934 (red), bold
Operator:         #fe8019 (orange)
Punctuation:      #ebdbb2
QualifiedName:    #83a598 (blue)
StringLiteral:    #b8bb26 (green)
NumericLiteral:   #d3869b
Comment:          #928374 (gray)
Pragma:           #928374, bold
ModuleName:       #83a598
PackageName:      #928374, italic
DocText:          #ebdbb2
DocCode:          #b8bb26
DocHeading:       #fabd2f (yellow), bold
DocLink:          #83a598, underline
SearchInput:      #ebdbb2
StatusBar:        bg=#3c3836, fg=#ebdbb2
Selected:         bg=#3c3836
Cursor:           bg=#504945
Border:           #928374
Error:            #fb4934
Spinner:          #fabd2f
```

Use similar derivation for Nord, Solarized Dark, and Monokai using their respective official palettes.

---

## Appendix B: Hoogle JSON Output Examples

### Function result

```json
{
  "url": "https://hackage.haskell.org/package/containers-0.6.7/docs/Data-Map-Strict.html#v:lookup",
  "module": {"name": "Data.Map.Strict", "url": "https://hackage.haskell.org/package/containers-0.6.7/docs/Data-Map-Strict.html"},
  "package": {"name": "containers", "url": "https://hackage.haskell.org/package/containers"},
  "item": "<span class=name><s0>lookup</s0></span> :: <a>Ord</a> k =&gt; k -&gt; <a>Map</a> k a -&gt; <a>Maybe</a> a",
  "type": "",
  "docs": "O(log n). Look up the value at a key in the map.\nThe function will return the corresponding value as (Just value),\nor Nothing if the key isn't in the map."
}
```

### Type result

```json
{
  "url": "https://hackage.haskell.org/package/containers-0.6.7/docs/Data-Map-Strict.html#t:Map",
  "module": {"name": "Data.Map.Strict", "url": "..."},
  "package": {"name": "containers", "url": "..."},
  "item": "<span class=name><s0>data</s0> <s0>Map</s0></span> k a",
  "type": "",
  "docs": "A Map from keys k to values a.\n..."
}
```

### Class result

```json
{
  "url": "https://hackage.haskell.org/package/base-4.18.0.0/docs/Control-Monad.html#t:Monad",
  "module": {"name": "Control.Monad", "url": "..."},
  "package": {"name": "base", "url": "..."},
  "item": "<span class=name><s0>class</s0> <a>Applicative</a> m =&gt; <s0>Monad</s0></span> m",
  "type": "",
  "docs": "The Monad class defines the basic operations over a monad..."
}
```

Use these as the basis for parsing test fixtures.

---

## Appendix C: Acceptance Criteria Summary

The tool is considered complete when ALL of the following are true:

1. `cargo install hoogle-tui` installs successfully
2. Launching with no args shows an empty search bar, ready for input
3. Launching with `hoogle-tui "map"` shows search results immediately
4. Type signatures are syntax-highlighted with distinguishable colors for constructors, variables, operators, keywords
5. Results are navigable with keyboard; selected item is visually distinct
6. Preview pane shows short docs for the selected result
7. Pressing Enter on a result shows the full Haddock documentation
8. Documentation includes syntax-highlighted code blocks with bordered boxes
9. GHCi examples (`>>>`) are distinctly rendered
10. Table of contents popup lists all declarations and supports filtering
11. Links in documentation can be followed; back navigation works
12. Source code can be viewed with syntax highlighting
13. Search history persists across sessions
14. Bookmarks persist across sessions
15. At least 6 color themes are available and selectable
16. Help overlay shows all keybindings
17. The tool works with both local Hoogle and the web API
18. The terminal is always cleanly restored on exit, even on panic
19. Performance: <200ms from keystroke to results displayed
20. The tool works in alacritty, kitty, iTerm2, and tmux without issues