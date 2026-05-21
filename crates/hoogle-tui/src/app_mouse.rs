use tokio::time::Instant;

use crate::app::{App, AppMode};

fn rect_contains(rect: ratatui::layout::Rect, col: u16, row: u16) -> bool {
    col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

impl App {
    pub(crate) fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) {
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
        let is_double_click = self
            .last_click_time
            .map(|t| {
                now.duration_since(t) < std::time::Duration::from_millis(400)
                    && self.last_click_row == row
            })
            .unwrap_or(false);

        self.last_click_time = Some(now);
        self.last_click_row = row;

        if self.popup.is_some() {
            self.close_popup();
            return;
        }

        if self.mode == AppMode::Search || self.mode == AppMode::Results {
            if rect_contains(self.hit_search_bar, col, row) {
                self.switch_mode(AppMode::Search);
                return;
            }

            if rect_contains(self.hit_result_list, col, row) {
                if self.mode != AppMode::Results && !self.results.items.is_empty() {
                    self.switch_mode(AppMode::Results);
                }

                let inner_top = self.hit_result_list.y + 1;
                if row > inner_top {
                    let relative_row = (row - inner_top) as usize;
                    let lines_per_result = self.results.lines_per_result();
                    let clicked_index =
                        self.results.scroll_offset + relative_row / lines_per_result;
                    let visible_count = self.results.visible_count();
                    if clicked_index < visible_count {
                        self.results.selected = clicked_index;
                        if is_double_click {
                            self.open_doc_for_selected();
                        }
                    }
                }
                return;
            }

            if let Some(preview_rect) = self.hit_preview_pane {
                if rect_contains(preview_rect, col, row) {
                    if is_double_click {
                        self.open_doc_for_selected();
                    }
                    return;
                }
            }
        }

        if self.mode == AppMode::Help {
            self.close_help();
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
                if rect_contains(self.hit_result_list, col, row) {
                    if down {
                        self.results.move_down();
                    } else {
                        self.results.move_up();
                    }
                } else if self
                    .hit_pinned_panel
                    .is_some_and(|r| rect_contains(r, col, row))
                {
                    if down {
                        self.pinned.scroll_down(3);
                    } else {
                        self.pinned.scroll_up(3);
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
}
