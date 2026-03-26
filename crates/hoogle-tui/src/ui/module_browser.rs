use hoogle_core::models::SearchResult;
use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use std::collections::BTreeMap;

/// A tree node in the module hierarchy.
#[derive(Debug)]
struct ModuleNode {
    /// Child nodes keyed by path segment.
    children: BTreeMap<String, ModuleNode>,
    /// Result count directly in this module.
    result_count: usize,
}

impl ModuleNode {
    fn new() -> Self {
        Self {
            children: BTreeMap::new(),
            result_count: 0,
        }
    }

    fn insert(&mut self, path: &[String]) {
        if path.is_empty() {
            self.result_count += 1;
            return;
        }
        let child = self.children.entry(path[0].clone()).or_insert_with(ModuleNode::new);
        child.insert(&path[1..]);
    }
}

/// Flattened view of the module tree for display.
#[derive(Debug, Clone)]
pub struct ModuleEntry {
    pub depth: usize,
    pub name: String,
    pub full_path: String,
    pub result_count: usize,
    pub has_children: bool,
    pub expanded: bool,
}

pub struct ModuleBrowserState {
    pub entries: Vec<ModuleEntry>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub viewport_height: usize,
    pub filter: String,
}

impl ModuleBrowserState {
    pub fn new(results: &[SearchResult]) -> Self {
        let mut root = ModuleNode::new();

        for r in results {
            if let Some(ref module) = r.module {
                root.insert(&module.0);
            }
        }

        let mut entries = Vec::new();
        flatten_tree(&root, 0, "", &mut entries);

        Self {
            entries,
            selected: 0,
            scroll_offset: 0,
            viewport_height: 0,
            filter: String::new(),
        }
    }

    pub fn move_down(&mut self) {
        let count = self.visible_entries().count();
        if count > 0 && self.selected < count - 1 {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn toggle_expand(&mut self) {
        let visible: Vec<usize> = self.visible_indices().collect();
        if let Some(&idx) = visible.get(self.selected) {
            self.entries[idx].expanded = !self.entries[idx].expanded;
        }
    }

    pub fn selected_module(&self) -> Option<&str> {
        let visible: Vec<usize> = self.visible_indices().collect();
        visible
            .get(self.selected)
            .map(|&idx| self.entries[idx].full_path.as_str())
    }

    pub fn add_filter_char(&mut self, c: char) {
        self.filter.push(c);
        self.selected = 0;
    }

    pub fn delete_filter_char(&mut self) {
        self.filter.pop();
        self.selected = 0;
    }

    fn visible_indices(&self) -> impl Iterator<Item = usize> + '_ {
        let filter_lower = self.filter.to_lowercase();
        self.entries.iter().enumerate().filter_map(move |(i, e)| {
            // Show entry if it's a top-level node or its parent is expanded
            let parent_visible = e.depth == 0 || self.is_parent_expanded(i);
            if !parent_visible {
                return None;
            }
            // Apply text filter
            if !filter_lower.is_empty()
                && !e.full_path.to_lowercase().contains(&filter_lower)
                && !e.name.to_lowercase().contains(&filter_lower)
            {
                return None;
            }
            Some(i)
        })
    }

    fn visible_entries(&self) -> impl Iterator<Item = &ModuleEntry> {
        let indices: Vec<usize> = self.visible_indices().collect();
        indices.into_iter().map(move |i| &self.entries[i])
    }

    fn is_parent_expanded(&self, idx: usize) -> bool {
        let target_depth = self.entries[idx].depth;
        if target_depth == 0 {
            return true;
        }
        // Walk backwards to find parent
        for j in (0..idx).rev() {
            if self.entries[j].depth < target_depth {
                return self.entries[j].expanded;
            }
        }
        true
    }
}

fn flatten_tree(node: &ModuleNode, depth: usize, parent_path: &str, out: &mut Vec<ModuleEntry>) {
    for (name, child) in &node.children {
        let full_path = if parent_path.is_empty() {
            name.clone()
        } else {
            format!("{parent_path}.{name}")
        };
        let total_count = count_results(child);
        out.push(ModuleEntry {
            depth,
            name: name.clone(),
            full_path: full_path.clone(),
            result_count: total_count,
            has_children: !child.children.is_empty(),
            expanded: depth < 1, // auto-expand top level
        });
        flatten_tree(child, depth + 1, &full_path, out);
    }
}

fn count_results(node: &ModuleNode) -> usize {
    let mut count = node.result_count;
    for child in node.children.values() {
        count += count_results(child);
    }
    count
}

pub fn render(frame: &mut Frame, state: &mut ModuleBrowserState, theme: &Theme) {
    let area = frame.area();
    let popup_width = (area.width.saturating_sub(8)).min(56);
    let popup_height = area.height.saturating_sub(6);
    let popup = centered_popup(area, popup_width, popup_height);

    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Module Browser ")
        .title_bottom(Span::styled(
            " Enter:select \u{2502} Space:expand \u{2502} type to filter \u{2502} Esc:close ",
            theme.style(SemanticToken::Comment),
        ))
        .border_style(theme.style(SemanticToken::Border));

    let inner = block.inner(popup);
    state.viewport_height = inner.height.saturating_sub(2) as usize; // -2 for filter line
    frame.render_widget(block, popup);

    // Filter line at top
    let filter_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: 1,
    };
    let content_area = Rect {
        x: inner.x,
        y: inner.y + 1,
        width: inner.width,
        height: inner.height.saturating_sub(1),
    };

