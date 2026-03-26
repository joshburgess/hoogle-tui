use std::time::{Duration, Instant};

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
            let mut pending_resize: Option<(u16, u16, Instant)> = None;
            let resize_debounce = Duration::from_millis(50);

            loop {
                // If we have a pending resize, use a shorter poll timeout
                let poll_timeout = if pending_resize.is_some() {
                    resize_debounce
                } else {
                    tick_rate
                };

                if event::poll(poll_timeout).unwrap_or(false) {
                    match event::read() {
                        Ok(Event::Key(key)) => {
                            // Flush any pending resize before the key event
                            if let Some((w, h, _)) = pending_resize.take() {
                                if event_tx.send(AppEvent::Resize(w, h)).is_err() {
                                    return;
                                }
                            }
                            if key.kind == KeyEventKind::Press
                                && event_tx.send(AppEvent::Key(key)).is_err()
                            {
                                return;
                            }
                        }
                        Ok(Event::Resize(w, h)) => {
                            // Coalesce resize events: store latest, don't send yet
                            pending_resize = Some((w, h, Instant::now()));
                        }
                        Ok(Event::Mouse(m)) => {
                            // Flush pending resize before mouse
                            if let Some((w, h, _)) = pending_resize.take() {
                                if event_tx.send(AppEvent::Resize(w, h)).is_err() {
                                    return;
                                }
                            }
                            if event_tx.send(AppEvent::Mouse(m)).is_err() {
                                return;
                            }
                        }
                        Ok(_) => {}
                        Err(_) => return,
                    }
                } else {
                    // Poll timed out — check if pending resize should be flushed
                    if let Some((w, h, time)) = pending_resize {
                        if time.elapsed() >= resize_debounce {
                            pending_resize = None;
                            if event_tx.send(AppEvent::Resize(w, h)).is_err() {
                                return;
                            }
                        }
                    } else {
                        // Normal tick
                        if event_tx.send(AppEvent::Tick).is_err() {
                            return;
                        }
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
