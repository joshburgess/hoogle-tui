# Current State

This document tracks the implementation state of the workspace.

## Crates

- `hoogle-core`: shared models, config loading, cache management, local/web search backends, and Haddock fetching/parsing.
- `hoogle-syntax`: Haskell signature and source tokenization, highlighting, and theme support.
- `hoogle-tui`: CLI, terminal event loop, app state, search/results/docs/source views, popups, bookmarks, history, export, and project detection.

## Implemented

- Local Hoogle CLI and web search backend auto-detection.
- Debounced live search, package scoping, project dependency detection, fuzzy result filtering, sorting, grouping, and module browsing.
- Search pagination through the shared backend contract. The web backend uses Hoogle's `start` parameter. Hoogle 5.0.19.0 CLI has no offset/start flag in `hoogle search --help`, verified on May 19, 2026, so the local backend over-fetches and slices.
- Haddock HTML fetch/cache/parse pipeline through `HaddockFetcher`.
- Haddock parser fixture coverage includes representative value and type declaration pages, constructor and record-field documentation, source links, since annotations, warning blocks, tables, details, definition lists, module links, anchor links, and rich inline markup.
- Documentation and source viewers with navigation, in-document search, link following, table of contents, and source fetching.
- Copy/yank flows, multi-select import copying, pinned results, export, history, bookmarks, themes, help, mouse support, and shell completions.

## Known Gaps

- Backend integration tests for local Hoogle and the public web API are ignored by default, and can be run through the manual `Backend smoke tests` CI job.

## Verification Coverage

- Ignored local and web backend smoke tests passed locally on May 19, 2026, against Hoogle 5.0.19.0 and the public web API.
- Hoogle result parsing now has deterministic coverage for functions, data, classes, modules, packages, array output, NDJSON output, noisy non-JSON local CLI output, invalid NDJSON errors, and hyphenated package versions.
- Local backend command construction has deterministic coverage for offset-adjusted fetch counts, type-signature queries, and custom database paths.
- Backend factory selection has deterministic coverage for explicit web mode, missing local Hoogle errors, and auto-mode fallback to the web backend.
- Render-level `TestBackend` coverage exercises result list, status bar, source viewer, and filter popup output without launching a real terminal.
- Checked-in render golden snapshots cover result list, status bar, source viewer, and filter popup output.
- CLI integration tests cover help, version, shell completions, and invalid flag handling.
- `scripts/verify.sh` runs formatting, unit tests, clippy, shipped-code panic checks, prose style scans, and app-module import checks. CI runs the same script. Set `RUN_SMOKE_TESTS=1` to include ignored live backend smoke tests.
- `app.rs` is now mostly state and message type definitions. Construction and initial-query setup live in `app_init.rs`, action dispatch lives in `app_actions.rs`, popup handling lives in `app_popups.rs`, result-mode helpers live in `app_results.rs`, doc/source helpers live in `app_docs.rs`, input handling lives in `app_input.rs`, mouse handling lives in `app_mouse.rs`, navigation helpers live in `app_navigation.rs`, rendering lives in `app_render.rs`, tick/async response handling lives in `app_runtime.rs`, search/filter/pagination helpers live in `app_search.rs`, and clipboard/bookmark/browser/project commands live in `app_commands.rs`.

## Suggested Next Work

1. Re-check local Hoogle offset support when upgrading Hoogle beyond 5.0.19.0.
2. Extend parser output if constructor/record-field subdocumentation should become separate navigable UI entries instead of content within the parent declaration.
3. Keep the ignored backend smoke tests as the live end-to-end check for local Hoogle and the public web API.
