use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

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

    /// Scroll to a specific line number (1-based).
    #[allow(dead_code)]
    pub fn scroll_to_line(&mut self, line: usize) {
        self.scroll_offset = line.saturating_sub(1);
    }
}

pub fn render(frame: &mut Frame, area: Rect, state: &mut SourceViewState, theme: &Theme) {
    let title = if state.title.is_empty() {
        " Source ".to_string()
    } else {
        format!(" Source: {} ", state.title)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(theme.style(SemanticToken::Border));

    let inner = block.inner(area);
    state.viewport_height = inner.height as usize;

    if state.loading {
        let loading = Paragraph::new(Line::from(Span::styled(
            "  Loading source code...",
            theme.style(SemanticToken::Spinner),
        )))
        .block(block);
        frame.render_widget(loading, area);
        return;
    }

    if let Some(ref err) = state.error {
        let error = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  {err}"),
                theme.style(SemanticToken::Error),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Press Esc to go back.",
                theme.style(SemanticToken::Comment),
            )),
        ])
        .block(block);
        frame.render_widget(error, area);
        return;
    }

    if state.rendered_lines.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "  No source loaded.",
            theme.style(SemanticToken::Comment),
        )))
        .block(block);
        frame.render_widget(empty, area);
        return;
    }

    let total = state.rendered_lines.len();
    let start = state.scroll_offset.min(total);
    let end = (start + inner.height as usize).min(total);
    let visible: Vec<Line> = state.rendered_lines[start..end].to_vec();

    // Line count indicator
    let block = block.title_bottom(Line::from(vec![Span::styled(
        format!(" {}-{}/{} ", start + 1, end, total),
        theme.style(SemanticToken::Comment),
    )]));

    let paragraph = Paragraph::new(visible).block(block);
    frame.render_widget(paragraph, area);

    // Scrollbar
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
