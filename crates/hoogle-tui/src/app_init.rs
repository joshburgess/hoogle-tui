use super::app_input::{search_textarea, search_textarea_with_query};
use std::sync::Arc;

use hoogle_core::backend::{BackendError, HoogleBackend};
use hoogle_core::cache::DiskCache;
use hoogle_core::config::Config;
use hoogle_core::haddock::fetcher::HaddockFetcher;
use hoogle_syntax::theme::Theme;
use ratatui::layout::Rect;
use tokio::sync::mpsc;

use crate::app::{App, AppMode};
use crate::bookmarks::{self, BookmarkStore, LoadStatus as BookmarkLoadStatus};
use crate::history::{self, LoadStatus as HistoryLoadStatus, SearchHistory};
use crate::ui::{
    doc_viewer, filter_popup, help_overlay, pinned_panel, preview_pane, result_list, sort_popup,
    source_viewer, status_bar,
};

impl App {
    pub(crate) fn new(
        config: Config,
        backend: Box<dyn HoogleBackend>,
    ) -> Result<Self, BackendError> {
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
        let fetcher = Arc::new(HaddockFetcher::new(cache, config.backend.timeout_secs)?);

        let preview_enabled = config.ui.preview_enabled;
        let (history, history_status) = SearchHistory::load_with_status(history::history_path());
        let (bookmark_store, bookmark_status) =
            BookmarkStore::load_with_status(bookmarks::bookmarks_path());
        let mut status = status_bar::StatusState::new(backend_name);
        if let Some(message) = persistence_load_message(&history_status, &bookmark_status) {
            status.set_error(message);
        }

        Ok(Self {
            mode: AppMode::Search,
            should_quit: false,
            config,
            theme,
            textarea: search_textarea(),
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
            command_palette: None,
            project_scope_enabled: true,
            pinned: pinned_panel::PinnedState::new(),
            preview_state: preview_pane::PreviewState::new(),
            completion_candidates: Vec::new(),
            completion_index: 0,
            viewed_docs: Vec::new(),
            has_more_results: false,
            loading_more: false,
            all_results: Vec::new(),
            doc_state: doc_viewer::DocViewState::new(),
            pending_doc_url: None,
            toc_state: None,
            source_state: source_viewer::SourceViewState::new(),
            pending_source_decl: None,
            help_state: help_overlay::HelpState::new(),
            help_previous_mode: None,
            history,
            bookmark_store,
            history_popup: None,
            bookmarks_popup: None,
            status,
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
            hit_pinned_panel: None,
            hit_doc_area: Rect::default(),
            last_click_time: None,
            last_click_col: 0,
            last_click_row: 0,
        })
    }

    pub(crate) fn set_initial_query(&mut self, query: &str) {
        self.textarea = search_textarea_with_query(query);
        self.trigger_search();
    }
}

fn persistence_load_message(
    history_status: &HistoryLoadStatus,
    bookmark_status: &BookmarkLoadStatus,
) -> Option<String> {
    let mut failed = Vec::new();

    match history_status {
        HistoryLoadStatus::Unreadable(_) => failed.push("history"),
        HistoryLoadStatus::Corrupt(_) => failed.push("history"),
        HistoryLoadStatus::Loaded | HistoryLoadStatus::Missing => {}
    }

    match bookmark_status {
        BookmarkLoadStatus::Unreadable(_) => failed.push("bookmarks"),
        BookmarkLoadStatus::Corrupt(_) => failed.push("bookmarks"),
        BookmarkLoadStatus::Loaded | BookmarkLoadStatus::Missing => {}
    }

    if failed.is_empty() {
        None
    } else {
        Some(format!("Could not load {}", failed.join(" and ")))
    }
}

#[cfg(test)]
mod tests {
    use super::{persistence_load_message, BookmarkLoadStatus, HistoryLoadStatus};

    #[test]
    fn persistence_load_message_ignores_missing_files() {
        assert_eq!(
            persistence_load_message(&HistoryLoadStatus::Missing, &BookmarkLoadStatus::Missing),
            None
        );
    }

    #[test]
    fn persistence_load_message_reports_corrupt_files() {
        assert_eq!(
            persistence_load_message(
                &HistoryLoadStatus::Corrupt("bad json".to_string()),
                &BookmarkLoadStatus::Loaded,
            ),
            Some("Could not load history".to_string())
        );
        assert_eq!(
            persistence_load_message(
                &HistoryLoadStatus::Unreadable("permission denied".to_string()),
                &BookmarkLoadStatus::Corrupt("bad json".to_string()),
            ),
            Some("Could not load history and bookmarks".to_string())
        );
    }
}
