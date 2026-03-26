mod actions;
mod app;
mod bookmarks;
mod cli;
mod clipboard;
mod event;
mod export;
mod history;
mod keymap;
mod project;
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

    // Handle --generate and exit
    if args.handle_generate() {
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
    // Auto-detect Haskell project for package scoping
    if let Some(info) = project::detect_project() {
        let count = info.dependencies.len();
        app.package_scope = info.dependencies;
        app.status.package_scope = app.package_scope.clone();
        tracing::info!("detected {:?} project with {count} deps", info.project_type);
    }

    if let Some(ref query) = args.query {
        app.set_initial_query(query);
    }

    let mut terminal = setup_terminal(app.config.ui.mouse_enabled)?;
    let mut events = EventHandler::new(Duration::from_millis(33));

    // Main event loop
    loop {
        terminal.draw(|frame| app.draw(frame))?;

        if let Some(event) = events.next().await {
            // Mouse events are handled directly by the app
            if let AppEvent::Mouse(mouse) = &event {
                app.handle_mouse(*mouse);
            }
            // Standard popups: intercept keys for j/k/Enter/Esc navigation
            // This must come before mode-specific routing so popups work
            // regardless of the underlying app mode.
            else if matches!(
                app.popup,
                Some(
                    app::PopupMode::Filter
                        | app::PopupMode::Sort
                        | app::PopupMode::Toc
                        | app::PopupMode::History
                        | app::PopupMode::Bookmarks
                        | app::PopupMode::YankMenu
                        | app::PopupMode::ThemeSwitcher
                )
            ) {
                match &event {
                    AppEvent::Key(key) => {
                        use crossterm::event::KeyCode;
                        let action = match key.code {
                            KeyCode::Char('j') | KeyCode::Down => actions::Action::MoveDown,
                            KeyCode::Char('k') | KeyCode::Up => actions::Action::MoveUp,
                            KeyCode::Enter => actions::Action::Select,
                            KeyCode::Esc => actions::Action::Back,
                            KeyCode::Char('q') => actions::Action::Back,
                            KeyCode::Char('d')
                                if key
                                    .modifiers
                                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
                            {
                                actions::Action::DeleteEntry
                            }
                            KeyCode::Char('t')
                                if key
                                    .modifiers
                                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
                            {
                                actions::Action::OpenThemeSwitcher
                            }
                            _ => actions::Action::Tick, // ignore other keys but still tick
                        };
                        app.handle_action(action);
                    }
                    AppEvent::Tick => app.handle_action(actions::Action::Tick),
                    _ => {
                        let action = map_event_to_action(&event, app.mode, &keymap);
                        app.handle_action(action);
                    }
                }
            }
            // Module browser popup needs text input for filter
            else if app.popup == Some(app::PopupMode::ModuleBrowser) {
                match &event {
                    AppEvent::Key(key) => {
                        use crossterm::event::KeyCode;
                        match key.code {
                            KeyCode::Enter => app.handle_action(actions::Action::Select),
                            KeyCode::Esc => app.handle_action(actions::Action::Back),
                            KeyCode::Char('j') | KeyCode::Down => {
                                app.handle_action(actions::Action::MoveDown);
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                app.handle_action(actions::Action::MoveUp);
                            }
                            KeyCode::Char(' ') => {
                                app.handle_action(actions::Action::ScrollDown);
                            }
                            KeyCode::Backspace => {
                                if let Some(ref mut mb) = app.module_browser {
                                    mb.delete_filter_char();
                                }
                            }
                            KeyCode::Char(c) => {
                                if let Some(ref mut mb) = app.module_browser {
                                    mb.add_filter_char(c);
                                }
                            }
                            _ => {}
                        }
                    }
                    AppEvent::Tick => app.handle_action(actions::Action::Tick),
                    _ => {
                        let action = map_event_to_action(&event, app.mode, &keymap);
                        app.handle_action(action);
                    }
                }
            }
            // Package scope popup needs text input
            else if app.popup == Some(app::PopupMode::PackageScope) {
                match &event {
                    AppEvent::Key(key) => {
                        use crossterm::event::KeyCode;
                        match key.code {
                            KeyCode::Enter => app.handle_action(actions::Action::Select),
                            KeyCode::Esc => app.handle_action(actions::Action::Back),
                            KeyCode::Backspace => {
                                if let Some(ref mut pp) = app.package_popup {
                                    pp.delete_char();
                                }
                            }
                            KeyCode::Char('u')
                                if key
                                    .modifiers
                                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
                            {
                                app.handle_action(actions::Action::ClearSearch);
                            }
                            KeyCode::Char(c) => {
                                if let Some(ref mut pp) = app.package_popup {
                                    pp.add_char(c);
                                }
                            }
                            _ => {}
                        }
                    }
                    AppEvent::Tick => app.handle_action(actions::Action::Tick),
                    _ => {
                        let action = map_event_to_action(&event, app.mode, &keymap);
                        app.handle_action(action);
                    }
                }
            }
            // In search mode, let the textarea handle key events first
            else if app.mode == AppMode::Search {
                if let AppEvent::Key(key) = &event {
                    // Tab triggers completion
                    if key.code == crossterm::event::KeyCode::Tab {
                        app.tab_complete();
                    } else {
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
                            // F1, Ctrl-/, Ctrl-t bypass textarea
                            actions::Action::ToggleHelp
                            | actions::Action::OpenThemeSwitcher => {
                                app.handle_action(action);
                            }
                            _ => {
                                // Let textarea consume the input
                                app.handle_search_input(*key);
                            }
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
