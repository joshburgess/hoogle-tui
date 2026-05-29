use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use super::text::{truncate_line, truncate_width};

pub struct SourceViewState {
    pub source: Option<String>,
    pub rendered_lines: Vec<Line<'static>>,
    pub scroll_offset: usize,
    pub viewport_height: usize,
    pub loading: bool,
    pub error: Option<String>,
    pub title: String,
}

impl SourceViewState {
    pub fn new() -> Self {
        Self {
            source: None,
            rendered_lines: Vec::new(),
            scroll_offset: 0,
            viewport_height: 0,
            loading: false,
            error: None,
            title: String::new(),
        }
    }

    pub fn start_loading(&mut self, title: impl Into<String>) {
        self.source = None;
        self.rendered_lines.clear();
        self.scroll_offset = 0;
        self.loading = true;
        self.error = None;
        self.title = title.into();
    }

    pub fn set_source(&mut self, source: String, decl_name: &str, theme: &Theme) {
        self.title = decl_name.to_string();
        let highlighted = hoogle_syntax::highlight_code(&source, theme);

        // Add line numbers
        let total_lines = highlighted.len();
        let gutter_width = total_lines.to_string().len();

        self.rendered_lines = highlighted
            .into_iter()
            .enumerate()
            .map(|(i, code_line)| {
                let line_num = i + 1;
                let mut spans = vec![
                    Span::styled(
                        format!("{:>gutter_width$} ", line_num),
                        theme.style(SemanticToken::Comment),
                    ),
                    Span::styled("\u{2502} ".to_string(), theme.style(SemanticToken::Border)),
                ];
                spans.extend(
                    code_line
                        .spans
                        .into_iter()
                        .map(|s| Span::styled(s.content.to_string(), s.style)),
                );
                Line::from(spans)
            })
            .collect();

        self.source = Some(source);
        self.scroll_offset = 0;
        self.loading = false;
        self.error = None;
    }

    pub fn scroll_down(&mut self, n: usize) {
        let max = self
            .rendered_lines
            .len()
            .saturating_sub(self.viewport_height);
        self.scroll_offset = (self.scroll_offset + n).min(max);
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        let max = self
            .rendered_lines
            .len()
            .saturating_sub(self.viewport_height);
        self.scroll_offset = max;
    }

    pub fn scroll_to_first_match(&mut self, needle: &str) -> bool {
        if needle.is_empty() {
            return false;
        }

        let Some(source) = self.source.as_ref() else {
            return false;
        };
        let Some(line_index) = source.lines().position(|line| line.contains(needle)) else {
            return false;
        };

        self.scroll_to_line(line_index + 1);
        true
    }

    /// Scroll to a specific line number (1-based).
    pub fn scroll_to_line(&mut self, line: usize) {
        self.scroll_offset = line.saturating_sub(1);
    }
}

