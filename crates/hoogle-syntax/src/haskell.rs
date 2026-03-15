use crate::tokens::Token;

const HASKELL_KEYWORDS: &[&str] = &[
    "module",
    "import",
    "qualified",
    "as",
    "hiding",
    "do",
    "let",
    "in",
    "case",
    "of",
    "if",
    "then",
    "else",
    "where",
    "deriving",
    "instance",
    "class",
    "data",
    "type",
    "newtype",
    "forall",
    "family",
    "infixl",
    "infixr",
    "infix",
    "foreign",
    "default",
    "pattern",
    "stock",
    "anyclass",
    "via",
    "role",
    "nominal",
    "representational",
    "phantom",
];

/// Tokenize a full Haskell code block, returning one token list per line.
pub fn tokenize_haskell(input: &str) -> Vec<Vec<Token>> {
    input.lines().map(tokenize_haskell_line).collect()
}

fn tokenize_haskell_line(line: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];

        // Line comment
        if c == '-' && i + 1 < len && chars[i + 1] == '-' {
            let rest: String = chars[i..].iter().collect();
            // Make sure it's not an operator like --->
            if i + 2 >= len || !is_symbol_char(chars[i + 2]) || chars[i + 2] == '-' {
                tokens.push(Token::Comment(rest));
                return tokens;
            }
        }

        // Pragma
        if c == '{' && i + 2 < len && chars[i + 1] == '-' && chars[i + 2] == '#' {
            let start = i;
            while i < len {
                if i + 2 < len && chars[i] == '#' && chars[i + 1] == '-' && chars[i + 2] == '}' {
                    i += 3;
                    break;
                }
                i += 1;
            }
            tokens.push(Token::Pragma(chars[start..i].iter().collect()));
            continue;
        }

        // Block comment start (on single line, consume what we can)
        if c == '{' && i + 1 < len && chars[i + 1] == '-' {
            let start = i;
            i += 2;
            let mut depth = 1;
            while i < len && depth > 0 {
                if i + 1 < len && chars[i] == '{' && chars[i + 1] == '-' {
                    depth += 1;
                    i += 2;
                } else if i + 1 < len && chars[i] == '-' && chars[i + 1] == '}' {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            tokens.push(Token::Comment(chars[start..i].iter().collect()));
            continue;
        }

        // Whitespace
        if c.is_ascii_whitespace() {
            let start = i;
            while i < len && chars[i].is_ascii_whitespace() {
                i += 1;
            }
            tokens.push(Token::Whitespace(i - start));
            continue;
        }

        // String literal
        if c == '"' {
            let s = consume_string(&chars, &mut i);
            tokens.push(Token::StringLiteral(s));
            continue;
        }

        // Char literal
        if c == '\'' && i + 1 < len && chars[i + 1] != '\'' {
            // Could be a char literal or a promoted type
            if i + 2 < len && chars[i + 1] != ' ' {
                // Try char literal: 'x', '\n', etc.
                let saved = i;
                i += 1;
                if chars[i] == '\\' {
                    i += 1; // skip escape
                    if i < len {
                        i += 1;
                    }
                } else {
                    i += 1;
                }
                if i < len && chars[i] == '\'' {
                    i += 1;
                    tokens.push(Token::StringLiteral(chars[saved..i].iter().collect()));
                    continue;
                }
                // Not a char literal, might be promoted constructor
                i = saved;
                if i + 1 < len && chars[i + 1].is_ascii_uppercase() {
                    i += 1;
                    let ident = consume_ident(&chars, &mut i);
                    tokens.push(Token::TypeConstructor(format!("'{ident}")));
                    continue;
                }
                // Just a tick
                tokens.push(Token::Unknown("'".into()));
                i += 1;
                continue;
            }
        }

        // Numeric literal
        if c.is_ascii_digit() {
            let num = consume_number(&chars, &mut i);
            tokens.push(Token::NumericLiteral(num));
            continue;
        }

        // Punctuation
        if matches!(c, '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';') {
            tokens.push(Token::Punctuation(c));
            i += 1;
            continue;
        }

        // Operator / symbol
        if is_symbol_char(c) {
            let start = i;
            while i < len && is_symbol_char(chars[i]) {
                i += 1;
            }
            tokens.push(Token::Operator(chars[start..i].iter().collect()));
            continue;
        }

        // Backtick operator
        if c == '`' {
            let start = i;
            i += 1;
            while i < len && chars[i] != '`' {
                i += 1;
            }
            if i < len {
                i += 1;
            }
            tokens.push(Token::Operator(chars[start..i].iter().collect()));
            continue;
        }

        // Identifier (possibly qualified)
        if c.is_alphabetic() || c == '_' {
            let ident = consume_qualified_ident(&chars, &mut i);
            if ident.contains('.') {
                tokens.push(Token::QualifiedName(ident));
            } else if HASKELL_KEYWORDS.contains(&ident.as_str()) {
                tokens.push(Token::Keyword(ident));
            } else if ident.starts_with(|c: char| c.is_ascii_uppercase()) {
                tokens.push(Token::TypeConstructor(ident));
            } else {
                tokens.push(Token::TypeVariable(ident));
            }
            continue;
        }

        // Hash
        if c == '#' {
            tokens.push(Token::Operator("#".into()));
            i += 1;
            continue;
        }

        tokens.push(Token::Unknown(c.to_string()));
        i += 1;
    }

    tokens
}

