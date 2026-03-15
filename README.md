# hoogle-tui

A terminal user interface for [Hoogle](https://hoogle.haskell.org/), Haskell's type-aware search engine.

Browse Haskell APIs, read Haddock documentation, and view source code — all from your terminal.

## Features

- **Live search** with debounced input and syntax-highlighted type signatures
- **Dual backend**: works with local `hoogle` CLI or the web API (auto-detected)
- **Haddock doc viewer**: read full module documentation in the terminal with syntax-highlighted code blocks, GHCi examples, lists, and headers
- **Source code viewer**: view Haskell source with syntax highlighting and line numbers
- **Table of contents**: jump between declarations in a module
- **Preview pane**: split-pane preview of the selected result's documentation
- **6 built-in color themes**: Dracula, Catppuccin Mocha, Gruvbox Dark, Solarized Dark, Monokai, Nord (plus custom TOML themes)
- **Result filtering & sorting**: filter by kind (function, type, class, etc.) and sort by name, package, or module
- **Clipboard integration**: yank type signatures, import statements, or Hackage URLs
- **Persistent search history & bookmarks**
- **Mouse support**: scroll wheel navigation in all views
- **Vim-style keybindings**: j/k navigation, g/G jumps, half/full page scrolling

## Installation

### From source (cargo)

```sh
cargo install hoogle-tui
```

### From source (git)

```sh
git clone https://github.com/joshburgess/hoogle-tui
cd hoogle-tui
cargo install --path crates/hoogle-tui
```

### Prerequisites

For the local backend (recommended), you need Hoogle installed:

```sh
cabal install hoogle
hoogle generate
```

If Hoogle is not installed, `hoogle-tui` automatically falls back to the web API at hoogle.haskell.org.

## Quick Start

```sh
# Launch with empty search bar
hoogle-tui

# Launch with an initial query
hoogle-tui "map"

# Search by type signature
hoogle-tui "a -> a"

# Force web backend (no local hoogle needed)
hoogle-tui --backend web
```

## Keybindings

### Search Bar

| Key | Action |
|-----|--------|
| `<typing>` | Live search (debounced) |
| `Enter` | Move focus to results |
| `Ctrl-r` | Search history |
| `Ctrl-u` | Clear search |
| `Esc` | Clear / quit if empty |

### Result List

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate up/down |
| `g` / `G` | First / last result |
| `Enter` | Open Haddock docs |
| `Tab` | Toggle preview pane |
| `/` | Focus search bar |
| `f` | Filter by kind |
| `s` | Sort results |
| `y` | Yank type signature |
| `Y` | Yank import statement |
| `Ctrl-y` | Yank Hackage URL |
| `m` | Bookmark result |
| `'` | Open bookmarks |
| `?` | Help |
| `q` | Quit |

### Doc Viewer

| Key | Action |
|-----|--------|
| `j` / `k` | Scroll line |
| `d` / `u` | Scroll half page |
| `f` / `b` | Scroll full page |
| `g` / `G` | Top / bottom |
| `o` | Table of contents |
| `n` / `p` | Next / prev declaration |
| `s` | View source |
| `Esc` | Back to results |

### Source Viewer

| Key | Action |
|-----|--------|
| `j` / `k` | Scroll |
| `g` / `G` | Top / bottom |
| `Esc` | Back to doc viewer |

## Configuration

Config file: `~/.config/hoogle-tui/config.toml`

```toml
# Color theme
theme = "dracula"  # dracula, catppuccin_mocha, gruvbox_dark, solarized_dark, monokai, nord

[backend]
mode = "auto"       # auto, local, web
timeout_secs = 5

[ui]
max_results = 50
preview_enabled = true
layout = "auto"     # auto, vertical, horizontal
mouse_enabled = true
debounce_ms = 150

[cache]
enabled = true
ttl_hours = 168     # 7 days
max_size_mb = 500
```

See `config/default.toml` for all options with documentation.

## Themes

Built-in themes: `dracula` (default), `catppuccin_mocha`, `gruvbox_dark`, `solarized_dark`, `monokai`, `nord`.

Custom themes can be defined in TOML. See `themes/` for examples.

## Architecture

Three-crate workspace:

- **hoogle-core**: Search backends (local CLI + web API), Haddock HTML parser, disk cache, configuration
- **hoogle-syntax**: Haskell tokenizer, syntax highlighting, theme system
- **hoogle-tui**: Terminal UI (ratatui + crossterm), keybindings, widgets

## CLI Options

```
Usage: hoogle-tui [OPTIONS] [QUERY]

Arguments:
  [QUERY]  Initial search query

Options:
  -b, --backend <BACKEND>      Backend: auto, local, web [default: auto]
  -d, --database <DATABASE>    Path to hoogle database
  -t, --theme <THEME>          Color theme name
  -c, --config <CONFIG>        Config file path
      --no-cache               Disable caching
      --max-results <N>        Max results
      --log-level <LEVEL>      Log level [default: warn]
  -h, --help                   Print help
  -V, --version                Print version
```

## Troubleshooting

**"hoogle binary not found"**: Install hoogle with `cabal install hoogle`, or use `--backend web`.

**"No results"**: Run `hoogle generate` to build the local search database.

**Broken terminal after crash**: Run `reset` in your shell.

**Logs**: Check `~/.local/share/hoogle-tui/hoogle-tui.log`

## License

MIT
