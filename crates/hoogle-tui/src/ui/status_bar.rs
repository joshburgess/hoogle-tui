use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::AppMode;

use super::text::{display_width, spans_width, truncate_width};

const SPINNER_FRAMES: &[char] = &[
    '\u{280b}', '\u{2819}', '\u{2839}', '\u{2838}', '\u{283c}', '\u{2834}', '\u{2826}', '\u{2827}',
    '\u{2807}', '\u{280f}',
];

pub struct StatusState {
    pub backend_name: String,
    pub result_count: usize,
    pub message: Option<StatusMessage>,
    pub spinner_tick: usize,
    pub search_by_type: bool,
    pub offline: bool,
    pub package_scope: Vec<String>,
}

pub enum StatusMessage {
    Info(String),
    Error(String),
    Loading(String),
}

impl StatusState {
    pub fn new(backend_name: String) -> Self {
        Self {
            backend_name,
            result_count: 0,
            message: None,
            spinner_tick: 0,
            search_by_type: false,
            offline: false,
            package_scope: Vec::new(),
        }
    }

    pub fn tick(&mut self) {
        self.spinner_tick = (self.spinner_tick + 1) % SPINNER_FRAMES.len();
    }

    pub fn set_info(&mut self, msg: impl Into<String>) {
        self.message = Some(StatusMessage::Info(msg.into()));
    }

    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.message = Some(StatusMessage::Error(msg.into()));
    }

    pub fn clear_message(&mut self) {
        self.message = None;
    }
}

fn mode_label(mode: AppMode) -> &'static str {
    match mode {
        AppMode::Search => "SEARCH",
        AppMode::Results => "RESULTS",
        AppMode::DocView => "DOCS",
        AppMode::SourceView => "SOURCE",
        AppMode::Help => "HELP",
    }
}

