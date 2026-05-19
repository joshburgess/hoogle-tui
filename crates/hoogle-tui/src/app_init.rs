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
use crate::bookmarks::{self, BookmarkStore};
use crate::history::{self, SearchHistory};
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
        })
    }

    pub(crate) fn set_initial_query(&mut self, query: &str) {
        self.textarea = search_textarea_with_query(query);
        self.trigger_search();
    }
}
