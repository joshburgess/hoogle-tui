use hoogle_core::models::SearchResult;
use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

const LINES_PER_RESULT_EXPANDED: usize = 3;
const LINES_PER_RESULT_COMPACT: usize = 1;
const SCROLL_CONTEXT: usize = 2;

/// Pre-computed display strings for a search result (avoids per-frame formatting).
pub struct CachedDisplay {
    pub module_str: String,
    pub pkg_str: String,
}

pub struct ResultListState {
    pub items: Vec<SearchResult>,
    /// Pre-computed display strings, parallel to `items`.
    pub display_cache: Vec<CachedDisplay>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub loading: bool,
    pub compact: bool,
    /// Multi-select: indices of selected items (for batch yank).
    pub multi_selected: std::collections::HashSet<usize>,
    pub multi_select_mode: bool,
    /// Group by module: when true, insert module headers in the display.
    pub group_by_module: bool,
    // Fuzzy filter within results
    pub fuzzy_filter: Option<String>,
    pub filtered_indices: Option<Vec<usize>>,
}

impl ResultListState {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            display_cache: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            loading: false,
            compact: false,
            multi_selected: std::collections::HashSet::new(),
            multi_select_mode: false,
            group_by_module: false,
            fuzzy_filter: None,
            filtered_indices: None,
        }
    }

    pub fn lines_per_result(&self) -> usize {
        if self.compact {
            LINES_PER_RESULT_COMPACT
        } else {
            LINES_PER_RESULT_EXPANDED
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
        self.display_cache = items
            .iter()
            .map(|r| CachedDisplay {
                module_str: r
                    .module
                    .as_ref()
                    .map(|m| m.to_string())
                    .unwrap_or_default(),
                pkg_str: r
                    .package
                    .as_ref()
                    .map(|p| p.to_string())
                    .unwrap_or_default(),
            })
            .collect();
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

    /// Toggle multi-select on the current item.
    pub fn toggle_select_current(&mut self) {
        let idx = self.visible_index(self.selected);
        if self.multi_selected.contains(&idx) {
            self.multi_selected.remove(&idx);
        } else {
            self.multi_selected.insert(idx);
        }
    }

    /// Get all multi-selected results.
    pub fn selected_results(&self) -> Vec<&SearchResult> {
        self.multi_selected
            .iter()
            .filter_map(|&idx| self.items.get(idx))
            .collect()
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
        let comment_style = theme.style(SemanticToken::Comment);
        let key_style = theme.style(SemanticToken::ModuleName);

        let lines: Vec<Line> = if state.fuzzy_filter.is_some() {
            vec![Line::from(Span::styled(
                "  No matches. Press Esc to clear filter.",
                comment_style,
            ))]
        } else if state.items.is_empty() {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Start typing to search Hoogle",
                    comment_style,
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  Try: ", comment_style),
                    Span::styled("map", key_style),
                    Span::styled("  ", comment_style),
                    Span::styled("Maybe a -> a", key_style),
                    Span::styled("  ", comment_style),
                    Span::styled("[a] -> Int", key_style),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  Press ", comment_style),
                    Span::styled("?", key_style),
                    Span::styled(" for all keybindings", comment_style),
                ]),
            ]
        } else {
            vec![Line::from(Span::styled(
                "  No results found.",
                comment_style,
            ))]
        };
        let empty = Paragraph::new(lines);
        frame.render_widget(empty, inner);
        return;
    }

    let viewport_height = inner.height as usize;
    let lpr = state.lines_per_result();
    let viewport_results = viewport_height / lpr.max(1);
    state.adjust_scroll(viewport_results);

    let mut lines: Vec<Line> = Vec::new();

    let available_width = inner.width as usize;
    let selected_style = theme.style(SemanticToken::Selected);
    let module_style = theme.style(SemanticToken::ModuleName);
    let pkg_style = theme.style(SemanticToken::PackageName);

    let mut last_module: Option<String> = None;

    let visible_end = (state.scroll_offset + viewport_results).min(visible_count);
    for vi in state.scroll_offset..visible_end {
        let idx = state.visible_index(vi);
        let result = &state.items[idx];
        let cached = &state.display_cache[idx];
        let is_selected = vi == state.selected;
        let is_multi = state.multi_selected.contains(&idx);

        // Module group header
        if state.group_by_module && !state.compact {
            let current_module = &cached.module_str;
            let show_header = match &last_module {
                Some(prev) => prev != current_module,
                None => true,
            };
            if show_header && !current_module.is_empty() {
                last_module = Some(current_module.clone());
                // Don't emit header if it would exceed viewport
                if lines.len() + 2 < viewport_height {
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("\u{2500}\u{2500} {current_module} "),
                            theme
                                .style(SemanticToken::ModuleName)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            "\u{2500}".repeat(
                                available_width.saturating_sub(current_module.len() + 4),
                            ),
                            theme.style(SemanticToken::Border),
                        ),
                    ]));
                }
            }
        }

        let base_style = if is_selected {
            selected_style
        } else {
            Style::default()
        };

        let marker = if state.multi_select_mode {
            if is_multi { "[x] " } else { "[ ] " }
        } else if is_selected {
            "> "
        } else {
            "  "
        };

        if state.compact {
            // Compact: single line: "> name :: sig  (module)"
            let mut spans = vec![Span::styled(
                marker,
                if is_selected {
                    module_style.add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                },
            )];
            if let Some(ref sig) = result.signature {
                let sig_text = format!("{} :: {sig}", result.name);
                let max = available_width.saturating_sub(cached.module_str.len() + 6);
                let truncated = if sig_text.len() > max {
                    format!("{}\u{2026}", &sig_text[..max.saturating_sub(1)])
                } else {
                    sig_text
                };
                let highlighted = hoogle_syntax::highlight_signature(&truncated, theme);
                for span in highlighted.spans {
                    spans.push(Span::styled(
                        span.content.to_string(),
                        span.style.patch(base_style),
                    ));
                }
            } else {
                spans.push(Span::styled(
                    result.name.as_str(),
                    theme
                        .style(SemanticToken::TypeConstructor)
                        .patch(base_style),
                ));
            }
            // Right-align module name
            let used: usize = spans.iter().map(|s| s.content.len()).sum();
            let pad = available_width.saturating_sub(used + cached.module_str.len() + 1);
            spans.push(Span::styled(" ".repeat(pad), base_style));
            spans.push(Span::styled(
                cached.module_str.as_str(),
                theme.style(SemanticToken::Comment).patch(base_style),
            ));
            lines.push(Line::from(spans));
        } else {
            // Expanded: 3 lines
            // Line 1: module + package (right-aligned package)
            let padding = available_width
                .saturating_sub(cached.module_str.len() + cached.pkg_str.len() + 4);

            lines.push(Line::from(vec![
                Span::styled(
                    marker,
                    if is_selected {
                        module_style.add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(
                    cached.module_str.as_str(),
                    module_style.patch(base_style),
                ),
                Span::styled(" ".repeat(padding), base_style),
                Span::styled(cached.pkg_str.as_str(), pkg_style.patch(base_style)),
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
