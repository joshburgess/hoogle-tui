use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::AppMode;

const SPINNER_FRAMES: &[char] = &[
    '\u{280b}', '\u{2819}', '\u{2839}', '\u{2838}', '\u{283c}', '\u{2834}', '\u{2826}', '\u{2827}',
    '\u{2807}', '\u{280f}',
];

pub struct StatusState {
    pub backend_name: String,
    pub result_count: usize,
    pub message: Option<StatusMessage>,
    pub spinner_tick: usize,
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

pub fn render(frame: &mut Frame, area: Rect, state: &StatusState, mode: AppMode, theme: &Theme) {
    let status_style = theme.style(SemanticToken::StatusBar);
    let key_style = theme.style(SemanticToken::ModuleName);
    let hint_style = status_style;

    let mut left_spans = vec![
        Span::styled(format!(" {} ", state.backend_name), status_style),
        Span::styled("\u{2502} ", status_style),
    ];

    // Show message or result count
    match &state.message {
        Some(StatusMessage::Loading(msg)) => {
            let spinner = SPINNER_FRAMES[state.spinner_tick];
            left_spans.push(Span::styled(
                format!("{spinner} {msg} "),
                theme.style(SemanticToken::Spinner),
            ));
        }
        Some(StatusMessage::Error(msg)) => {
            left_spans.push(Span::styled(
                format!(" {msg} "),
                theme.style(SemanticToken::Error),
            ));
        }
        Some(StatusMessage::Info(msg)) => {
            left_spans.push(Span::styled(format!(" {msg} "), status_style));
        }
        None => {
            left_spans.push(Span::styled(
                format!("{} results ", state.result_count),
                status_style,
            ));
        }
    }

    // Right side: contextual key hints
    let hints = match mode {
        AppMode::Search => vec![("Enter", "results"), ("Esc", "quit")],
        AppMode::Results => vec![
            ("j/k", "navigate"),
            ("Enter", "open"),
            ("/", "search"),
            ("?", "help"),
            ("q", "quit"),
        ],
        AppMode::DocView => vec![
            ("j/k", "scroll"),
            ("n/p", "next/prev"),
            ("o", "toc"),
            ("Esc", "back"),
        ],
        AppMode::SourceView => vec![("j/k", "scroll"), ("Esc", "back")],
        AppMode::Help => vec![("j/k", "scroll"), ("Esc", "close")],
    };

    let mut right_spans: Vec<Span> = Vec::new();
    for (key, desc) in hints {
        right_spans.push(Span::styled(format!(" {key}"), key_style));
        right_spans.push(Span::styled(format!(":{desc}"), hint_style));
    }
    right_spans.push(Span::styled(" ", status_style));

    // Combine: fill middle with spaces
    let left_len: usize = left_spans.iter().map(|s| s.content.len()).sum();
    let right_len: usize = right_spans.iter().map(|s| s.content.len()).sum();
    let padding = (area.width as usize).saturating_sub(left_len + right_len);

    let mut all_spans = left_spans;
    all_spans.push(Span::styled(" ".repeat(padding), status_style));
    all_spans.extend(right_spans);

    let bar = Paragraph::new(Line::from(all_spans)).style(status_style);
    frame.render_widget(bar, area);
}
