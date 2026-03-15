use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

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
                let max =
                    (inner.width as usize).saturating_sub(bm.name.len() + module_str.len() + 8);
                if s.len() > max {
                    format!(" :: {}...", &s[..max.saturating_sub(3)])
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

fn centered_popup(area: Rect, width: u16, height: u16) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Length((area.height.saturating_sub(height)) / 2),
        Constraint::Length(height),
        Constraint::Min(0),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Length((area.width.saturating_sub(width)) / 2),
        Constraint::Length(width),
        Constraint::Min(0),
    ])
    .split(vertical[1])[1]
}
