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
            AppMode::DocView => self.mode = AppMode::Results,
            AppMode::SourceView => self.mode = AppMode::DocView,
            AppMode::Help => self.mode = AppMode::Results,
        }
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
            (AppMode::Results, true) => self.results.move_down(),
            (AppMode::Results, false) => self.results.move_up(),
            (AppMode::Search, _) => {}
        }
    }

    pub(crate) fn scroll_doc_by_fraction(&mut self, divisor: usize, down: bool) {
        let amount = (self.doc_state.viewport_height / divisor).max(1);
        if down {
            self.doc_state.scroll_down(amount);
        } else {
            self.doc_state.scroll_up(amount);
        }
    }

    pub(crate) fn scroll_doc_page(&mut self, down: bool) {
        let amount = self.doc_state.viewport_height.saturating_sub(2).max(1);
        if down {
            self.doc_state.scroll_down(amount);
        } else {
            self.doc_state.scroll_up(amount);
        }
    }
}
