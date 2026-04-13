use hoogle_core::models::SearchResult;
use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

pub struct PreviewState {
    pub scroll_offset: usize,
    pub total_lines: usize,
    pub viewport_height: usize,
    /// Track which result we last rendered to reset scroll on change.
    last_result_name: String,
}

impl PreviewState {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
            total_lines: 0,
            viewport_height: 0,
            last_result_name: String::new(),
        }
    }

    pub fn scroll_down(&mut self, n: usize) {
        let max = self.total_lines.saturating_sub(self.viewport_height);
        self.scroll_offset = (self.scroll_offset + n).min(max);
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    fn reset_if_changed(&mut self, name: &str) {
        if self.last_result_name != name {
            self.scroll_offset = 0;
            self.last_result_name = name.to_string();
        }
    }
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    result: Option<&SearchResult>,
    state: &mut PreviewState,
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Preview ")
        .border_style(theme.style(SemanticToken::Border));

    let Some(result) = result else {
        state.last_result_name.clear();
        let empty = Paragraph::new(Line::from(Span::styled(
            "  Select a result to preview",
            theme.style(SemanticToken::Comment),
        )))
        .block(block);
        frame.render_widget(empty, area);
        return;
    };

    state.reset_if_changed(&result.name);

    let inner = block.inner(area);
    state.viewport_height = inner.height as usize;

    let mut lines: Vec<Line> = Vec::new();

    // Module + package header
    let module_str = result
        .module
        .as_ref()
        .map(|m| m.to_string())
        .unwrap_or_default();
    let pkg_str = result
        .package
        .as_ref()
        .map(|p| p.to_string())
        .unwrap_or_default();

    if !module_str.is_empty() || !pkg_str.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(&module_str, theme.style(SemanticToken::ModuleName)),
            Span::styled("  ", theme.style(SemanticToken::DocText)),
            Span::styled(&pkg_str, theme.style(SemanticToken::PackageName)),
        ]));
        lines.push(Line::from(""));
    }

    // Type signature (syntax-highlighted)
    if let Some(ref sig) = result.signature {
        let full_sig = format!("{} :: {sig}", result.name);
        let highlighted = hoogle_syntax::highlight_signature(&full_sig, theme);
        lines.push(highlighted);
        lines.push(Line::from(""));
    } else {
        lines.push(Line::from(Span::styled(
            &result.name,
            theme
                .style(SemanticToken::TypeConstructor)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
    }

    // Separator
    let inner_width = area.width.saturating_sub(2) as usize;
    lines.push(Line::from(Span::styled(
        "\u{2500}".repeat(inner_width),
        theme.style(SemanticToken::Border),
    )));
    lines.push(Line::from(""));

    // Kind
    lines.push(Line::from(vec![
        Span::styled("Kind: ", theme.style(SemanticToken::Comment)),
        Span::styled(
            result.result_kind.to_string(),
            theme.style(SemanticToken::Keyword),
        ),
    ]));
    lines.push(Line::from(""));

    // Documentation (with syntax highlighting for code examples)
    if let Some(ref doc) = result.short_doc {
        for line in wrap_text(doc, inner_width) {
            let trimmed = line.trim();
            if trimmed.starts_with(">>>") {
                // GHCi prompt line — highlight the code
                let prompt_end = 3;
                let code = &trimmed[prompt_end..].trim_start();
                let mut spans = vec![Span::styled(
                    ">>> ",
                    theme
                        .style(SemanticToken::Comment)
                        .add_modifier(Modifier::BOLD),
                )];
                let highlighted = hoogle_syntax::highlight_signature(code, theme);
                spans.extend(
                    highlighted
                        .spans
                        .into_iter()
                        .map(|s| Span::styled(s.content.to_string(), s.style)),
                );
                lines.push(Line::from(spans));
            } else if trimmed.starts_with("@") || (line.starts_with("    ") && !trimmed.is_empty())
            {
                // Indented code or @-block — syntax highlight
                let highlighted = hoogle_syntax::highlight_signature(trimmed, theme);
                let mut spans = vec![Span::styled("  ", theme.style(SemanticToken::DocCode))];
                spans.extend(
                    highlighted
                        .spans
                        .into_iter()
                        .map(|s| Span::styled(s.content.to_string(), s.style)),
                );
                lines.push(Line::from(spans));
            } else {
                lines.push(Line::from(Span::styled(
                    line,
                    theme.style(SemanticToken::DocText),
                )));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "No documentation available.",
            theme.style(SemanticToken::Comment),
        )));
    }

    // URL
    if let Some(ref url) = result.doc_url {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            url.as_str().to_string(),
            theme.style(SemanticToken::DocLink),
        )));
    }

    state.total_lines = lines.len();
    // Clamp scroll
    let max_scroll = state.total_lines.saturating_sub(state.viewport_height);
    state.scroll_offset = state.scroll_offset.min(max_scroll);

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((state.scroll_offset as u16, 0));
    frame.render_widget(paragraph, area);

    // Scrollbar
    if state.total_lines > state.viewport_height {
        let mut sb_state = ScrollbarState::new(max_scroll).position(state.scroll_offset);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area,
            &mut sb_state,
        );
    }
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
