use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub struct TocState {
    pub items: Vec<TocEntry>,
    pub selected: usize,
    pub filter: String,
    pub filtered_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct TocEntry {
    pub name: String,
    pub signature: Option<String>,
    pub line_offset: usize,
}

impl TocState {
    pub fn new(items: Vec<TocEntry>) -> Self {
        let filtered_indices: Vec<usize> = (0..items.len()).collect();
        Self {
            items,
            selected: 0,
            filter: String::new(),
            filtered_indices,
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

    pub fn selected_offset(&self) -> Option<usize> {
        let idx = *self.filtered_indices.get(self.selected)?;
        Some(self.items[idx].line_offset)
    }

    #[allow(dead_code)]
    pub fn add_filter_char(&mut self, c: char) {
        self.filter.push(c);
        self.apply_filter();
    }

    #[allow(dead_code)]
    pub fn delete_filter_char(&mut self) {
        self.filter.pop();
        self.apply_filter();
    }

    fn apply_filter(&mut self) {
        let query = self.filter.to_lowercase();
        self.filtered_indices = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, entry)| query.is_empty() || entry.name.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
    }
}

pub fn render(frame: &mut Frame, state: &TocState, theme: &Theme) {
    let area = frame.area();

    let popup_width = (area.width * 3 / 4).min(70);
    let popup_height = (area.height * 3 / 4).min(30);

    let popup = centered_popup(area, popup_width, popup_height);
    frame.render_widget(Clear, popup);

    let title = if state.filter.is_empty() {
        " Table of Contents ".to_string()
    } else {
        format!(" TOC: {} ", state.filter)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(theme.style(SemanticToken::Border));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let max_visible = inner.height as usize;
    let total = state.filtered_indices.len();

    // Compute scroll offset to keep selected visible
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
        let entry = &state.items[idx];
        let is_selected = vi == state.selected;

        let marker = if is_selected { "> " } else { "  " };
        let style = if is_selected {
            theme
                .style(SemanticToken::Selected)
                .add_modifier(Modifier::BOLD)
        } else {
            theme.style(SemanticToken::DocText)
        };

        let sig_style = if is_selected {
            theme.style(SemanticToken::Selected)
        } else {
            theme.style(SemanticToken::Comment)
        };

        let sig_text = entry
            .signature
            .as_ref()
            .map(|s| {
                let max_len = (inner.width as usize).saturating_sub(entry.name.len() + 6);
                if s.len() > max_len {
                    format!(" :: {}...", &s[..max_len.saturating_sub(3)])
                } else {
                    format!(" :: {s}")
                }
            })
            .unwrap_or_default();

        lines.push(Line::from(vec![
            Span::styled(marker.to_string(), style),
            Span::styled(entry.name.clone(), style),
            Span::styled(sig_text, sig_style),
        ]));
    }

    if total == 0 {
        lines.push(Line::from(Span::styled(
            "  No declarations found.",
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
