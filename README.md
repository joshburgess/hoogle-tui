# hoogle-tui

A terminal user interface for [Hoogle](https://hoogle.haskell.org/), Haskell's type-aware search engine.

Browse Haskell APIs, read Haddock documentation, and view source code — all from your terminal.

## Features

### Search
- **Live search** with debounced input and syntax-highlighted type signatures
- **Dual backend**: works with local `hoogle` CLI or the web API (auto-detected)
- **Type signature detection**: automatically detects `->` / `=>` queries and shows a `[type]` indicator
- **Package scoping**: restrict search to specific packages (`Ctrl-p`) — uses Hoogle's `+pkg` syntax
- **Tab completion**: press `Tab` in the search bar to complete from current result names
- **Search history**: persistent across sessions, browse with `Ctrl-r`
- **Cabal/Stack project awareness**: auto-detects your Haskell project and scopes search to its dependencies

### Browsing Results
- **Compact/expanded toggle**: `v` switches between 1-line (shows 3x more) and 3-line result display
- **Module grouping**: `w` groups results by module with collapsible headers
- **Filtering**: `f` filters by kind (function, type, class, module, etc.)
- **Sorting**: `s` sorts by relevance, name, package, or module
- **Fuzzy filter**: type letters in result mode to narrow results client-side
- **Preview pane**: scrollable split-pane preview with syntax-highlighted code examples

### Multi-Select and Comparison
- **Multi-select**: `x` toggles selection on results, auto-advances for rapid picking
- **Batch import yanking**: `I` copies all selected results as import statements to clipboard
- **Pinned results**: `P` pins a result to a comparison panel below the preview pane
- **Module browser**: `Ctrl-m` opens a hierarchical module tree built from search results

### Documentation
- **Haddock doc viewer**: full module documentation with syntax-highlighted code blocks, GHCi examples (`>>>`), headers, lists, and notes
- **Table of contents**: `o` opens a filterable declaration list for quick navigation
- **Declaration navigation**: `n`/`p` jumps between declarations
- **In-document search**: `/` with incremental highlighting and match counter
- **Link following**: `Tab` cycles through links, `Enter` follows, `Backspace` goes back
- **Source code viewer**: `s` opens Haskell source with syntax highlighting and line numbers

### Copy and Integration
- **Copy menu**: `c` opens a popup with 7 copy options:
  - Type signature
  - Qualified name (e.g., `Data.Map.Strict.lookup`)
  - Import statement
  - Hackage URL
  - GHCi `:type` command
  - GHCi `:info` command
  - Deep link with declaration anchor
- **Quick yank**: `y` (signature), `Y` (import), `Ctrl-y` (URL)
- **Open in browser**: `Ctrl-o` opens the Hackage page in your system browser
- **Export session**: `Ctrl-e` exports search results and viewed docs to a markdown file

### UI and Themes
- **6 built-in themes**: Dracula, Catppuccin Mocha, Gruvbox Dark, Solarized Dark, Monokai, Nord
- **Live theme switching**: `Ctrl-t` changes theme without restarting (re-renders docs)
- **Custom themes** via TOML files
- **Mouse support**: click to focus panels, click results to select, double-click to open docs, scroll wheel in all panels
- **Vim-style keybindings** throughout, all overridable via config
- **Offline detection**: shows `OFFLINE` badge on network failures, clears on recovery
- **Bookmarks**: `m` to bookmark, `'` to browse, persistent across sessions
- **Resize debouncing**: smooth terminal resize without flicker

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

For the local backend (recommended):

```sh
cabal install hoogle
hoogle generate
```

If Hoogle is not installed, `hoogle-tui` automatically falls back to the web API at hoogle.haskell.org.

## Quick Start

```sh
# Launch with empty search bar
hoogle-tui

# Search by name
hoogle-tui "map"

# Search by type signature
hoogle-tui "a -> a"

# Force web backend (no local hoogle needed)
hoogle-tui --backend web
```

## Navigation Model

The app uses a modal drill-down flow:

```
Search  -->  Results  -->  Docs  -->  Source
        <--          <--       <--
       Enter        Enter     s
        Esc          Esc     Esc
```

- **Enter** goes deeper (forward)
- **Esc** goes back (shallower)
- **/** jumps to search from any mode
- **F1** or **Ctrl-/** opens help from any mode (including while typing in search)

## Keybindings

### Global

| Key | Action |
|-----|--------|
| `F1` / `Ctrl-/` | Help (works everywhere, including search bar) |
| `?` | Help (all modes except search bar) |
| `Ctrl-c` | Quit immediately |
| `Ctrl-l` | Force redraw |

### Search Bar

| Key | Action |
|-----|--------|
| `<typing>` | Live search (debounced) |
| `Enter` | Move focus to results |
| `Tab` | Complete from result names |
| `Ctrl-r` | Search history |
| `Ctrl-u` | Clear search |
| `Esc` | Clear / quit if empty |

### Result List

| Key | Action |
|-----|--------|
| `j`/`k` or arrows | Navigate up/down |
| `g` / `G` | First / last result |
| `Enter` | Open Haddock docs |
| `Tab` | Toggle preview pane |
| `Space` | Scroll preview pane |
| `/` | Focus search bar |
| `v` | Toggle compact/expanded view |
| `w` | Toggle group by module |
| `f` | Filter by kind |
| `s` | Sort results |
| `c` | Copy menu (7 options) |
| `y` / `Y` / `Ctrl-y` | Quick yank: signature / import / URL |
| `T` | Copy GHCi `:type` command |
| `D` | Copy GHCi `:info` command |
| `x` | Multi-select toggle |
| `I` | Yank all selected imports |
| `P` | Pin result for comparison |
| `Ctrl-x` | Clear all pins |
| `Ctrl-m` | Module browser |
| `Ctrl-o` | Open in browser |
| `Ctrl-p` | Set package scope |
| `Ctrl-t` | Switch theme |
| `Ctrl-e` | Export session to markdown |
| `m` | Bookmark result |
| `'` | Open bookmarks |
| `q` | Quit |

### Doc Viewer

| Key | Action |
|-----|--------|
| `j`/`k` or arrows | Scroll line |
| `d`/`u` or `Ctrl-d`/`Ctrl-u` | Scroll half page |
| `f`/`b` or `Ctrl-f`/`Ctrl-b` | Scroll full page |
| `g` / `G` | Top / bottom |
| `o` | Table of contents |
| `n` / `p` | Next / prev declaration |
| `s` | View source |
| `/` | Search within document |
| `Tab` | Cycle through links |
| `Enter` | Follow focused link |
| `Backspace` | Navigate back |
| `y` | Copy deep link to declaration |
| `T` / `D` | Copy GHCi `:type` / `:info` command |
| `Ctrl-o` | Open in browser |
| `Esc` | Back to results |

### Source Viewer

| Key | Action |
|-----|--------|
| `j`/`k` or arrows | Scroll |
| `g` / `G` | Top / bottom |
| `y` | Yank source |
| `Esc` | Back to doc viewer |

### Mouse

| Action | Effect |
|--------|--------|
| Click search bar | Focus search mode |
| Click result | Select that result |
| Double-click result | Open docs |
| Scroll wheel | Scroll the panel under cursor |
| Click while popup open | Close popup |

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

[keybinds]
# Override any keybinding:
# scroll_down = "ctrl-n"
# scroll_up = "ctrl-p"
```

See `config/default.toml` for all options with inline documentation.

## Data Files

| Path | Purpose |
|------|---------|
| `~/.config/hoogle-tui/config.toml` | Configuration |
| `~/.cache/hoogle-tui/` | Haddock page cache |
| `~/.local/share/hoogle-tui/history.json` | Search history |
| `~/.local/share/hoogle-tui/bookmarks.json` | Bookmarks |
| `~/.local/share/hoogle-tui/hoogle-tui.log` | Debug logs |

## Themes

Built-in: `dracula` (default), `catppuccin_mocha`, `gruvbox_dark`, `solarized_dark`, `monokai`, `nord`.

Custom themes can be defined in TOML:

```toml
name = "my_theme"

[styles.keyword]
fg = "#ff79c6"
modifiers = ["bold"]

[styles.type_constructor]
fg = "#bd93f9"
modifiers = ["bold"]
```

See `themes/` for complete examples covering all 23 semantic token types.

## Architecture

Three-crate Cargo workspace:

- **hoogle-core**: Search backends (local CLI + web API), Haddock HTML-to-AST parser, disk cache, configuration
- **hoogle-syntax**: Haskell tokenizer (byte-indexed), syntax highlighting, array-backed theme system
- **hoogle-tui**: Terminal UI (ratatui + crossterm), modal navigation, async event loop, 18 widget modules

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
      --completions <SHELL>    Generate shell completions (bash, zsh, fish, powershell)
  -h, --help                   Print help
  -V, --version                Print version
```

## Troubleshooting

**"hoogle binary not found"**: Install hoogle with `cabal install hoogle`, or use `--backend web`.

**"No results"**: Run `hoogle generate` to build the local search database.

**Broken terminal after crash**: Run `reset` in your shell. The app installs a panic hook to restore the terminal, but if the process is killed with SIGKILL this can't run.

**Logs**: Check `~/.local/share/hoogle-tui/hoogle-tui.log` — set `--log-level debug` for verbose output.

**Offline mode**: If the web backend can't connect, the status bar shows `OFFLINE`. Results from the disk cache are still available.

## License

MIT
