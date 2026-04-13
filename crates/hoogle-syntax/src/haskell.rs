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
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];

        // Line comment
        if b == b'-' && i + 1 < len && bytes[i + 1] == b'-' {
            // Make sure it's not an operator like --->
            if i + 2 >= len || !is_symbol_byte(bytes[i + 2]) || bytes[i + 2] == b'-' {
                tokens.push(Token::Comment(line[i..].to_string()));
                return tokens;
            }
        }

        // Pragma
        if b == b'{' && i + 2 < len && bytes[i + 1] == b'-' && bytes[i + 2] == b'#' {
            let start = i;
            while i < len {
                if i + 2 < len && bytes[i] == b'#' && bytes[i + 1] == b'-' && bytes[i + 2] == b'}' {
                    i += 3;
                    break;
                }
                i += 1;
            }
            tokens.push(Token::Pragma(line[start..i].to_string()));
            continue;
        }

        // Block comment start (on single line, consume what we can)
        if b == b'{' && i + 1 < len && bytes[i + 1] == b'-' {
            let start = i;
            i += 2;
            let mut depth = 1;
            while i < len && depth > 0 {
                if i + 1 < len && bytes[i] == b'{' && bytes[i + 1] == b'-' {
                    depth += 1;
                    i += 2;
                } else if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'}' {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            tokens.push(Token::Comment(line[start..i].to_string()));
            continue;
        }

        // Whitespace
        if b.is_ascii_whitespace() {
            let start = i;
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            tokens.push(Token::Whitespace(i - start));
            continue;
        }

        // String literal
        if b == b'"' {
            let s = consume_string(line, &mut i);
            tokens.push(Token::StringLiteral(s));
            continue;
        }

        // Char literal
        if b == b'\'' && i + 1 < len && bytes[i + 1] != b'\'' && i + 2 < len && bytes[i + 1] != b' '
        {
            let saved = i;
            i += 1;
            if bytes[i] == b'\\' {
                i += 1;
                if i < len {
                    i += 1;
                }
            } else {
                i += 1;
            }
            if i < len && bytes[i] == b'\'' {
                i += 1;
                tokens.push(Token::StringLiteral(line[saved..i].to_string()));
                continue;
            }
            // Not a char literal, might be promoted constructor
            i = saved;
            if i + 1 < len && bytes[i + 1].is_ascii_uppercase() {
                i += 1;
                let ident = consume_ident(line, &mut i);
                tokens.push(Token::TypeConstructor(format!("'{ident}")));
                continue;
            }
            // Just a tick
            tokens.push(Token::Unknown("'".into()));
            i += 1;
            continue;
        }

        // Numeric literal
        if b.is_ascii_digit() {
            let num = consume_number(line, &mut i);
            tokens.push(Token::NumericLiteral(num));
            continue;
        }

        // Punctuation
        if matches!(b, b'(' | b')' | b'[' | b']' | b'{' | b'}' | b',' | b';') {
            tokens.push(Token::Punctuation(b as char));
            i += 1;
            continue;
        }

        // Operator / symbol
        if is_symbol_byte(b) {
            let start = i;
            while i < len && is_symbol_byte(bytes[i]) {
                i += 1;
            }
            tokens.push(Token::Operator(line[start..i].to_string()));
            continue;
        }

        // Backtick operator
        if b == b'`' {
            let start = i;
            i += 1;
            while i < len && bytes[i] != b'`' {
                i += 1;
            }
            if i < len {
                i += 1;
            }
            tokens.push(Token::Operator(line[start..i].to_string()));
            continue;
        }

        // Identifier (possibly qualified)
        if b.is_ascii_alphabetic() || b == b'_' {
            let ident = consume_qualified_ident(line, &mut i);
            if ident.contains('.') {
                tokens.push(Token::QualifiedName(ident));
            } else if HASKELL_KEYWORDS.contains(&ident.as_str()) {
                tokens.push(Token::Keyword(ident));
            } else if ident.as_bytes()[0].is_ascii_uppercase() {
                tokens.push(Token::TypeConstructor(ident));
            } else {
                tokens.push(Token::TypeVariable(ident));
            }
            continue;
        }

        // Hash
        if b == b'#' {
            tokens.push(Token::Operator("#".into()));
            i += 1;
            continue;
        }

        // Non-ASCII or unknown
        let c = line[i..].chars().next().unwrap();
        tokens.push(Token::Unknown(c.to_string()));
        i += c.len_utf8();
    }

    tokens
}

