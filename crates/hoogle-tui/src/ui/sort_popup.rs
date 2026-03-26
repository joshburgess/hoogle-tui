use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Relevance,
    Package,
    Module,
    Name,
}

const SORT_OPTIONS: &[(SortMode, &str)] = &[
    (SortMode::Relevance, "Relevance"),
    (SortMode::Package, "Package"),
    (SortMode::Module, "Module"),
    (SortMode::Name, "Name"),
];

#[derive(Debug, Clone)]
pub struct SortState {
    pub selected: usize,
    pub active_sort: SortMode,
}

impl SortState {
    pub fn new() -> Self {
        Self {
            selected: 0,
            active_sort: SortMode::Relevance,
        }
    }

    pub fn move_down(&mut self) {
        if self.selected < SORT_OPTIONS.len() - 1 {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn confirm(&mut self) -> SortMode {
        let (mode, _) = SORT_OPTIONS[self.selected];
        self.active_sort = mode;
        mode
    }

    pub fn sync_selection(&mut self) {
        self.selected = SORT_OPTIONS
            .iter()
            .position(|(m, _)| *m == self.active_sort)
            .unwrap_or(0);
    }
}

pub fn render(frame: &mut Frame, state: &SortState, theme: &Theme) {
    let area = frame.area();

    let popup_width = 20u16;
    let popup_height = (SORT_OPTIONS.len() as u16) + 2;

    let popup = centered_popup(area, popup_width, popup_height);

    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Sort Results ")
        .border_style(theme.style(SemanticToken::Border));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let lines: Vec<Line> = SORT_OPTIONS
        .iter()
        .enumerate()
        .map(|(i, (mode, label))| {
            let is_selected = i == state.selected;
            let is_active = *mode == state.active_sort;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults() {
        let state = SortState::new();
        assert_eq!(state.selected, 0);
        assert_eq!(state.active_sort, SortMode::Relevance);
    }

    #[test]
    fn move_down_increments() {
        let mut state = SortState::new();
        state.move_down();
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn move_down_clamps_at_end() {
        let mut state = SortState::new();
        for _ in 0..20 {
            state.move_down();
        }
        assert_eq!(state.selected, SORT_OPTIONS.len() - 1);
    }

    #[test]
    fn move_up_decrements() {
        let mut state = SortState::new();
        state.move_down();
        state.move_down();
        state.move_up();
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn move_up_clamps_at_zero() {
        let mut state = SortState::new();
        state.move_up();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn confirm_returns_relevance_by_default() {
        let mut state = SortState::new();
        let mode = state.confirm();
        assert_eq!(mode, SortMode::Relevance);
        assert_eq!(state.active_sort, SortMode::Relevance);
    }

    #[test]
    fn confirm_package() {
        let mut state = SortState::new();
        state.move_down(); // Package
        let mode = state.confirm();
        assert_eq!(mode, SortMode::Package);
        assert_eq!(state.active_sort, SortMode::Package);
    }

    #[test]
    fn confirm_module() {
        let mut state = SortState::new();
        state.move_down();
        state.move_down(); // Module
        let mode = state.confirm();
        assert_eq!(mode, SortMode::Module);
    }

    #[test]
    fn confirm_name() {
        let mut state = SortState::new();
        for _ in 0..3 {
            state.move_down();
        }
        let mode = state.confirm();
        assert_eq!(mode, SortMode::Name);
    }

    #[test]
    fn sync_selection_matches_active_sort() {
        let mut state = SortState::new();
        state.active_sort = SortMode::Module;
        state.sync_selection();
        assert_eq!(state.selected, 2);
    }

    #[test]
    fn sync_selection_relevance() {
        let mut state = SortState::new();
        state.selected = 3;
        state.active_sort = SortMode::Relevance;
        state.sync_selection();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn confirm_then_sync_roundtrip() {
        let mut state = SortState::new();
        state.move_down(); // Package
        state.confirm();
        assert_eq!(state.active_sort, SortMode::Package);

        state.selected = 0;
        state.sync_selection();
        assert_eq!(state.selected, 1);
    }
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
