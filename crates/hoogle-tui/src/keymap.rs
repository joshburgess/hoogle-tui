use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use hoogle_core::config::KeybindOverrides;

use crate::actions::Action;
use crate::app::AppMode;

pub struct Keymap {
    bindings: HashMap<(AppMode, KeyEvent), Action>,
}

impl Keymap {
    pub fn new(overrides: &KeybindOverrides) -> Self {
        let mut km = Self {
            bindings: HashMap::new(),
        };
        km.load_defaults();
        km.apply_overrides(overrides);
        km
    }

    pub fn resolve(&self, mode: AppMode, event: KeyEvent) -> Action {
        self.bindings
            .get(&(mode, normalize_key(event)))
            .cloned()
            .unwrap_or(Action::None)
    }

    fn bind(&mut self, mode: AppMode, code: KeyCode, modifiers: KeyModifiers, action: Action) {
        let event = KeyEvent::new(code, modifiers);
        self.bindings.insert((mode, event), action);
    }

    fn bind_key(&mut self, mode: AppMode, code: KeyCode, action: Action) {
        self.bind(mode, code, KeyModifiers::NONE, action);
    }

    fn load_defaults(&mut self) {
        use Action::*;
        use KeyCode::*;
        // Global — these work in every mode
        for mode in AppMode::ALL {
            self.bind(mode, Char('c'), KeyModifiers::CONTROL, Quit);
            self.bind(mode, Char('l'), KeyModifiers::CONTROL, Redraw);
            self.bind_key(mode, F(1), ToggleHelp);
            self.bind(mode, Char('/'), KeyModifiers::CONTROL, ToggleHelp);
            self.bind(mode, Char('t'), KeyModifiers::CONTROL, OpenThemeSwitcher);
        }

        // Search mode
        self.bind_key(AppMode::Search, Enter, FocusResults);
        self.bind_key(AppMode::Search, Esc, Back);
        self.bind(
            AppMode::Search,
            Char('r'),
            KeyModifiers::CONTROL,
            SearchHistory,
        );
        self.bind(
            AppMode::Search,
            Char('u'),
            KeyModifiers::CONTROL,
            ClearSearch,
        );

        // Results mode
        self.bind_key(AppMode::Results, Char('j'), MoveDown);
        self.bind_key(AppMode::Results, Down, MoveDown);
        self.bind_key(AppMode::Results, Char('k'), MoveUp);
        self.bind_key(AppMode::Results, Up, MoveUp);
        self.bind_key(AppMode::Results, Char('g'), MoveToTop);
        self.bind_key(AppMode::Results, Char('G'), MoveToBottom);
        self.bind_key(AppMode::Results, Enter, Select);
        self.bind_key(AppMode::Results, Tab, TogglePreview);
        self.bind_key(AppMode::Results, Char(' '), ScrollDown);
        self.bind_key(AppMode::Results, Char('/'), FocusSearch);
        self.bind_key(AppMode::Results, Char('f'), OpenFilter);
        self.bind_key(AppMode::Results, Char('s'), OpenSort);
        self.bind_key(AppMode::Results, Char('y'), YankSignature);
        self.bind_key(AppMode::Results, Char('Y'), YankImport);
        self.bind(AppMode::Results, Char('y'), KeyModifiers::CONTROL, YankUrl);
        self.bind_key(AppMode::Results, Char('m'), Bookmark);
        self.bind_key(AppMode::Results, Char('\''), OpenBookmarks);
        self.bind(
            AppMode::Results,
            Char('d'),
            KeyModifiers::CONTROL,
            DeleteEntry,
        );
        // d in Results is unbound (DeleteEntry only makes sense in History/Bookmarks popups)
        self.bind(
            AppMode::Results,
            Char('d'),
            KeyModifiers::CONTROL,
            DeleteEntry,
        );
        self.bind_key(AppMode::Results, Char('q'), Quit);
        self.bind_key(AppMode::Results, Esc, Back);
        self.bind_key(AppMode::Results, Char('?'), ToggleHelp);
        // Yank menu, package scope, theme, compact, browser, export
        self.bind_key(AppMode::Results, Char('c'), OpenYankMenu);
        self.bind(
            AppMode::Results,
            Char('p'),
            KeyModifiers::CONTROL,
            OpenPackageScope,
        );
        // Ctrl-t for theme is now global (bound above)
        self.bind_key(AppMode::Results, Char('v'), ToggleCompact);
        self.bind(
            AppMode::Results,
            Char('o'),
            KeyModifiers::CONTROL,
            OpenInBrowser,
        );
        self.bind(
            AppMode::Results,
            Char('e'),
            KeyModifiers::CONTROL,
            ExportSession,
        );
        // Module browser: use M (capital) since Ctrl-m == Enter on most terminals
        self.bind_key(AppMode::Results, Char('M'), OpenModuleBrowser);
        self.bind_key(AppMode::Results, Char('P'), PinResult);
        self.bind(AppMode::Results, Char('x'), KeyModifiers::CONTROL, UnpinAll);
        self.bind_key(AppMode::Results, Char('x'), ToggleMultiSelect);
        self.bind_key(AppMode::Results, Char('I'), YankSelectedImports);
        self.bind_key(AppMode::Results, Char('w'), ToggleGroupByModule);
        self.bind_key(AppMode::Results, Char('T'), YankGhciType);
        self.bind_key(AppMode::Results, Char('D'), YankGhciInfo);

        // DocView mode
        self.bind_key(AppMode::DocView, Char('j'), ScrollDown);
        self.bind_key(AppMode::DocView, Down, ScrollDown);
        self.bind_key(AppMode::DocView, Char('k'), ScrollUp);
        self.bind_key(AppMode::DocView, Up, ScrollUp);
        self.bind_key(AppMode::DocView, Char('d'), ScrollHalfDown);
        self.bind(
            AppMode::DocView,
            Char('d'),
            KeyModifiers::CONTROL,
            ScrollHalfDown,
        );
        self.bind_key(AppMode::DocView, Char('u'), ScrollHalfUp);
        self.bind(
            AppMode::DocView,
            Char('u'),
            KeyModifiers::CONTROL,
            ScrollHalfUp,
        );
        self.bind_key(AppMode::DocView, Char('f'), ScrollPageDown);
        self.bind(
            AppMode::DocView,
            Char('f'),
            KeyModifiers::CONTROL,
            ScrollPageDown,
        );
        self.bind_key(AppMode::DocView, Char('b'), ScrollPageUp);
        self.bind(
            AppMode::DocView,
            Char('b'),
            KeyModifiers::CONTROL,
            ScrollPageUp,
        );
        self.bind_key(AppMode::DocView, Char('g'), MoveToTop);
        self.bind_key(AppMode::DocView, Char('G'), MoveToBottom);
        self.bind_key(AppMode::DocView, Char('o'), OpenTOC);
        self.bind_key(AppMode::DocView, Enter, FollowLink);
        self.bind_key(AppMode::DocView, Backspace, NavBack);
        self.bind_key(AppMode::DocView, Tab, CycleLink);
        self.bind_key(AppMode::DocView, Char('n'), NextDeclaration);
        self.bind_key(AppMode::DocView, Char('p'), PrevDeclaration);
        self.bind_key(AppMode::DocView, Char('s'), ViewSource);
        self.bind_key(AppMode::DocView, Char('/'), SearchInDoc);
        self.bind_key(AppMode::DocView, Esc, Back);
        self.bind_key(AppMode::DocView, Char('q'), Quit);
        self.bind_key(AppMode::DocView, Char('?'), ToggleHelp);
        self.bind(
            AppMode::DocView,
            Char('o'),
            KeyModifiers::CONTROL,
            OpenInBrowser,
        );
        // Ctrl-t for theme is now global (bound above)
        self.bind(
            AppMode::DocView,
            Char('e'),
            KeyModifiers::CONTROL,
            ExportSession,
        );
        self.bind_key(AppMode::DocView, Char('y'), YankDeclLink);
        self.bind_key(AppMode::DocView, Char('T'), YankGhciType);
        self.bind_key(AppMode::DocView, Char('D'), YankGhciInfo);

        // SourceView mode
        self.bind_key(AppMode::SourceView, Char('j'), ScrollDown);
        self.bind_key(AppMode::SourceView, Down, ScrollDown);
        self.bind_key(AppMode::SourceView, Char('k'), ScrollUp);
        self.bind_key(AppMode::SourceView, Up, ScrollUp);
        self.bind_key(AppMode::SourceView, Char('g'), MoveToTop);
        self.bind_key(AppMode::SourceView, Char('G'), MoveToBottom);
        self.bind_key(AppMode::SourceView, Esc, Back);
        self.bind_key(AppMode::SourceView, Char('q'), Quit);
        self.bind_key(AppMode::SourceView, Char('?'), ToggleHelp);
        self.bind_key(AppMode::SourceView, Char('y'), YankSignature);

        // Help mode — q closes help (not quit app)
        self.bind_key(AppMode::Help, Esc, Back);
        self.bind_key(AppMode::Help, Char('?'), Back);
        self.bind_key(AppMode::Help, Char('q'), Back);
        self.bind_key(AppMode::Help, Char('j'), ScrollDown);
        self.bind_key(AppMode::Help, Down, ScrollDown);
        self.bind_key(AppMode::Help, Char('k'), ScrollUp);
        self.bind_key(AppMode::Help, Up, ScrollUp);
    }

