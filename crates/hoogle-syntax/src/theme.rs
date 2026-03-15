use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticToken {
    TypeConstructor,
    TypeVariable,
    Keyword,
    Operator,
    Punctuation,
    QualifiedName,
    StringLiteral,
    NumericLiteral,
    Comment,
    Pragma,
    ModuleName,
    PackageName,
    DocText,
    DocCode,
    DocHeading,
    DocLink,
    SearchInput,
    StatusBar,
    Selected,
    Cursor,
    Border,
    Error,
    Spinner,
}

impl SemanticToken {
    pub const ALL: &'static [SemanticToken] = &[
        Self::TypeConstructor,
        Self::TypeVariable,
        Self::Keyword,
        Self::Operator,
        Self::Punctuation,
        Self::QualifiedName,
        Self::StringLiteral,
        Self::NumericLiteral,
        Self::Comment,
        Self::Pragma,
        Self::ModuleName,
        Self::PackageName,
        Self::DocText,
        Self::DocCode,
        Self::DocHeading,
        Self::DocLink,
        Self::SearchInput,
        Self::StatusBar,
        Self::Selected,
        Self::Cursor,
        Self::Border,
        Self::Error,
        Self::Spinner,
    ];
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub styles: HashMap<SemanticToken, Style>,
}

impl Theme {
    pub fn style(&self, token: SemanticToken) -> Style {
        self.styles.get(&token).copied().unwrap_or_default()
    }

    pub fn by_name(name: &str) -> Self {
        match name {
            "catppuccin_mocha" => Self::catppuccin_mocha(),
            "gruvbox_dark" => Self::gruvbox_dark(),
            "solarized_dark" => Self::solarized_dark(),
            "monokai" => Self::monokai(),
            "nord" => Self::nord(),
            _ => Self::dracula(),
        }
    }

    pub fn from_toml(path: &Path) -> Result<Self, String> {
        let contents =
            std::fs::read_to_string(path).map_err(|e| format!("failed to read theme: {e}"))?;
        let raw: TomlTheme =
            toml::from_str(&contents).map_err(|e| format!("failed to parse theme: {e}"))?;
        Ok(raw.into_theme())
    }

    pub fn dracula() -> Self {
        let mut s = HashMap::new();
        s.insert(
            SemanticToken::TypeConstructor,
            style(0xbd93f9, Modifier::BOLD),
        );
        s.insert(
            SemanticToken::TypeVariable,
            style(0xf8f8f2, Modifier::ITALIC),
        );
        s.insert(SemanticToken::Keyword, style(0xff79c6, Modifier::BOLD));
        s.insert(SemanticToken::Operator, style_fg(0xff5555));
        s.insert(SemanticToken::Punctuation, style_fg(0xf8f8f2));
        s.insert(SemanticToken::QualifiedName, style_fg(0x8be9fd));
        s.insert(SemanticToken::StringLiteral, style_fg(0xf1fa8c));
        s.insert(SemanticToken::NumericLiteral, style_fg(0xbd93f9));
        s.insert(SemanticToken::Comment, style_fg(0x6272a4));
        s.insert(SemanticToken::Pragma, style(0x6272a4, Modifier::BOLD));
        s.insert(SemanticToken::ModuleName, style_fg(0x8be9fd));
        s.insert(
            SemanticToken::PackageName,
            style(0x6272a4, Modifier::ITALIC),
        );
        s.insert(SemanticToken::DocText, style_fg(0xf8f8f2));
        s.insert(SemanticToken::DocCode, style_fg(0xf1fa8c));
        s.insert(SemanticToken::DocHeading, style(0xbd93f9, Modifier::BOLD));
        s.insert(
            SemanticToken::DocLink,
            style(0x8be9fd, Modifier::UNDERLINED),
        );
        s.insert(SemanticToken::SearchInput, style_fg(0xf8f8f2));
        s.insert(
            SemanticToken::StatusBar,
            Style::default().fg(hex(0xf8f8f2)).bg(hex(0x44475a)),
        );
        s.insert(SemanticToken::Selected, Style::default().bg(hex(0x44475a)));
        s.insert(SemanticToken::Cursor, Style::default().bg(hex(0x6272a4)));
        s.insert(SemanticToken::Border, style_fg(0x6272a4));
        s.insert(SemanticToken::Error, style_fg(0xff5555));
        s.insert(SemanticToken::Spinner, style_fg(0xbd93f9));
        Self {
            name: "dracula".into(),
            styles: s,
        }
    }

