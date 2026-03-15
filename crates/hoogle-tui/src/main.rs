mod actions;
mod app;
mod bookmarks;
mod cli;
mod clipboard;
mod event;
mod history;
mod keymap;
mod ui;

use std::io;
use std::panic;
use std::time::Duration;

use clap::Parser;
use crossterm::{
    event::DisableMouseCapture,
    event::EnableMouseCapture,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;
use tracing_subscriber::EnvFilter;

use app::{App, AppMode};
use cli::CliArgs;
use event::{map_event_to_action, AppEvent, EventHandler};
use keymap::Keymap;

fn setup_terminal(mouse: bool) -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    if mouse {
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    } else {
        execute!(stdout, EnterAlternateScreen)?;
    }
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
}

fn setup_logging(log_level: &str) {
    let log_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("hoogle-tui");

    if std::fs::create_dir_all(&log_dir).is_err() {
        return;
    }

    let log_file = log_dir.join("hoogle-tui.log");

    // Simple log rotation: truncate if over 5MB
    if let Ok(meta) = std::fs::metadata(&log_file) {
        if meta.len() > 5 * 1024 * 1024 {
            let _ = std::fs::write(&log_file, "");
        }
    }

    let file = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
    {
        Ok(f) => f,
        Err(_) => return,
    };

    let filter = EnvFilter::try_new(log_level).unwrap_or_else(|_| EnvFilter::new("warn"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(file)
        .with_ansi(false)
        .init();
}

fn install_panic_hook() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let args = CliArgs::parse();

    // Handle --completions and exit
    if args.handle_completions() {
        return Ok(());
    }

    setup_logging(&args.log_level);
    install_panic_hook();

    let mut config = hoogle_core::config::Config::load(args.config.as_ref());
    args.apply_to_config(&mut config);

    tracing::info!("starting hoogle-tui");

    // Create backend
    let backend = match hoogle_core::backend::create_backend(&config.backend).await {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error: {e}");
            eprintln!("Tip: Install hoogle (`cabal install hoogle && hoogle generate`) or use --backend web");
            std::process::exit(1);
        }
    };

    let keymap = Keymap::new(&config.keybinds);
    let mut app = App::new(config, backend);

    // Handle initial query from CLI
    if let Some(ref query) = args.query {
        app.set_initial_query(query);
    }

    let mut terminal = setup_terminal(app.config.ui.mouse_enabled)?;
    let mut events = EventHandler::new(Duration::from_millis(33));

    // Main event loop
    loop {
        terminal.draw(|frame| app.draw(frame))?;

        if let Some(event) = events.next().await {
            // In search mode, let the textarea handle key events first
            if app.mode == AppMode::Search {
                if let AppEvent::Key(key) = &event {
                    let action = map_event_to_action(&event, app.mode, &keymap);
                    // Let certain actions bypass textarea
                    match action {
                        actions::Action::Back
                        | actions::Action::FocusResults
                        | actions::Action::Quit
                        | actions::Action::SearchHistory
                        | actions::Action::ClearSearch => {
                            app.handle_action(action);
                        }
                        _ => {
                            // Let textarea consume the input
                            app.handle_search_input(*key);
                        }
                    }
                } else {
                    let action = map_event_to_action(&event, app.mode, &keymap);
                    app.handle_action(action);
                }
            } else if app.doc_state.search_active {
                // Doc search sub-mode: keypresses go to search input
                if let AppEvent::Key(key) = &event {
                    app.handle_doc_search_input(*key);
                } else {
                    let action = map_event_to_action(&event, app.mode, &keymap);
                    app.handle_action(action);
                }
            } else if app.mode == AppMode::Results
                && app.results.fuzzy_filter.is_some()
                && app.popup.is_none()
            {
                // Fuzzy filter sub-mode in results
                if let AppEvent::Key(key) = &event {
                    app.handle_fuzzy_filter_input(*key, &keymap);
                } else {
                    let action = map_event_to_action(&event, app.mode, &keymap);
                    app.handle_action(action);
                }
            } else {
                if let AppEvent::Key(key) = &event {
                    let action = map_event_to_action(&event, app.mode, &keymap);
                    // In Results mode, unbound letter keys start fuzzy filter
                    if app.mode == AppMode::Results
                        && action == actions::Action::None
                        && app.popup.is_none()
                    {
                        if let crossterm::event::KeyCode::Char(c) = key.code {
                            if c.is_alphanumeric()
                                && !key
                                    .modifiers
                                    .intersects(crossterm::event::KeyModifiers::CONTROL)
                            {
                                app.results.start_fuzzy_filter();
                                app.results.fuzzy_add_char(c);
                                continue;
                            }
                        }
                    }
                    app.handle_action(action);
                } else {
                    let action = map_event_to_action(&event, app.mode, &keymap);
                    app.handle_action(action);
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    restore_terminal();
    tracing::info!("hoogle-tui exited cleanly");

    Ok(())
}