    let filter_line = Line::from(vec![
        Span::styled(
            "\u{1f50d} ",
            theme
                .style(SemanticToken::ModuleName)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(state.filter.as_str(), theme.style(SemanticToken::SearchInput)),
        Span::styled("\u{2588}", theme.style(SemanticToken::SearchInput)),
    ]);
    frame.render_widget(Paragraph::new(filter_line), filter_area);

    // Collect visible entries
    let visible: Vec<(usize, &ModuleEntry)> = state
        .visible_indices()
        .map(|i| (i, &state.entries[i]))
        .collect();
    let total = visible.len();

    // Adjust scroll
    let vh = state.viewport_height;
    if state.selected >= state.scroll_offset + vh {
        state.scroll_offset = state.selected.saturating_sub(vh) + 1;
    }
    if state.selected < state.scroll_offset {
        state.scroll_offset = state.selected;
    }

    let start = state.scroll_offset;
    let end = (start + vh).min(total);

    let lines: Vec<Line> = visible[start..end]
        .iter()
        .enumerate()
        .map(|(vi, (_idx, entry))| {
            let is_selected = vi + start == state.selected;
            let style = if is_selected {
                theme
                    .style(SemanticToken::Selected)
                    .add_modifier(Modifier::BOLD)
            } else {
                theme.style(SemanticToken::DocText)
            };

            let indent = "  ".repeat(entry.depth);
            let arrow = if entry.has_children {
                if entry.expanded {
                    "\u{25bc} "
                } else {
                    "\u{25b6} "
                }
            } else {
                "  "
            };
            let count_str = if entry.result_count > 0 {
                format!(" ({})", entry.result_count)
            } else {
                String::new()
            };

            Line::from(vec![
                Span::styled(format!("{indent}{arrow}"), style),
                Span::styled(entry.name.as_str(), style),
                Span::styled(
                    count_str,
                    theme.style(SemanticToken::Comment).patch(if is_selected {
                        theme.style(SemanticToken::Selected)
                    } else {
                        ratatui::style::Style::default()
                    }),
                ),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), content_area);

    // Scrollbar
    if total > vh {
        let max_scroll = total.saturating_sub(vh);
        let mut sb_state = ScrollbarState::new(max_scroll).position(state.scroll_offset);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            popup,
            &mut sb_state,
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use hoogle_core::models::{ModulePath, PackageInfo, ResultKind, SearchResult};

    fn make_result(module_path: &[&str]) -> SearchResult {
        SearchResult {
            name: "func".to_string(),
            module: if module_path.is_empty() {
                None
            } else {
                Some(ModulePath(module_path.iter().map(|s| s.to_string()).collect()))
            },
            package: Some(PackageInfo {
                name: "pkg".to_string(),
                version: None,
            }),
            signature: None,
            doc_url: None,
            short_doc: None,
            result_kind: ResultKind::Function,
        }
    }

    #[test]
    fn build_from_empty_results() {
        let state = ModuleBrowserState::new(&[]);
        assert!(state.entries.is_empty());
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn build_from_results_with_no_modules() {
        let results = vec![make_result(&[])];
        let state = ModuleBrowserState::new(&results);
        assert!(state.entries.is_empty());
    }

    #[test]
    fn build_from_multiple_modules() {
        let results = vec![
            make_result(&["Data", "Map", "Strict"]),
            make_result(&["Data", "Map", "Lazy"]),
            make_result(&["Data", "List"]),
            make_result(&["Control", "Monad"]),
        ];
        let state = ModuleBrowserState::new(&results);

        // Should have entries for: Control, Monad, Data, List, Map, Lazy, Strict
        assert!(!state.entries.is_empty());

        // Top-level entries should be Control and Data
        let top_level: Vec<&str> = state.entries.iter()
            .filter(|e| e.depth == 0)
            .map(|e| e.name.as_str())
            .collect();
        assert!(top_level.contains(&"Control"));
        assert!(top_level.contains(&"Data"));
    }

    #[test]
    fn result_counts_accumulate() {
        let results = vec![
            make_result(&["Data", "Map"]),
            make_result(&["Data", "Map"]),
            make_result(&["Data", "List"]),
        ];
        let state = ModuleBrowserState::new(&results);

        // "Data" entry should have total count of 3
        let data_entry = state.entries.iter().find(|e| e.name == "Data").unwrap();
        assert_eq!(data_entry.result_count, 3);

        // "Map" entry should have count of 2
        let map_entry = state.entries.iter().find(|e| e.name == "Map").unwrap();
        assert_eq!(map_entry.result_count, 2);
    }

    #[test]
    fn move_down_increments_selected() {
        let results = vec![
            make_result(&["Data", "Map"]),
            make_result(&["Control", "Monad"]),
        ];
        let mut state = ModuleBrowserState::new(&results);
        assert_eq!(state.selected, 0);

        state.move_down();
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn move_down_clamps_at_end() {
        let results = vec![make_result(&["Data"])];
        let mut state = ModuleBrowserState::new(&results);
        // Only 1 visible entry (Data at depth 0)
        state.move_down();
        state.move_down();
        state.move_down();
        // Should not exceed the number of visible entries - 1
        let visible_count = state.visible_indices().count();
        assert!(state.selected < visible_count);
    }

    #[test]
    fn move_up_decrements_selected() {
        let results = vec![
            make_result(&["Data"]),
            make_result(&["Control"]),
        ];
        let mut state = ModuleBrowserState::new(&results);
        state.move_down();
        assert_eq!(state.selected, 1);

        state.move_up();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn move_up_clamps_at_zero() {
        let results = vec![make_result(&["Data"])];
        let mut state = ModuleBrowserState::new(&results);
        state.move_up();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn toggle_expand() {
        let results = vec![
            make_result(&["Data", "Map"]),
        ];
        let mut state = ModuleBrowserState::new(&results);

        // "Data" is at depth 0 and auto-expanded (depth < 1 => expanded)
        let data_entry = state.entries.iter().find(|e| e.name == "Data").unwrap();
        assert!(data_entry.expanded);

        // Toggle should collapse it
        state.toggle_expand();
        let data_entry = state.entries.iter().find(|e| e.name == "Data").unwrap();
        assert!(!data_entry.expanded);

        // Toggle again should expand
        state.toggle_expand();
        let data_entry = state.entries.iter().find(|e| e.name == "Data").unwrap();
        assert!(data_entry.expanded);
    }

    #[test]
    fn filter_narrows_entries() {
        let results = vec![
            make_result(&["Data", "Map"]),
            make_result(&["Control", "Monad"]),
        ];
        let mut state = ModuleBrowserState::new(&results);

        let initial_count = state.visible_indices().count();

        state.add_filter_char('D');
        state.add_filter_char('a');
        state.add_filter_char('t');
        state.add_filter_char('a');

        let filtered_count = state.visible_indices().count();
        assert!(filtered_count < initial_count);
        // All visible entries should contain "Data" in name or full_path
        for idx in state.visible_indices() {
            let entry = &state.entries[idx];
            assert!(
                entry.full_path.to_lowercase().contains("data")
                    || entry.name.to_lowercase().contains("data")
            );
        }
    }

    #[test]
    fn filter_resets_selection() {
        let results = vec![
            make_result(&["Data"]),
            make_result(&["Control"]),
        ];
        let mut state = ModuleBrowserState::new(&results);
        state.move_down();
        assert_eq!(state.selected, 1);

        state.add_filter_char('x');
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn delete_filter_char() {
        let results = vec![
            make_result(&["Data"]),
            make_result(&["Control"]),
        ];
        let mut state = ModuleBrowserState::new(&results);

        state.add_filter_char('z');
        state.add_filter_char('z');
        assert_eq!(state.filter, "zz");

        state.delete_filter_char();
        assert_eq!(state.filter, "z");

        state.delete_filter_char();
        assert_eq!(state.filter, "");
    }

    #[test]
    fn selected_module_returns_full_path() {
        let results = vec![
            make_result(&["Data", "Map"]),
            make_result(&["Control", "Monad"]),
        ];
        let state = ModuleBrowserState::new(&results);

        // First visible entry
        let selected = state.selected_module();
        assert!(selected.is_some());
        // Should be one of the top-level entries
        let path = selected.unwrap();
        assert!(path == "Control" || path == "Data");
    }

    #[test]
    fn selected_module_none_when_empty() {
        let state = ModuleBrowserState::new(&[]);
        assert!(state.selected_module().is_none());
    }

    #[test]
    fn full_paths_are_dotted() {
        let results = vec![
            make_result(&["Data", "Map", "Strict"]),
        ];
        let state = ModuleBrowserState::new(&results);

        let strict_entry = state.entries.iter().find(|e| e.name == "Strict").unwrap();
        assert_eq!(strict_entry.full_path, "Data.Map.Strict");
    }

    #[test]
    fn has_children_flag() {
        let results = vec![
            make_result(&["Data", "Map"]),
        ];
        let state = ModuleBrowserState::new(&results);

        let data_entry = state.entries.iter().find(|e| e.name == "Data").unwrap();
        assert!(data_entry.has_children);

        let map_entry = state.entries.iter().find(|e| e.name == "Map").unwrap();
        assert!(!map_entry.has_children);
    }
}