    pub fn catppuccin_mocha() -> Self {
        let mut s = HashMap::new();
        s.insert(
            SemanticToken::TypeConstructor,
            style(0xcba6f7, Modifier::BOLD),
        );
        s.insert(
            SemanticToken::TypeVariable,
            style(0xcdd6f4, Modifier::ITALIC),
        );
        s.insert(SemanticToken::Keyword, style(0xf38ba8, Modifier::BOLD));
        s.insert(SemanticToken::Operator, style_fg(0xfab387));
        s.insert(SemanticToken::Punctuation, style_fg(0xcdd6f4));
        s.insert(SemanticToken::QualifiedName, style_fg(0x89dceb));
        s.insert(SemanticToken::StringLiteral, style_fg(0xa6e3a1));
        s.insert(SemanticToken::NumericLiteral, style_fg(0xfab387));
        s.insert(SemanticToken::Comment, style_fg(0x6c7086));
        s.insert(SemanticToken::Pragma, style(0x6c7086, Modifier::BOLD));
        s.insert(SemanticToken::ModuleName, style_fg(0x89dceb));
        s.insert(
            SemanticToken::PackageName,
            style(0x6c7086, Modifier::ITALIC),
        );
        s.insert(SemanticToken::DocText, style_fg(0xcdd6f4));
        s.insert(SemanticToken::DocCode, style_fg(0xa6e3a1));
        s.insert(SemanticToken::DocHeading, style(0xcba6f7, Modifier::BOLD));
        s.insert(
            SemanticToken::DocLink,
            style(0x89b4fa, Modifier::UNDERLINED),
        );
        s.insert(SemanticToken::SearchInput, style_fg(0xcdd6f4));
        s.insert(
            SemanticToken::StatusBar,
            Style::default().fg(hex(0xcdd6f4)).bg(hex(0x313244)),
        );
        s.insert(SemanticToken::Selected, Style::default().bg(hex(0x313244)));
        s.insert(SemanticToken::Cursor, Style::default().bg(hex(0x45475a)));
        s.insert(SemanticToken::Border, style_fg(0x6c7086));
        s.insert(SemanticToken::Error, style_fg(0xf38ba8));
        s.insert(SemanticToken::Spinner, style_fg(0xcba6f7));
        Self {
            name: "catppuccin_mocha".into(),
            styles: s,
        }
    }

    pub fn gruvbox_dark() -> Self {
        let mut s = HashMap::new();
        s.insert(
            SemanticToken::TypeConstructor,
            style(0xd3869b, Modifier::BOLD),
        );
        s.insert(
            SemanticToken::TypeVariable,
            style(0xebdbb2, Modifier::ITALIC),
        );
        s.insert(SemanticToken::Keyword, style(0xfb4934, Modifier::BOLD));
        s.insert(SemanticToken::Operator, style_fg(0xfe8019));
        s.insert(SemanticToken::Punctuation, style_fg(0xebdbb2));
        s.insert(SemanticToken::QualifiedName, style_fg(0x83a598));
        s.insert(SemanticToken::StringLiteral, style_fg(0xb8bb26));
        s.insert(SemanticToken::NumericLiteral, style_fg(0xd3869b));
        s.insert(SemanticToken::Comment, style_fg(0x928374));
        s.insert(SemanticToken::Pragma, style(0x928374, Modifier::BOLD));
        s.insert(SemanticToken::ModuleName, style_fg(0x83a598));
        s.insert(
            SemanticToken::PackageName,
            style(0x928374, Modifier::ITALIC),
        );
        s.insert(SemanticToken::DocText, style_fg(0xebdbb2));
        s.insert(SemanticToken::DocCode, style_fg(0xb8bb26));
        s.insert(SemanticToken::DocHeading, style(0xfabd2f, Modifier::BOLD));
        s.insert(
            SemanticToken::DocLink,
            style(0x83a598, Modifier::UNDERLINED),
        );
        s.insert(SemanticToken::SearchInput, style_fg(0xebdbb2));
        s.insert(
            SemanticToken::StatusBar,
            Style::default().fg(hex(0xebdbb2)).bg(hex(0x3c3836)),
        );
        s.insert(SemanticToken::Selected, Style::default().bg(hex(0x3c3836)));
        s.insert(SemanticToken::Cursor, Style::default().bg(hex(0x504945)));
        s.insert(SemanticToken::Border, style_fg(0x928374));
        s.insert(SemanticToken::Error, style_fg(0xfb4934));
        s.insert(SemanticToken::Spinner, style_fg(0xfabd2f));
        Self {
            name: "gruvbox_dark".into(),
            styles: s,
        }
    }

