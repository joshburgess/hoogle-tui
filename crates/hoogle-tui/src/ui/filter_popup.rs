use hoogle_core::models::ResultKind;
use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

#[derive(Debug, Clone)]
pub struct FilterState {
    pub selected: usize,
    pub active_filter: Option<ResultKind>,
}

const FILTER_OPTIONS: &[(Option<ResultKind>, &str)] = &[
    (None, "All"),
    (Some(ResultKind::Function), "Functions"),
    (Some(ResultKind::DataType), "Data Types"),
    (Some(ResultKind::TypeAlias), "Type Aliases"),
    (Some(ResultKind::Newtype), "Newtypes"),
    (Some(ResultKind::Class), "Classes"),
    (Some(ResultKind::Module), "Modules"),
    (Some(ResultKind::Package), "Packages"),
];

impl FilterState {
    pub fn new() -> Self {
        Self {
            selected: 0,
            active_filter: None,
        }
    }

    pub fn move_down(&mut self) {
        if self.selected < FILTER_OPTIONS.len() - 1 {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn confirm(&mut self) -> Option<ResultKind> {
        let (filter, _) = FILTER_OPTIONS[self.selected];
        self.active_filter = filter;
        filter
    }

    /// Sync selected index with the current active filter.
    pub fn sync_selection(&mut self) {
        self.selected = FILTER_OPTIONS
            .iter()
            .position(|(f, _)| *f == self.active_filter)
            .unwrap_or(0);
    }
}

pub fn render(frame: &mut Frame, state: &FilterState, theme: &Theme) {
    let area = frame.area();

    let popup_width = 22u16;
    let popup_height = (FILTER_OPTIONS.len() as u16) + 2; // +2 for border

    let popup = centered_popup(area, popup_width, popup_height);

    // Clear background
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Filter Results ")
        .border_style(theme.style(SemanticToken::Border));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let lines: Vec<Line> = FILTER_OPTIONS
        .iter()
        .enumerate()
        .map(|(i, (filter, label))| {
            let is_selected = i == state.selected;
            let is_active = *filter == state.active_filter;

            let bullet = if is_active { "\u{25cf} " } else { "\u{25cb} " };

            let style = if is_selected {
                theme
                    .style(SemanticToken::Selected)
                    .add_modifier(Modifier::BOLD)
            } else {
                theme.style(SemanticToken::DocText)
            };

            Line::from(vec![
                Span::styled(bullet, style),
                Span::styled(*label, style),
            ])
        })
        .collect();

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
