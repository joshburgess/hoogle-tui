use crate::app::{App, AppMode};

impl App {
    pub(crate) fn handle_back(&mut self) {
        match self.mode {
            AppMode::Search => {
                if self.query_text().is_empty() {
                    self.should_quit = true;
                } else {
                    self.clear_search_state();
                }
            }
            AppMode::Results => self.mode = AppMode::Search,
            AppMode::DocView => {
                self.pending_doc_url = None;
                self.doc_state.loading = false;
                self.mode = AppMode::Results;
            }
            AppMode::SourceView => {
                self.pending_source_decl = None;
                self.source_state.loading = false;
                self.mode = AppMode::DocView;
            }
            AppMode::Help => self.close_help(),
        }
    }

    pub(crate) fn close_help(&mut self) {
        self.mode = self.help_previous_mode.take().unwrap_or(AppMode::Results);
    }

    pub(crate) fn move_selection_or_scroll(&mut self, delta: i16) {
        let amount = delta.unsigned_abs() as usize;
        match (self.mode, delta.is_positive()) {
            (AppMode::DocView, true) => self.doc_state.scroll_down(amount),
            (AppMode::DocView, false) => self.doc_state.scroll_up(amount),
            (AppMode::SourceView, true) => self.source_state.scroll_down(amount),
            (AppMode::SourceView, false) => self.source_state.scroll_up(amount),
            (AppMode::Help, true) => self.help_state.scroll_down(amount),
            (AppMode::Help, false) => self.help_state.scroll_up(amount),
            (_, true) => self.results.move_down(),
            (_, false) => self.results.move_up(),
        }
    }

    pub(crate) fn move_selection_or_scroll_to_edge(&mut self, bottom: bool) {
        match (self.mode, bottom) {
            (AppMode::DocView, true) => self.doc_state.scroll_to_bottom(),
            (AppMode::DocView, false) => self.doc_state.scroll_to_top(),
            (AppMode::SourceView, true) => self.source_state.scroll_to_bottom(),
            (AppMode::SourceView, false) => self.source_state.scroll_to_top(),
            (AppMode::Help, true) => self.help_state.scroll_down(usize::MAX),
            (AppMode::Help, false) => self.help_state.scroll_up(usize::MAX),
            (_, true) => self.results.move_to_bottom(),
            (_, false) => self.results.move_to_top(),
        }
    }

    pub(crate) fn scroll_active_view(&mut self, delta: i16) {
        let amount = delta.unsigned_abs() as usize;
        match (self.mode, delta.is_positive()) {
            (AppMode::DocView, true) => self.doc_state.scroll_down(amount),
            (AppMode::DocView, false) => self.doc_state.scroll_up(amount),
            (AppMode::SourceView, true) => self.source_state.scroll_down(amount),
            (AppMode::SourceView, false) => self.source_state.scroll_up(amount),
            (AppMode::Help, true) => self.help_state.scroll_down(amount),
            (AppMode::Help, false) => self.help_state.scroll_up(amount),
            (AppMode::Results, true) if self.preview_enabled => {
                self.preview_state.scroll_down(amount);
            }
            (AppMode::Results, false) if self.preview_enabled => {
                self.preview_state.scroll_up(amount);
            }
            (AppMode::Results, true) => self.results.move_down(),
            (AppMode::Results, false) => self.results.move_up(),
            (AppMode::Search, _) => {}
        }
    }

    pub(crate) fn scroll_active_view_by_fraction(&mut self, divisor: usize, down: bool) {
        let viewport_height = match self.mode {
            AppMode::DocView => self.doc_state.viewport_height,
            AppMode::SourceView => self.source_state.viewport_height,
            AppMode::Help => self.help_state.viewport_height,
            AppMode::Results if self.preview_enabled => self.preview_state.viewport_height,
            AppMode::Search | AppMode::Results => return,
        };
        let amount = (viewport_height / divisor).max(1);
        self.scroll_active_view_by(amount, down);
    }

    pub(crate) fn scroll_active_view_page(&mut self, down: bool) {
        let viewport_height = match self.mode {
            AppMode::DocView => self.doc_state.viewport_height,
            AppMode::SourceView => self.source_state.viewport_height,
            AppMode::Help => self.help_state.viewport_height,
            AppMode::Results if self.preview_enabled => self.preview_state.viewport_height,
            AppMode::Search | AppMode::Results => return,
        };
        let amount = viewport_height.saturating_sub(2).max(1);
        self.scroll_active_view_by(amount, down);
    }

    fn scroll_active_view_by(&mut self, amount: usize, down: bool) {
        match (self.mode, down) {
            (AppMode::DocView, true) => self.doc_state.scroll_down(amount),
            (AppMode::DocView, false) => self.doc_state.scroll_up(amount),
            (AppMode::SourceView, true) => self.source_state.scroll_down(amount),
            (AppMode::SourceView, false) => self.source_state.scroll_up(amount),
            (AppMode::Help, true) => self.help_state.scroll_down(amount),
            (AppMode::Help, false) => self.help_state.scroll_up(amount),
            (AppMode::Results, true) if self.preview_enabled => {
                self.preview_state.scroll_down(amount);
            }
            (AppMode::Results, false) if self.preview_enabled => {
                self.preview_state.scroll_up(amount);
            }
            (AppMode::Search | AppMode::Results, _) => {}
        }
    }
}