    pub fn solarized_dark() -> Self {
        let mut s = HashMap::new();
        s.insert(
            SemanticToken::TypeConstructor,
            style(0x6c71c4, Modifier::BOLD),
        );
        s.insert(
            SemanticToken::TypeVariable,
            style(0x839496, Modifier::ITALIC),
        );
        s.insert(SemanticToken::Keyword, style(0x859900, Modifier::BOLD));
        s.insert(SemanticToken::Operator, style_fg(0xcb4b16));
        s.insert(SemanticToken::Punctuation, style_fg(0x839496));
        s.insert(SemanticToken::QualifiedName, style_fg(0x268bd2));
        s.insert(SemanticToken::StringLiteral, style_fg(0x2aa198));
        s.insert(SemanticToken::NumericLiteral, style_fg(0xd33682));
        s.insert(SemanticToken::Comment, style_fg(0x586e75));
        s.insert(SemanticToken::Pragma, style(0x586e75, Modifier::BOLD));
        s.insert(SemanticToken::ModuleName, style_fg(0x268bd2));
        s.insert(
            SemanticToken::PackageName,
            style(0x586e75, Modifier::ITALIC),
        );
        s.insert(SemanticToken::DocText, style_fg(0x839496));
        s.insert(SemanticToken::DocCode, style_fg(0x2aa198));
        s.insert(SemanticToken::DocHeading, style(0xb58900, Modifier::BOLD));
        s.insert(
            SemanticToken::DocLink,
            style(0x268bd2, Modifier::UNDERLINED),
        );
        s.insert(SemanticToken::SearchInput, style_fg(0x839496));
        s.insert(
            SemanticToken::StatusBar,
            Style::default().fg(hex(0x839496)).bg(hex(0x073642)),
        );
        s.insert(SemanticToken::Selected, Style::default().bg(hex(0x073642)));
        s.insert(SemanticToken::Cursor, Style::default().bg(hex(0x586e75)));
        s.insert(SemanticToken::Border, style_fg(0x586e75));
        s.insert(SemanticToken::Error, style_fg(0xdc322f));
        s.insert(SemanticToken::Spinner, style_fg(0xb58900));
        Self {
            name: "solarized_dark".into(),
            styles: s,
        }
    }

    pub fn monokai() -> Self {
        let mut s = HashMap::new();
        s.insert(
            SemanticToken::TypeConstructor,
            style(0x66d9ef, Modifier::BOLD),
        );
        s.insert(
            SemanticToken::TypeVariable,
            style(0xf8f8f2, Modifier::ITALIC),
        );
        s.insert(SemanticToken::Keyword, style(0xf92672, Modifier::BOLD));
        s.insert(SemanticToken::Operator, style_fg(0xf92672));
        s.insert(SemanticToken::Punctuation, style_fg(0xf8f8f2));
        s.insert(SemanticToken::QualifiedName, style_fg(0x66d9ef));
        s.insert(SemanticToken::StringLiteral, style_fg(0xe6db74));
        s.insert(SemanticToken::NumericLiteral, style_fg(0xae81ff));
        s.insert(SemanticToken::Comment, style_fg(0x75715e));
        s.insert(SemanticToken::Pragma, style(0x75715e, Modifier::BOLD));
        s.insert(SemanticToken::ModuleName, style_fg(0xa6e22e));
        s.insert(
            SemanticToken::PackageName,
            style(0x75715e, Modifier::ITALIC),
        );
        s.insert(SemanticToken::DocText, style_fg(0xf8f8f2));
        s.insert(SemanticToken::DocCode, style_fg(0xe6db74));
        s.insert(SemanticToken::DocHeading, style(0xa6e22e, Modifier::BOLD));
        s.insert(
            SemanticToken::DocLink,
            style(0x66d9ef, Modifier::UNDERLINED),
        );
        s.insert(SemanticToken::SearchInput, style_fg(0xf8f8f2));
        s.insert(
            SemanticToken::StatusBar,
            Style::default().fg(hex(0xf8f8f2)).bg(hex(0x3e3d32)),
        );
        s.insert(SemanticToken::Selected, Style::default().bg(hex(0x3e3d32)));
        s.insert(SemanticToken::Cursor, Style::default().bg(hex(0x49483e)));
        s.insert(SemanticToken::Border, style_fg(0x75715e));
        s.insert(SemanticToken::Error, style_fg(0xf92672));
        s.insert(SemanticToken::Spinner, style_fg(0xae81ff));
        Self {
            name: "monokai".into(),
            styles: s,
        }
    }