fn is_symbol_char(c: char) -> bool {
    matches!(
        c,
        '!' | '#'
            | '$'
            | '%'
            | '&'
            | '*'
            | '+'
            | '.'
            | '/'
            | '<'
            | '='
            | '>'
            | '?'
            | '@'
            | '\\'
            | '^'
            | '|'
            | '-'
            | '~'
            | ':'
    )
}

fn consume_ident(chars: &[char], i: &mut usize) -> String {
    let start = *i;
    let len = chars.len();
    while *i < len && (chars[*i].is_alphanumeric() || chars[*i] == '_' || chars[*i] == '\'') {
        *i += 1;
    }
    chars[start..*i].iter().collect()
}

fn consume_qualified_ident(chars: &[char], i: &mut usize) -> String {
    let start = *i;
    let len = chars.len();

    while *i < len && (chars[*i].is_alphanumeric() || chars[*i] == '_' || chars[*i] == '\'') {
        *i += 1;
    }

    while *i < len && chars[*i] == '.' {
        if *i + 1 < len && chars[*i + 1].is_alphabetic() {
            *i += 1;
            while *i < len && (chars[*i].is_alphanumeric() || chars[*i] == '_' || chars[*i] == '\'')
            {
                *i += 1;
            }
        } else {
            break;
        }
    }

    chars[start..*i].iter().collect()
}

fn consume_string(chars: &[char], i: &mut usize) -> String {
    let start = *i;
    let len = chars.len();
    *i += 1;
    while *i < len && chars[*i] != '"' {
        if chars[*i] == '\\' && *i + 1 < len {
            *i += 1;
        }
        *i += 1;
    }
    if *i < len {
        *i += 1;
    }
    chars[start..*i].iter().collect()
}

