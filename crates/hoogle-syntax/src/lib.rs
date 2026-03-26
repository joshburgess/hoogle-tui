//! Haskell syntax highlighting for type signatures and code blocks.
//!
//! Provides tokenization and theme-aware rendering into styled ratatui `Line` values.

pub mod haskell;
pub mod theme;
pub mod tokenizer;
pub mod tokens;

use ratatui::text::{Line, Span};
use theme::{SemanticToken, Theme};
use tokens::Token;

/// Map a syntax token to its semantic token for theme lookup.
fn token_to_semantic(token: &Token) -> SemanticToken {
    match token {
        Token::Keyword(_) => SemanticToken::Keyword,
        Token::TypeConstructor(_) => SemanticToken::TypeConstructor,
        Token::TypeVariable(_) => SemanticToken::TypeVariable,
        Token::Operator(_) => SemanticToken::Operator,
        Token::QualifiedName(_) => SemanticToken::QualifiedName,
        Token::Punctuation(_) => SemanticToken::Punctuation,
        Token::StringLiteral(_) => SemanticToken::StringLiteral,
        Token::NumericLiteral(_) => SemanticToken::NumericLiteral,
        Token::Whitespace(_) => SemanticToken::Punctuation, // neutral
        Token::Comment(_) => SemanticToken::Comment,
        Token::Pragma(_) => SemanticToken::Pragma,
        Token::Unknown(_) => SemanticToken::Punctuation,
    }
}

/// Extract the text content from a token.
fn token_text(token: &Token) -> std::borrow::Cow<'_, str> {
    match token {
        Token::Keyword(s)
        | Token::TypeConstructor(s)
        | Token::TypeVariable(s)
        | Token::Operator(s)
        | Token::QualifiedName(s)
        | Token::StringLiteral(s)
        | Token::NumericLiteral(s)
        | Token::Comment(s)
        | Token::Pragma(s)
        | Token::Unknown(s) => std::borrow::Cow::Borrowed(s),
        Token::Punctuation(c) => std::borrow::Cow::Owned(c.to_string()),
        Token::Whitespace(n) => std::borrow::Cow::Owned(" ".repeat(*n)),
    }
}

/// Highlight a Haskell type signature, returning a styled ratatui Line.
pub fn highlight_signature(sig: &str, theme: &Theme) -> Line<'static> {
    let tokens = tokenizer::tokenize_signature(sig);
    let spans: Vec<Span<'static>> = tokens
        .iter()
        .map(|tok| {
            let semantic = token_to_semantic(tok);
            let style = theme.style(semantic);
            Span::styled(token_text(tok).into_owned(), style)
        })
        .collect();
    Line::from(spans)
}

/// Highlight a Haskell code block, returning one styled Line per source line.
pub fn highlight_code(code: &str, theme: &Theme) -> Vec<Line<'static>> {
    let line_tokens = haskell::tokenize_haskell(code);
    line_tokens
        .iter()
        .map(|tokens| {
            let spans: Vec<Span<'static>> = tokens
                .iter()
                .map(|tok| {
                    let semantic = token_to_semantic(tok);
                    let style = theme.style(semantic);
                    Span::styled(token_text(tok).into_owned(), style)
                })
                .collect();
            Line::from(spans)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlight_signature_produces_spans() {
        let theme = Theme::dracula();
        let line = highlight_signature("Ord k => k -> Map k a -> Maybe a", &theme);
        assert!(!line.spans.is_empty());
        // Reconstruct the text
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "Ord k => k -> Map k a -> Maybe a");
    }

    #[test]
    fn highlight_signature_empty() {
        let theme = Theme::dracula();
        let line = highlight_signature("", &theme);
        assert!(line.spans.is_empty());
    }

    #[test]
    fn highlight_code_multiline() {
        let theme = Theme::catppuccin_mocha();
        let lines = highlight_code("module Foo where\n\nfoo :: Int\nfoo = 42", &theme);
        assert_eq!(lines.len(), 4);

        // First line should contain "module" and "Foo"
        let first_text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(first_text.contains("module"));
        assert!(first_text.contains("Foo"));
    }

    #[test]
    fn highlight_code_empty() {
        let theme = Theme::nord();
        let lines = highlight_code("", &theme);
        assert!(lines.is_empty());
    }

    #[test]
    fn all_themes_produce_styled_output() {
        let sig = "forall a. Show a => a -> String";
        let themes = [
            Theme::dracula(),
            Theme::catppuccin_mocha(),
            Theme::gruvbox_dark(),
            Theme::solarized_dark(),
            Theme::monokai(),
            Theme::nord(),
        ];
        for theme in &themes {
            let line = highlight_signature(sig, theme);
            assert!(
                !line.spans.is_empty(),
                "theme {} produced no spans",
                theme.name
            );
        }
    }
}
