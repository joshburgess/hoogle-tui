use hoogle_core::haddock::types::{DocBlock, HaddockDoc, Inline};
use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use url::Url;

pub struct DocViewState {
    pub doc: Option<HaddockDoc>,
    pub rendered_lines: Vec<Line<'static>>,
    /// Pre-computed lowercased text for each rendered line (for fast search).
    lowered_lines: Vec<String>,
    pub scroll_offset: usize,
    pub viewport_height: usize,
    pub declaration_offsets: Vec<(String, usize)>,
    pub links: Vec<(usize, Url)>,
    pub focused_link: Option<usize>,
    pub nav_stack: Vec<Url>,
    pub loading: bool,
    pub error: Option<String>,
    // In-document search
    pub search_active: bool,
    pub search_query: String,
    pub search_matches: Vec<usize>,
    pub current_match: Option<usize>,
}

impl DocViewState {
    pub fn new() -> Self {
        Self {
            doc: None,
            rendered_lines: Vec::new(),
            lowered_lines: Vec::new(),
            scroll_offset: 0,
            viewport_height: 0,
            declaration_offsets: Vec::new(),
            links: Vec::new(),
            focused_link: None,
            nav_stack: Vec::new(),
            loading: false,
            error: None,
            search_active: false,
            search_query: String::new(),
            search_matches: Vec::new(),
            current_match: None,
        }
    }

    pub fn set_doc(&mut self, doc: HaddockDoc, theme: &Theme, width: u16) {
        let w = width.saturating_sub(4) as usize;
        let (lines, decl_offsets, links) = render_doc(&doc, theme, w);
        // Pre-compute lowercased line text for search
        self.lowered_lines = lines
            .iter()
            .map(|line| {
                let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
                text.to_lowercase()
            })
            .collect();
        self.rendered_lines = lines;
        self.declaration_offsets = decl_offsets;
        self.links = links;
        self.doc = Some(doc);
        self.scroll_offset = 0;
        self.focused_link = None;
        self.loading = false;
        self.error = None;
        self.clear_search();
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

    pub fn next_declaration(&mut self) {
        if let Some((_, offset)) = self
            .declaration_offsets
            .iter()
            .find(|(_, off)| *off > self.scroll_offset + 1)
        {
            self.scroll_offset = offset.saturating_sub(1);
        }
    }

    pub fn prev_declaration(&mut self) {
        if let Some((_, offset)) = self
            .declaration_offsets
            .iter()
            .rev()
            .find(|(_, off)| *off + 1 < self.scroll_offset)
        {
            self.scroll_offset = offset.saturating_sub(1);
        }
    }

    pub fn push_nav(&mut self, url: Url) {
        self.nav_stack.push(url);
    }

    pub fn pop_nav(&mut self) -> Option<Url> {
        self.nav_stack.pop()
    }

    // --- Link focus cycling ---

    /// Cycle to the next link visible on screen.
    pub fn focus_next_link(&mut self) {
        if self.links.is_empty() {
            return;
        }
        let visible_start = self.scroll_offset;
        let visible_end = self.scroll_offset + self.viewport_height;

        match self.focused_link {
            Some(idx) => {
                // Find next link after current that's on screen
                let next = self
                    .links
                    .iter()
                    .enumerate()
                    .skip(idx + 1)
                    .find(|(_, (line, _))| *line >= visible_start && *line < visible_end);
                if let Some((i, _)) = next {
                    self.focused_link = Some(i);
                    // Scroll to keep it visible
                    self.ensure_line_visible(self.links[i].0);
                } else {
                    // Wrap around to first visible link
                    let first = self
                        .links
                        .iter()
                        .enumerate()
                        .find(|(_, (line, _))| *line >= visible_start && *line < visible_end);
                    self.focused_link = first.map(|(i, _)| i);
                }
            }
            None => {
                // Focus first visible link
                let first = self
                    .links
                    .iter()
                    .enumerate()
                    .find(|(_, (line, _))| *line >= visible_start && *line < visible_end);
                self.focused_link = first.map(|(i, _)| i);
            }
        }
    }

    pub fn focused_link_url(&self) -> Option<&Url> {
        self.focused_link
            .and_then(|idx| self.links.get(idx))
            .map(|(_, url)| url)
    }

    pub fn focused_link_line(&self) -> Option<usize> {
        self.focused_link
            .and_then(|idx| self.links.get(idx))
            .map(|(line, _)| *line)
    }

    fn ensure_line_visible(&mut self, line: usize) {
        if line < self.scroll_offset {
            self.scroll_offset = line;
        } else if line >= self.scroll_offset + self.viewport_height {
            self.scroll_offset = line.saturating_sub(self.viewport_height) + 1;
        }
    }

    // --- In-document search ---

    pub fn start_search(&mut self) {
        self.search_active = true;
        self.search_query.clear();
        self.search_matches.clear();
        self.current_match = None;
    }

    pub fn search_add_char(&mut self, c: char) {
        self.search_query.push(c);
        self.update_search_matches();
    }

    pub fn search_delete_char(&mut self) {
        self.search_query.pop();
        self.update_search_matches();
    }

    pub fn confirm_search(&mut self) {
        self.search_active = false;
        // Keep matches and position visible
    }

    pub fn clear_search(&mut self) {
        self.search_active = false;
        self.search_query.clear();
        self.search_matches.clear();
        self.current_match = None;
    }

    pub fn next_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        match self.current_match {
            Some(idx) => {
                let next = (idx + 1) % self.search_matches.len();
                self.current_match = Some(next);
                self.scroll_offset = self.search_matches[next].saturating_sub(3);
            }
            None => {
                self.current_match = Some(0);
                self.scroll_offset = self.search_matches[0].saturating_sub(3);
            }
        }
    }

