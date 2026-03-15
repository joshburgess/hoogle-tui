use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::history::SearchHistory;

pub struct HistoryPopupState {
    pub selected: usize,
    pub filter: String,
    pub filtered_indices: Vec<usize>,
}

impl HistoryPopupState {
    pub fn new(total: usize) -> Self {
        Self {
            selected: 0,
            filter: String::new(),
            filtered_indices: (0..total).collect(),
        }
    }

    pub fn move_down(&mut self) {
        if !self.filtered_indices.is_empty() && self.selected < self.filtered_indices.len() - 1 {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.filtered_indices.get(self.selected).copied()
    }

    #[allow(dead_code)]
    pub fn update_filter(&mut self, history: &SearchHistory) {
        let query = self.filter.to_lowercase();
        self.filtered_indices = history
            .entries()
            .iter()
            .enumerate()
            .filter(|(_, e)| query.is_empty() || e.query.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
    }
}

pub fn render(
    frame: &mut Frame,
    state: &HistoryPopupState,
    history: &SearchHistory,
    theme: &Theme,
) {
    let area = frame.area();
    let popup_width = (area.width * 3 / 4).min(60);
    let popup_height = (area.height * 3 / 4).min(20);
    let popup = centered_popup(area, popup_width, popup_height);
    frame.render_widget(Clear, popup);

    let title = if state.filter.is_empty() {
        " Search History (Ctrl-d to delete) ".to_string()
    } else {
        format!(" History: {} ", state.filter)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
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

    for (vi, &idx) in state
        .filtered_indices
        .iter()
        .enumerate()
        .skip(scroll)
        .take(max_visible)
    {
        let entry = &history.entries()[idx];
        let is_selected = vi == state.selected;

        let marker = if is_selected { "> " } else { "  " };
        let style = if is_selected {
            theme
                .style(SemanticToken::Selected)
                .add_modifier(Modifier::BOLD)
        } else {
            theme.style(SemanticToken::DocText)
        };

        let count_style = if is_selected {
            theme.style(SemanticToken::Selected)
        } else {
            theme.style(SemanticToken::Comment)
        };

        lines.push(Line::from(vec![
            Span::styled(marker.to_string(), style),
            Span::styled(entry.query.clone(), style),
            Span::styled(format!("  ({} results)", entry.result_count), count_style),
        ]));
    }

    if state.filtered_indices.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No history.",
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
