use hoogle_core::models::SearchResult;
use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::collections::HashSet;

use super::text::{display_width, normalized_filter, spans_width, truncate_width};

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

    pub fn visible_index(&self, pos: usize) -> Option<usize> {
        match &self.filtered_indices {
            Some(indices) => indices.get(pos).copied(),
            None => Some(pos),
        }
    }

    pub fn visible_result_at_render_row(
        &self,
        row: usize,
        viewport_height: usize,
    ) -> Option<usize> {
        if viewport_height == 0 {
            return None;
        }

        let visible_count = self.visible_count();
        let lpr = self.lines_per_result();

        if !self.group_by_module || self.compact {
            let visible_pos = self.scroll_offset + row / lpr;
            let visible_end = (self.scroll_offset + viewport_height / lpr).min(visible_count);
            return (visible_pos < visible_end).then_some(visible_pos);
        }

        let viewport_results = viewport_height / lpr.max(1);
        let visible_end = (self.scroll_offset + viewport_results).min(visible_count);
        let mut rendered_row = 0;
        let mut last_module: Option<&str> = None;

        for visible_pos in self.scroll_offset..visible_end {
            let idx = self.visible_index(visible_pos)?;
            let current_module = self.display_cache.get(idx)?.module_str.as_str();
            let show_header = !current_module.is_empty() && last_module != Some(current_module);

            if show_header {
                last_module = Some(current_module);
                if rendered_row + 2 < viewport_height {
                    if row == rendered_row {
                        return None;
                    }
                    rendered_row += 1;
                }
            }

            if row >= rendered_row && row < rendered_row + lpr {
                return Some(visible_pos);
            }
            rendered_row += lpr;
        }

        None
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
        self.replace_items(items);
        self.selected = 0;
        self.scroll_offset = 0;
        self.fuzzy_filter = None;
        self.filtered_indices = None;
        self.multi_selected.clear();
        self.multi_select_mode = false;
    }

    pub fn set_items_preserving_view(&mut self, items: Vec<SearchResult>) {
        let selected_key = self.selected_result().map(result_key);
        let selected_keys: HashSet<String> = self
            .multi_selected
            .iter()
            .filter_map(|idx| self.items.get(*idx).map(result_key))
            .collect();

        self.replace_items(items);

        if self.fuzzy_filter.is_some() {
            self.apply_fuzzy_filter();
        } else {
            self.clamp_selection();
        }

        if let Some(key) = selected_key {
            if let Some(position) = (0..self.visible_count()).find(|pos| {
                self.visible_index(*pos)
                    .and_then(|idx| self.items.get(idx))
                    .is_some_and(|result| result_key(result) == key)
            }) {
                self.selected = position;
            }
        }

        self.multi_selected = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(idx, result)| selected_keys.contains(&result_key(result)).then_some(idx))
            .collect();
    }

    fn replace_items(&mut self, items: Vec<SearchResult>) {
        self.display_cache = items
            .iter()
            .map(|r| CachedDisplay {
                module_str: r.module.as_ref().map(|m| m.to_string()).unwrap_or_default(),
                pkg_str: r
                    .package
                    .as_ref()
                    .map(|p| p.to_string())
                    .unwrap_or_default(),
            })
            .collect();
        self.items = items;
    }

    fn clamp_selection(&mut self) {
        let count = self.visible_count();
        if count == 0 {
            self.selected = 0;
            self.scroll_offset = 0;
        } else if self.selected >= count {
            self.selected = count - 1;
        }
    }

    pub fn selected_result(&self) -> Option<&SearchResult> {
        let idx = self.visible_index(self.selected)?;
        self.items.get(idx)
    }

    /// Toggle multi-select on the current item.
    pub fn toggle_select_current(&mut self) {
        if let Some(idx) = self.visible_index(self.selected) {
            if self.multi_selected.contains(&idx) {
                self.multi_selected.remove(&idx);
            } else {
                self.multi_selected.insert(idx);
            }
        }
    }

    /// Get all multi-selected results.
    pub fn selected_results(&self) -> Vec<&SearchResult> {
        (0..self.visible_count())
            .filter_map(|pos| self.visible_index(pos))
            .filter(|idx| self.multi_selected.contains(idx))
            .filter_map(|idx| self.items.get(idx))
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
        let query = normalized_filter(filter);
        if query.is_empty() {
            self.filtered_indices = None;
            self.selected = 0;
            self.scroll_offset = 0;
            return;
        }

        self.filtered_indices = Some(
            self.items
                .iter()
                .enumerate()
                .filter(|(_, result)| result_matches_fuzzy_filter(result, &query))
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

fn truncate_empty_examples_line(
    width: usize,
    comment_style: Style,
    key_style: Style,
) -> Line<'static> {
    let examples = [("map", 3), ("Maybe a -> a", 12), ("[a] -> Int", 10)];
    let prefix = "  Try: ";
    let prefix_width = display_width(prefix);
    let mut spans = vec![Span::styled(prefix, comment_style)];
    let mut used = prefix_width;

    for (example, example_width) in examples {
        let separator_width = if used == prefix_width { 0 } else { 2 };
        let required = separator_width + example_width;
        if used + required > width {
            break;
        }
        if separator_width > 0 {
            spans.push(Span::styled("  ", comment_style));
            used += separator_width;
        }
        spans.push(Span::styled(example, key_style));
        used += example_width;
    }

    if spans.len() == 1 {
        spans[0] = Span::styled(truncate_width("  Try: map", width, "..."), comment_style);
    }

    Line::from(spans)
}

fn truncate_empty_bindings_line(
    width: usize,
    comment_style: Style,
    key_style: Style,
) -> Line<'static> {
    let prefix = "  Press ?";
    let prefix_width = display_width(prefix);
    if width <= prefix_width {
        return Line::from(Span::styled(
            truncate_width(prefix, width, "..."),
            comment_style,
        ));
    }

    Line::from(vec![
        Span::styled("  Press ", comment_style),
        Span::styled("?", key_style),
        Span::styled(
            truncate_width(" for all keybindings", width - prefix_width, "..."),
            comment_style,
        ),
    ])
}

