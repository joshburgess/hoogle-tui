use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

const YANK_OPTIONS: &[&str] = &[
    "Type signature",
    "Qualified name",
    "Import statement",
    "Hackage URL",
    "GHCi :type command",
    "GHCi :info command",
    "Deep link (with anchor)",
];

pub struct YankPopupState {
    pub selected: usize,
}

impl YankPopupState {
    pub fn new() -> Self {
        Self { selected: 0 }
    }

    pub fn move_down(&mut self) {
        if self.selected < YANK_OPTIONS.len() - 1 {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }
}

pub fn render(frame: &mut Frame, state: &YankPopupState, theme: &Theme) {
    let area = frame.area();
    let popup_width = 30u16;
    let popup_height = (YANK_OPTIONS.len() as u16) + 2;
    let popup = centered_popup(area, popup_width, popup_height);

    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Copy to Clipboard ")
        .border_style(theme.style(SemanticToken::Border));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let lines: Vec<Line> = YANK_OPTIONS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let is_selected = i == state.selected;
            let marker = if is_selected { "> " } else { "  " };
            let style = if is_selected {
                theme
                    .style(SemanticToken::Selected)
                    .add_modifier(Modifier::BOLD)
            } else {
                theme.style(SemanticToken::DocText)
            };
            Line::from(vec![
                Span::styled(marker, style),
                Span::styled(*label, style),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), inner);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_at_zero() {
        let state = YankPopupState::new();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn move_down_increments() {
        let mut state = YankPopupState::new();
        state.move_down();
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn move_down_clamps_at_end() {
        let mut state = YankPopupState::new();
        for _ in 0..20 {
            state.move_down();
        }
        assert_eq!(state.selected, YANK_OPTIONS.len() - 1);
    }

    #[test]
    fn move_up_from_zero_stays() {
        let mut state = YankPopupState::new();
        state.move_up();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn move_up_decrements() {
        let mut state = YankPopupState::new();
        state.move_down();
        state.move_down();
        state.move_up();
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn move_down_then_up_roundtrip() {
        let mut state = YankPopupState::new();
        state.move_down();
        state.move_down();
        state.move_down();
        state.move_up();
        state.move_up();
        state.move_up();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn bounds_check_all_positions() {
        let mut state = YankPopupState::new();
        for i in 0..YANK_OPTIONS.len() {
            assert_eq!(state.selected, i);
            state.move_down();
        }
        // Should be clamped at last
        assert_eq!(state.selected, YANK_OPTIONS.len() - 1);
        // Move down again should not exceed
        state.move_down();
        assert_eq!(state.selected, YANK_OPTIONS.len() - 1);
    }

    #[test]
    fn option_count_is_seven() {
        // Verify the constant matches expectations
        assert_eq!(YANK_OPTIONS.len(), 7);
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
