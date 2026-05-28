use ratatui::Frame;

use crate::app::{App, AppMode, PopupMode};
use crate::ui::{
    bookmarks_popup, command_palette, doc_viewer, filter_popup, help_overlay, history_popup,
    layout, module_browser, package_popup, pinned_panel, preview_pane, result_list, search_bar,
    sort_popup, source_viewer, status_bar, theme_popup, toc_popup, yank_popup,
};

impl App {
    pub(crate) fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        self.last_width = area.width;

        if area.width < 40 || area.height < 10 {
            let msg = ratatui::widgets::Paragraph::new(vec![
                ratatui::text::Line::from(""),
                ratatui::text::Line::from(ratatui::text::Span::styled(
                    "Terminal too small",
                    ratatui::style::Style::default()
                        .fg(ratatui::style::Color::Red)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                )),
                ratatui::text::Line::from(ratatui::text::Span::styled(
                    format!("Need at least 40x10, got {}x{}", area.width, area.height),
                    ratatui::style::Style::default().fg(ratatui::style::Color::Gray),
                )),
                ratatui::text::Line::from(""),
                ratatui::text::Line::from("Press q to quit."),
            ])
            .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(msg, area);
            return;
        }

        if self.mode == AppMode::DocView || self.mode == AppMode::SourceView {
            let chunks = ratatui::layout::Layout::vertical([
                ratatui::layout::Constraint::Min(1),
                ratatui::layout::Constraint::Length(1),
            ])
            .split(area);

            self.hit_doc_area = chunks[0];

            if self.mode == AppMode::DocView {
                doc_viewer::render(frame, chunks[0], &mut self.doc_state, &self.theme);
            } else {
                source_viewer::render(frame, chunks[0], &mut self.source_state, &self.theme);
            }
            status_bar::render(frame, chunks[1], &self.status, self.mode, &self.theme);
        } else {
            let ly = layout::compute_layout(area, self.preview_enabled, self.config.ui.layout);

            self.hit_search_bar = ly.search_bar;
            self.hit_result_list = ly.result_list;
            self.hit_preview_pane = ly.preview_pane;
            self.hit_pinned_panel = None;

            let has_query = !self.last_searched.is_empty();
            search_bar::render(
                frame,
                ly.search_bar,
                &mut self.textarea,
                self.mode,
                has_query,
                &self.theme,
            );
            result_list::render(frame, ly.result_list, &mut self.results, &self.theme);

            if let Some(preview_area) = ly.preview_pane {
                if !self.pinned.is_empty() {
                    let split = ratatui::layout::Layout::vertical([
                        ratatui::layout::Constraint::Percentage(60),
                        ratatui::layout::Constraint::Percentage(40),
                    ])
                    .split(preview_area);
                    self.hit_preview_pane = Some(split[0]);
                    self.hit_pinned_panel = Some(split[1]);
                    let selected = self.results.selected_result().cloned();
                    preview_pane::render(
                        frame,
                        split[0],
                        selected.as_ref(),
                        &mut self.preview_state,
                        &self.theme,
                    );
                    pinned_panel::render(frame, split[1], &mut self.pinned, &self.theme);
                } else {
                    let selected = self.results.selected_result().cloned();
                    preview_pane::render(
                        frame,
                        preview_area,
                        selected.as_ref(),
                        &mut self.preview_state,
                        &self.theme,
                    );
                }
            }

            status_bar::render(frame, ly.status_bar, &self.status, self.mode, &self.theme);
        }

        match self.popup {
            Some(PopupMode::Filter) => {
                filter_popup::render(frame, &self.filter_state, &self.theme);
            }
            Some(PopupMode::Sort) => {
                sort_popup::render(frame, &self.sort_state, &self.theme);
            }
            Some(PopupMode::Toc) => {
                if let Some(ref toc) = self.toc_state {
                    toc_popup::render(frame, toc, &self.theme);
                }
            }
            Some(PopupMode::History) => {
                if let Some(ref hp) = self.history_popup {
                    history_popup::render(frame, hp, &self.history, &self.theme);
                }
            }
            Some(PopupMode::Bookmarks) => {
                if let Some(ref bp) = self.bookmarks_popup {
                    bookmarks_popup::render(frame, bp, &self.bookmark_store, &self.theme);
                }
            }
            Some(PopupMode::YankMenu) => {
                if let Some(ref yp) = self.yank_popup {
                    yank_popup::render(frame, yp, &self.theme);
                }
            }
            Some(PopupMode::PackageScope) => {
                if let Some(ref pp) = self.package_popup {
                    package_popup::render(frame, pp, &self.theme);
                }
            }
            Some(PopupMode::ThemeSwitcher) => {
                if let Some(ref tp) = self.theme_popup {
                    theme_popup::render(frame, tp, &self.theme);
                }
            }
            Some(PopupMode::ModuleBrowser) => {
                if let Some(ref mut mb) = self.module_browser {
                    module_browser::render(frame, mb, &self.theme);
                }
            }
            Some(PopupMode::CommandPalette) => {
                if let Some(ref cp) = self.command_palette {
                    command_palette::render(frame, cp, &self.theme);
                }
            }
            None => {}
        }

        if self.mode == AppMode::Help {
            help_overlay::render(frame, &mut self.help_state, &self.theme);
        }
    }
}