fn result_key(result: &SearchResult) -> String {
    format!(
        "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{:?}",
        result.name,
        result
            .module
            .as_ref()
            .map(|m| m.as_dotted())
            .unwrap_or_default(),
        result
            .package
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_default(),
        result.signature.as_deref().unwrap_or_default(),
        result.result_kind
    )
}

fn result_matches_fuzzy_filter(result: &SearchResult, query: &str) -> bool {
    let haystack = format!(
        "{} {} {}",
        result.name,
        result
            .module
            .as_ref()
            .map(|module| module.to_string())
            .unwrap_or_default(),
        result
            .package
            .as_ref()
            .map(|package| package.name.as_str())
            .unwrap_or("")
    )
    .to_lowercase();
    haystack.contains(query)
}

pub fn render(frame: &mut Frame, area: Rect, state: &mut ResultListState, theme: &Theme) {
    let visible_count = state.visible_count();
    let raw_title = if let Some(ref filter) = state.fuzzy_filter {
        let display_filter = filter.trim();
        format!(
            " Results ({}/{}) Filter: {} ",
            visible_count,
            state.items.len(),
            display_filter
        )
    } else {
        format!(" Results ({}) ", state.items.len())
    };
    let title = truncate_width(&raw_title, area.width.saturating_sub(2) as usize, "...");

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(theme.style(SemanticToken::Border));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let inner_width = inner.width as usize;

    if state.loading {
        let message = truncate_width("  Searching...", inner_width, "...");
        let loading = Paragraph::new(Line::from(vec![Span::styled(
            message,
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
                truncate_width(
                    "  No matches. Press Esc to clear filter.",
                    inner_width,
                    "...",
                ),
                comment_style,
            ))]
        } else if state.items.is_empty() {
            let prompt = truncate_width("  Start typing to search Hoogle", inner_width, "...");
            vec![
                Line::from(""),
                Line::from(Span::styled(prompt, comment_style)),
                Line::from(""),
                truncate_empty_examples_line(inner_width, comment_style, key_style),
                Line::from(""),
                truncate_empty_bindings_line(inner_width, comment_style, key_style),
            ]
        } else {
            vec![Line::from(Span::styled(
                truncate_width("  No results found.", inner_width, "..."),
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
        let Some(idx) = state.visible_index(vi) else {
            continue;
        };
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
                                available_width.saturating_sub(display_width(current_module) + 4),
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
            if is_multi {
                "[x] "
            } else {
                "[ ] "
            }
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
                let max = available_width.saturating_sub(display_width(&cached.module_str) + 6);
                let truncated = if display_width(&sig_text) > max {
                    truncate_width(&sig_text, max, "\u{2026}")
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
            let used = spans_width(spans.iter());
            let pad = available_width.saturating_sub(used + display_width(&cached.module_str) + 1);
            spans.push(Span::styled(" ".repeat(pad), base_style));
            spans.push(Span::styled(
                cached.module_str.as_str(),
                theme.style(SemanticToken::Comment).patch(base_style),
            ));
            lines.push(Line::from(spans));
        } else {
            // Expanded: 3 lines
            // Line 1: module + package (right-aligned package)
            let padding = available_width.saturating_sub(
                display_width(&cached.module_str) + display_width(&cached.pkg_str) + 4,
            );

            lines.push(Line::from(vec![
                Span::styled(
                    marker,
                    if is_selected {
                        module_style.add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(cached.module_str.as_str(), module_style.patch(base_style)),
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
                    if display_width(d) > max_len {
                        format!("    {}", truncate_width(d, max_len, "..."))
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

    fn make_module_result(name: &str, module: &[&str]) -> SearchResult {
        let mut result = make_result(name);
        result.module = Some(ModulePath(
            module.iter().map(|segment| (*segment).into()).collect(),
        ));
        result
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
    fn set_items_clears_multi_select_state() {
        let mut state = ResultListState::new();
        state.set_items(vec![make_result("a"), make_result("b")]);
        state.multi_select_mode = true;
        state.toggle_select_current();
        assert!(!state.multi_selected.is_empty());

        state.set_items(vec![make_result("x")]);

        assert!(!state.multi_select_mode);
        assert!(state.multi_selected.is_empty());
    }

    #[test]
    fn selected_results_follow_visible_order() {
        let mut state = ResultListState::new();
        state.set_items(vec![
            make_result("alpha"),
            make_result("beta"),
            make_result("gamma"),
        ]);
        state.multi_selected.insert(2);
        state.multi_selected.insert(0);

        let names: Vec<&str> = state
            .selected_results()
            .iter()
            .map(|result| result.name.as_str())
            .collect();

        assert_eq!(names, vec!["alpha", "gamma"]);
    }

    #[test]
    fn selected_results_follow_filtered_visible_order() {
        let mut state = ResultListState::new();
        state.set_items(vec![
            make_result("alpha"),
            make_result("beta"),
            make_result("gamma"),
        ]);
        state.multi_selected.insert(0);
        state.multi_selected.insert(2);
        state.start_fuzzy_filter();
        for c in "gamm".chars() {
            state.fuzzy_add_char(c);
        }

        let names: Vec<&str> = state
            .selected_results()
            .iter()
            .map(|result| result.name.as_str())
            .collect();

        assert_eq!(names, vec!["gamma"]);
    }

    #[test]
    fn visible_result_at_render_row_ignores_group_headers() {
        let mut state = ResultListState::new();
        state.group_by_module = true;
        state.set_items(vec![
            make_module_result("map", &["Data", "Map"]),
            make_module_result("set", &["Data", "Set"]),
        ]);

        assert_eq!(state.visible_result_at_render_row(0, 12), None);
        assert_eq!(state.visible_result_at_render_row(1, 12), Some(0));
        assert_eq!(state.visible_result_at_render_row(4, 12), None);
        assert_eq!(state.visible_result_at_render_row(5, 12), Some(1));
    }

    #[test]
    fn visible_result_at_render_row_uses_compact_row_height() {
        let mut state = ResultListState::new();
        state.compact = true;
        state.group_by_module = true;
        state.set_items(vec![
            make_module_result("map", &["Data", "Map"]),
            make_module_result("set", &["Data", "Set"]),
        ]);

        assert_eq!(state.visible_result_at_render_row(0, 2), Some(0));
        assert_eq!(state.visible_result_at_render_row(1, 2), Some(1));
        assert_eq!(state.visible_result_at_render_row(2, 2), None);
    }

    #[test]
    fn selected_result_returns_correct_item() {
        let mut state = ResultListState::new();
        state.set_items(vec![make_result("a"), make_result("b")]);
        assert_eq!(
            state.selected_result().map(|result| result.name.as_str()),
            Some("a")
        );
        state.move_down();
        assert_eq!(
            state.selected_result().map(|result| result.name.as_str()),
            Some("b")
        );
    }

    #[test]
    fn selected_result_on_empty() {
        let state = ResultListState::new();
        assert!(state.selected_result().is_none());
    }

    #[test]
    fn selected_result_none_when_filter_has_no_matches() {
        let mut state = ResultListState::new();
        state.set_items(vec![make_result("a")]);
        state.start_fuzzy_filter();
        state.fuzzy_add_char('z');

        assert_eq!(state.visible_count(), 0);
        assert!(state.selected_result().is_none());
    }

    #[test]
    fn fuzzy_filter_ignores_surrounding_whitespace() {
        let mut state = ResultListState::new();
        state.set_items(vec![make_result("map"), make_result("filter")]);
        state.start_fuzzy_filter();
        for c in "  filter  ".chars() {
            state.fuzzy_add_char(c);
        }

        assert_eq!(state.visible_count(), 1);
        assert_eq!(
            state.selected_result().map(|result| result.name.as_str()),
            Some("filter")
        );
    }

    #[test]
    fn fuzzy_filter_matches_module_and_package_metadata() {
        let mut state = ResultListState::new();
        state.set_items(vec![
            make_module_result("lookup", &["Data", "Map"]),
            make_result("decode"),
        ]);
        state.items[1].package = Some(hoogle_core::models::PackageInfo {
            name: "aeson".to_string(),
            version: None,
        });
        state.replace_items(state.items.clone());

        state.start_fuzzy_filter();
        for c in "data.map".chars() {
            state.fuzzy_add_char(c);
        }

        assert_eq!(
            state.selected_result().map(|result| result.name.as_str()),
            Some("lookup")
        );

        state.clear_fuzzy_filter();
        state.start_fuzzy_filter();
        for c in "aeson".chars() {
            state.fuzzy_add_char(c);
        }

        assert_eq!(
            state.selected_result().map(|result| result.name.as_str()),
            Some("decode")
        );
    }

    #[test]
    fn toggle_select_current_ignores_empty_filtered_view() {
        let mut state = ResultListState::new();
        state.set_items(vec![make_result("a")]);
        state.start_fuzzy_filter();
        state.fuzzy_add_char('z');

        state.toggle_select_current();

        assert!(state.multi_selected.is_empty());
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