    pub fn prev_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        match self.current_match {
            Some(idx) => {
                let prev = if idx == 0 {
                    self.search_matches.len() - 1
                } else {
                    idx - 1
                };
                self.current_match = Some(prev);
                self.scroll_offset = self.search_matches[prev].saturating_sub(3);
            }
            None => {
                let last = self.search_matches.len() - 1;
                self.current_match = Some(last);
                self.scroll_offset = self.search_matches[last].saturating_sub(3);
            }
        }
    }

    fn update_search_matches(&mut self) {
        self.search_matches.clear();
        self.current_match = None;

        if self.search_query.is_empty() {
            return;
        }

        let query_lower = self.search_query.to_lowercase();
        for (i, lowered) in self.lowered_lines.iter().enumerate() {
            if lowered.contains(&query_lower) {
                self.search_matches.push(i);
            }
        }

        // Jump to first match at or after current scroll
        if !self.search_matches.is_empty() {
            let first_visible = self
                .search_matches
                .binary_search(&self.scroll_offset)
                .unwrap_or_else(|pos| pos.min(self.search_matches.len() - 1));
            self.current_match = Some(first_visible);
            self.scroll_offset = self.search_matches[first_visible].saturating_sub(3);
        }
    }
}

pub fn render(frame: &mut Frame, area: Rect, state: &mut DocViewState, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(doc_title(state))
        .border_style(theme.style(SemanticToken::Border));

    // Reserve space for search bar at bottom if active
    let (doc_area, search_area) = if state.search_active {
        let chunks = ratatui::layout::Layout::vertical([
            ratatui::layout::Constraint::Min(1),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    let inner = block.inner(doc_area);
    state.viewport_height = inner.height as usize;

    if state.loading {
        let loading = Paragraph::new(Line::from(Span::styled(
            "  Loading documentation...",
            theme.style(SemanticToken::Spinner),
        )))
        .block(block);
        frame.render_widget(loading, doc_area);
        render_search_bar(frame, search_area, state, theme);
        return;
    }

    if let Some(ref err) = state.error {
        let error = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  Error: {err}"),
                theme.style(SemanticToken::Error),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Press Esc to go back.",
                theme.style(SemanticToken::Comment),
            )),
        ])
        .block(block);
        frame.render_widget(error, doc_area);
        render_search_bar(frame, search_area, state, theme);
        return;
    }

    if state.rendered_lines.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "  No documentation loaded.",
            theme.style(SemanticToken::Comment),
        )))
        .block(block);
        frame.render_widget(empty, doc_area);
        render_search_bar(frame, search_area, state, theme);
        return;
    }

    // Slice visible lines, apply search highlighting and link focus
    let total = state.rendered_lines.len();
    let start = state.scroll_offset.min(total);
    let end = (start + inner.height as usize).min(total);

    let highlight_style = Style::default()
        .bg(ratatui::style::Color::Yellow)
        .fg(ratatui::style::Color::Black);
    let current_match_style = Style::default()
        .bg(ratatui::style::Color::Red)
        .fg(ratatui::style::Color::White)
        .add_modifier(Modifier::BOLD);
    let focused_link_style = theme
        .style(SemanticToken::DocLink)
        .add_modifier(Modifier::REVERSED);

    let focused_line = state.focused_link_line();
    let has_search = !state.search_query.is_empty() && !state.search_matches.is_empty();

    let visible: Vec<Line> = state.rendered_lines[start..end]
        .iter()
        .enumerate()
        .map(|(vi, line)| {
            let abs_line = start + vi;
            let needs_link_highlight = Some(abs_line) == focused_line;
            let is_search_match =
                has_search && state.search_matches.binary_search(&abs_line).is_ok();

            // Skip cloning lines that don't need modification
            if !needs_link_highlight && !is_search_match {
                return line.clone();
            }

            let mut new_line = line.clone();

            // Highlight focused link line
            if needs_link_highlight {
                new_line = Line::from(
                    new_line
                        .spans
                        .into_iter()
                        .map(|s| Span::styled(s.content.to_string(), focused_link_style))
                        .collect::<Vec<_>>(),
                );
            }

            // Highlight search matches
            if is_search_match {
                let is_current = state
                    .current_match
                    .map(|idx| state.search_matches.get(idx) == Some(&abs_line))
                    .unwrap_or(false);
                let style = if is_current {
                    current_match_style
                } else {
                    highlight_style
                };
                new_line = Line::from(
                    new_line
                        .spans
                        .into_iter()
                        .map(|s| Span::styled(s.content.to_string(), s.style.patch(style)))
                        .collect::<Vec<_>>(),
                );
            }

            new_line
        })
        .collect();

    let paragraph = Paragraph::new(visible).block(block);
    frame.render_widget(paragraph, doc_area);

    // Scrollbar
    if total > inner.height as usize {
        let mut scrollbar_state = ScrollbarState::new(total.saturating_sub(inner.height as usize))
            .position(state.scroll_offset);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            doc_area,
            &mut scrollbar_state,
        );
    }

    // Search bar
    render_search_bar(frame, search_area, state, theme);
}

