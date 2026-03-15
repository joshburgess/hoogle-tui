use hoogle_core::models::SearchResult;
use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

const LINES_PER_RESULT: usize = 3;
const SCROLL_CONTEXT: usize = 2;

pub struct ResultListState {
    pub items: Vec<SearchResult>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub loading: bool,
    // Fuzzy filter within results
    pub fuzzy_filter: Option<String>,
    pub filtered_indices: Option<Vec<usize>>,
}

impl ResultListState {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            loading: false,
            fuzzy_filter: None,
            filtered_indices: None,
        }
    }

    pub fn visible_count(&self) -> usize {
        self.filtered_indices
            .as_ref()
            .map(|v| v.len())
            .unwrap_or(self.items.len())
    }

    pub fn visible_index(&self, pos: usize) -> usize {
        self.filtered_indices
            .as_ref()
            .and_then(|v| v.get(pos).copied())
            .unwrap_or(pos)
    }

    pub fn move_down(&mut self) {
        let count = self.visible_count();
        if count > 0 && self.selected < count - 1 {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_to_top(&mut self) {
        self.selected = 0;
    }

    pub fn move_to_bottom(&mut self) {
        let count = self.visible_count();
        if count > 0 {
            self.selected = count - 1;
        }
    }

    pub fn set_items(&mut self, items: Vec<SearchResult>) {
        self.items = items;
        self.selected = 0;
        self.scroll_offset = 0;
        self.fuzzy_filter = None;
        self.filtered_indices = None;
    }

    pub fn selected_result(&self) -> Option<&SearchResult> {
        let idx = self.visible_index(self.selected);
        self.items.get(idx)
    }

    // --- Fuzzy filter ---

    pub fn start_fuzzy_filter(&mut self) {
        self.fuzzy_filter = Some(String::new());
        self.filtered_indices = None;
    }

    pub fn fuzzy_add_char(&mut self, c: char) {
        if let Some(ref mut filter) = self.fuzzy_filter {
            filter.push(c);
            self.apply_fuzzy_filter();
        }
    }

    pub fn fuzzy_delete_char(&mut self) {
        if let Some(ref mut filter) = self.fuzzy_filter {
            filter.pop();
            if filter.is_empty() {
                self.clear_fuzzy_filter();
            } else {
                self.apply_fuzzy_filter();
            }
        }
    }

    pub fn clear_fuzzy_filter(&mut self) {
        self.fuzzy_filter = None;
        self.filtered_indices = None;
        self.selected = 0;
        self.scroll_offset = 0;
    }

    fn apply_fuzzy_filter(&mut self) {
        let Some(ref filter) = self.fuzzy_filter else {
            self.filtered_indices = None;
            return;
        };
        if filter.is_empty() {
            self.filtered_indices = None;
            self.selected = 0;
            return;
        }

        let query = filter.to_lowercase();
        self.filtered_indices = Some(
            self.items
                .iter()
                .enumerate()
                .filter(|(_, r)| {
                    let haystack = format!(
                        "{} {} {}",
                        r.name,
                        r.module.as_ref().map(|m| m.to_string()).unwrap_or_default(),
                        r.package.as_ref().map(|p| p.name.as_str()).unwrap_or("")
                    )
                    .to_lowercase();
                    haystack.contains(&query)
                })
                .map(|(i, _)| i)
                .collect(),
        );
        self.selected = 0;
        self.scroll_offset = 0;
    }

    fn adjust_scroll(&mut self, viewport_results: usize) {
        if viewport_results == 0 {
            return;
        }

        // Ensure selected is visible with context
        if self.selected < self.scroll_offset + SCROLL_CONTEXT {
            self.scroll_offset = self.selected.saturating_sub(SCROLL_CONTEXT);
        }

        let max_visible = self.scroll_offset + viewport_results;
        if self.selected + SCROLL_CONTEXT >= max_visible {
            self.scroll_offset =
                (self.selected + SCROLL_CONTEXT + 1).saturating_sub(viewport_results);
        }

        // Clamp scroll
        let count = self.visible_count();
        if count > 0 {
            let max_scroll = count.saturating_sub(viewport_results);
            self.scroll_offset = self.scroll_offset.min(max_scroll);
        }
    }
}

pub fn render(frame: &mut Frame, area: Rect, state: &mut ResultListState, theme: &Theme) {
    let visible_count = state.visible_count();
    let title = if let Some(ref filter) = state.fuzzy_filter {
        format!(
            " Results ({}/{}) Filter: {} ",
            visible_count,
            state.items.len(),
            filter
        )
    } else {
        format!(" Results ({}) ", state.items.len())
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(theme.style(SemanticToken::Border));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if state.loading {
        let loading = Paragraph::new(Line::from(vec![Span::styled(
            "  Searching...",
            theme.style(SemanticToken::Spinner),
        )]));
        frame.render_widget(loading, inner);
        return;
    }

    if visible_count == 0 {
        let msg = if state.fuzzy_filter.is_some() {
            "  No matches. Press Esc to clear filter."
        } else {
            "  No results. Type a query to search Hoogle."
        };
        let empty = Paragraph::new(Line::from(vec![Span::styled(
            msg,
            theme.style(SemanticToken::Comment),
        )]));
        frame.render_widget(empty, inner);
        return;
    }

    let viewport_height = inner.height as usize;
    let viewport_results = viewport_height / LINES_PER_RESULT;
    state.adjust_scroll(viewport_results);

    let mut lines: Vec<Line> = Vec::new();

    let visible_end = (state.scroll_offset + viewport_results).min(visible_count);
    for vi in state.scroll_offset..visible_end {
        let idx = state.visible_index(vi);
        let result = &state.items[idx];
        let is_selected = vi == state.selected;

        let base_style = if is_selected {
            theme.style(SemanticToken::Selected)
        } else {
            Style::default()
        };

        // Line 1: module + package (right-aligned package)
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

        let available_width = inner.width as usize;
        let padding = available_width.saturating_sub(module_str.len() + pkg_str.len() + 4); // 2 prefix + 2 margin

        let marker = if is_selected { "> " } else { "  " };

        lines.push(Line::from(vec![
            Span::styled(
                marker.to_string(),
                if is_selected {
                    theme
                        .style(SemanticToken::ModuleName)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                },
            ),
            Span::styled(
                module_str,
                theme.style(SemanticToken::ModuleName).patch(base_style),
            ),
            Span::styled(" ".repeat(padding), base_style),
            Span::styled(
                pkg_str,
                theme.style(SemanticToken::PackageName).patch(base_style),
            ),
        ]));

        // Line 2: syntax-highlighted signature
        let sig_line = if let Some(ref sig) = result.signature {
            let mut spans = vec![Span::styled("    ", base_style)];
            let highlighted = hoogle_syntax::highlight_signature(sig, theme);
            for span in highlighted.spans {
                spans.push(Span::styled(
                    span.content.to_string(),
                    span.style.patch(base_style),
                ));
            }
            Line::from(spans)
        } else {
            Line::from(vec![Span::styled(
                format!("    {}", result.name),
                theme
                    .style(SemanticToken::TypeConstructor)
                    .patch(base_style),
            )])
        };
        lines.push(sig_line);

        // Line 3: short doc (truncated)
        let doc_str = result
            .short_doc
            .as_ref()
            .map(|d| {
                let max_len = available_width.saturating_sub(6);
                if d.len() > max_len {
                    format!("    {}...", &d[..max_len.saturating_sub(3)])
                } else {
                    format!("    {d}")
                }
            })
            .unwrap_or_else(|| "    ".to_string());
        lines.push(Line::from(vec![Span::styled(
            doc_str,
            theme.style(SemanticToken::Comment).patch(base_style),
        )]));
    }

    // Pad remaining space
    while lines.len() < viewport_height {
        lines.push(Line::from(""));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use hoogle_core::models::{ModulePath, PackageInfo, ResultKind};

    fn make_result(name: &str) -> SearchResult {
        SearchResult {
            name: name.into(),
            module: Some(ModulePath(vec!["Data".into(), "Map".into()])),
            package: Some(PackageInfo {
                name: "containers".into(),
                version: Some("0.6.7".into()),
            }),
            signature: Some("Ord k => k -> Map k a -> Maybe a".into()),
            doc_url: None,
            short_doc: Some("A short doc.".into()),
            result_kind: ResultKind::Function,
        }
    }

    #[test]
    fn new_state_is_empty() {
        let state = ResultListState::new();
        assert!(state.items.is_empty());
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn move_down_clamps() {
        let mut state = ResultListState::new();
        state.set_items(vec![make_result("a"), make_result("b"), make_result("c")]);
        state.move_down();
        assert_eq!(state.selected, 1);
        state.move_down();
        assert_eq!(state.selected, 2);
        state.move_down(); // should clamp
        assert_eq!(state.selected, 2);
    }

    #[test]
    fn move_up_clamps() {
        let mut state = ResultListState::new();
        state.set_items(vec![make_result("a")]);
        state.move_up(); // already at 0
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn move_to_top_and_bottom() {
        let mut state = ResultListState::new();
        state.set_items(vec![make_result("a"), make_result("b"), make_result("c")]);
        state.move_to_bottom();
        assert_eq!(state.selected, 2);
        state.move_to_top();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn set_items_resets_selection() {
        let mut state = ResultListState::new();
        state.set_items(vec![make_result("a"), make_result("b")]);
        state.move_down();
        assert_eq!(state.selected, 1);
        state.set_items(vec![make_result("x")]);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn selected_result_returns_correct_item() {
        let mut state = ResultListState::new();
        state.set_items(vec![make_result("a"), make_result("b")]);
        assert_eq!(state.selected_result().unwrap().name, "a");
        state.move_down();
        assert_eq!(state.selected_result().unwrap().name, "b");
    }

    #[test]
    fn selected_result_on_empty() {
        let state = ResultListState::new();
        assert!(state.selected_result().is_none());
    }

    #[test]
    fn move_on_empty_does_not_panic() {
        let mut state = ResultListState::new();
        state.move_down();
        state.move_up();
        state.move_to_top();
        state.move_to_bottom();
        assert_eq!(state.selected, 0);
    }
}
