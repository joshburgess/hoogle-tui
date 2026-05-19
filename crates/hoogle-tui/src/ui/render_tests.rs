use hoogle_core::models::{ModulePath, PackageInfo, ResultKind, SearchResult};
use hoogle_syntax::theme::Theme;
use ratatui::{backend::TestBackend, layout::Rect, Terminal};

use crate::app::AppMode;
use crate::ui::{filter_popup, result_list, source_viewer, status_bar};

fn buffer_text(backend: &TestBackend) -> String {
    backend
        .buffer()
        .content
        .chunks(backend.buffer().area.width as usize)
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_to_text<F>(width: u16, height: u16, render: F) -> String
where
    F: FnOnce(&mut ratatui::Frame),
{
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("failed to create test terminal");
    terminal.draw(render).expect("failed to draw test frame");
    buffer_text(terminal.backend())
}

fn search_result(name: &str, signature: &str) -> SearchResult {
    SearchResult {
        name: name.to_string(),
        module: Some(ModulePath(vec![
            "Data".to_string(),
            "Map".to_string(),
            "Strict".to_string(),
        ])),
        package: Some(PackageInfo {
            name: "containers".to_string(),
            version: Some("0.6.7".to_string()),
        }),
        signature: Some(signature.to_string()),
        doc_url: None,
        short_doc: Some("Look up a key in the map.".to_string()),
        result_kind: ResultKind::Function,
    }
}

#[test]
fn result_list_render_includes_selected_result_metadata() {
    let theme = Theme::dracula();
    let mut state = result_list::ResultListState::new();
    state.set_items(vec![search_result(
        "lookup",
        "Ord k => k -> Map k a -> Maybe a",
    )]);

    let output = render_to_text(80, 12, |frame| {
        result_list::render(frame, Rect::new(0, 0, 80, 12), &mut state, &theme);
    });

    assert!(output.contains("Results"));
    assert!(output.contains("Data.Map.Strict"));
    assert!(output.contains("containers-0.6.7"));
    assert!(output.contains("Map k a -> Maybe a"));
    assert!(output.contains("Look up a key in the map."));
}

#[test]
fn status_bar_render_includes_mode_backend_badges_and_message() {
    let theme = Theme::dracula();
    let mut state = status_bar::StatusState::new("local".to_string());
    state.result_count = 42;
    state.search_by_type = true;
    state.package_scope = vec!["base".to_string(), "containers".to_string()];
    state.set_info("Ready");

    let output = render_to_text(100, 1, |frame| {
        status_bar::render(
            frame,
            Rect::new(0, 0, 100, 1),
            &state,
            AppMode::Results,
            &theme,
        );
    });

    assert!(output.contains("RESULTS"));
    assert!(output.contains("local"));
    assert!(output.contains("[type]"));
    assert!(output.contains("[base,containers]"));
    assert!(output.contains("Ready"));
}

#[test]
fn source_viewer_render_includes_title_line_numbers_and_code() {
    let theme = Theme::dracula();
    let mut state = source_viewer::SourceViewState::new();
    state.set_source(
        "module Demo where\nanswer = 42\n".to_string(),
        "answer",
        &theme,
    );

    let output = render_to_text(80, 8, |frame| {
        source_viewer::render(frame, Rect::new(0, 0, 80, 8), &mut state, &theme);
    });

    assert!(output.contains("Source: answer"));
    assert!(output.contains("1"));
    assert!(output.contains("module Demo where"));
    assert!(output.contains("answer"));
}

#[test]
fn filter_popup_render_includes_all_options() {
    let theme = Theme::dracula();
    let state = filter_popup::FilterState::new();

    let output = render_to_text(80, 24, |frame| {
        filter_popup::render(frame, &state, &theme);
    });

    assert!(output.contains("Filter Results"));
    assert!(output.contains("All"));
    assert!(output.contains("Functions"));
    assert!(output.contains("Data Types"));
    assert!(output.contains("Packages"));
}
