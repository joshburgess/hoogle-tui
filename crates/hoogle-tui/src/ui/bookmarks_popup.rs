use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::popup_layout::centered_popup;
use super::text::{display_width, truncate_width};
use crate::bookmarks::BookmarkStore;

pub struct BookmarksPopupState {
    pub selected: usize,
}

impl BookmarksPopupState {
    pub fn new() -> Self {
        Self { selected: 0 }
    }

    pub fn move_down(&mut self, total: usize) {
        if total > 0 && self.selected < total - 1 {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn clamp_selection(&mut self, total: usize) {
        if total == 0 {
            self.selected = 0;
        } else if self.selected >= total {
            self.selected = total - 1;
        }
    }
}

pub fn render(
    frame: &mut Frame,
    state: &BookmarksPopupState,
    store: &BookmarkStore,
    theme: &Theme,
) {
    let area = frame.area();
    let popup_width = (area.width * 3 / 4).min(70);
    let popup_height = (area.height * 3 / 4).min(20);
    let popup = centered_popup(area, popup_width, popup_height);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Bookmarks (d to delete) ")
        .border_style(theme.style(SemanticToken::Border));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let max_visible = inner.height as usize;

    let scroll = if state.selected >= max_visible {
        state.selected - max_visible + 1
    } else {
        0
    };

    let mut lines: Vec<Line> = Vec::new();
    let bookmarks = store.bookmarks();

    for (vi, bm) in bookmarks.iter().enumerate().skip(scroll).take(max_visible) {
        let is_selected = vi == state.selected;
        let marker = if is_selected { "> " } else { "  " };
        let style = if is_selected {
            theme
                .style(SemanticToken::Selected)
                .add_modifier(Modifier::BOLD)
        } else {
            theme.style(SemanticToken::DocText)
        };

        let module_str = bm
            .module
            .as_ref()
            .map(|m| format!(" ({m})"))
            .unwrap_or_default();

        let sig_str = bm
            .signature
            .as_ref()
            .map(|s| {
                let max = (inner.width as usize)
                    .saturating_sub(display_width(&bm.name) + display_width(&module_str) + 8);
                if display_width(s) > max {
                    format!(" :: {}", truncate_width(s, max, "..."))
                } else {
                    format!(" :: {s}")
                }
            })
            .unwrap_or_default();

        let meta_style = if is_selected {
            theme.style(SemanticToken::Selected)
        } else {
            theme.style(SemanticToken::Comment)
        };

        lines.push(Line::from(vec![
            Span::styled(marker.to_string(), style),
            Span::styled(bm.name.clone(), style),
            Span::styled(module_str, meta_style),
            Span::styled(sig_str, meta_style),
        ]));
    }

    if bookmarks.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No bookmarks. Press m on a result to bookmark it.",
            theme.style(SemanticToken::Comment),
        )));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}