fn render_search_bar(frame: &mut Frame, area: Option<Rect>, state: &DocViewState, theme: &Theme) {
    let Some(area) = area else { return };

    let match_info = if state.search_matches.is_empty() {
        if state.search_query.is_empty() {
            String::new()
        } else {
            " (no matches)".to_string()
        }
    } else {
        let current = state.current_match.map(|i| i + 1).unwrap_or(0);
        format!(" ({}/{})", current, state.search_matches.len())
    };

    let bar = Paragraph::new(Line::from(vec![
        Span::styled(
            " /",
            theme
                .style(SemanticToken::ModuleName)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            state.search_query.clone(),
            theme.style(SemanticToken::SearchInput),
        ),
        Span::styled("\u{2588}", theme.style(SemanticToken::SearchInput)),
        Span::styled(match_info, theme.style(SemanticToken::Comment)),
    ]))
    .style(theme.style(SemanticToken::StatusBar));
    frame.render_widget(bar, area);
}

fn doc_title(state: &DocViewState) -> String {
    if let Some(ref doc) = state.doc {
        format!(" {} ({}) ", doc.module, doc.package)
    } else {
        " Documentation ".to_string()
    }
}

// --- Document rendering ---

type RenderResult = (Vec<Line<'static>>, Vec<(String, usize)>, Vec<(usize, Url)>);

fn render_doc(doc: &HaddockDoc, theme: &Theme, width: usize) -> RenderResult {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut decl_offsets: Vec<(String, usize)> = Vec::new();
    let mut links: Vec<(usize, Url)> = Vec::new();

    // Module header
    lines.push(Line::from(Span::styled(
        doc.module.clone(),
        theme
            .style(SemanticToken::ModuleName)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        doc.package.clone(),
        theme.style(SemanticToken::PackageName),
    )));
    lines.push(Line::from(""));

    // Description
    if !doc.description.is_empty() {
        render_blocks(&doc.description, theme, width, &mut lines, &mut links);
        lines.push(Line::from(""));
    }

    // Declarations
    for decl in &doc.declarations {
        let offset = lines.len();
        decl_offsets.push((decl.name.clone(), offset));

        // Separator
        lines.push(Line::from(Span::styled(
            "\u{2501}".repeat(width.min(60)),
            theme.style(SemanticToken::Operator),
        )));

        // Signature
        if let Some(ref sig) = decl.signature {
            let highlighted = hoogle_syntax::highlight_signature(sig, theme);
            lines.push(highlighted);
        } else {
            lines.push(Line::from(Span::styled(
                decl.name.clone(),
                theme
                    .style(SemanticToken::TypeConstructor)
                    .add_modifier(Modifier::BOLD),
            )));
        }

        // Since badge
        if let Some(ref since) = decl.since {
            lines.push(Line::from(Span::styled(
                format!("[{since}]"),
                theme.style(SemanticToken::Comment),
            )));
        }

        lines.push(Line::from(Span::styled(
            "\u{2501}".repeat(width.min(60)),
            theme.style(SemanticToken::Operator),
        )));
        lines.push(Line::from(""));

        // Documentation
        render_blocks(&decl.doc, theme, width, &mut lines, &mut links);
        lines.push(Line::from(""));
    }

    (lines, decl_offsets, links)
}

fn render_blocks(
    blocks: &[DocBlock],
    theme: &Theme,
    width: usize,
    lines: &mut Vec<Line<'static>>,
    links: &mut Vec<(usize, Url)>,
) {
    for block in blocks {
        match block {
            DocBlock::Paragraph(inlines) => {
                let wrapped = wrap_inlines(inlines, theme, width, lines.len(), links);
                lines.extend(wrapped);
                lines.push(Line::from(""));
            }

            DocBlock::CodeBlock { code, .. } => {
                // Top border
                let border_w = width.min(70);
                lines.push(Line::from(Span::styled(
                    format!(
                        "\u{250c}{}\u{2510}",
                        "\u{2500}".repeat(border_w.saturating_sub(2))
                    ),
                    theme.style(SemanticToken::Border),
                )));

                // Syntax-highlighted code lines
                let code_lines = hoogle_syntax::highlight_code(code, theme);
                for code_line in code_lines {
                    let mut spans = vec![Span::styled(
                        "\u{2502} ".to_string(),
                        theme.style(SemanticToken::Border),
                    )];

                    // Check if it's a GHCi prompt line
                    let line_text: String =
                        code_line.spans.iter().map(|s| s.content.as_ref()).collect();
                    if line_text.trim_start().starts_with(">>>") {
                        let trimmed = line_text.trim_start();
                        let prefix_ws = line_text.len() - trimmed.len();
                        if prefix_ws > 0 {
                            spans.push(Span::styled(" ".repeat(prefix_ws), Style::default()));
                        }
                        spans.push(Span::styled(
                            ">>> ".to_string(),
                            theme
                                .style(SemanticToken::Comment)
                                .add_modifier(Modifier::BOLD),
                        ));
                        let rest = trimmed.strip_prefix(">>>").unwrap_or("").trim_start();
                        let rest_highlighted = hoogle_syntax::highlight_signature(rest, theme);
                        spans.extend(
                            rest_highlighted
                                .spans
                                .into_iter()
                                .map(|s| Span::styled(s.content.to_string(), s.style)),
                        );
                    } else {
                        spans.extend(
                            code_line
                                .spans
                                .into_iter()
                                .map(|s| Span::styled(s.content.to_string(), s.style)),
                        );
                    }

                    lines.push(Line::from(spans));
                }

                // Bottom border
                lines.push(Line::from(Span::styled(
                    format!(
                        "\u{2514}{}\u{2518}",
                        "\u{2500}".repeat(border_w.saturating_sub(2))
                    ),
                    theme.style(SemanticToken::Border),
                )));
                lines.push(Line::from(""));
            }

            DocBlock::UnorderedList(items) => {
                for item in items {
                    let item_lines =
                        wrap_inlines(item, theme, width.saturating_sub(4), lines.len(), links);
                    for (i, line) in item_lines.into_iter().enumerate() {
                        let prefix = if i == 0 { "  \u{2022} " } else { "    " };
                        let mut spans = vec![Span::styled(
                            prefix.to_string(),
                            theme.style(SemanticToken::DocText),
                        )];
                        spans.extend(line.spans);
                        lines.push(Line::from(spans));
                    }
                }
                lines.push(Line::from(""));
            }

            DocBlock::OrderedList(items) => {
                for (idx, item) in items.iter().enumerate() {
                    let item_lines =
                        wrap_inlines(item, theme, width.saturating_sub(5), lines.len(), links);
                    for (i, line) in item_lines.into_iter().enumerate() {
                        let prefix = if i == 0 {
                            format!("  {}. ", idx + 1)
                        } else {
                            "     ".to_string()
                        };
                        let mut spans =
                            vec![Span::styled(prefix, theme.style(SemanticToken::DocText))];
                        spans.extend(line.spans);
                        lines.push(Line::from(spans));
                    }
                }
                lines.push(Line::from(""));
            }

            DocBlock::Header { level, content } => {
                lines.push(Line::from(""));
                let text = inlines_to_plain_text(content);
                let style = theme
                    .style(SemanticToken::DocHeading)
                    .add_modifier(Modifier::BOLD);
                lines.push(Line::from(Span::styled(text.clone(), style)));
                if *level <= 2 {
                    let underline_char = if *level == 1 { "\u{2501}" } else { "\u{2500}" };
                    lines.push(Line::from(Span::styled(
                        underline_char.repeat(text.len().min(width)),
                        style,
                    )));
                }
                lines.push(Line::from(""));
            }

            DocBlock::HorizontalRule => {
                lines.push(Line::from(Span::styled(
                    "\u{2500}".repeat(width.min(40)),
                    theme.style(SemanticToken::Border),
                )));
                lines.push(Line::from(""));
            }

            DocBlock::Note(inlines) => {
                let wrapped =
                    wrap_inlines(inlines, theme, width.saturating_sub(4), lines.len(), links);
                for line in wrapped {
                    let mut spans = vec![Span::styled(
                        "  \u{26a0} ".to_string(),
                        theme.style(SemanticToken::Keyword),
                    )];
                    spans.extend(line.spans);
                    lines.push(Line::from(spans));
                }
                lines.push(Line::from(""));
            }

            DocBlock::Table { headers, rows } => {
                render_table(headers, rows, theme, width, lines, links);
                lines.push(Line::from(""));
            }
        }
    }
}

fn render_table(
    headers: &[Vec<Inline>],
    rows: &[Vec<Vec<Inline>>],
    theme: &Theme,
    width: usize,
    lines: &mut Vec<Line<'static>>,
    _links: &mut Vec<(usize, Url)>,
) {
    let border_style = theme.style(SemanticToken::Border);
    let header_style = theme
        .style(SemanticToken::DocHeading)
        .add_modifier(Modifier::BOLD);
    let cell_style = theme.style(SemanticToken::DocText);

    // Compute column count
    let num_cols = headers
        .len()
        .max(rows.iter().map(|r| r.len()).max().unwrap_or(0));
    if num_cols == 0 {
        return;
    }

    // Helper: flatten inlines to plain text for width calculation
    let to_text = |inlines: &[Inline]| -> String {
        inlines
            .iter()
            .map(|i| match i {
                Inline::Text(t)
                | Inline::Code(t)
                | Inline::Emphasis(t)
                | Inline::Bold(t)
                | Inline::Math(t)
                | Inline::ModuleLink(t) => t.as_str(),
                Inline::Link { text, .. } => text.as_str(),
            })
            .collect()
    };

    // Compute column widths
    let mut col_widths: Vec<usize> = vec![0; num_cols];
    for (i, h) in headers.iter().enumerate() {
        col_widths[i] = col_widths[i].max(to_text(h).len());
    }
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < num_cols {
                col_widths[i] = col_widths[i].max(to_text(cell).len());
            }
        }
    }

    // Cap column widths to fit terminal
    let total: usize = col_widths.iter().sum::<usize>() + (num_cols + 1) * 3;
    if total > width {
        let available = width.saturating_sub((num_cols + 1) * 3);
        let per_col = available / num_cols.max(1);
        for w in &mut col_widths {
            *w = (*w).min(per_col.max(4));
        }
    }

    // Build separator line
    let sep: String = col_widths
        .iter()
        .map(|w| "\u{2500}".repeat(w + 2))
        .collect::<Vec<_>>()
        .join("\u{253c}");
    let sep_line = format!("\u{251c}{sep}\u{2524}");

    // Build top border
    let top: String = col_widths
        .iter()
        .map(|w| "\u{2500}".repeat(w + 2))
        .collect::<Vec<_>>()
        .join("\u{252c}");
    lines.push(Line::from(Span::styled(
        format!("\u{250c}{top}\u{2510}"),
        border_style,
    )));

    // Render header row
    if !headers.is_empty() {
        let mut spans = vec![Span::styled("\u{2502} ", border_style)];
        for (i, h) in headers.iter().enumerate() {
            let text = to_text(h);
            let w = col_widths.get(i).copied().unwrap_or(10);
            let padded = format!("{:<width$}", text, width = w);
            spans.push(Span::styled(padded, header_style));
            spans.push(Span::styled(" \u{2502} ", border_style));
        }
        lines.push(Line::from(spans));
        lines.push(Line::from(Span::styled(sep_line.clone(), border_style)));
    }

    // Render data rows
    for row in rows {
        let mut spans = vec![Span::styled("\u{2502} ", border_style)];
        for i in 0..num_cols {
            let text = row.get(i).map(|c| to_text(c)).unwrap_or_default();
            let w = col_widths.get(i).copied().unwrap_or(10);
            let truncated = if text.len() > w {
                format!("{}\u{2026}", &text[..w.saturating_sub(1)])
            } else {
                format!("{:<width$}", text, width = w)
            };
            spans.push(Span::styled(truncated, cell_style));
            spans.push(Span::styled(" \u{2502} ", border_style));
        }
        lines.push(Line::from(spans));
    }

    // Bottom border
    let bottom: String = col_widths
        .iter()
        .map(|w| "\u{2500}".repeat(w + 2))
        .collect::<Vec<_>>()
        .join("\u{2534}");
    lines.push(Line::from(Span::styled(
        format!("\u{2514}{bottom}\u{2518}"),
        border_style,
    )));
}

