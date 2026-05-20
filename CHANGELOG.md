# Changelog

## Unreleased

### Fixed
- Removed a stale backend documentation-fetching method that had been superseded by `HaddockFetcher`.
- Replaced the broken load-more path with backend pagination.
- Cabal project detection now ignores dependency names that only appear after inline comments.
- Removed a non-test panic path from Haddock CSS selector handling.
- Removed non-test panic paths from Hoogle result parser regex initialization.
- Fixed duplicate `<summary>` text when parsing Haddock `<details>` blocks while preserving body code blocks.
- Haddock module-link detection now avoids byte-index suffix stripping.
- Cleared clippy warnings across the workspace.

### Changed
- Aligned the README CLI option synopsis with the generated command help.
- CLI integration tests now guard the README CLI option synopsis against generated help drift.
- CLI integration tests now exercise every supported shell completion generator.
- Bookmark and history writes now report persistence errors instead of silently ignoring them.
- Bookmark and history loads now distinguish missing, unreadable, and corrupt persistence files.
- Cache TTL and size limit calculations now saturate instead of overflowing on extreme config values.
- CI now runs the same `scripts/verify.sh` workflow used for local verification.
- Release verification now runs the same `scripts/verify.sh` workflow used for CI and local checks.
- Verification now rejects mismatched workspace, Nix flake, and Homebrew formula versions.
- Verification now rejects crate package and internal path dependency versions that drift from the workspace version.
- Package-version verification now has deterministic regression coverage.
- Updated the packaged Homebrew formula version to match the workspace version.
- Homebrew installs now generate shell completions from the installed binary.
- Keybinding overrides now accept uppercase modifier prefixes and named keys.
- Verification now checks shell and Ruby script syntax, including the packaged Homebrew formula.
- Verification now runs `actionlint` when it is available locally.
- Ignored backend smoke tests now include explicit ignore reasons.
- Documentation navigation now tracks the current page URL so internal-link back navigation returns to the previous document.
- Table-of-contents and history popups now accept typed filters instead of carrying unused filtering state.
- Pinned result panels now handle mouse-wheel scrolling independently from the preview pane.
- Pressing `P` on an already pinned result now unpins it.
- User-facing keybinding docs now list `M` for the module browser, matching the runtime keymap.
- Project detection messages now include the detected project root.
- Source views now jump to the loaded declaration when its name appears in the source.
- Source view `y` now copies the loaded source text as documented.
- Markdown session exports now escape table cells so pipes and newlines do not corrupt result rows.
- Empty fuzzy result filters no longer leave the first hidden result selectable.
- Replacing result sets now clears stale multi-select state.
- Doc view `Ctrl-o` now opens the current document URL after internal navigation.
- Doc view deep-link copying now uses the current document URL after internal navigation.
- Disk cache writes now enforce the configured maximum cache size.
- Help overlay scroll bounds now derive from the actual help content instead of a fixed placeholder count.
- UI truncation now preserves UTF-8 character boundaries in rendered lists and documentation tables.
- UI truncation now accounts for terminal display width when shortening wide Unicode text.
- Preview and documentation wrapping now account for terminal display width.
- Result-list and status-bar padding now account for terminal display width.
- Documentation table column widths now account for terminal display width.
- Popup and result-list truncation limits now account for terminal display width.
- Documentation and help underlines now account for terminal display width.
- Verification now rejects `dbg!`, `todo!`, and `unimplemented!` in shipped Rust code alongside panic-prone calls.
- Shipped-code panic scanning now resumes correctly after next-line-braced `#[cfg(test)] mod tests` blocks.
- Verification now rejects `#[allow(dead_code)]` in shipped Rust code.
- Extracted shared popup centering layout for TUI popups.
- Moved TUI action dispatch into a dedicated `app_actions.rs` module.
- Moved TUI app construction and initial-query setup into a dedicated `app_init.rs` module.
- Split popup action handling out of the main TUI action dispatcher and centralized popup cleanup.
- Extracted common TUI back-navigation and scroll/move handling from the main action dispatcher.
- Extracted doc-view navigation and current-declaration lookup helpers from the main TUI dispatcher.
- Extracted result-mode action helpers for pinning, multi-select, grouping, module browsing, export, and popup opening.
- Moved popup action handling and popup cleanup into a dedicated `app_popups.rs` module.
- Moved result-mode helper methods into a dedicated `app_results.rs` module.
- Moved doc-view, TOC, and source-loading helper methods into a dedicated `app_docs.rs` module.
- Moved mouse click and scroll handling into a dedicated `app_mouse.rs` module.
- Moved back, selection, and scroll navigation helpers into a dedicated `app_navigation.rs` module.
- Moved TUI draw orchestration into a dedicated `app_render.rs` module.
- Moved tick and async response handling into a dedicated `app_runtime.rs` module.
- Moved search, filter, sort, and pagination helper methods into a dedicated `app_search.rs` module.
- Moved clipboard, bookmark, browser, and project-detection command helpers into a dedicated `app_commands.rs` module.
- Moved search, fuzzy-filter, and doc-search input handlers into a dedicated `app_input.rs` module.
- Moved app helper tests into a dedicated `app_tests.rs` module.
- Tightened extracted TUI app helper methods to crate-private visibility.
- Source view status hints now advertise help and quit bindings.
- Results-mode preview scroll-up now scrolls the preview pane instead of moving result selection when previews are enabled.
- Half-page and page scroll actions now target the active scrollable view instead of always mutating the document view.
- Help now closes back to the mode it was opened from, and edge navigation scrolls help instead of moving hidden results.
- Direct view switches now clear stale help restore state.
- Clearing pins now reports when there are no pins to clear instead of claiming success.
- Bookmark and yank commands now explain unavailable result, signature, import, and URL cases instead of silently doing nothing.
- Pin, qualified-name yank, and open-document actions now report when no result is selected.
- User-facing keybinding docs now describe `/` as result-mode search focus and doc-mode document search.
- Keybinding overrides now accept `delete_entry` and `redraw` action names.
- CLI backend selection now rejects values outside `auto`, `local`, and `web`.
- CLI `--backend auto` now restores auto mode when overriding a config file.
- CLI log-level selection now rejects values outside `error`, `warn`, `info`, `debug`, and `trace`.
- CLI `--max-results` now rejects zero.
- The configured `ui.layout` mode now controls result and preview layout instead of always using auto layout.
- Opening the module browser without available modules now reports the empty state instead of showing an unusable popup.
- Clearing search now also clears stale results, counts, pagination state, completion state, type-search badges, and pending status messages.

