use std::sync::Arc;

use hoogle_core::backend::{BackendError, HoogleBackend};
use hoogle_core::cache::DiskCache;
use hoogle_core::config::Config;
use hoogle_core::haddock::fetcher::HaddockFetcher;
use hoogle_core::haddock::types::HaddockDoc;
use hoogle_core::models::SearchResult;
use hoogle_syntax::theme::Theme;
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
    bookmarks_popup, doc_viewer, filter_popup, help_overlay, history_popup, layout, preview_pane,
    result_list, search_bar, sort_popup, source_viewer, status_bar, toc_popup,
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
                AppMode::Results => self.results.move_down(),
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
                // Confirm filter, keep filtered view, or select result
                if self.results.visible_count() > 0 {
                    self.open_doc_for_selected();
                }
            }
            KeyCode::Char(c) => {
                // Check if it's a navigation key (j/k) — pass through
                let action = keymap.resolve(
                    AppMode::Results,
                    crossterm::event::KeyEvent::new(key.code, key.modifiers),
                );
                match action {
                    crate::actions::Action::MoveDown
                    | crate::actions::Action::MoveUp
                    | crate::actions::Action::MoveToTop
                    | crate::actions::Action::MoveToBottom => {
                        self.handle_action(action);
                    }
                    _ => {
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
                        self.all_results = items;
                        self.apply_filter_and_sort();
                        self.results.loading = false;
                        self.status.message = None;
                        // Save to history
                        self.history.add(&self.last_searched, count);
                        self.history.save();
                    }
                    Err(e) => {
                        self.results.loading = false;
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
                    self.doc_state.set_doc(doc, &self.theme, self.last_width);
                    self.status.clear_message();
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
            self.last_searched.clear();
            return;
        }

        tracing::info!("searching for: {query}");
        self.search_generation += 1;
        let generation = self.search_generation;
        self.last_searched = query.clone();
        self.results.loading = true;
        self.status.message = Some(status_bar::StatusMessage::Loading("Searching...".into()));

        let backend = self.backend.clone();
        let max_results = self.config.ui.max_results;
        let tx = self.search_tx.clone();

        tokio::spawn(async move {
            let results = backend.search(&query, max_results).await;
            let _ = tx.send(SearchResponse {
                generation,
                results,
            });
        });
    }

    fn apply_filter_and_sort(&mut self) {
        let mut items: Vec<SearchResult> = if let Some(kind) = self.filter_state.active_filter {
            self.all_results
                .iter()
                .filter(|r| r.result_kind == kind)
                .cloned()
                .collect()
        } else {
            self.all_results.clone()
        };

        match self.sort_state.active_sort {
            sort_popup::SortMode::Relevance => {} // keep original order
            sort_popup::SortMode::Package => {
                items.sort_by(|a, b| {
                    let pa = a.package.as_ref().map(|p| &p.name);
                    let pb = b.package.as_ref().map(|p| &p.name);
                    pa.cmp(&pb)
                });
            }
            sort_popup::SortMode::Module => {
                items.sort_by(|a, b| {
                    let ma = a.module.as_ref().map(|m| m.to_string());
                    let mb = b.module.as_ref().map(|m| m.to_string());
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

    fn show_info(&mut self, msg: &str) {
        self.status.set_info(msg);
        self.message_deadline = Some(Instant::now() + std::time::Duration::from_secs(2));
    }

    fn show_error(&mut self, msg: &str) {
        self.status.set_error(msg);
        self.message_deadline = Some(Instant::now() + std::time::Duration::from_secs(3));
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

            if self.mode == AppMode::DocView {
                doc_viewer::render(frame, chunks[0], &mut self.doc_state, &self.theme);
            } else {
                source_viewer::render(frame, chunks[0], &mut self.source_state, &self.theme);
            }
            status_bar::render(frame, chunks[1], &self.status, self.mode, &self.theme);
        } else {
            let ly = layout::compute_layout(area, self.preview_enabled);

            search_bar::render(frame, ly.search_bar, &self.textarea, self.mode, &self.theme);
            result_list::render(frame, ly.result_list, &mut self.results, &self.theme);

            if let Some(preview_area) = ly.preview_pane {
                let selected = self.results.selected_result().cloned();
                preview_pane::render(frame, preview_area, selected.as_ref(), &self.theme);
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
            None => {}
        }

        // Help overlay on top of everything
        if self.mode == AppMode::Help {
            help_overlay::render(frame, &mut self.help_state, &self.theme);
        }
    }
}