fn wrap_inlines(
    inlines: &[Inline],
    theme: &Theme,
    width: usize,
    base_line: usize,
    links: &mut Vec<(usize, Url)>,
) -> Vec<Line<'static>> {
    if width == 0 {
        return vec![Line::from("")];
    }

    let mut result_lines: Vec<Vec<Span<'static>>> = vec![vec![]];
    let mut col = 0;

    for inline in inlines {
        let (text, style) = match inline {
            Inline::Text(t) => (t.clone(), theme.style(SemanticToken::DocText)),
            Inline::Code(t) => (t.clone(), theme.style(SemanticToken::DocCode)),
            Inline::Link { text, url } => {
                let line_idx = base_line + result_lines.len() - 1;
                links.push((line_idx, url.clone()));
                (
                    text.clone(),
                    theme
                        .style(SemanticToken::DocLink)
                        .add_modifier(Modifier::UNDERLINED),
                )
            }
            Inline::ModuleLink(name) => (name.clone(), theme.style(SemanticToken::ModuleName)),
            Inline::Emphasis(t) => (
                t.clone(),
                theme
                    .style(SemanticToken::DocText)
                    .add_modifier(Modifier::ITALIC),
            ),
            Inline::Bold(t) => (
                t.clone(),
                theme
                    .style(SemanticToken::DocText)
                    .add_modifier(Modifier::BOLD),
            ),
            Inline::Math(t) => (format!("[{t}]"), theme.style(SemanticToken::DocCode)),
        };

        // Word wrap
        for word in text.split_inclusive(|c: char| c.is_whitespace()) {
            let word_len = word.len();
            if col + word_len > width && col > 0 {
                result_lines.push(vec![]);
                col = 0;
            }
            if let Some(last) = result_lines.last_mut() {
                last.push(Span::styled(word.to_string(), style));
            }
            col += word_len;
        }
    }

    result_lines.into_iter().map(Line::from).collect()
}