### Added
- Restored `L` to load more results.
- Added a manual CI smoke-test job for local and web backend integration tests.
- Added CLI integration tests for help, version, completions, and invalid flags.
- Added checked-in render golden snapshots for key TUI surfaces.
- Added wide Unicode render regression coverage for compact results and documentation tables.
- Added wide Unicode render regression coverage for bookmarks and table-of-contents popups.
- Added tiny-terminal render regression coverage for TUI popups.
- Added `scripts/check-rust-panics.rb` to reject `.unwrap()` and `.expect()` in shipped Rust code.
- Added regression tests for the shipped-code panic scanner.
- Added `scripts/verify.sh` for one-command local verification.
- Added render coverage for Source View status-bar help and quit hints.

### Documentation
- Added current implementation state notes for contributors.
- Removed the obsolete implementation plan in favor of the current state document.
- Removed stale release wording from shipped `0.1.1` changes.
- Documented verified Hoogle 5.0.19.0 local pagination behavior.
- Documented the May 19, 2026 local verification of ignored backend smoke tests.
- Documented the `--generate` CLI option.

### Tests
- Added a Hackage-like Haddock parser fixture covering metadata, declarations, source links, tables, details, and definition lists.
- Added an offline Haddock HTML fixture file for parser regression coverage.
- Added Haddock fixture coverage for constructor and record-field documentation.
- Added a Hackage-like type declaration fixture covering warning blocks, module links, anchor links, math, emphasis, bold text, and subscript/superscript inline markup.
- Added render-level `TestBackend` coverage for result list, status bar, source viewer, and filter popup surfaces.
- Added shared popup layout tests.
- Added regression coverage for Haddock `<details>` blocks so summaries render once and body paragraphs remain distinct.
- Added deterministic unit coverage for local backend pagination slicing.
- Added deterministic unit coverage for local backend Hoogle command arguments.
- Added deterministic unit coverage for backend factory selection and auto-mode fallback.
- Added deterministic unit coverage for module/package Hoogle result parsing, noisy CLI output, invalid NDJSON, and hyphenated package versions.
- Added direct app helper tests for extracted search, popup, result, and pin behavior.
- Added direct app helper tests for doc-view declaration lookup and navigation behavior.
- Added direct app helper tests for doc link following and source-view loading state.

## v0.1.1

### New Features

#### Search and Navigation
- Type signature detection: status bar shows `[type]` badge when query contains `->` or `=>`
- Package scoping (`Ctrl-p`): restrict search to specific packages using Hoogle's `+pkg` syntax
- Tab completion: press `Tab` in search bar to cycle through matching result names
- Cabal/Stack project awareness: auto-detects project on startup and scopes search to dependencies
- Module browser (`M`): hierarchical tree view of modules with expand/collapse and filtering
- Compact/expanded display toggle (`v`): switch between 1-line and 3-line result display
- Result grouping by module (`w`): visual module headers with horizontal rules

#### Multi-Select and Comparison
- Multi-select (`x`): toggle selection on results with `[x]`/`[ ]` markers
- Batch import yank (`I`): copy all selected results as import statements
- Pinned results (`P`): pin or unpin results in a comparison panel below the preview pane
- Clear pins (`Ctrl-x`)

#### Copy and Integration
- Copy menu (`c`): popup with 7 options: signature, qualified name, import, URL, `:type`, `:info`, deep link
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

## v0.1.0: Initial Release

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