pub fn render(frame: &mut Frame, area: Rect, state: &mut SourceViewState, theme: &Theme) {
    let raw_title = if state.title.is_empty() {
        " Source ".to_string()
    } else {
        format!(" Source: {} ", state.title)
    };
    let title = truncate_width(&raw_title, area.width.saturating_sub(2) as usize, "...");

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(theme.style(SemanticToken::Border));

    let inner = block.inner(area);
    state.viewport_height = inner.height as usize;

    if state.loading {
        let message = truncate_width("  Loading source code...", inner.width as usize, "...");
        let loading = Paragraph::new(Line::from(Span::styled(
            message,
            theme.style(SemanticToken::Spinner),
        )))
        .block(block);
        frame.render_widget(loading, area);
        return;
    }

    if let Some(ref err) = state.error {
        let message = truncate_width(&format!("  {err}"), inner.width as usize, "...");
        let hint = truncate_width("  Press Esc to go back.", inner.width as usize, "...");
        let error = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(message, theme.style(SemanticToken::Error))),
            Line::from(""),
            Line::from(Span::styled(hint, theme.style(SemanticToken::Comment))),
        ])
        .block(block);
        frame.render_widget(error, area);
        return;
    }

    if state.rendered_lines.is_empty() {
        let message = truncate_width("  No source loaded.", inner.width as usize, "...");
        let empty = Paragraph::new(Line::from(Span::styled(
            message,
            theme.style(SemanticToken::Comment),
        )))
        .block(block);
        frame.render_widget(empty, area);
        return;
    }

    let total = state.rendered_lines.len();
    let start = state.scroll_offset.min(total);
    let end = (start + inner.height as usize).min(total);
    let visible: Vec<Line> = state.rendered_lines[start..end]
        .iter()
        .map(|line| truncate_line(line, inner.width as usize))
        .collect();

    let range = truncate_width(
        &format!(" {}-{}/{} ", start + 1, end, total),
        area.width.saturating_sub(2) as usize,
        "...",
    );
    let block = block.title_bottom(Line::from(vec![Span::styled(
        range,
        theme.style(SemanticToken::Comment),
    )]));

    let paragraph = Paragraph::new(visible).block(block);
    frame.render_widget(paragraph, area);

    if total > inner.height as usize {
        let mut scrollbar_state = ScrollbarState::new(total.saturating_sub(inner.height as usize))
            .position(state.scroll_offset);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area,
            &mut scrollbar_state,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hoogle_syntax::theme::Theme;

    #[test]
    fn new_defaults() {
        let state = SourceViewState::new();
        assert_eq!(state.source.as_deref(), None);
        assert!(state.rendered_lines.is_empty());
        assert_eq!(state.scroll_offset, 0);
        assert_eq!(state.viewport_height, 0);
        assert!(!state.loading);
        assert_eq!(state.error.as_deref(), None);
        assert!(state.title.is_empty());
    }

    #[test]
    fn scroll_down_basic() {
        let mut state = SourceViewState::new();
        state.rendered_lines = (0..20).map(|_| Line::from("x")).collect();
        state.viewport_height = 10;

        state.scroll_down(3);
        assert_eq!(state.scroll_offset, 3);
    }

    #[test]
    fn scroll_down_clamps_to_max() {
        let mut state = SourceViewState::new();
        state.rendered_lines = (0..20).map(|_| Line::from("x")).collect();
        state.viewport_height = 10;

        state.scroll_down(100);
        assert_eq!(state.scroll_offset, 10);
    }

    #[test]
    fn scroll_down_when_no_lines() {
        let mut state = SourceViewState::new();
        state.viewport_height = 10;
        state.scroll_down(5);
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn scroll_up_basic() {
        let mut state = SourceViewState::new();
        state.scroll_offset = 5;
        state.scroll_up(3);
        assert_eq!(state.scroll_offset, 2);
    }

    #[test]
    fn scroll_up_clamps_to_zero() {
        let mut state = SourceViewState::new();
        state.scroll_offset = 2;
        state.scroll_up(10);
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn scroll_to_top() {
        let mut state = SourceViewState::new();
        state.scroll_offset = 50;
        state.scroll_to_top();
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn scroll_to_bottom() {
        let mut state = SourceViewState::new();
        state.rendered_lines = (0..30).map(|_| Line::from("x")).collect();
        state.viewport_height = 10;
        state.scroll_to_bottom();
        assert_eq!(state.scroll_offset, 20);
    }

    #[test]
    fn scroll_to_bottom_empty() {
        let mut state = SourceViewState::new();
        state.viewport_height = 10;
        state.scroll_to_bottom();
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn set_source_populates_rendered_lines() {
        let mut state = SourceViewState::new();
        let theme = Theme::dracula();
        let source = "module Main where\n\nmain :: IO ()\nmain = putStrLn \"hello\"\n".to_string();
        state.set_source(source.clone(), "main", &theme);

        assert_eq!(state.source.as_deref(), Some(source.as_str()));
        assert!(!state.rendered_lines.is_empty());
        assert!(state.rendered_lines.len() >= 4);
        assert_eq!(state.title, "main");
        assert_eq!(state.scroll_offset, 0);
        assert!(!state.loading);
        assert_eq!(state.error.as_deref(), None);
    }

    #[test]
    fn set_source_resets_scroll() {
        let mut state = SourceViewState::new();
        let theme = Theme::dracula();
        state.scroll_offset = 50;
        state.loading = true;
        state.error = Some("old error".to_string());

        state.set_source("x = 1".to_string(), "x", &theme);
        assert_eq!(state.scroll_offset, 0);
        assert!(!state.loading);
        assert_eq!(state.error.as_deref(), None);
    }

    #[test]
    fn start_loading_clears_stale_source() {
        let mut state = SourceViewState::new();
        state.source = Some("old = 1".to_string());
        state.rendered_lines = vec![Line::from("old = 1")];
        state.scroll_offset = 9;
        state.error = Some("old error".to_string());

        state.start_loading("newDecl");

        assert_eq!(state.source.as_deref(), None);
        assert!(state.rendered_lines.is_empty());
        assert_eq!(state.scroll_offset, 0);
        assert!(state.loading);
        assert_eq!(state.error.as_deref(), None);
        assert_eq!(state.title, "newDecl");
    }

    #[test]
    fn scroll_to_line_one_based() {
        let mut state = SourceViewState::new();
        state.scroll_to_line(5);
        assert_eq!(state.scroll_offset, 4);
    }

    #[test]
    fn scroll_to_line_zero_stays_zero() {
        let mut state = SourceViewState::new();
        state.scroll_to_line(0);
        assert_eq!(state.scroll_offset, 0);
    }
}