    fn apply_overrides(&mut self, overrides: &KeybindOverrides) {
        for (action_name, key_str) in &overrides.overrides {
            let Some(action) = parse_action_name(action_name) else {
                tracing::warn!("unknown action in keybind override: {action_name}");
                continue;
            };
            let Some(event) = parse_key_string(key_str) else {
                tracing::warn!("invalid key string in keybind override: {key_str}");
                continue;
            };
            // Apply to all modes where this action makes sense
            for mode in AppMode::ALL {
                self.bindings.insert((mode, event), action.clone());
            }
        }
    }
}

fn parse_action_name(name: &str) -> Option<Action> {
    Some(match name {
        "quit" => Action::Quit,
        "back" => Action::Back,
        "focus_search" => Action::FocusSearch,
        "focus_results" => Action::FocusResults,
        "move_up" => Action::MoveUp,
        "move_down" => Action::MoveDown,
        "move_to_top" => Action::MoveToTop,
        "move_to_bottom" => Action::MoveToBottom,
        "select" => Action::Select,
        "toggle_preview" => Action::TogglePreview,
        "toggle_help" => Action::ToggleHelp,
        "scroll_down" => Action::ScrollDown,
        "scroll_up" => Action::ScrollUp,
        "scroll_half_down" => Action::ScrollHalfDown,
        "scroll_half_up" => Action::ScrollHalfUp,
        "scroll_page_down" => Action::ScrollPageDown,
        "scroll_page_up" => Action::ScrollPageUp,
        "next_declaration" => Action::NextDeclaration,
        "prev_declaration" => Action::PrevDeclaration,
        "open_toc" => Action::OpenTOC,
        "follow_link" => Action::FollowLink,
        "cycle_link" => Action::CycleLink,
        "nav_back" => Action::NavBack,
        "open_filter" => Action::OpenFilter,
        "open_sort" => Action::OpenSort,
        "yank_signature" => Action::YankSignature,
        "yank_import" => Action::YankImport,
        "yank_url" => Action::YankUrl,
        "view_source" => Action::ViewSource,
        "search_in_doc" => Action::SearchInDoc,
        "search_history" => Action::SearchHistory,
        "clear_search" => Action::ClearSearch,
        "bookmark" => Action::Bookmark,
        "open_bookmarks" => Action::OpenBookmarks,
        "open_yank_menu" => Action::OpenYankMenu,
        "open_package_scope" => Action::OpenPackageScope,
        "open_theme_switcher" => Action::OpenThemeSwitcher,
        "toggle_compact" => Action::ToggleCompact,
        "open_in_browser" => Action::OpenInBrowser,
        "export_session" => Action::ExportSession,
        "tab_complete" => Action::TabComplete,
        "load_more" => Action::LoadMore,
        "open_module_browser" => Action::OpenModuleBrowser,
        "pin_result" => Action::PinResult,
        "unpin_all" => Action::UnpinAll,
        "toggle_multi_select" => Action::ToggleMultiSelect,
        "yank_selected_imports" => Action::YankSelectedImports,
        "toggle_group_by_module" => Action::ToggleGroupByModule,
        "yank_ghci_type" => Action::YankGhciType,
        "yank_ghci_info" => Action::YankGhciInfo,
        "yank_decl_link" => Action::YankDeclLink,
        "detect_project" => Action::DetectProject,
        _ => return None,
    })
}

