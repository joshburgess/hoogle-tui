# Changelog

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
