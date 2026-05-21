use crate::app::{App, AppMode};

impl App {
    pub(crate) fn switch_mode(&mut self, mode: AppMode) {
        if mode != AppMode::Help {
            self.help_previous_mode = None;
        }
        self.mode = mode;
    }

    pub(crate) fn open_help(&mut self) {
        self.help_state = crate::ui::help_overlay::HelpState::new();
        self.help_previous_mode = Some(self.mode);
        self.mode = AppMode::Help;
    }

    pub(crate) fn handle_back(&mut self) {
        match self.mode {
            AppMode::Search => {
                if self.query_text().is_empty() {
                    self.should_quit = true;
                } else {
                    self.clear_search_state();
                }
            }
            AppMode::Results => self.switch_mode(AppMode::Search),
            AppMode::DocView => {
                self.pending_doc_url = None;
                self.doc_state.loading = false;
                self.switch_mode(AppMode::Results);
            }
            AppMode::SourceView => {
                self.pending_source_decl = None;
                self.source_state.loading = false;
                self.switch_mode(AppMode::DocView);
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
            AppMode::Results => {
                let amount = (self.results_page_step() / divisor).max(1);
                self.move_results_by(amount, down);
                return;
            }
            AppMode::Search => return,
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
            AppMode::Results => {
                self.move_results_by(self.results_page_step(), down);
                return;
            }
            AppMode::Search => return,
        };
        let amount = viewport_height.saturating_sub(2).max(1);
        self.scroll_active_view_by(amount, down);
    }

    fn results_page_step(&self) -> usize {
        let visible_rows = usize::from(self.hit_result_list.height).saturating_sub(2);
        let visible_results = visible_rows / self.results.lines_per_result();
        visible_results.saturating_sub(1).max(1)
    }

    fn move_results_by(&mut self, amount: usize, down: bool) {
        for _ in 0..amount {
            if down {
                self.results.move_down();
            } else {
                self.results.move_up();
            }
        }
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
