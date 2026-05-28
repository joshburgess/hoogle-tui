use crate::app::{App, PopupMode};
use crate::clipboard;
use crate::ui::{command_palette, module_browser, package_popup, theme_popup, yank_popup};

impl App {
    pub(crate) fn open_command_palette(&mut self) {
        use crate::actions::Action;
        use command_palette::CommandEntry;

        fn cmd(
            group: &'static str,
            label: &'static str,
            hint: &'static str,
            action: Action,
        ) -> CommandEntry {
            CommandEntry {
                group,
                label,
                hint,
                action,
            }
        }

        let entries = vec![
            cmd("Global", "Focus search", "/", Action::FocusSearch),
            cmd("Global", "Show help", "F1", Action::ToggleHelp),
            cmd(
                "Global",
                "Switch theme",
                "Ctrl-t",
                Action::OpenThemeSwitcher,
            ),
            cmd("Docs", "Open selected docs", "Enter", Action::Select),
            cmd("Docs", "Open table of contents", "o", Action::OpenTOC),
            cmd("Docs", "Search in document", "/", Action::SearchInDoc),
            cmd("Docs", "View source", "s", Action::ViewSource),
            cmd("Docs", "Next declaration", "n", Action::NextDeclaration),
            cmd("Docs", "Previous declaration", "p", Action::PrevDeclaration),
            cmd("Results", "Toggle preview", "Tab", Action::TogglePreview),
            cmd(
                "Results",
                "Toggle compact results",
                "v",
                Action::ToggleCompact,
            ),
            cmd(
                "Results",
                "Toggle module grouping",
                "w",
                Action::ToggleGroupByModule,
            ),
            cmd("Results", "Filter results", "f", Action::OpenFilter),
            cmd("Results", "Sort results", "s", Action::OpenSort),
            cmd("Results", "Load more results", "L", Action::LoadMore),
            cmd(
                "Results",
                "Open module browser",
                "M",
                Action::OpenModuleBrowser,
            ),
            cmd("Pins", "Pin selected result", "P", Action::PinResult),
            cmd("Pins", "Clear pinned results", "Ctrl-x", Action::UnpinAll),
            cmd(
                "Selection",
                "Toggle multi-select",
                "x",
                Action::ToggleMultiSelect,
            ),
            cmd(
                "Selection",
                "Copy selected imports",
                "I",
                Action::YankSelectedImports,
            ),
            cmd(
                "Project",
                "Toggle project scope",
                "project packages",
                Action::ToggleProjectScope,
            ),
            cmd(
                "Project",
                "Set package scope",
                "Ctrl-p",
                Action::OpenPackageScope,
            ),
            cmd(
                "Copy",
                "Copy pinned imports",
                "pins",
                Action::YankPinnedImports,
            ),
            cmd("Copy", "Copy signature", "y", Action::YankSignature),
            cmd("Copy", "Copy import", "Y", Action::YankImport),
            cmd("Copy", "Copy Hackage URL", "Ctrl-y", Action::YankUrl),
            cmd("Copy", "Copy GHCi :type", "T", Action::YankGhciType),
            cmd("Copy", "Copy GHCi :info", "D", Action::YankGhciInfo),
            cmd(
                "External",
                "Open in browser",
                "Ctrl-o",
                Action::OpenInBrowser,
            ),
            cmd("Saved", "Open bookmarks", "'", Action::OpenBookmarks),
            cmd("Saved", "Open history", "Ctrl-r", Action::SearchHistory),
            cmd("Session", "Export session", "Ctrl-e", Action::ExportSession),
            cmd("Global", "Redraw", "Ctrl-l", Action::Redraw),
        ];
        self.command_palette = Some(command_palette::CommandPaletteState::new(entries));
        self.popup = Some(PopupMode::CommandPalette);
    }

