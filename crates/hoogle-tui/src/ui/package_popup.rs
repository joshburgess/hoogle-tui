use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub struct PackageScopeState {
    pub input: String,
    pub packages: Vec<String>,
}

impl PackageScopeState {
    pub fn new(current: &[String]) -> Self {
        let input = current.join(", ");
        Self {
            input,
            packages: current.to_vec(),
        }
    }

    pub fn add_char(&mut self, c: char) {
        self.input.push(c);
    }

    pub fn delete_char(&mut self) {
        self.input.pop();
    }

    pub fn confirm(&mut self) -> Vec<String> {
        self.packages = self
            .input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        self.packages.clone()
    }

    pub fn clear(&mut self) {
        self.input.clear();
        self.packages.clear();
    }
}

pub fn render(frame: &mut Frame, state: &PackageScopeState, theme: &Theme) {
    let area = frame.area();
    let popup_width = 44u16.min(area.width.saturating_sub(4));
    let popup_height = 7u16;
    let popup = centered_popup(area, popup_width, popup_height);

    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Package Scope ")
        .title_bottom(Span::styled(
            " Enter:confirm \u{2502} Esc:cancel \u{2502} Ctrl-u:clear ",
            theme.style(SemanticToken::Comment),
        ))
        .border_style(theme.style(SemanticToken::Border));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let hint_style = theme.style(SemanticToken::Comment);
    let input_style = theme.style(SemanticToken::SearchInput);

    let lines = vec![
        Line::from(Span::styled("Comma-separated package names:", hint_style)),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "> ",
                theme
                    .style(SemanticToken::ModuleName)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(state.input.as_str(), input_style),
            Span::styled("\u{2588}", input_style),
        ]),
        Line::from(""),
        Line::from(Span::styled("e.g.: base, containers, text", hint_style)),
    ];

    frame.render_widget(Paragraph::new(lines), inner);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_empty() {
        let state = PackageScopeState::new(&[]);
        assert!(state.input.is_empty());
        assert!(state.packages.is_empty());
    }

    #[test]
    fn new_with_packages() {
        let pkgs = vec!["base".to_string(), "containers".to_string()];
        let state = PackageScopeState::new(&pkgs);
        assert_eq!(state.input, "base, containers");
        assert_eq!(state.packages, pkgs);
    }

    #[test]
    fn new_single_package() {
        let pkgs = vec!["text".to_string()];
        let state = PackageScopeState::new(&pkgs);
        assert_eq!(state.input, "text");
    }

    #[test]
    fn add_char_appends() {
        let mut state = PackageScopeState::new(&[]);
        state.add_char('b');
        state.add_char('a');
        state.add_char('s');
        state.add_char('e');
        assert_eq!(state.input, "base");
    }

    #[test]
    fn delete_char_removes_last() {
        let mut state = PackageScopeState::new(&[]);
        state.add_char('a');
        state.add_char('b');
        state.delete_char();
        assert_eq!(state.input, "a");
    }

    #[test]
    fn delete_char_on_empty_no_panic() {
        let mut state = PackageScopeState::new(&[]);
        state.delete_char();
        assert!(state.input.is_empty());
    }

    #[test]
    fn confirm_parses_comma_separated() {
        let mut state = PackageScopeState::new(&[]);
        state.input = "base, containers, text".to_string();
        let result = state.confirm();
        assert_eq!(result, vec!["base", "containers", "text"]);
        assert_eq!(state.packages, result);
    }

    #[test]
    fn confirm_trims_whitespace() {
        let mut state = PackageScopeState::new(&[]);
        state.input = "  base ,  containers  ".to_string();
        let result = state.confirm();
        assert_eq!(result, vec!["base", "containers"]);
    }

    #[test]
    fn confirm_filters_empty_entries() {
        let mut state = PackageScopeState::new(&[]);
        state.input = "base,,, containers, ,text".to_string();
        let result = state.confirm();
        assert_eq!(result, vec!["base", "containers", "text"]);
    }

    #[test]
    fn confirm_empty_input_returns_empty() {
        let mut state = PackageScopeState::new(&[]);
        let result = state.confirm();
        assert!(result.is_empty());
    }

    #[test]
    fn confirm_only_commas_returns_empty() {
        let mut state = PackageScopeState::new(&[]);
        state.input = ", , ,".to_string();
        let result = state.confirm();
        assert!(result.is_empty());
    }

    #[test]
    fn clear_resets_everything() {
        let mut state = PackageScopeState::new(&["base".to_string(), "text".to_string()]);
        assert!(!state.input.is_empty());
        state.clear();
        assert!(state.input.is_empty());
        assert!(state.packages.is_empty());
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
