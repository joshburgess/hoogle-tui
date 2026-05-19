use ratatui::layout::{Constraint, Layout, Rect};

pub fn centered_popup(area: Rect, width: u16, height: u16) -> Rect {
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

    #[test]
    fn centers_popup_in_larger_area() {
        let area = Rect::new(0, 0, 100, 40);

        assert_eq!(centered_popup(area, 20, 10), Rect::new(40, 15, 20, 10));
    }

    #[test]
    fn clamps_origin_when_popup_is_larger_than_area() {
        let area = Rect::new(0, 0, 10, 5);

        assert_eq!(centered_popup(area, 20, 10), Rect::new(0, 0, 10, 5));
    }

    #[test]
    fn preserves_area_origin() {
        let area = Rect::new(5, 3, 80, 20);

        assert_eq!(centered_popup(area, 40, 8), Rect::new(25, 9, 40, 8));
    }
}
