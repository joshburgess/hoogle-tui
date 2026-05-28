use tokio::time::Instant;

use crate::actions::Action;
use crate::app::{App, AppMode};

pub(crate) fn search_textarea() -> tui_textarea::TextArea<'static> {
    let mut textarea = tui_textarea::TextArea::default();
    textarea.set_cursor_line_style(ratatui::style::Style::default());
    textarea.set_placeholder_text("Type to search Hoogle...");
    textarea
}

pub(crate) fn search_textarea_with_query(query: &str) -> tui_textarea::TextArea<'static> {
    let mut textarea = tui_textarea::TextArea::from([query]);
    textarea.set_cursor_line_style(ratatui::style::Style::default());
    textarea.set_placeholder_text("Type to search Hoogle...");
    textarea.move_cursor(tui_textarea::CursorMove::End);
    textarea
}

impl App {
    /// Handle a raw key event when in search mode.
    /// Returns true if the textarea consumed the event.
    pub(crate) fn handle_search_input(&mut self, input: crossterm::event::KeyEvent) -> bool {
        let before = self.query_text();
        let consumed = self.textarea.input(input);
        let after = self.query_text();

        if before != after {
            self.debounce_deadline =
                Some(Instant::now() + std::time::Duration::from_millis(self.config.ui.debounce_ms));
        }

        consumed
    }

    pub(crate) fn handle_fuzzy_filter_input(
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
            KeyCode::Enter if self.results.visible_count() > 0 => {
                self.open_doc_for_selected();
            }
            KeyCode::Enter => {
                self.show_info("No filtered result selected");
            }
            KeyCode::Char(c) => {
                let action = keymap.resolve(
                    AppMode::Results,
                    crossterm::event::KeyEvent::new(key.code, key.modifiers),
                );
                match action {
                    Action::MoveDown
                    | Action::MoveUp
                    | Action::MoveToTop
                    | Action::MoveToBottom
                    | Action::Quit
                    | Action::ToggleHelp
                    | Action::FocusSearch
                    | Action::OpenFilter
                    | Action::OpenSort
                    | Action::Select
                    | Action::TogglePreview
                    | Action::YankSignature
                    | Action::OpenYankMenu
                    | Action::OpenCommandPalette => {
                        self.results.clear_fuzzy_filter();
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

    pub(crate) fn handle_doc_search_input(&mut self, key: crossterm::event::KeyEvent) {
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
                if self.doc_state.search_matches.is_empty() {
                    self.show_info("No document search matches");
                } else {
                    self.doc_state.next_match();
                }
            }
            KeyCode::Char('p')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                if self.doc_state.search_matches.is_empty() {
                    self.show_info("No document search matches");
                } else {
                    self.doc_state.prev_match();
                }
            }
            KeyCode::Char(c) => {
                self.doc_state.search_add_char(c);
            }
            _ => {}
        }
    }

    pub(crate) fn query_text(&self) -> String {
        self.textarea.lines().join("")
    }
}