    pub fn nord() -> Self {
        let mut s = HashMap::new();
        s.insert(
            SemanticToken::TypeConstructor,
            style(0x81a1c1, Modifier::BOLD),
        );
        s.insert(
            SemanticToken::TypeVariable,
            style(0xd8dee9, Modifier::ITALIC),
        );
        s.insert(SemanticToken::Keyword, style(0x81a1c1, Modifier::BOLD));
        s.insert(SemanticToken::Operator, style_fg(0x81a1c1));
        s.insert(SemanticToken::Punctuation, style_fg(0xeceff4));
        s.insert(SemanticToken::QualifiedName, style_fg(0x88c0d0));
        s.insert(SemanticToken::StringLiteral, style_fg(0xa3be8c));
        s.insert(SemanticToken::NumericLiteral, style_fg(0xb48ead));
        s.insert(SemanticToken::Comment, style_fg(0x616e88));
        s.insert(SemanticToken::Pragma, style(0x616e88, Modifier::BOLD));
        s.insert(SemanticToken::ModuleName, style_fg(0x88c0d0));
        s.insert(
            SemanticToken::PackageName,
            style(0x616e88, Modifier::ITALIC),
        );
        s.insert(SemanticToken::DocText, style_fg(0xd8dee9));
        s.insert(SemanticToken::DocCode, style_fg(0xa3be8c));
        s.insert(SemanticToken::DocHeading, style(0x88c0d0, Modifier::BOLD));
        s.insert(
            SemanticToken::DocLink,
            style(0x5e81ac, Modifier::UNDERLINED),
        );
        s.insert(SemanticToken::SearchInput, style_fg(0xeceff4));
        s.insert(
            SemanticToken::StatusBar,
            Style::default().fg(hex(0xd8dee9)).bg(hex(0x3b4252)),
        );
        s.insert(SemanticToken::Selected, Style::default().bg(hex(0x3b4252)));
        s.insert(SemanticToken::Cursor, Style::default().bg(hex(0x434c5e)));
        s.insert(SemanticToken::Border, style_fg(0x616e88));
        s.insert(SemanticToken::Error, style_fg(0xbf616a));
        s.insert(SemanticToken::Spinner, style_fg(0xb48ead));
        Self {
            name: "nord".into(),
            styles: s,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dracula()
    }
}

// --- Helper functions ---

fn hex(rgb: u32) -> Color {
    Color::Rgb(
        ((rgb >> 16) & 0xFF) as u8,
        ((rgb >> 8) & 0xFF) as u8,
        (rgb & 0xFF) as u8,
    )
}

fn style_fg(rgb: u32) -> Style {
    Style::default().fg(hex(rgb))
}

fn style(rgb: u32, modifier: Modifier) -> Style {
    Style::default().fg(hex(rgb)).add_modifier(modifier)
}

// --- TOML theme loading ---

#[derive(Debug, Deserialize)]
struct TomlTheme {
    name: String,
    #[serde(default)]
    styles: HashMap<SemanticToken, TomlStyle>,
}

#[derive(Debug, Deserialize)]
struct TomlStyle {
    #[serde(default)]
    fg: Option<String>,
    #[serde(default)]
    bg: Option<String>,
    #[serde(default)]
    modifiers: Vec<String>,
}

impl TomlTheme {
    fn into_theme(self) -> Theme {
        let mut styles = HashMap::new();
        for (token, ts) in self.styles {
            let mut s = Style::default();
            if let Some(ref fg) = ts.fg {
                if let Some(c) = parse_hex_color(fg) {
                    s = s.fg(c);
                }
            }
            if let Some(ref bg) = ts.bg {
                if let Some(c) = parse_hex_color(bg) {
                    s = s.bg(c);
                }
            }
            for m in &ts.modifiers {
                match m.as_str() {
                    "bold" => s = s.add_modifier(Modifier::BOLD),
                    "italic" => s = s.add_modifier(Modifier::ITALIC),
                    "underlined" | "underline" => s = s.add_modifier(Modifier::UNDERLINED),
                    "dim" => s = s.add_modifier(Modifier::DIM),
                    "reversed" | "reverse" => s = s.add_modifier(Modifier::REVERSED),
                    _ => {}
                }
            }
            styles.insert(token, s);
        }
        Theme {
            name: self.name,
            styles,
        }
    }
}

fn parse_hex_color(s: &str) -> Option<Color> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtin_themes_cover_all_tokens() {
        let themes = [
            Theme::dracula(),
            Theme::catppuccin_mocha(),
            Theme::gruvbox_dark(),
            Theme::solarized_dark(),
            Theme::monokai(),
            Theme::nord(),
        ];

        for theme in &themes {
            for token in SemanticToken::ALL {
                assert!(
                    theme.styles.contains_key(token),
                    "theme '{}' missing style for {:?}",
                    theme.name,
                    token
                );
            }
        }
    }