    pub(crate) fn open_yank_menu(&mut self) {
        if self.results.selected_result().is_none() {
            self.show_info("No result selected");
            return;
        }
        self.yank_popup = Some(yank_popup::YankPopupState::new());
        self.popup = Some(PopupMode::YankMenu);
    }

    pub(crate) fn open_package_scope_popup(&mut self) {
        self.package_popup = Some(package_popup::PackageScopeState::new(&self.package_scope));
        self.popup = Some(PopupMode::PackageScope);
    }

    pub(crate) fn toggle_theme_switcher(&mut self) {
        if self.popup == Some(PopupMode::ThemeSwitcher) {
            self.close_popup();
        } else {
            self.theme_popup = Some(theme_popup::ThemePopupState::new(&self.theme.name));
            self.popup = Some(PopupMode::ThemeSwitcher);
        }
    }

    pub(crate) fn export_session(&mut self) {
        match crate::export::export_session(
            &self.last_searched,
            &self.all_results,
            &self.viewed_docs,
        ) {
            Ok(path) => self.show_info(&format!("Exported to {}", path.display())),
            Err(e) => self.show_error(&format!("Export failed: {e}")),
        }
    }

    pub(crate) fn toggle_preview(&mut self) {
        self.preview_enabled = !self.preview_enabled;
        if self.preview_enabled {
            self.show_info("Preview enabled");
        } else {
            self.show_info("Preview disabled");
        }
    }

    pub(crate) fn toggle_compact_results(&mut self) {
        self.results.compact = !self.results.compact;
        if self.results.compact {
            self.show_info("Compact results enabled");
        } else {
            self.show_info("Compact results disabled");
        }
    }

    pub(crate) fn open_module_browser(&mut self) {
        let state = module_browser::ModuleBrowserState::new(&self.all_results);
        if state.selected_module().is_none() {
            self.show_info("No modules available");
            return;
        }
        self.module_browser = Some(state);
        self.popup = Some(PopupMode::ModuleBrowser);
    }

    pub(crate) fn pin_selected_result(&mut self) {
        let Some(result) = self.results.selected_result().cloned() else {
            self.show_info("No result selected");
            return;
        };

        if self.pinned.toggle(&result) {
            self.show_info("Pinned");
        } else {
            self.show_info("Unpinned");
        }
    }

    pub(crate) fn clear_pinned_results(&mut self) {
        if self.pinned.is_empty() {
            self.show_info("No pins to clear");
            return;
        }
        self.pinned.clear();
        self.show_info("All pins cleared");
    }

    pub(crate) fn yank_pinned_imports(&mut self) {
        let Some(imports) = self.pinned_imports_text() else {
            self.show_info("No pinned imports available");
            return;
        };

        let count = imports.lines().count();
        match clipboard::copy_to_clipboard(&imports) {
            Ok(()) => self.show_info(&format!("Copied {count} pinned imports")),
            Err(e) => self.show_error(&e),
        }
    }

    pub(crate) fn pinned_imports_text(&self) -> Option<String> {
        let imports: Vec<String> = self
            .pinned
            .pins
            .iter()
            .filter_map(|result| {
                let module = result.module.as_ref().map(|m| m.to_string());
                clipboard::generate_import(&result.name, module.as_deref())
            })
            .collect();

        if imports.is_empty() {
            None
        } else {
            Some(imports.join("\n"))
        }
    }

    pub(crate) fn toggle_multi_select_current(&mut self) {
        if self.results.selected_result().is_none() {
            self.show_info("No result selected");
            return;
        }
        if !self.results.multi_select_mode {
            self.results.multi_select_mode = true;
        }
        self.results.toggle_select_current();
        self.results.move_down();
    }

    pub(crate) fn toggle_group_by_module(&mut self) {
        self.results.group_by_module = !self.results.group_by_module;
        if self.results.group_by_module {
            self.show_info("Grouped by module");
        } else {
            self.show_info("Module grouping disabled");
        }
    }
}
