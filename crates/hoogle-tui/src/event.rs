use std::time::Duration;

use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use tokio::sync::mpsc;

use crate::actions::Action;
use crate::app::AppMode;
use crate::keymap::Keymap;

#[allow(dead_code)]
pub enum AppEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    Mouse(event::MouseEvent),
    Tick,
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<AppEvent>,
    _tx: mpsc::UnboundedSender<AppEvent>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let event_tx = tx.clone();

        // Spawn a blocking thread for reading terminal events
        std::thread::spawn(move || {
            loop {
                if event::poll(tick_rate).unwrap_or(false) {
                    match event::read() {
                        Ok(Event::Key(key)) => {
                            if key.kind == KeyEventKind::Press
                                && event_tx.send(AppEvent::Key(key)).is_err()
                            {
                                return;
                            }
                        }
                        Ok(Event::Resize(w, h)) => {
                            if event_tx.send(AppEvent::Resize(w, h)).is_err() {
                                return;
                            }
                        }
                        Ok(Event::Mouse(m)) => {
                            if event_tx.send(AppEvent::Mouse(m)).is_err() {
                                return;
                            }
                        }
                        Ok(_) => {}
                        Err(_) => return,
                    }
                } else {
                    // Tick on timeout
                    if event_tx.send(AppEvent::Tick).is_err() {
                        return;
                    }
                }
            }
        });

        Self { rx, _tx: tx }
    }

    pub async fn next(&mut self) -> Option<AppEvent> {
        self.rx.recv().await
    }
}

pub fn map_event_to_action(event: &AppEvent, mode: AppMode, keymap: &Keymap) -> Action {
    match event {
        AppEvent::Key(key) => keymap.resolve(mode, *key),
        AppEvent::Tick => Action::Tick,
        AppEvent::Resize(_, _) => Action::Redraw,
        AppEvent::Mouse(mouse) => match mouse.kind {
            event::MouseEventKind::ScrollDown => Action::ScrollDown,
            event::MouseEventKind::ScrollUp => Action::ScrollUp,
            _ => Action::None,
        },
    }
}
