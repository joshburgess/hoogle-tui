use ratatui::layout::{Constraint, Layout, Rect};

pub struct AppLayout {
    pub search_bar: Rect,
    pub result_list: Rect,
    pub preview_pane: Option<Rect>,
    pub status_bar: Rect,
}

pub fn compute_layout(area: Rect, preview_enabled: bool) -> AppLayout {
    let chunks = Layout::vertical([
        Constraint::Length(3), // search bar
        Constraint::Min(1),    // main area
        Constraint::Length(1), // status bar
    ])
    .split(area);

    let main_area = chunks[1];

    if preview_enabled && main_area.width >= 80 {
        // Vertical split: results left, preview right
        let ratio = if main_area.width >= 120 { 55 } else { 50 };
        let split = Layout::horizontal([
            Constraint::Percentage(ratio),
            Constraint::Percentage(100 - ratio),
        ])
        .split(main_area);

        AppLayout {
            search_bar: chunks[0],
            result_list: split[0],
            preview_pane: Some(split[1]),
            status_bar: chunks[2],
        }
    } else {
        AppLayout {
            search_bar: chunks[0],
            result_list: main_area,
            preview_pane: None,
            status_bar: chunks[2],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(width: u16, height: u16) -> Rect {
        Rect::new(0, 0, width, height)
    }

    #[test]
    fn layout_search_bar_always_3_rows() {
        let layout = compute_layout(rect(120, 40), false);
        assert_eq!(layout.search_bar.height, 3);
        assert_eq!(layout.search_bar.y, 0);
    }

    #[test]
    fn layout_status_bar_always_1_row_at_bottom() {
        let layout = compute_layout(rect(120, 40), false);
        assert_eq!(layout.status_bar.height, 1);
        assert_eq!(layout.status_bar.y, 39);
    }

    #[test]
    fn layout_no_preview_when_disabled() {
        let layout = compute_layout(rect(200, 50), false);
        assert!(layout.preview_pane.is_none());
        // Result list takes the full main area width
        assert_eq!(layout.result_list.width, 200);
    }

    #[test]
    fn layout_no_preview_when_too_narrow() {
        // Width 79 is below the 80 threshold for the main area
        let layout = compute_layout(rect(79, 40), true);
        assert!(layout.preview_pane.is_none());
        assert_eq!(layout.result_list.width, 79);
    }

    #[test]
    fn layout_preview_appears_at_80_width() {
        // At exactly 80 width, the main area is 80 wide, preview should appear
        let layout = compute_layout(rect(80, 40), true);
        assert!(layout.preview_pane.is_some());
    }

    #[test]
    fn layout_wide_terminal_55_45_split() {
        let layout = compute_layout(rect(120, 40), true);
        let preview = layout.preview_pane.unwrap();
        // At 120 width, ratio is 55%, so result_list gets ~66 cols, preview ~54
        assert!(layout.result_list.width > preview.width);
    }

    #[test]
    fn layout_medium_terminal_50_50_split() {
        // Width 80-119 uses 50/50 split
        let layout = compute_layout(rect(100, 40), true);
        let preview = layout.preview_pane.unwrap();
        // 50/50 split: both should be equal or differ by at most 1 due to rounding
        let diff = layout.result_list.width.abs_diff(preview.width);
        assert!(diff <= 1);
    }

    #[test]
    fn layout_very_small_terminal() {
        let layout = compute_layout(rect(20, 5), false);
        assert_eq!(layout.search_bar.height, 3);
        assert_eq!(layout.status_bar.height, 1);
        // Main area gets whatever is left: 5 - 3 - 1 = 1
        assert_eq!(layout.result_list.height, 1);
    }

    #[test]
    fn layout_result_list_and_preview_same_y() {
        let layout = compute_layout(rect(120, 40), true);
        let preview = layout.preview_pane.unwrap();
        assert_eq!(layout.result_list.y, preview.y);
        assert_eq!(layout.result_list.height, preview.height);
    }

    #[test]
    fn layout_areas_dont_overlap_vertically() {
        let layout = compute_layout(rect(100, 30), false);
        // search_bar ends before result_list starts
        assert!(layout.search_bar.y + layout.search_bar.height <= layout.result_list.y);
        // result_list ends before status_bar starts
        assert!(layout.result_list.y + layout.result_list.height <= layout.status_bar.y);
    }
}
