use std::sync::Arc;

use hoogle_core::backend::{BackendError, HoogleBackend};
use hoogle_core::cache::DiskCache;
use hoogle_core::config::Config;
use hoogle_core::haddock::fetcher::HaddockFetcher;
use hoogle_core::haddock::types::HaddockDoc;
use hoogle_core::models::SearchResult;
use hoogle_syntax::theme::Theme;
use ratatui::layout::Rect;
use ratatui::Frame;
use tokio::sync::mpsc;
use tokio::time::Instant;
use tui_textarea::TextArea;
use url::Url;

use crate::actions::Action;
use crate::bookmarks::{self, Bookmark, BookmarkStore};
use crate::clipboard;
use crate::history::{self, SearchHistory};
use crate::ui::{
    bookmarks_popup, doc_viewer, filter_popup, help_overlay, history_popup, layout, module_browser,
    package_popup, pinned_panel, preview_pane, result_list, search_bar, sort_popup, source_viewer,
    status_bar, theme_popup, toc_popup, yank_popup,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppMode {
    Search,
    Results,
    DocView,
    SourceView,
    Help,
}

impl AppMode {
    pub const ALL: [AppMode; 5] = [
        AppMode::Search,
        AppMode::Results,
        AppMode::DocView,
        AppMode::SourceView,
        AppMode::Help,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupMode {
    Filter,
    Sort,
    Toc,
    History,
    Bookmarks,
    YankMenu,
    PackageScope,
    ThemeSwitcher,
    ModuleBrowser,
}

/// Message sent from async search tasks back to the app.
pub struct SearchResponse {
    pub generation: u64,
    pub results: Result<Vec<SearchResult>, BackendError>,
}

/// Message sent from async doc fetch tasks.
pub struct DocResponse {
    #[allow(dead_code)]
    pub url: Url,
    pub result: Result<HaddockDoc, BackendError>,
}

pub struct SourceResponse {
    pub decl_name: String,
    pub result: Result<String, BackendError>,
}

pub struct App {
    pub mode: AppMode,
    pub should_quit: bool,
    pub config: Config,
    pub theme: Theme,

    // Search state
    pub textarea: TextArea<'static>,
    pub last_searched: String,
    pub debounce_deadline: Option<Instant>,
    pub search_generation: u64,

    // Results
    pub results: result_list::ResultListState,
    pub preview_enabled: bool,

    // Popups
    pub popup: Option<PopupMode>,
    pub filter_state: filter_popup::FilterState,
    pub sort_state: sort_popup::SortState,
    pub yank_popup: Option<yank_popup::YankPopupState>,
    pub package_popup: Option<package_popup::PackageScopeState>,
    pub theme_popup: Option<theme_popup::ThemePopupState>,
    pub package_scope: Vec<String>,
    pub module_browser: Option<module_browser::ModuleBrowserState>,

    // Pinned results
    pub pinned: pinned_panel::PinnedState,

    // Preview pane scroll state
    pub preview_state: preview_pane::PreviewState,

    // Tab completion
    pub completion_candidates: Vec<String>,
    pub completion_index: usize,

    // Viewed docs (for export + recent docs)
    pub viewed_docs: Vec<(String, String)>,

    // Pagination
    pub has_more_results: bool,
    pub loading_more: bool,

    // All results (unfiltered) for client-side filter/sort
    pub all_results: Vec<SearchResult>,

    // Doc viewer
    pub doc_state: doc_viewer::DocViewState,
    pub toc_state: Option<toc_popup::TocState>,

    // Source viewer
    pub source_state: source_viewer::SourceViewState,

    // History & bookmarks
    pub history: SearchHistory,
    pub bookmark_store: BookmarkStore,
    pub help_state: help_overlay::HelpState,
    pub history_popup: Option<history_popup::HistoryPopupState>,
    pub bookmarks_popup: Option<bookmarks_popup::BookmarksPopupState>,

    // Status
    pub status: status_bar::StatusState,

    // Backend
    pub backend: Arc<dyn HoogleBackend>,
    pub fetcher: Arc<HaddockFetcher>,

    // Channel for receiving search results
    pub search_tx: mpsc::UnboundedSender<SearchResponse>,
    pub search_rx: mpsc::UnboundedReceiver<SearchResponse>,

    // Channel for receiving doc results
    pub doc_tx: mpsc::UnboundedSender<DocResponse>,
    pub doc_rx: mpsc::UnboundedReceiver<DocResponse>,

    // Channel for receiving source results
    pub source_tx: mpsc::UnboundedSender<SourceResponse>,
    pub source_rx: mpsc::UnboundedReceiver<SourceResponse>,

    // Message timeout
    pub message_deadline: Option<Instant>,

    // Last terminal width (for doc re-rendering)
    pub last_width: u16,

    // Mouse support: cached layout rects from last draw
    pub hit_search_bar: Rect,
    pub hit_result_list: Rect,
    pub hit_preview_pane: Option<Rect>,
    pub hit_doc_area: Rect,

    // Double-click tracking
    pub last_click_time: Option<Instant>,
    pub last_click_row: u16,
}

fn rect_contains(rect: Rect, col: u16, row: u16) -> bool {
    col >= rect.x
        && col < rect.x + rect.width
        && row >= rect.y
        && row < rect.y + rect.height
}

impl App {
    pub fn new(config: Config, backend: Box<dyn HoogleBackend>) -> Self {
        let theme = Theme::by_name(&config.theme);
        let backend_name = backend.name().to_string();
        let backend: Arc<dyn HoogleBackend> = Arc::from(backend);
        let (search_tx, search_rx) = mpsc::unbounded_channel();
        let (doc_tx, doc_rx) = mpsc::unbounded_channel();
        let (source_tx, source_rx) = mpsc::unbounded_channel();

        let cache = DiskCache::new(
            config.cache.cache_dir(),
            config.cache.ttl_hours,
            config.cache.max_size_mb,
        );
        let fetcher = Arc::new(
            HaddockFetcher::new(cache, config.backend.timeout_secs)
                .expect("failed to create HTTP client"),
        );

        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(ratatui::style::Style::default());
        textarea.set_placeholder_text("Type to search Hoogle...");

        let preview_enabled = config.ui.preview_enabled;
        Self {
            mode: AppMode::Search,
            should_quit: false,
            config,
            theme,
            textarea,
            last_searched: String::new(),
            debounce_deadline: None,
            search_generation: 0,
            results: result_list::ResultListState::new(),
            preview_enabled,
            popup: None,
            filter_state: filter_popup::FilterState::new(),
            sort_state: sort_popup::SortState::new(),
            yank_popup: None,
            package_popup: None,
            theme_popup: None,
            package_scope: Vec::new(),
            module_browser: None,
            pinned: pinned_panel::PinnedState::new(),
            preview_state: preview_pane::PreviewState::new(),
            completion_candidates: Vec::new(),
            completion_index: 0,
            viewed_docs: Vec::new(),
            has_more_results: false,
            loading_more: false,
            all_results: Vec::new(),
            doc_state: doc_viewer::DocViewState::new(),
            toc_state: None,
            source_state: source_viewer::SourceViewState::new(),
            help_state: help_overlay::HelpState::new(),
            history: SearchHistory::load(history::history_path()),
            bookmark_store: BookmarkStore::load(bookmarks::bookmarks_path()),
            history_popup: None,
            bookmarks_popup: None,
            status: status_bar::StatusState::new(backend_name),
            backend,
            fetcher,
            search_tx,
            search_rx,
            doc_tx,
            doc_rx,
            source_tx,
            source_rx,
            message_deadline: None,
            last_width: 80,
            hit_search_bar: Rect::default(),
            hit_result_list: Rect::default(),
            hit_preview_pane: None,
            hit_doc_area: Rect::default(),
            last_click_time: None,
            last_click_row: 0,
        }
    }

    /// Set initial query from CLI args and trigger immediate search.
    pub fn set_initial_query(&mut self, query: &str) {
        self.textarea = TextArea::from([query]);
        self.textarea
            .set_cursor_line_style(ratatui::style::Style::default());
        self.textarea
            .set_placeholder_text("Type to search Hoogle...");
        // Move cursor to end of line
        self.textarea.move_cursor(tui_textarea::CursorMove::End);
        self.trigger_search();
    }

    pub fn handle_action(&mut self, action: Action) {
        // Handle popup mode first
        if let Some(popup) = self.popup {
            match popup {
                PopupMode::Filter => match action {
                    Action::MoveDown => self.filter_state.move_down(),
                    Action::MoveUp => self.filter_state.move_up(),
                    Action::Select => {
                        self.filter_state.confirm();
                        self.popup = None;
                        self.apply_filter_and_sort();
                    }
                    Action::Back | Action::Quit => self.popup = None,
                    Action::Tick => self.on_tick(),
                    _ => {}
                },
                PopupMode::Sort => match action {
                    Action::MoveDown => self.sort_state.move_down(),
                    Action::MoveUp => self.sort_state.move_up(),
                    Action::Select => {
                        self.sort_state.confirm();
                        self.popup = None;
                        self.apply_filter_and_sort();
                    }
                    Action::Back | Action::Quit => self.popup = None,
                    Action::Tick => self.on_tick(),
                    _ => {}
                },
                PopupMode::Toc => match action {
                    Action::MoveDown => {
                        if let Some(ref mut toc) = self.toc_state {
                            toc.move_down();
                        }
                    }
                    Action::MoveUp => {
                        if let Some(ref mut toc) = self.toc_state {
                            toc.move_up();
                        }
                    }
                    Action::Select => {
                        if let Some(ref toc) = self.toc_state {
                            if let Some(offset) = toc.selected_offset() {
                                self.doc_state.scroll_offset = offset.saturating_sub(1);
                            }
                        }
                        self.popup = None;
                    }
                    Action::Back | Action::Quit => self.popup = None,
                    Action::Tick => self.on_tick(),
                    _ => {}
                },
                PopupMode::History => match action {
                    Action::MoveDown => {
                        if let Some(ref mut hp) = self.history_popup {
                            hp.move_down();
                        }
                    }
                    Action::MoveUp => {
                        if let Some(ref mut hp) = self.history_popup {
                            hp.move_up();
                        }
                    }
                    Action::Select => {
                        if let Some(ref hp) = self.history_popup {
                            if let Some(idx) = hp.selected_index() {
                                if let Some(entry) = self.history.entries().get(idx) {
                                    let query = entry.query.clone();
                                    self.popup = None;
                                    self.history_popup = None;
                                    self.set_initial_query(&query);
                                    return;
                                }
                            }
                        }
                        self.popup = None;
                    }
                    Action::DeleteEntry => {
                        if let Some(ref hp) = self.history_popup {
                            if let Some(idx) = hp.selected_index() {
                                self.history.remove(idx);
                                self.history.save();
                            }
                        }
                        // Refresh popup
                        let total = self.history.entries().len();
                        self.history_popup = Some(history_popup::HistoryPopupState::new(total));
                    }
                    Action::Back | Action::Quit => {
                        self.popup = None;
                        self.history_popup = None;
                    }
                    Action::Tick => self.on_tick(),
                    _ => {}
                },
                PopupMode::Bookmarks => match action {
                    Action::MoveDown => {
                        if let Some(ref mut bp) = self.bookmarks_popup {
                            bp.move_down(self.bookmark_store.bookmarks().len());
                        }
                    }
                    Action::MoveUp => {
                        if let Some(ref mut bp) = self.bookmarks_popup {
                            bp.move_up();
                        }
                    }
                    Action::Select => {
                        if let Some(ref bp) = self.bookmarks_popup {
                            let idx = bp.selected;
                            if let Some(bm) = self.bookmark_store.bookmarks().get(idx) {
                                if let Some(ref url) = bm.doc_url {
                                    self.popup = None;
                                    self.bookmarks_popup = None;
                                    self.mode = AppMode::DocView;
                                    self.doc_state.loading = true;
                                    self.fetch_doc(url.clone());
                                    return;
                                }
                            }
                        }
                        self.popup = None;
                    }
                    Action::DeleteEntry => {
                        if let Some(ref bp) = self.bookmarks_popup {
                            self.bookmark_store.remove(bp.selected);
                            self.bookmark_store.save();
                        }
                        self.bookmarks_popup = Some(bookmarks_popup::BookmarksPopupState::new());
                    }
                    Action::Back | Action::Quit => {
                        self.popup = None;
                        self.bookmarks_popup = None;
                    }
                    Action::Tick => self.on_tick(),
                    _ => {}
                },
                PopupMode::YankMenu => match action {
                    Action::MoveDown => {
                        if let Some(ref mut yp) = self.yank_popup {
                            yp.move_down();
                        }
                    }
                    Action::MoveUp => {
                        if let Some(ref mut yp) = self.yank_popup {
                            yp.move_up();
                        }
                    }
                    Action::Select => {
                        if let Some(ref yp) = self.yank_popup {
                            let idx = yp.selected;
                            self.popup = None;
                            self.yank_popup = None;
                            match idx {
                                0 => self.yank_signature(),
                                1 => self.yank_qualified_name(),
                                2 => self.yank_import(),
                                3 => self.yank_url(),
                                4 => self.yank_ghci_command(":type"),
                                5 => self.yank_ghci_command(":info"),
                                6 => self.yank_decl_deep_link(),
                                _ => {}
                            }
                            return;
                        }
                        self.popup = None;
                    }
                    Action::Back | Action::Quit => {
                        self.popup = None;
                        self.yank_popup = None;
                    }
                    Action::Tick => self.on_tick(),
                    _ => {}
                },
                PopupMode::PackageScope => match action {
                    Action::Select => {
                        if let Some(ref mut pp) = self.package_popup {
                            self.package_scope = pp.confirm();
                            self.status.package_scope = self.package_scope.clone();
                        }
                        self.popup = None;
                        self.package_popup = None;
                        // Re-trigger search with new scope
                        self.trigger_search();
                    }
                    Action::ClearSearch => {
                        if let Some(ref mut pp) = self.package_popup {
                            pp.clear();
                        }
                    }
                    Action::Back | Action::Quit => {
                        self.popup = None;
                        self.package_popup = None;
                    }
                    Action::Tick => self.on_tick(),
                    _ => {}
                },
                PopupMode::ThemeSwitcher => match action {
                    Action::MoveDown => {
                        if let Some(ref mut tp) = self.theme_popup {
                            tp.move_down();
                        }
                    }
                    Action::MoveUp => {
                        if let Some(ref mut tp) = self.theme_popup {
                            tp.move_up();
                        }
                    }
                    Action::Select => {
                        if let Some(ref mut tp) = self.theme_popup {
                            let name = tp.confirm();
                            self.theme = hoogle_syntax::theme::Theme::by_name(name);
                            // Re-render doc if loaded
                            if let Some(doc) = self.doc_state.doc.take() {
                                self.doc_state.set_doc(doc, &self.theme, self.last_width);
                            }
                        }
                        self.popup = None;
                        self.theme_popup = None;
                    }
                    Action::Back | Action::Quit | Action::OpenThemeSwitcher => {
                        self.popup = None;
                        self.theme_popup = None;
                    }
                    Action::Tick => self.on_tick(),
                    _ => {}
                },
                PopupMode::ModuleBrowser => match action {
                    Action::MoveDown => {
                        if let Some(ref mut mb) = self.module_browser {
                            mb.move_down();
                        }
                    }
                    Action::MoveUp => {
                        if let Some(ref mut mb) = self.module_browser {
                            mb.move_up();
                        }
                    }
                    Action::ScrollDown => {
                        // Space toggles expand
                        if let Some(ref mut mb) = self.module_browser {
                            mb.toggle_expand();
                        }
                    }
                    Action::Select => {
                        // Enter: filter results to selected module
                        if let Some(ref mb) = self.module_browser {
                            if let Some(module) = mb.selected_module() {
                                let module_owned = module.to_string();
                                self.popup = None;
                                self.module_browser = None;
                                // Set search to module: prefix
                                let query = format!("module:{module_owned}");
                                self.set_initial_query(&query);
                                return;
                            }
                        }
                        self.popup = None;
                        self.module_browser = None;
                    }
                    Action::Back | Action::Quit => {
                        self.popup = None;
                        self.module_browser = None;
                    }
                    Action::Tick => self.on_tick(),
                    _ => {}
                },
            }
            return;
        }

        match action {
            Action::Quit => self.should_quit = true,
            Action::Back => match self.mode {
                AppMode::Search => {
                    if self.query_text().is_empty() {
                        self.should_quit = true;
                    } else {
                        self.textarea = TextArea::default();
                        self.textarea
                            .set_cursor_line_style(ratatui::style::Style::default());
                        self.textarea
                            .set_placeholder_text("Type to search Hoogle...");
                        self.results.set_items(Vec::new());
                        self.all_results.clear();
                        self.status.result_count = 0;
                        self.last_searched.clear();
                    }
                }
                AppMode::Results => self.mode = AppMode::Search,
                AppMode::DocView => self.mode = AppMode::Results,
                AppMode::SourceView => self.mode = AppMode::DocView,
                AppMode::Help => self.mode = AppMode::Results,
            },

            // Navigation
            Action::FocusSearch => self.mode = AppMode::Search,
            Action::FocusResults => {
                if !self.results.items.is_empty() {
                    self.mode = AppMode::Results;
                }
            }

            // Results navigation
            Action::MoveDown => match self.mode {
                AppMode::DocView => self.doc_state.scroll_down(1),
                AppMode::SourceView => self.source_state.scroll_down(1),
                AppMode::Help => self.help_state.scroll_down(1),
                _ => self.results.move_down(),
            },
            Action::MoveUp => match self.mode {
                AppMode::DocView => self.doc_state.scroll_up(1),
                AppMode::SourceView => self.source_state.scroll_up(1),
                AppMode::Help => self.help_state.scroll_up(1),
                _ => self.results.move_up(),
            },
            Action::MoveToTop => match self.mode {
                AppMode::DocView => self.doc_state.scroll_to_top(),
                AppMode::SourceView => self.source_state.scroll_to_top(),
                _ => self.results.move_to_top(),
            },
            Action::MoveToBottom => match self.mode {
                AppMode::DocView => self.doc_state.scroll_to_bottom(),
                AppMode::SourceView => self.source_state.scroll_to_bottom(),
                _ => self.results.move_to_bottom(),
            },
            Action::Select => {
                if self.mode == AppMode::Results {
                    self.open_doc_for_selected();
                }
            }

            // Scrolling
            Action::ScrollDown => match self.mode {
                AppMode::DocView => self.doc_state.scroll_down(1),
                AppMode::SourceView => self.source_state.scroll_down(1),
                AppMode::Help => self.help_state.scroll_down(1),
                AppMode::Results => {
                    // Space scrolls preview pane when visible, otherwise moves selection
                    if self.preview_enabled {
                        self.preview_state.scroll_down(1);
                    } else {
                        self.results.move_down();
                    }
                }
                _ => {}
            },
            Action::ScrollUp => match self.mode {
                AppMode::DocView => self.doc_state.scroll_up(1),
                AppMode::SourceView => self.source_state.scroll_up(1),
                AppMode::Help => self.help_state.scroll_up(1),
                AppMode::Results => self.results.move_up(),
                _ => {}
            },
            Action::ScrollHalfDown => {
                let half = self.doc_state.viewport_height / 2;
                self.doc_state.scroll_down(half.max(1));
            }
            Action::ScrollHalfUp => {
                let half = self.doc_state.viewport_height / 2;
                self.doc_state.scroll_up(half.max(1));
            }
            Action::ScrollPageDown => {
                let page = self.doc_state.viewport_height.saturating_sub(2);
                self.doc_state.scroll_down(page.max(1));
            }
            Action::ScrollPageUp => {
                let page = self.doc_state.viewport_height.saturating_sub(2);
                self.doc_state.scroll_up(page.max(1));
            }

            // Declaration navigation (or search match cycling if search active)
            Action::NextDeclaration => {
                if !self.doc_state.search_matches.is_empty() {
                    self.doc_state.next_match();
                } else {
                    self.doc_state.next_declaration();
                }
            }
            Action::PrevDeclaration => {
                if !self.doc_state.search_matches.is_empty() {
                    self.doc_state.prev_match();
                } else {
                    self.doc_state.prev_declaration();
                }
            }

            // TOC
            Action::OpenTOC => {
                if self.mode == AppMode::DocView {
                    self.open_toc();
                }
            }

            // Tab cycles through links, Enter follows focused link
            Action::FollowLink => {
                if self.mode == AppMode::DocView {
                    if let Some(url) = self.doc_state.focused_link_url().cloned() {
                        // Follow the focused link
                        if url.as_str().contains("/docs/") || url.as_str().contains("#") {
                            self.doc_state.push_nav(url.clone());
                            self.fetch_doc(url);
                        } else {
                            self.show_info(&format!("Link: {url}"));
                        }
                    } else {
                        // No focused link — cycle to first one via Tab
                        self.doc_state.focus_next_link();
                    }
                }
            }

            // Tab cycles through links in doc view
            Action::CycleLink => {
                if self.mode == AppMode::DocView {
                    self.doc_state.focus_next_link();
                }
            }

            // In-document search
            Action::SearchInDoc => {
                if self.mode == AppMode::DocView {
                    self.doc_state.start_search();
                }
            }

            // Back navigation in doc view
            Action::NavBack => {
                if self.mode == AppMode::DocView {
                    if let Some(url) = self.doc_state.pop_nav() {
                        self.fetch_doc(url);
                    } else {
                        self.mode = AppMode::Results;
                    }
                }
            }

            // View source
            Action::ViewSource => {
                if self.mode == AppMode::DocView {
                    self.open_source_for_current_decl();
                }
            }

            // Preview
            Action::TogglePreview => {
                self.preview_enabled = !self.preview_enabled;
            }

            // Popups
            Action::OpenFilter => {
                self.filter_state.sync_selection();
                self.popup = Some(PopupMode::Filter);
            }
            Action::OpenSort => {
                self.sort_state.sync_selection();
                self.popup = Some(PopupMode::Sort);
            }

            // Bookmarks
            Action::Bookmark => self.bookmark_selected(),
            Action::OpenBookmarks => {
                self.bookmarks_popup = Some(bookmarks_popup::BookmarksPopupState::new());
                self.popup = Some(PopupMode::Bookmarks);
            }

            // History
            Action::SearchHistory => {
                let total = self.history.entries().len();
                self.history_popup = Some(history_popup::HistoryPopupState::new(total));
                self.popup = Some(PopupMode::History);
            }

            // Clipboard
            Action::YankSignature => self.yank_signature(),
            Action::YankImport => self.yank_import(),
            Action::YankUrl => self.yank_url(),

            // Help
            Action::ToggleHelp => {
                if self.mode == AppMode::Help {
                    self.mode = AppMode::Results;
                } else {
                    self.help_state = help_overlay::HelpState::new();
                    self.mode = AppMode::Help;
                }
            }

            // Search
            Action::ClearSearch => {
                self.textarea = TextArea::default();
                self.textarea
                    .set_cursor_line_style(ratatui::style::Style::default());
                self.textarea
                    .set_placeholder_text("Type to search Hoogle...");
            }

            // New: Yank menu
            Action::OpenYankMenu => {
                if self.mode == AppMode::Results {
                    self.yank_popup = Some(yank_popup::YankPopupState::new());
                    self.popup = Some(PopupMode::YankMenu);
                }
            }

            // New: Package scope
            Action::OpenPackageScope => {
                self.package_popup =
                    Some(package_popup::PackageScopeState::new(&self.package_scope));
                self.popup = Some(PopupMode::PackageScope);
            }

            // New: Theme switcher
            Action::OpenThemeSwitcher => {
                if self.popup == Some(PopupMode::ThemeSwitcher) {
                    // Toggle off
                    self.popup = None;
                    self.theme_popup = None;
                } else {
                    self.theme_popup =
                        Some(theme_popup::ThemePopupState::new(&self.theme.name));
                    self.popup = Some(PopupMode::ThemeSwitcher);
                }
            }

            // New: Compact toggle
            Action::ToggleCompact => {
                if self.mode == AppMode::Results {
                    self.results.compact = !self.results.compact;
                }
            }

            // New: Open in browser
            Action::OpenInBrowser => {
                self.open_in_browser();
            }

            // New: Export session
            Action::ExportSession => {
                match crate::export::export_session(
                    &self.last_searched,
                    &self.all_results,
                    &self.viewed_docs,
                ) {
                    Ok(path) => self.show_info(&format!("Exported to {}", path.display())),
                    Err(e) => self.show_error(&format!("Export failed: {e}")),
                }
            }

            // Tab complete (handled in main.rs for search mode)
            Action::TabComplete => {}

            // Load more results
            Action::LoadMore => {
                if self.has_more_results && !self.loading_more {
                    self.load_more_results();
                }
            }

            // Module browser
            Action::OpenModuleBrowser => {
                self.module_browser =
                    Some(module_browser::ModuleBrowserState::new(&self.all_results));
                self.popup = Some(PopupMode::ModuleBrowser);
            }

            // Pin/unpin
            Action::PinResult => {
                if let Some(result) = self.results.selected_result() {
                    self.pinned.pin(result);
                    self.show_info("Pinned!");
                }
            }
            Action::UnpinAll => {
                self.pinned.clear();
                self.show_info("All pins cleared");
            }

            // Multi-select
            Action::ToggleMultiSelect => {
                if self.mode == AppMode::Results {
                    if self.results.multi_select_mode {
                        self.results.toggle_select_current();
                        self.results.move_down();
                    } else {
                        self.results.multi_select_mode = true;
                        self.results.toggle_select_current();
                        self.results.move_down();
                    }
                }
            }

            // Batch yank imports
            Action::YankSelectedImports => {
                if self.mode == AppMode::Results {
                    self.yank_multi_imports();
                }
            }

            // Group by module
            Action::ToggleGroupByModule => {
                if self.mode == AppMode::Results {
                    self.results.group_by_module = !self.results.group_by_module;
                }
            }

            // GHCi: yank :type command
            Action::YankGhciType => {
                self.yank_ghci_command(":type");
            }

            // GHCi: yank :info command
            Action::YankGhciInfo => {
                self.yank_ghci_command(":info");
            }

            // Haddock deep link: yank URL with declaration anchor
            Action::YankDeclLink => {
                self.yank_decl_deep_link();
            }

            // Detect Haskell project and set package scope
            Action::DetectProject => {
                self.detect_and_apply_project();
            }

            // Tick
            Action::Tick => self.on_tick(),

            _ => {}
        }
    }

    /// Handle a raw key event when in search mode.
    /// Returns true if the textarea consumed the event.
    pub fn handle_search_input(&mut self, input: crossterm::event::KeyEvent) -> bool {
        let before = self.query_text();
        let consumed = self.textarea.input(input);
        let after = self.query_text();

        if before != after {
            // Text changed — reset debounce
            self.debounce_deadline =
                Some(Instant::now() + std::time::Duration::from_millis(self.config.ui.debounce_ms));
        }

        consumed
    }

    pub fn handle_fuzzy_filter_input(
        &mut self,
        key: crossterm::event::KeyEvent,
        keymap: &crate::keymap::Keymap,
    ) {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc => {
                self.results.clear_fuzzy_filter();
            }
            KeyCode::Backspace => {
                self.results.fuzzy_delete_char();
            }
            KeyCode::Enter => {
                // Confirm filter and open docs for selected result
                if self.results.visible_count() > 0 {
                    self.open_doc_for_selected();
                }
            }
            KeyCode::Char(c) => {
                let action = keymap.resolve(
                    AppMode::Results,
                    crossterm::event::KeyEvent::new(key.code, key.modifiers),
                );
                // Pass through navigation and important actions
                match action {
                    crate::actions::Action::MoveDown
                    | crate::actions::Action::MoveUp
                    | crate::actions::Action::MoveToTop
                    | crate::actions::Action::MoveToBottom
                    | crate::actions::Action::Quit
                    | crate::actions::Action::ToggleHelp
                    | crate::actions::Action::FocusSearch
                    | crate::actions::Action::OpenFilter
                    | crate::actions::Action::OpenSort
                    | crate::actions::Action::Select
                    | crate::actions::Action::TogglePreview
                    | crate::actions::Action::YankSignature
                    | crate::actions::Action::OpenYankMenu => {
                        self.results.clear_fuzzy_filter();
                        self.handle_action(action);
                    }
                    _ => {
                        // Only add printable chars to filter
                        if c.is_alphanumeric() || c == '_' || c == '.' || c == ' ' {
                            self.results.fuzzy_add_char(c);
                        }
                    }
                }
            }
            KeyCode::Up => self.results.move_up(),
            KeyCode::Down => self.results.move_down(),
            _ => {}
        }
    }

    pub fn handle_doc_search_input(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc => {
                self.doc_state.clear_search();
            }
            KeyCode::Enter => {
                self.doc_state.confirm_search();
            }
            KeyCode::Backspace => {
                self.doc_state.search_delete_char();
            }
            KeyCode::Char('n')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.doc_state.next_match();
            }
            KeyCode::Char('p')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.doc_state.prev_match();
            }
            KeyCode::Char(c) => {
                self.doc_state.search_add_char(c);
            }
            _ => {}
        }
    }

    fn query_text(&self) -> String {
        self.textarea.lines().join("")
    }

    fn on_tick(&mut self) {
        self.status.tick();

        // Check debounce
        if let Some(deadline) = self.debounce_deadline {
            if Instant::now() >= deadline {
                self.debounce_deadline = None;
                let query = self.query_text();
                if query != self.last_searched {
                    self.trigger_search();
                }
            }
        }

        // Check for search results
        while let Ok(response) = self.search_rx.try_recv() {
            if response.generation == self.search_generation {
                match response.results {
                    Ok(items) => {
                        let count = items.len();
                        self.has_more_results = count >= self.config.ui.max_results;
                        if self.loading_more {
                            // Append to existing results
                            self.all_results.extend(items);
                            self.loading_more = false;
                        } else {
                            self.all_results = items;
                        }
                        self.apply_filter_and_sort();
                        self.results.loading = false;
                        self.status.message = None;
                        self.status.offline = false;
                        // Save to history
                        if !self.loading_more {
                            self.history.add(&self.last_searched, count);
                            self.history.save();
                        }
                    }
                    Err(e) => {
                        self.results.loading = false;
                        self.loading_more = false;
                        // Detect offline
                        let err_str = format!("{e}");
                        if err_str.contains("network") || err_str.contains("timeout") {
                            self.status.offline = true;
                        }
                        self.status.set_error(format!("Search failed: {e}"));
                        self.message_deadline =
                            Some(Instant::now() + std::time::Duration::from_secs(5));
                    }
                }
            }
        }

        // Check for doc results
        while let Ok(response) = self.doc_rx.try_recv() {
            match response.result {
                Ok(doc) => {
                    // Track viewed docs for export
                    self.viewed_docs
                        .push((doc.module.clone(), doc.package.clone()));
                    self.doc_state.set_doc(doc, &self.theme, self.last_width);
                    self.status.clear_message();
                    self.status.offline = false;
                }
                Err(e) => {
                    self.doc_state.loading = false;
                    self.doc_state.error = Some(format!("{e}"));
                    self.status.set_error(format!("Doc fetch failed: {e}"));
                    self.message_deadline =
                        Some(Instant::now() + std::time::Duration::from_secs(5));
                }
            }
        }

        // Check for source results
        while let Ok(response) = self.source_rx.try_recv() {
            match response.result {
                Ok(source) => {
                    self.source_state
                        .set_source(source, &response.decl_name, &self.theme);
                    self.status.clear_message();
                }
                Err(e) => {
                    self.source_state.loading = false;
                    self.source_state.error = Some(format!("{e}"));
                    self.status.set_error(format!("Source fetch failed: {e}"));
                    self.message_deadline =
                        Some(Instant::now() + std::time::Duration::from_secs(5));
                }
            }
        }

        // Clear timed messages
        if let Some(deadline) = self.message_deadline {
            if Instant::now() >= deadline {
                self.message_deadline = None;
                self.status.clear_message();
            }
        }
    }

    fn trigger_search(&mut self) {
        let query = self.query_text();
        if query.is_empty() {
            self.results.set_items(Vec::new());
            self.status.result_count = 0;
            self.status.search_by_type = false;
            self.last_searched.clear();
            return;
        }

        // Detect type signature search
        self.status.search_by_type = query.contains("->") || query.contains("=>");

        // Reset completion state
        self.completion_candidates.clear();
        self.completion_index = 0;

        tracing::info!("searching for: {query}");
        self.search_generation += 1;
        let generation = self.search_generation;
        self.last_searched = query.clone();
        self.results.loading = true;
        self.has_more_results = false;
        self.loading_more = false;
        self.status.message = Some(status_bar::StatusMessage::Loading("Searching...".into()));

        let backend = self.backend.clone();
        let max_results = self.config.ui.max_results;
        let tx = self.search_tx.clone();
        let full_query = self.build_scoped_query(&query);

        tokio::spawn(async move {
            let results = backend.search(&full_query, max_results).await;
            let _ = tx.send(SearchResponse {
                generation,
                results,
            });
        });
    }

    fn apply_filter_and_sort(&mut self) {
        let needs_sort = self.sort_state.active_sort != sort_popup::SortMode::Relevance;

        // Only clone when we actually need to filter or sort
        let mut items: Vec<SearchResult> = if let Some(kind) = self.filter_state.active_filter {
            self.all_results
                .iter()
                .filter(|r| r.result_kind == kind)
                .cloned()
                .collect()
        } else if needs_sort {
            self.all_results.clone()
        } else {
            // No filter, no sort — move directly to avoid cloning
            self.status.result_count = self.all_results.len();
            self.results.set_items(self.all_results.clone());
            return;
        };

        match self.sort_state.active_sort {
            sort_popup::SortMode::Relevance => {}
            sort_popup::SortMode::Package => {
                items.sort_by(|a, b| {
                    let pa = a.package.as_ref().map(|p| &p.name);
                    let pb = b.package.as_ref().map(|p| &p.name);
                    pa.cmp(&pb)
                });
            }
            sort_popup::SortMode::Module => {
                items.sort_by(|a, b| {
                    let ma = a.module.as_ref().map(|m| m.as_dotted());
                    let mb = b.module.as_ref().map(|m| m.as_dotted());
                    ma.cmp(&mb)
                });
            }
            sort_popup::SortMode::Name => {
                items.sort_by(|a, b| a.name.cmp(&b.name));
            }
        }

        self.status.result_count = items.len();
        self.results.set_items(items);
    }

    fn open_doc_for_selected(&mut self) {
        let Some(result) = self.results.selected_result().cloned() else {
            return;
        };
        let Some(ref url) = result.doc_url else {
            self.show_error("No documentation URL available");
            return;
        };
        self.mode = AppMode::DocView;
        self.doc_state.loading = true;
        self.doc_state.error = None;
        self.status.message = Some(status_bar::StatusMessage::Loading("Loading docs...".into()));
        self.fetch_doc(url.clone());
    }

    fn fetch_doc(&mut self, url: Url) {
        tracing::info!("fetching doc: {url}");
        self.doc_state.loading = true;
        self.doc_state.error = None;

        let fetcher = self.fetcher.clone();
        let tx = self.doc_tx.clone();
        let fetch_url = url.clone();

        tokio::spawn(async move {
            let result = fetcher.fetch_doc(&fetch_url).await;
            let _ = tx.send(DocResponse {
                url: fetch_url,
                result,
            });
        });
    }

    fn open_source_for_current_decl(&mut self) {
        let Some(ref doc) = self.doc_state.doc else {
            return;
        };

        // Find the current declaration based on scroll offset
        let current_decl = self
            .doc_state
            .declaration_offsets
            .iter()
            .rev()
            .find(|(_, off)| *off <= self.doc_state.scroll_offset + 2)
            .and_then(|(name, _)| doc.declarations.iter().find(|d| d.name == *name));

        let Some(decl) = current_decl else {
            self.show_info("No declaration selected");
            return;
        };

        let Some(ref source_url) = decl.source_url else {
            self.show_info("Source not available for this declaration");
            return;
        };

        self.mode = AppMode::SourceView;
        self.source_state.loading = true;
        self.source_state.error = None;
        self.status.message = Some(status_bar::StatusMessage::Loading(
            "Loading source...".into(),
        ));

        let fetcher = self.fetcher.clone();
        let tx = self.source_tx.clone();
        let url = source_url.clone();
        let decl_name = decl.name.clone();

        tokio::spawn(async move {
            let result = fetcher.fetch_source(&url).await;
            let _ = tx.send(SourceResponse { decl_name, result });
        });
    }

    fn open_toc(&mut self) {
        if let Some(ref doc) = self.doc_state.doc {
            let entries: Vec<toc_popup::TocEntry> = doc
                .declarations
                .iter()
                .zip(self.doc_state.declaration_offsets.iter())
                .map(|(decl, (_, offset))| toc_popup::TocEntry {
                    name: decl.name.clone(),
                    signature: decl.signature.clone(),
                    line_offset: *offset,
                })
                .collect();
            self.toc_state = Some(toc_popup::TocState::new(entries));
            self.popup = Some(PopupMode::Toc);
        }
    }

    fn bookmark_selected(&mut self) {
        if let Some(result) = self.results.selected_result() {
            let bookmark = Bookmark {
                name: result.name.clone(),
                module: result.module.as_ref().map(|m| m.to_string()),
                package: result.package.as_ref().map(|p| p.to_string()),
                signature: result.signature.clone(),
                doc_url: result.doc_url.clone(),
                added: chrono::Utc::now(),
            };
            self.bookmark_store.add(bookmark);
            self.bookmark_store.save();
            self.show_info("Bookmarked!");
        }
    }

    fn yank_signature(&mut self) {
        if let Some(result) = self.results.selected_result() {
            if let Some(ref sig) = result.signature {
                let text = format!("{} :: {sig}", result.name);
                match clipboard::copy_to_clipboard(&text) {
                    Ok(()) => self.show_info("Copied signature to clipboard"),
                    Err(e) => self.show_error(&e),
                }
            }
        }
    }

    fn yank_import(&mut self) {
        if let Some(result) = self.results.selected_result() {
            let module_str = result.module.as_ref().map(|m| m.to_string());
            if let Some(import) = clipboard::generate_import(&result.name, module_str.as_deref()) {
                match clipboard::copy_to_clipboard(&import) {
                    Ok(()) => self.show_info("Copied import to clipboard"),
                    Err(e) => self.show_error(&e),
                }
            }
        }
    }

    fn yank_url(&mut self) {
        if let Some(result) = self.results.selected_result() {
            if let Some(ref url) = result.doc_url {
                match clipboard::copy_to_clipboard(url.as_str()) {
                    Ok(()) => self.show_info("Copied URL to clipboard"),
                    Err(e) => self.show_error(&e),
                }
            }
        }
    }

    fn yank_qualified_name(&mut self) {
        if let Some(result) = self.results.selected_result() {
            let module = result.module.as_ref().map(|m| m.to_string());
            let qualified = if let Some(module) = module {
                format!("{module}.{}", result.name)
            } else {
                result.name.clone()
            };
            match clipboard::copy_to_clipboard(&qualified) {
                Ok(()) => self.show_info("Copied qualified name to clipboard"),
                Err(e) => self.show_error(&e),
            }
        }
    }

    fn open_in_browser(&mut self) {
        let url = match self.mode {
            AppMode::Results => self
                .results
                .selected_result()
                .and_then(|r| r.doc_url.as_ref())
                .map(|u| u.to_string()),
            AppMode::DocView => self
                .doc_state
                .doc
                .as_ref()
                .and_then(|_| self.doc_state.nav_stack.last().map(|u| u.to_string()))
                .or_else(|| {
                    self.results
                        .selected_result()
                        .and_then(|r| r.doc_url.as_ref())
                        .map(|u| u.to_string())
                }),
            _ => None,
        };
        if let Some(url) = url {
            match open::that(&url) {
                Ok(()) => self.show_info("Opened in browser"),
                Err(e) => self.show_error(&format!("Failed to open browser: {e}")),
            }
        } else {
            self.show_info("No URL available");
        }
    }

    fn load_more_results(&mut self) {
        let query = self.query_text();
        if query.is_empty() {
            return;
        }
        self.loading_more = true;
        let offset = self.all_results.len();
        let backend = self.backend.clone();
        let max_results = self.config.ui.max_results;
        let tx = self.search_tx.clone();
        let generation = self.search_generation;

        // Build query with package scope
        let full_query = self.build_scoped_query(&query);

        tokio::spawn(async move {
            let results = backend.search(&full_query, max_results).await;
            // Send as same generation so it's accepted
            let _ = tx.send(SearchResponse {
                generation,
                results: results.map(|mut r| {
                    // Skip already-fetched results
                    if r.len() > offset {
                        r.split_off(offset)
                    } else {
                        Vec::new()
                    }
                }),
            });
        });
    }

    pub fn tab_complete(&mut self) {
        let partial = self.query_text();
        if partial.is_empty() {
            return;
        }
        let partial_lower = partial.to_lowercase();

        // Build candidates from current results
        if self.completion_candidates.is_empty()
            || !self.completion_candidates[0]
                .to_lowercase()
                .starts_with(&partial_lower)
        {
            self.completion_candidates = self
                .results
                .items
                .iter()
                .filter_map(|r| {
                    if r.name.to_lowercase().starts_with(&partial_lower) {
                        Some(r.name.clone())
                    } else {
                        None
                    }
                })
                .collect();
            self.completion_candidates.dedup();
            self.completion_index = 0;
        }

        if self.completion_candidates.is_empty() {
            return;
        }

        let candidate = &self.completion_candidates[self.completion_index];
        self.textarea = TextArea::from([candidate.as_str()]);
        self.textarea
            .set_cursor_line_style(ratatui::style::Style::default());
        self.textarea
            .set_placeholder_text("Type to search Hoogle...");
        self.textarea
            .move_cursor(tui_textarea::CursorMove::End);
        self.completion_index = (self.completion_index + 1) % self.completion_candidates.len();
    }

    fn yank_multi_imports(&mut self) {
        let results = self.results.selected_results();
        if results.is_empty() {
            self.show_info("No results selected (use 'x' to multi-select)");
            return;
        }
        let imports: Vec<String> = results
            .iter()
            .filter_map(|r| {
                let module = r.module.as_ref().map(|m| m.to_string())?;
                Some(format!("import {} ({})", module, r.name))
            })
            .collect();

        if imports.is_empty() {
            self.show_info("No importable results selected");
            return;
        }

        let text = imports.join("\n");
        match clipboard::copy_to_clipboard(&text) {
            Ok(()) => self.show_info(&format!("Copied {} imports to clipboard", imports.len())),
            Err(e) => self.show_error(&e),
        }
        self.results.multi_select_mode = false;
        self.results.multi_selected.clear();
    }

    fn yank_ghci_command(&mut self, cmd: &str) {
        // In Results mode, use the selected result name
        // In DocView mode, use the current declaration name
        let name = match self.mode {
            AppMode::Results => self
                .results
                .selected_result()
                .map(|r| {
                    let module = r.module.as_ref().map(|m| m.to_string());
                    if let Some(module) = module {
                        format!("{module}.{}", r.name)
                    } else {
                        r.name.clone()
                    }
                }),
            AppMode::DocView => self.current_decl_name(),
            _ => None,
        };

        if let Some(name) = name {
            let text = format!("{cmd} {name}");
            match clipboard::copy_to_clipboard(&text) {
                Ok(()) => self.show_info(&format!("Copied: {text}")),
                Err(e) => self.show_error(&e),
            }
        } else {
            self.show_info("No declaration selected");
        }
    }

    fn current_decl_name(&self) -> Option<String> {
        let doc = self.doc_state.doc.as_ref()?;
        let (name, _) = self
            .doc_state
            .declaration_offsets
            .iter()
            .rev()
            .find(|(_, off)| *off <= self.doc_state.scroll_offset + 2)?;
        let decl = doc.declarations.iter().find(|d| &d.name == name)?;
        let module = &doc.module;
        Some(format!("{module}.{}", decl.name))
    }

    fn yank_decl_deep_link(&mut self) {
        if self.mode != AppMode::DocView {
            // In results mode, just yank the result URL
            self.yank_url();
            return;
        }

        let doc = match &self.doc_state.doc {
            Some(d) => d,
            None => {
                self.show_info("No documentation loaded");
                return;
            }
        };

        // Find the current declaration
        let current_decl = self
            .doc_state
            .declaration_offsets
            .iter()
            .rev()
            .find(|(_, off)| *off <= self.doc_state.scroll_offset + 2)
            .and_then(|(name, _)| doc.declarations.iter().find(|d| d.name == *name));

        if let Some(decl) = current_decl {
            // Build deep link URL using the anchor
            if let Some(ref anchor) = decl.anchor {
                // Try to build URL from the doc's module URL pattern
                let base = self
                    .results
                    .selected_result()
                    .and_then(|r| r.doc_url.as_ref())
                    .map(|u| {
                        let mut u = u.clone();
                        u.set_fragment(Some(anchor));
                        u.to_string()
                    });
                if let Some(url) = base {
                    match clipboard::copy_to_clipboard(&url) {
                        Ok(()) => self.show_info("Copied deep link to clipboard"),
                        Err(e) => self.show_error(&e),
                    }
                    return;
                }
            }
            // Fallback: construct from module name and declaration
            let kind_prefix = if decl
                .signature
                .as_ref()
                .is_some_and(|s| s.starts_with("data ") || s.starts_with("type ") || s.starts_with("class ") || s.starts_with("newtype "))
            {
                "t"
            } else {
                "v"
            };
            let url = format!(
                "https://hackage.haskell.org/package/{}/docs/{}.html#{kind_prefix}:{}",
                doc.package,
                doc.module.replace('.', "-"),
                decl.name
            );
            match clipboard::copy_to_clipboard(&url) {
                Ok(()) => self.show_info("Copied deep link to clipboard"),
                Err(e) => self.show_error(&e),
            }
        } else {
            self.show_info("No declaration at cursor");
        }
    }

    fn detect_and_apply_project(&mut self) {
        match crate::project::detect_project() {
            Some(info) => {
                let pkg_count = info.dependencies.len();
                self.package_scope = info.dependencies;
                self.status.package_scope = self.package_scope.clone();
                let proj_type = match info.project_type {
                    crate::project::ProjectType::Cabal => "cabal",
                    crate::project::ProjectType::Stack => "stack",
                };
                self.show_info(&format!(
                    "Detected {proj_type} project: {pkg_count} dependencies scoped"
                ));
                // Re-trigger search with new scope if we have a query
                if !self.last_searched.is_empty() {
                    self.trigger_search();
                }
            }
            None => {
                self.show_info("No Haskell project detected in current directory");
            }
        }
    }

    fn build_scoped_query(&self, query: &str) -> String {
        if self.package_scope.is_empty() {
            query.to_string()
        } else {
            let prefix: String = self
                .package_scope
                .iter()
                .map(|p| format!("+{p} "))
                .collect();
            format!("{prefix}{query}")
        }
    }

    fn show_info(&mut self, msg: &str) {
        self.status.set_info(msg);
        self.message_deadline = Some(Instant::now() + std::time::Duration::from_secs(2));
    }

    fn show_error(&mut self, msg: &str) {
        self.status.set_error(msg);
        self.message_deadline = Some(Instant::now() + std::time::Duration::from_secs(3));
    }

    /// Handle a mouse event. Returns true if the event was consumed.
    pub fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) {
        use crossterm::event::MouseEventKind;

        let col = mouse.column;
        let row = mouse.row;

        match mouse.kind {
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                self.handle_mouse_click(col, row);
            }
            MouseEventKind::ScrollDown => {
                self.handle_mouse_scroll(col, row, true);
            }
            MouseEventKind::ScrollUp => {
                self.handle_mouse_scroll(col, row, false);
            }
            _ => {}
        }
    }

    fn handle_mouse_click(&mut self, col: u16, row: u16) {
        let now = Instant::now();

        // Detect double-click (two clicks within 400ms on the same row)
        let is_double_click = self
            .last_click_time
            .map(|t| {
                now.duration_since(t) < std::time::Duration::from_millis(400)
                    && self.last_click_row == row
            })
            .unwrap_or(false);

        self.last_click_time = Some(now);
        self.last_click_row = row;

        // If a popup is open, clicks outside close it
        if self.popup.is_some() {
            self.popup = None;
            self.toc_state = None;
            self.history_popup = None;
            self.bookmarks_popup = None;
            return;
        }

        // In search/results mode: hit test panels
        if self.mode == AppMode::Search || self.mode == AppMode::Results {
            if rect_contains(self.hit_search_bar, col, row) {
                self.mode = AppMode::Search;
                return;
            }

            if rect_contains(self.hit_result_list, col, row) {
                // Click on result list: select the clicked item
                if self.mode != AppMode::Results && !self.results.items.is_empty() {
                    self.mode = AppMode::Results;
                }

                // Calculate which result was clicked
                // Inner area is result_list minus 1px border on each side
                let inner_top = self.hit_result_list.y + 1;
                if row > inner_top {
                    let relative_row = (row - inner_top) as usize;
                    let lines_per_result = 3usize;
                    let clicked_index =
                        self.results.scroll_offset + relative_row / lines_per_result;
                    let visible_count = self.results.visible_count();
                    if clicked_index < visible_count {
                        self.results.selected = clicked_index;
                        // Double-click opens the doc
                        if is_double_click {
                            self.open_doc_for_selected();
                        }
                    }
                }
                return;
            }

            if let Some(preview_rect) = self.hit_preview_pane {
                if rect_contains(preview_rect, col, row) {
                    // Click on preview: open docs for the selected result
                    if is_double_click {
                        self.open_doc_for_selected();
                    }
                    return;
                }
            }
        }

        // In help mode: click anywhere to close
        if self.mode == AppMode::Help {
            self.mode = AppMode::Results;
        }
    }

    fn handle_mouse_scroll(&mut self, col: u16, row: u16, down: bool) {
        match self.mode {
            AppMode::DocView => {
                if down {
                    self.doc_state.scroll_down(3);
                } else {
                    self.doc_state.scroll_up(3);
                }
            }
            AppMode::SourceView => {
                if down {
                    self.source_state.scroll_down(3);
                } else {
                    self.source_state.scroll_up(3);
                }
            }
            AppMode::Help => {
                if down {
                    self.help_state.scroll_down(3);
                } else {
                    self.help_state.scroll_up(3);
                }
            }
            AppMode::Search | AppMode::Results => {
                // Scroll the panel the mouse is over
                if rect_contains(self.hit_result_list, col, row) {
                    if down {
                        self.results.move_down();
                    } else {
                        self.results.move_up();
                    }
                } else if self
                    .hit_preview_pane
                    .is_some_and(|r| rect_contains(r, col, row))
                {
                    if down {
                        self.preview_state.scroll_down(3);
                    } else {
                        self.preview_state.scroll_up(3);
                    }
                }
            }
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        self.last_width = area.width;

        // Guard: terminal too small
        if area.width < 40 || area.height < 10 {
            let msg = ratatui::widgets::Paragraph::new(vec![
                ratatui::text::Line::from(""),
                ratatui::text::Line::from(ratatui::text::Span::styled(
                    "Terminal too small",
                    ratatui::style::Style::default()
                        .fg(ratatui::style::Color::Red)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                )),
                ratatui::text::Line::from(ratatui::text::Span::styled(
                    format!("Need at least 40x10, got {}x{}", area.width, area.height),
                    ratatui::style::Style::default().fg(ratatui::style::Color::Gray),
                )),
                ratatui::text::Line::from(""),
                ratatui::text::Line::from("Press q to quit."),
            ])
            .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(msg, area);
            return;
        }

        if self.mode == AppMode::DocView || self.mode == AppMode::SourceView {
            let chunks = ratatui::layout::Layout::vertical([
                ratatui::layout::Constraint::Min(1),
                ratatui::layout::Constraint::Length(1),
            ])
            .split(area);

            self.hit_doc_area = chunks[0];

            if self.mode == AppMode::DocView {
                doc_viewer::render(frame, chunks[0], &mut self.doc_state, &self.theme);
            } else {
                source_viewer::render(frame, chunks[0], &mut self.source_state, &self.theme);
            }
            status_bar::render(frame, chunks[1], &self.status, self.mode, &self.theme);
        } else {
            let ly = layout::compute_layout(area, self.preview_enabled);

            // Save hit areas for mouse support
            self.hit_search_bar = ly.search_bar;
            self.hit_result_list = ly.result_list;
            self.hit_preview_pane = ly.preview_pane;

            let has_query = !self.last_searched.is_empty();
            search_bar::render(frame, ly.search_bar, &mut self.textarea, self.mode, has_query, &self.theme);
            result_list::render(frame, ly.result_list, &mut self.results, &self.theme);

            if let Some(preview_area) = ly.preview_pane {
                if !self.pinned.is_empty() {
                    // Split preview area: top = preview, bottom = pinned
                    let split = ratatui::layout::Layout::vertical([
                        ratatui::layout::Constraint::Percentage(60),
                        ratatui::layout::Constraint::Percentage(40),
                    ])
                    .split(preview_area);
                    let selected = self.results.selected_result().cloned();
                    preview_pane::render(
                        frame,
                        split[0],
                        selected.as_ref(),
                        &mut self.preview_state,
                        &self.theme,
                    );
                    pinned_panel::render(frame, split[1], &mut self.pinned, &self.theme);
                } else {
                    let selected = self.results.selected_result().cloned();
                    preview_pane::render(
                        frame,
                        preview_area,
                        selected.as_ref(),
                        &mut self.preview_state,
                        &self.theme,
                    );
                }
            }

            status_bar::render(frame, ly.status_bar, &self.status, self.mode, &self.theme);
        }

        // Render popups on top
        match self.popup {
            Some(PopupMode::Filter) => {
                filter_popup::render(frame, &self.filter_state, &self.theme);
            }
            Some(PopupMode::Sort) => {
                sort_popup::render(frame, &self.sort_state, &self.theme);
            }
            Some(PopupMode::Toc) => {
                if let Some(ref toc) = self.toc_state {
                    toc_popup::render(frame, toc, &self.theme);
                }
            }
            Some(PopupMode::History) => {
                if let Some(ref hp) = self.history_popup {
                    history_popup::render(frame, hp, &self.history, &self.theme);
                }
            }
            Some(PopupMode::Bookmarks) => {
                if let Some(ref bp) = self.bookmarks_popup {
                    bookmarks_popup::render(frame, bp, &self.bookmark_store, &self.theme);
                }
            }
            Some(PopupMode::YankMenu) => {
                if let Some(ref yp) = self.yank_popup {
                    yank_popup::render(frame, yp, &self.theme);
                }
            }
            Some(PopupMode::PackageScope) => {
                if let Some(ref pp) = self.package_popup {
                    package_popup::render(frame, pp, &self.theme);
                }
            }
            Some(PopupMode::ThemeSwitcher) => {
                if let Some(ref tp) = self.theme_popup {
                    theme_popup::render(frame, tp, &self.theme);
                }
            }
            Some(PopupMode::ModuleBrowser) => {
                if let Some(ref mut mb) = self.module_browser {
                    module_browser::render(frame, mb, &self.theme);
                }
            }
            None => {}
        }

        // Help overlay on top of everything
        if self.mode == AppMode::Help {
            help_overlay::render(frame, &mut self.help_state, &self.theme);
        }
    }
}
