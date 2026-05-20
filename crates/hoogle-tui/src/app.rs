use std::sync::Arc;

use hoogle_core::backend::{BackendError, HoogleBackend};
use hoogle_core::config::Config;
use hoogle_core::haddock::fetcher::HaddockFetcher;
use hoogle_core::haddock::types::HaddockDoc;
use hoogle_core::models::SearchResult;
use hoogle_syntax::theme::Theme;
use ratatui::layout::Rect;
use tokio::sync::mpsc;
use tokio::time::Instant;
use tui_textarea::TextArea;
use url::Url;

use crate::bookmarks::BookmarkStore;
use crate::history::SearchHistory;
use crate::ui::{
    bookmarks_popup, doc_viewer, filter_popup, help_overlay, history_popup, module_browser,
    package_popup, pinned_panel, preview_pane, result_list, sort_popup, source_viewer, status_bar,
    theme_popup, toc_popup, yank_popup,
};

#[path = "app_actions.rs"]
mod app_actions;
#[path = "app_commands.rs"]
mod app_commands;
#[path = "app_docs.rs"]
mod app_docs;
#[path = "app_init.rs"]
mod app_init;
#[path = "app_input.rs"]
mod app_input;
#[path = "app_mouse.rs"]
mod app_mouse;
#[path = "app_navigation.rs"]
mod app_navigation;
#[path = "app_popups.rs"]
mod app_popups;
#[path = "app_render.rs"]
mod app_render;
#[path = "app_results.rs"]
mod app_results;
#[path = "app_runtime.rs"]
mod app_runtime;
#[path = "app_search.rs"]
mod app_search;

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
    pub append: bool,
    pub results: Result<Vec<SearchResult>, BackendError>,
}

/// Message sent from async doc fetch tasks.
pub struct DocResponse {
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

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
