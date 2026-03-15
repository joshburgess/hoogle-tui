use arboard::Clipboard;

pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| format!("clipboard unavailable: {e}"))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("failed to copy: {e}"))
}

/// Generate an import statement for a search result.
pub fn generate_import(name: &str, module: Option<&str>) -> Option<String> {
    let module = module?;
    Some(format!("import {module} ({name})"))
}