pub fn render(frame: &mut Frame, area: Rect, state: &StatusState, mode: AppMode, theme: &Theme) {
    let area_width = area.width as usize;
    let status_style = theme.style(SemanticToken::StatusBar);
    let key_style = theme.style(SemanticToken::ModuleName);
    let mode_style = theme
        .style(SemanticToken::Keyword)
        .add_modifier(Modifier::BOLD);
    let hint_style = status_style;
    let backend_width = (area_width / 5).clamp(3, 16);
    let backend_name = truncate_width(&state.backend_name, backend_width, "...");

    // Left side: mode indicator + backend + badges + message/count
    let mut left_spans = vec![
        Span::styled(format!(" {} ", mode_label(mode)), mode_style),
        Span::styled("\u{2502} ", status_style),
        Span::styled(format!("{backend_name} "), status_style),
    ];

    if state.offline {
        left_spans.push(Span::styled(
            "OFFLINE ",
            theme
                .style(SemanticToken::Error)
                .add_modifier(Modifier::BOLD),
        ));
    }

    if state.search_by_type {
        left_spans.push(Span::styled("[type] ", theme.style(SemanticToken::Keyword)));
    }

    if !state.package_scope.is_empty() {
        let max_scope_width = (area_width / 4).clamp(8, 28);
        let scope = truncate_width(&state.package_scope.join(","), max_scope_width, "...");
        left_spans.push(Span::styled(
            format!("[{scope}] "),
            theme.style(SemanticToken::ModuleName),
        ));
    }

    left_spans.push(Span::styled("\u{2502} ", status_style));

    match &state.message {
        Some(StatusMessage::Loading(msg)) => {
            let spinner = SPINNER_FRAMES[state.spinner_tick];
            let max_message_width = message_width(area_width, spans_width(left_spans.iter()) + 2);
            let msg = truncate_width(msg, max_message_width, "...");
            left_spans.push(Span::styled(
                format!("{spinner} {msg} "),
                theme.style(SemanticToken::Spinner),
            ));
        }
        Some(StatusMessage::Error(msg)) => {
            let max_message_width = message_width(area_width, spans_width(left_spans.iter()));
            let msg = truncate_width(msg, max_message_width, "...");
            left_spans.push(Span::styled(
                format!("{msg} "),
                theme.style(SemanticToken::Error),
            ));
        }
        Some(StatusMessage::Info(msg)) => {
            let max_message_width = message_width(area_width, spans_width(left_spans.iter()));
            let msg = truncate_width(msg, max_message_width, "...");
            left_spans.push(Span::styled(format!("{msg} "), status_style));
        }
        None => {
            if state.result_count > 0 {
                let count_value = compact_count(state.result_count);
                let max_count_width = message_width(area_width, spans_width(left_spans.iter()));
                let mut count = format!("{count_value} results");
                if display_width(&count) > max_count_width {
                    count = format!("{count_value} res");
                }
                let count = truncate_width(&count, max_count_width, "...");
                left_spans.push(Span::styled(format!("{count} "), status_style));
            }
        }
    }

    // Right side: contextual key hints (most important actions for this mode)
    let hints: Vec<(&str, &str)> = match mode {
        AppMode::Search => vec![
            ("Enter", "focus results"),
            ("Ctrl-k", "commands"),
            ("Ctrl-r", "history"),
            ("F1/Ctrl-/", "help"),
            ("Esc", "clear/quit"),
        ],
        AppMode::Results => vec![
            ("\u{2191}\u{2193}/jk", "navigate"),
            ("Enter", "open docs"),
            ("Tab", "preview"),
            ("Ctrl-k", "commands"),
            ("/", "search"),
            ("?", "all keys"),
            ("q", "quit"),
        ],
        AppMode::DocView => vec![
            ("\u{2191}\u{2193}/jk", "scroll"),
            ("n/p", "decl"),
            ("o", "toc"),
            ("/", "find"),
            ("Ctrl-k", "commands"),
            ("s", "source"),
            ("?", "help"),
            ("Esc", "back"),
        ],
        AppMode::SourceView => vec![
            ("\u{2191}\u{2193}/jk", "scroll"),
            ("g/G", "top/bottom"),
            ("y", "copy"),
            ("?", "help"),
            ("Esc", "back"),
            ("q", "quit"),
        ],
        AppMode::Help => vec![("\u{2191}\u{2193}/jk", "scroll"), ("?/Esc", "close")],
    };

    let mut right_spans: Vec<Span> = Vec::new();
    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            right_spans.push(Span::styled(" \u{2502} ", status_style));
        }
        right_spans.push(Span::styled(*key, key_style));
        right_spans.push(Span::styled(format!(" {desc}"), hint_style));
    }
    right_spans.push(Span::styled(" ", status_style));

    // Combine: fill middle with spaces
    let left_len = spans_width(left_spans.iter());
    let right_len = spans_width(right_spans.iter());
    let padding = area_width.saturating_sub(left_len + right_len);

    let mut all_spans = left_spans;
    all_spans.push(Span::styled(" ".repeat(padding), status_style));
    all_spans.extend(right_spans);

    let bar = Paragraph::new(Line::from(all_spans)).style(status_style);
    frame.render_widget(bar, area);
}

fn message_width(area_width: usize, used_width: usize) -> usize {
    area_width
        .saturating_sub(used_width + 1)
        .min((area_width / 2).clamp(8, 48))
}

fn compact_count(count: usize) -> String {
    const UNITS: &[(usize, &str)] = &[
        (1_000_000_000_000_000_000, "E"),
        (1_000_000_000_000_000, "P"),
        (1_000_000_000_000, "T"),
        (1_000_000_000, "B"),
        (1_000_000, "M"),
        (1_000, "K"),
    ];

    for (unit, suffix) in UNITS {
        if count >= *unit {
            let scaled = count as f64 / *unit as f64;
            return if scaled >= 100.0 {
                format!("{scaled:.0}{suffix}")
            } else if scaled >= 10.0 {
                format!("{scaled:.1}{suffix}")
            } else {
                format!("{scaled:.2}{suffix}")
            };
        }
    }

    count.to_string()
}