fn parse_key_string(s: &str) -> Option<KeyEvent> {
    let s = s.trim();
    let (modifiers, key_part) = if let Some(rest) = s.strip_prefix("ctrl-") {
        (KeyModifiers::CONTROL, rest)
    } else if let Some(rest) = s.strip_prefix("alt-") {
        (KeyModifiers::ALT, rest)
    } else {
        (KeyModifiers::NONE, s)
    };

    let code = match key_part {
        "enter" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "tab" => KeyCode::Tab,
        "backspace" => KeyCode::Backspace,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "space" => KeyCode::Char(' '),
        k if k.len() == 1 => KeyCode::Char(k.chars().next().unwrap()),
        _ => return None,
    };

    Some(KeyEvent::new(code, modifiers))
}

/// Normalize key events by stripping state flags that vary across platforms
/// (e.g., SHIFT is implicit for uppercase chars).
fn normalize_key(mut event: KeyEvent) -> KeyEvent {
    // For uppercase chars, crossterm may or may not include SHIFT in modifiers.
    // Normalize by removing SHIFT for Char events since we match on the char itself.
    if let KeyCode::Char(_) = event.code {
        event.modifiers.remove(KeyModifiers::SHIFT);
    }
    // Strip platform-specific bits
    event.modifiers.remove(KeyModifiers::SUPER);
    event.modifiers.remove(KeyModifiers::HYPER);
    event.modifiers.remove(KeyModifiers::META);
    // Zero out the key event kind differences
    event.kind = crossterm::event::KeyEventKind::Press;
    event.state = crossterm::event::KeyEventState::NONE;
    event
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_keymap() -> Keymap {
        Keymap::new(&KeybindOverrides::default())
    }

    #[test]
    fn results_j_moves_down() {
        let km = default_keymap();
        let action = km.resolve(
            AppMode::Results,
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
        );
        assert_eq!(action, Action::MoveDown);
    }

    #[test]
    fn results_k_moves_up() {
        let km = default_keymap();
        let action = km.resolve(
            AppMode::Results,
            KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE),
        );
        assert_eq!(action, Action::MoveUp);
    }

    #[test]
    fn results_q_quits() {
        let km = default_keymap();
        let action = km.resolve(
            AppMode::Results,
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
        );
        assert_eq!(action, Action::Quit);
    }

    #[test]
    fn search_enter_focuses_results() {
        let km = default_keymap();
        let action = km.resolve(
            AppMode::Search,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );
        assert_eq!(action, Action::FocusResults);
    }

    #[test]
    fn docview_scroll_keys() {
        let km = default_keymap();
        assert_eq!(
            km.resolve(
                AppMode::DocView,
                KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)
            ),
            Action::ScrollDown
        );
        assert_eq!(
            km.resolve(
                AppMode::DocView,
                KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE)
            ),
            Action::MoveToTop
        );
        assert_eq!(
            km.resolve(
                AppMode::DocView,
                KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE)
            ),
            Action::MoveToBottom
        );
    }

    #[test]
    fn ctrl_c_quits_all_modes() {
        let km = default_keymap();
        for mode in AppMode::ALL {
            let action = km.resolve(
                mode,
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            );
            assert_eq!(action, Action::Quit, "Ctrl-c should quit in {mode:?}");
        }
    }

    #[test]
    fn unknown_key_returns_none() {
        let km = default_keymap();
        let action = km.resolve(
            AppMode::Results,
            KeyEvent::new(KeyCode::F(12), KeyModifiers::NONE),
        );
        assert_eq!(action, Action::None);
    }

    #[test]
    fn parse_key_string_simple() {
        let event = parse_key_string("j").unwrap();
        assert_eq!(event.code, KeyCode::Char('j'));
        assert_eq!(event.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn parse_key_string_ctrl() {
        let event = parse_key_string("ctrl-c").unwrap();
        assert_eq!(event.code, KeyCode::Char('c'));
        assert_eq!(event.modifiers, KeyModifiers::CONTROL);
    }

    #[test]
    fn parse_key_string_special() {
        assert_eq!(parse_key_string("enter").unwrap().code, KeyCode::Enter);
        assert_eq!(parse_key_string("esc").unwrap().code, KeyCode::Esc);
        assert_eq!(parse_key_string("tab").unwrap().code, KeyCode::Tab);
        assert_eq!(parse_key_string("space").unwrap().code, KeyCode::Char(' '));
    }

    #[test]
    fn parse_key_string_invalid() {
        assert!(parse_key_string("nonexistent").is_none());
    }

    #[test]
    fn parse_action_name_valid() {
        assert_eq!(parse_action_name("quit"), Some(Action::Quit));
        assert_eq!(parse_action_name("scroll_down"), Some(Action::ScrollDown));
        assert_eq!(
            parse_action_name("yank_signature"),
            Some(Action::YankSignature)
        );
    }

    #[test]
    fn parse_action_name_invalid() {
        assert!(parse_action_name("nonexistent").is_none());
    }

    // --- Tests for fixes to keybinding bugs ---

    #[test]
    fn ctrl_t_opens_theme_in_all_modes() {
        let km = default_keymap();
        for mode in AppMode::ALL {
            let action = km.resolve(
                mode,
                KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL),
            );
            assert_eq!(
                action,
                Action::OpenThemeSwitcher,
                "Ctrl-t should open theme switcher in {mode:?}"
            );
        }
    }

    #[test]
    fn q_in_help_goes_back_not_quit() {
        let km = default_keymap();
        let action = km.resolve(
            AppMode::Help,
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
        );
        assert_eq!(action, Action::Back);
    }

    #[test]
    fn q_in_source_view_quits() {
        let km = default_keymap();
        let action = km.resolve(
            AppMode::SourceView,
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
        );
        assert_eq!(action, Action::Quit);
    }

    #[test]
    fn help_available_in_source_view() {
        let km = default_keymap();
        let action = km.resolve(
            AppMode::SourceView,
            KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE),
        );
        assert_eq!(action, Action::ToggleHelp);
    }

    #[test]
    fn d_in_results_is_not_delete_entry() {
        let km = default_keymap();
        let action = km.resolve(
            AppMode::Results,
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
        );
        // 'd' should be unbound in Results (Action::None), not DeleteEntry
        assert_eq!(action, Action::None);
    }

    #[test]
    fn ctrl_d_in_results_is_delete_entry() {
        let km = default_keymap();
        let action = km.resolve(
            AppMode::Results,
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
        );
        assert_eq!(action, Action::DeleteEntry);
    }

    #[test]
    fn capital_m_opens_module_browser() {
        let km = default_keymap();
        let action = km.resolve(
            AppMode::Results,
            KeyEvent::new(KeyCode::Char('M'), KeyModifiers::NONE),
        );
        assert_eq!(action, Action::OpenModuleBrowser);
    }

    #[test]
    fn docview_arrows_are_scroll_not_move() {
        // This verifies the bug: in DocView, j/Down map to ScrollDown, not MoveDown.
        // The popup router in main.rs must handle this by mapping keys directly,
        // not relying on the mode-specific keymap.
        let km = default_keymap();
        assert_eq!(
            km.resolve(
                AppMode::DocView,
                KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)
            ),
            Action::ScrollDown
        );
        assert_eq!(
            km.resolve(
                AppMode::DocView,
                KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)
            ),
            Action::ScrollDown
        );
        // This is why the popup router maps j/Down directly to MoveDown,
        // bypassing the mode-specific keymap.
    }

    #[test]
    fn keybind_overrides_apply() {
        let mut overrides = KeybindOverrides::default();
        overrides
            .overrides
            .insert("scroll_down".into(), "ctrl-n".into());

        let km = Keymap::new(&overrides);
        // Ctrl-n should now map to ScrollDown in all modes
        let action = km.resolve(
            AppMode::Results,
            KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL),
        );
        assert_eq!(action, Action::ScrollDown);
    }
}