fn is_symbol_byte(b: u8) -> bool {
    matches!(
        b,
        b'!' | b'#'
            | b'$'
            | b'%'
            | b'&'
            | b'*'
            | b'+'
            | b'.'
            | b'/'
            | b'<'
            | b'='
            | b'>'
            | b'?'
            | b'@'
            | b'\\'
            | b'^'
            | b'|'
            | b'-'
            | b'~'
            | b':'
    )
}

fn consume_ident(input: &str, i: &mut usize) -> String {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let start = *i;
    while *i < len && (bytes[*i].is_ascii_alphanumeric() || bytes[*i] == b'_' || bytes[*i] == b'\'')
    {
        *i += 1;
    }
    input[start..*i].to_string()
}

fn consume_qualified_ident(input: &str, i: &mut usize) -> String {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let start = *i;

    while *i < len && (bytes[*i].is_ascii_alphanumeric() || bytes[*i] == b'_' || bytes[*i] == b'\'')
    {
        *i += 1;
    }

    while *i < len && bytes[*i] == b'.' {
        if *i + 1 < len && bytes[*i + 1].is_ascii_alphabetic() {
            *i += 1;
            while *i < len
                && (bytes[*i].is_ascii_alphanumeric() || bytes[*i] == b'_' || bytes[*i] == b'\'')
            {
                *i += 1;
            }
        } else {
            break;
        }
    }

    input[start..*i].to_string()
}

fn consume_string(input: &str, i: &mut usize) -> String {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let start = *i;
    *i += 1;
    while *i < len && bytes[*i] != b'"' {
        if bytes[*i] == b'\\' && *i + 1 < len {
            *i += 1;
        }
        *i += 1;
    }
    if *i < len {
        *i += 1;
    }
    input[start..*i].to_string()
}

fn consume_number(input: &str, i: &mut usize) -> String {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let start = *i;

    // Hex
    if *i + 1 < len && bytes[*i] == b'0' && (bytes[*i + 1] == b'x' || bytes[*i + 1] == b'X') {
        *i += 2;
        while *i < len && bytes[*i].is_ascii_hexdigit() {
            *i += 1;
        }
        return input[start..*i].to_string();
    }

    // Octal
    if *i + 1 < len && bytes[*i] == b'0' && (bytes[*i + 1] == b'o' || bytes[*i + 1] == b'O') {
        *i += 2;
        while *i < len && (b'0'..=b'7').contains(&bytes[*i]) {
            *i += 1;
        }
        return input[start..*i].to_string();
    }

    // Decimal / float
    while *i < len && bytes[*i].is_ascii_digit() {
        *i += 1;
    }
    if *i < len && bytes[*i] == b'.' && *i + 1 < len && bytes[*i + 1].is_ascii_digit() {
        *i += 1;
        while *i < len && bytes[*i].is_ascii_digit() {
            *i += 1;
        }
    }
    // Exponent
    if *i < len && (bytes[*i] == b'e' || bytes[*i] == b'E') {
        *i += 1;
        if *i < len && (bytes[*i] == b'+' || bytes[*i] == b'-') {
            *i += 1;
        }
        while *i < len && bytes[*i].is_ascii_digit() {
            *i += 1;
        }
    }

    input[start..*i].to_string()
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
        assert!(lines.is_empty());
    }
}
