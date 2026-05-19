use crate::app::{App, PopupMode};
use crate::ui::{module_browser, package_popup, theme_popup, yank_popup};

impl App {
    pub(crate) fn open_yank_menu(&mut self) {
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

    pub(crate) fn toggle_compact_results(&mut self) {
        self.results.compact = !self.results.compact;
    }

    pub(crate) fn open_module_browser(&mut self) {
        self.module_browser = Some(module_browser::ModuleBrowserState::new(&self.all_results));
        self.popup = Some(PopupMode::ModuleBrowser);
    }

    pub(crate) fn pin_selected_result(&mut self) {
        if let Some(result) = self.results.selected_result() {
            self.pinned.pin(result);
            self.show_info("Pinned!");
        }
    }

    pub(crate) fn clear_pinned_results(&mut self) {
        self.pinned.clear();
        self.show_info("All pins cleared");
    }

    pub(crate) fn toggle_multi_select_current(&mut self) {
        if !self.results.multi_select_mode {
            self.results.multi_select_mode = true;
        }
        self.results.toggle_select_current();
        self.results.move_down();
    }

    pub(crate) fn toggle_group_by_module(&mut self) {
        self.results.group_by_module = !self.results.group_by_module;
    }
}