fn inlines_to_plain_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for i in inlines {
        match i {
            Inline::Text(t)
            | Inline::Code(t)
            | Inline::Emphasis(t)
            | Inline::Bold(t)
            | Inline::Math(t)
            | Inline::ModuleLink(t) => out.push_str(t),
            Inline::Link { text, .. } => out.push_str(text),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Set rendered lines and pre-compute lowered lines for search tests.
    fn set_test_lines(state: &mut DocViewState, lines: Vec<Line<'static>>) {
        state.lowered_lines = lines
            .iter()
            .map(|line| {
                let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
                text.to_lowercase()
            })
            .collect();
        state.rendered_lines = lines;
    }

    #[test]
    fn search_finds_matches() {
        let mut state = DocViewState::new();
        set_test_lines(
            &mut state,
            vec![
                Line::from("hello world"),
                Line::from("foo bar"),
                Line::from("hello again"),
            ],
        );
        state.viewport_height = 10;
        state.start_search();
        state.search_add_char('h');
        state.search_add_char('e');
        state.search_add_char('l');
        assert_eq!(state.search_matches.len(), 2);
        assert_eq!(state.search_matches, vec![0, 2]);
        assert_eq!(state.current_match, Some(0));
    }

    #[test]
    fn search_next_prev_cycles() {
        let mut state = DocViewState::new();
        set_test_lines(
            &mut state,
            vec![
                Line::from("match a"),
                Line::from("no"),
                Line::from("match b"),
                Line::from("match c"),
            ],
        );
        state.viewport_height = 10;
        state.start_search();
        state.search_add_char('m');
        state.search_add_char('a');
        assert_eq!(state.search_matches.len(), 3);

        state.next_match();
        assert_eq!(state.current_match, Some(1));
        state.next_match();
        assert_eq!(state.current_match, Some(2));
        state.next_match(); // wraps
        assert_eq!(state.current_match, Some(0));

        state.prev_match();
        assert_eq!(state.current_match, Some(2));
    }

    #[test]
    fn search_clear_resets() {
        let mut state = DocViewState::new();
        set_test_lines(&mut state, vec![Line::from("hello")]);
        state.viewport_height = 10;
        state.start_search();
        state.search_add_char('h');
        assert!(!state.search_matches.is_empty());
        state.clear_search();
        assert!(state.search_matches.is_empty());
        assert!(!state.search_active);
    }

    #[test]
    fn search_case_insensitive() {
        let mut state = DocViewState::new();
        set_test_lines(
            &mut state,
            vec![Line::from("Hello World"), Line::from("HELLO")],
        );
        state.viewport_height = 10;
        state.start_search();
        state.search_add_char('h');
        state.search_add_char('e');
        assert_eq!(state.search_matches.len(), 2);
    }

    #[test]
    fn link_focus_cycling() {
        let mut state = DocViewState::new();
        state.rendered_lines = vec![
            Line::from("line 0"),
            Line::from("line 1"),
            Line::from("line 2"),
            Line::from("line 3"),
        ];
        state.viewport_height = 10;
        let url1 = Url::parse("https://example.com/1").unwrap();
        let url2 = Url::parse("https://example.com/2").unwrap();
        state.links = vec![(1, url1), (3, url2)];

        assert!(state.focused_link.is_none());
        state.focus_next_link();
        assert_eq!(state.focused_link, Some(0));
        assert_eq!(state.focused_link_line(), Some(1));
        state.focus_next_link();
        assert_eq!(state.focused_link, Some(1));
        assert_eq!(state.focused_link_line(), Some(3));
        state.focus_next_link(); // wraps
        assert_eq!(state.focused_link, Some(0));
    }

    #[test]
    fn link_focus_empty() {
        let mut state = DocViewState::new();
        state.viewport_height = 10;
        state.focus_next_link();
        assert!(state.focused_link.is_none());
    }
}
