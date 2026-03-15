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
