# Changelog

## Unreleased

### New Features

#### Search and Navigation
- Type signature detection: status bar shows `[type]` badge when query contains `->` or `=>`
- Package scoping (`Ctrl-p`): restrict search to specific packages using Hoogle's `+pkg` syntax
- Tab completion: press `Tab` in search bar to cycle through matching result names
- Cabal/Stack project awareness: auto-detects project on startup and scopes search to dependencies
- Module browser (`Ctrl-m`): hierarchical tree view of modules with expand/collapse and filtering
- Compact/expanded display toggle (`v`): switch between 1-line and 3-line result display
- Result grouping by module (`w`): visual module headers with horizontal rules

#### Multi-Select and Comparison
- Multi-select (`x`): toggle selection on results with `[x]`/`[ ]` markers
- Batch import yank (`I`): copy all selected results as import statements
- Pinned results (`P`): pin results to a comparison panel below the preview pane
- Clear pins (`Ctrl-x`)

#### Copy and Integration
- Copy menu (`c`): popup with 7 options — signature, qualified name, import, URL, `:type`, `:info`, deep link
- GHCi commands (`T`/`D`): copy `:type Module.name` or `:info Module.name` to clipboard
- Haddock deep linking (`y` in doc view): copy URL with `#v:name` or `#t:name` anchor
- Open in browser (`Ctrl-o`): opens Hackage URL in system browser
- Export session (`Ctrl-e`): export results and viewed docs to markdown file

#### UI and Themes
- Live theme switching (`Ctrl-t`): change theme without restarting, re-renders loaded docs
- Scrollable preview pane: `Space` scrolls, scroll wheel works, scrollbar shown
- Syntax-highlighted code examples in preview pane (GHCi prompts and indented code)
- Mouse support: click to focus panels, click results to select, double-click to open docs
- Mode indicator in status bar (`SEARCH`, `RESULTS`, `DOCS`, `SOURCE`, `HELP`)
- Search syntax cheatsheet in search bar bottom border
- Offline detection with persistent `OFFLINE` badge
- Resize debouncing (50ms coalesce window)
- `F1` and `Ctrl-/` open help from any mode including search bar

### Performance
- Theme lookup: `HashMap` replaced with array indexed by enum discriminant
- Tokenizers: `Vec<char>` replaced with direct byte indexing on `&str` slices
- `token_text` returns `Cow<str>` to avoid cloning string-variant tokens
- Doc search: pre-computed lowercased line text eliminates per-keystroke allocations
- Search match lookup: `Vec::contains` replaced with `binary_search`
- Result list: cached module/package display strings, theme styles hoisted out of per-result loop
- Search bar: `&mut TextArea` passed instead of cloning every frame
- HTML parsing: single `strip_html` pass per result instead of redundant regex calls
- `apply_filter_and_sort`: avoids cloning when no filter or sort is active

### Documentation
- Rustdoc comments on all public types in hoogle-core and hoogle-syntax
- Comprehensive README with all keybindings, features, and configuration
- Updated CLAUDE.md with complete feature summary and project layout

### Tests
- 44 new tests: export, project detection, clipboard, layout, module browser

## v0.1.0 — Initial Release

### Features

- Live search with debounced input against local Hoogle CLI or web API
- Syntax-highlighted Haskell type signatures in search results
- Split-pane preview of selected result documentation
- Full Haddock documentation viewer with:
  - Syntax-highlighted code blocks with bordered boxes
  - GHCi example prompts (`>>>`) distinctly styled
  - Headers, lists, paragraphs, notes, and horizontal rules
  - Table of contents popup with declaration list
  - Declaration navigation (next/prev)
  - Scrollbar
- Source code viewer with line numbers and syntax highlighting
- 6 built-in color themes: Dracula, Catppuccin Mocha, Gruvbox Dark, Solarized Dark, Monokai, Nord
- Custom theme support via TOML files
- Result filtering by kind (function, type, class, module, etc.)
- Result sorting by relevance, package, module, or name
- Clipboard integration: yank signatures, import statements, and URLs
- Persistent search history (Ctrl-r to browse)
- Persistent bookmarks (m to bookmark, ' to browse)
- Comprehensive help overlay (?)
- Mouse scroll wheel support (configurable)
- Vim-style keybindings throughout
- Auto-detection of local hoogle with web API fallback
- Disk cache for Haddock pages with TTL and stale fallback
- Configurable via TOML config file
- Clean terminal restoration on exit and panic
- Small terminal guard with friendly message
