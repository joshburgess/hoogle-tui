use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub const THEME_NAMES: &[&str] = &[
    "dracula",
    "catppuccin_mocha",
    "gruvbox_dark",
    "solarized_dark",
    "monokai",
    "nord",
];

pub struct ThemePopupState {
    pub selected: usize,
    pub active: usize,
}

impl ThemePopupState {
    pub fn new(current_theme: &str) -> Self {
        let active = THEME_NAMES
            .iter()
            .position(|&n| n == current_theme)
            .unwrap_or(0);
        Self {
            selected: active,
            active,
        }
    }

    pub fn move_down(&mut self) {
        if self.selected < THEME_NAMES.len() - 1 {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn confirm(&mut self) -> &'static str {
        self.active = self.selected;
        THEME_NAMES[self.selected]
    }
}

pub fn render(frame: &mut Frame, state: &ThemePopupState, theme: &Theme) {
    let area = frame.area();
    let popup_width = 26u16;
    let popup_height = (THEME_NAMES.len() as u16) + 2;
    let popup = centered_popup(area, popup_width, popup_height);

    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Switch Theme ")
        .border_style(theme.style(SemanticToken::Border));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let lines: Vec<Line> = THEME_NAMES
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let is_selected = i == state.selected;
            let is_active = i == state.active;
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
                Span::styled(*name, style),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), inner);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_with_known_theme() {
        let state = ThemePopupState::new("dracula");
        assert_eq!(state.selected, 0);
        assert_eq!(state.active, 0);
    }

    #[test]
    fn new_with_catppuccin() {
        let state = ThemePopupState::new("catppuccin_mocha");
        assert_eq!(state.selected, 1);
        assert_eq!(state.active, 1);
    }

    #[test]
    fn new_with_nord() {
        let state = ThemePopupState::new("nord");
        let expected = THEME_NAMES.iter().position(|&n| n == "nord").unwrap();
        assert_eq!(state.selected, expected);
        assert_eq!(state.active, expected);
    }

    #[test]
    fn new_with_unknown_theme_defaults_to_zero() {
        let state = ThemePopupState::new("nonexistent");
        assert_eq!(state.selected, 0);
        assert_eq!(state.active, 0);
    }

    #[test]
    fn new_with_empty_string() {
        let state = ThemePopupState::new("");
        assert_eq!(state.selected, 0);
        assert_eq!(state.active, 0);
    }

    #[test]
    fn move_down_increments() {
        let mut state = ThemePopupState::new("dracula");
        state.move_down();
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn move_down_clamps_at_end() {
        let mut state = ThemePopupState::new("dracula");
        for _ in 0..20 {
            state.move_down();
        }
        assert_eq!(state.selected, THEME_NAMES.len() - 1);
    }

    #[test]
    fn move_up_decrements() {
        let mut state = ThemePopupState::new("dracula");
        state.move_down();
        state.move_down();
        state.move_up();
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn move_up_clamps_at_zero() {
        let mut state = ThemePopupState::new("dracula");
        state.move_up();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn confirm_returns_selected_name() {
        let mut state = ThemePopupState::new("dracula");
        state.move_down(); // catppuccin_mocha
        let name = state.confirm();
        assert_eq!(name, "catppuccin_mocha");
        assert_eq!(state.active, 1);
    }

    #[test]
    fn confirm_first_returns_dracula() {
        let mut state = ThemePopupState::new("dracula");
        let name = state.confirm();
        assert_eq!(name, "dracula");
        assert_eq!(state.active, 0);
    }

    #[test]
    fn confirm_last() {
        let mut state = ThemePopupState::new("dracula");
        for _ in 0..THEME_NAMES.len() {
            state.move_down();
        }
        let name = state.confirm();
        assert_eq!(name, THEME_NAMES[THEME_NAMES.len() - 1]);
    }

    #[test]
    fn confirm_updates_active() {
        let mut state = ThemePopupState::new("dracula");
        assert_eq!(state.active, 0);
        state.move_down();
        state.move_down();
        state.confirm();
        assert_eq!(state.active, 2);
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