    #[test]
    fn by_name_returns_correct_theme() {
        assert_eq!(Theme::by_name("dracula").name, "dracula");
        assert_eq!(Theme::by_name("nord").name, "nord");
        assert_eq!(Theme::by_name("catppuccin_mocha").name, "catppuccin_mocha");
        // Unknown falls back to dracula
        assert_eq!(Theme::by_name("unknown").name, "dracula");
    }

    #[test]
    fn parse_hex_color_valid() {
        assert_eq!(parse_hex_color("#ff0000"), Some(Color::Rgb(255, 0, 0)));
        assert_eq!(parse_hex_color("00ff00"), Some(Color::Rgb(0, 255, 0)));
    }

    #[test]
    fn parse_hex_color_invalid() {
        assert_eq!(parse_hex_color("xyz"), None);
        assert_eq!(parse_hex_color(""), None);
    }

    #[test]
    fn toml_theme_roundtrip() {
        let toml_str = r##"
            name = "test_theme"

            [styles.keyword]
            fg = "#ff79c6"
            modifiers = ["bold"]

            [styles.type_constructor]
            fg = "#bd93f9"
            bg = "#282a36"
            modifiers = ["bold", "italic"]
        "##;

        let raw: TomlTheme = toml::from_str(toml_str).unwrap();
        let theme = raw.into_theme();
        assert_eq!(theme.name, "test_theme");
        assert!(theme.styles.contains_key(&SemanticToken::Keyword));
        assert!(theme.styles.contains_key(&SemanticToken::TypeConstructor));
    }

    #[test]
    fn default_theme_is_dracula() {
        let theme = Theme::default();
        assert_eq!(theme.name, "dracula");
    }

    #[test]
    fn load_all_theme_toml_files() {
        // Find the themes directory relative to the workspace root
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let themes_dir = std::path::Path::new(manifest_dir)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("themes");

        let expected = [
            "dracula",
            "catppuccin_mocha",
            "gruvbox_dark",
            "solarized_dark",
            "monokai",
            "nord",
        ];

        for name in expected {
            let path = themes_dir.join(format!("{name}.toml"));
            assert!(path.exists(), "theme file missing: {}", path.display());
            let theme = Theme::from_toml(&path)
                .unwrap_or_else(|e| panic!("failed to load theme {name}: {e}"));
            assert_eq!(theme.name, name);
            // Verify it has all tokens
            for token in SemanticToken::ALL {
                assert!(
                    theme.styles.contains_key(token),
                    "theme file '{name}' missing style for {token:?}"
                );
            }
        }
    }
}