fn consume_number(chars: &[char], i: &mut usize) -> String {
    let start = *i;
    let len = chars.len();

    // Hex
    if *i + 1 < len && chars[*i] == '0' && (chars[*i + 1] == 'x' || chars[*i + 1] == 'X') {
        *i += 2;
        while *i < len && chars[*i].is_ascii_hexdigit() {
            *i += 1;
        }
        return chars[start..*i].iter().collect();
    }

    // Octal
    if *i + 1 < len && chars[*i] == '0' && (chars[*i + 1] == 'o' || chars[*i + 1] == 'O') {
        *i += 2;
        while *i < len && ('0'..='7').contains(&chars[*i]) {
            *i += 1;
        }
        return chars[start..*i].iter().collect();
    }

    // Decimal / float
    while *i < len && chars[*i].is_ascii_digit() {
        *i += 1;
    }
    if *i < len && chars[*i] == '.' && *i + 1 < len && chars[*i + 1].is_ascii_digit() {
        *i += 1;
        while *i < len && chars[*i].is_ascii_digit() {
            *i += 1;
        }
    }
    // Exponent
    if *i < len && (chars[*i] == 'e' || chars[*i] == 'E') {
        *i += 1;
        if *i < len && (chars[*i] == '+' || chars[*i] == '-') {
            *i += 1;
        }
        while *i < len && chars[*i].is_ascii_digit() {
            *i += 1;
        }
    }

    chars[start..*i].iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_comment() {
        let lines = tokenize_haskell("x = 1 -- comment");
        assert!(lines[0]
            .iter()
            .any(|t| matches!(t, Token::Comment(s) if s.contains("comment"))));
    }

    #[test]
    fn pragma() {
        let lines = tokenize_haskell("{-# LANGUAGE GADTs #-}");
        assert!(lines[0]
            .iter()
            .any(|t| matches!(t, Token::Pragma(s) if s.contains("LANGUAGE"))));
    }

    #[test]
    fn string_literal() {
        let lines = tokenize_haskell(r#"x = "hello world""#);
        assert!(lines[0]
            .iter()
            .any(|t| matches!(t, Token::StringLiteral(s) if s.contains("hello"))));
    }

    #[test]
    fn numeric_literals() {
        let lines = tokenize_haskell("x = 42 + 0xFF + 3.14");
        let nums: Vec<_> = lines[0]
            .iter()
            .filter(|t| matches!(t, Token::NumericLiteral(_)))
            .collect();
        assert_eq!(nums.len(), 3);
    }

    #[test]
    fn keywords_recognized() {
        let lines = tokenize_haskell("module Foo where");
        assert!(lines[0].contains(&Token::Keyword("module".into())));
        assert!(lines[0].contains(&Token::Keyword("where".into())));
        assert!(lines[0].contains(&Token::TypeConstructor("Foo".into())));
    }

    #[test]
    fn import_line() {
        let lines = tokenize_haskell("import qualified Data.Map.Strict as Map");
        assert!(lines[0].contains(&Token::Keyword("import".into())));
        assert!(lines[0].contains(&Token::Keyword("qualified".into())));
        assert!(lines[0].contains(&Token::Keyword("as".into())));
        assert!(lines[0]
            .iter()
            .any(|t| matches!(t, Token::QualifiedName(s) if s == "Data.Map.Strict")));
        assert!(lines[0].contains(&Token::TypeConstructor("Map".into())));
    }

    #[test]
    fn operators() {
        let lines = tokenize_haskell("x >>= f <$> g");
        assert!(lines[0]
            .iter()
            .any(|t| matches!(t, Token::Operator(s) if s == ">>=")));
        assert!(lines[0]
            .iter()
            .any(|t| matches!(t, Token::Operator(s) if s == "<$>")));
    }

    #[test]
    fn multiline() {
        let lines = tokenize_haskell("foo :: Int\nfoo = 42");
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains(&Token::TypeConstructor("Int".into())));
        assert!(lines[1]
            .iter()
            .any(|t| matches!(t, Token::NumericLiteral(s) if s == "42")));
    }

    #[test]
    fn block_comment() {
        let lines = tokenize_haskell("{- block -} x");
        assert!(lines[0]
            .iter()
            .any(|t| matches!(t, Token::Comment(s) if s.contains("block"))));
        assert!(lines[0].contains(&Token::TypeVariable("x".into())));
    }

    #[test]
    fn empty_input() {
        let lines = tokenize_haskell("");
        // "".lines() yields 0 items
        assert!(lines.is_empty());
    }
}
