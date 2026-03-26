use crate::tokens::Token;

const KEYWORDS: &[&str] = &[
    "forall", "where", "type", "data", "class", "newtype", "family", "instance", "deriving",
    "infixl", "infixr", "infix",
];

/// Tokenize a Haskell type signature into a sequence of tokens.
/// Works directly on the UTF-8 string using byte indexing for ASCII-dominated
/// Haskell syntax, with char-boundary awareness for correctness.
pub fn tokenize_signature(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];

        // Whitespace (ASCII-only for Haskell)
        if b.is_ascii_whitespace() {
            let start = i;
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            tokens.push(Token::Whitespace(i - start));
            continue;
        }

        // String literal (type-level)
        if b == b'"' {
            let s = consume_string_literal(input, &mut i);
            tokens.push(Token::StringLiteral(s));
            continue;
        }

        // Promoted constructor: 'True, 'Just (but not ' for char literals in sigs)
        if b == b'\'' && i + 1 < len && bytes[i + 1].is_ascii_uppercase() {
            i += 1; // skip the tick
            let ident = consume_ident(input, &mut i);
            tokens.push(Token::TypeConstructor(format!("'{ident}")));
            continue;
        }

        // Punctuation
        if matches!(b, b'(' | b')' | b'[' | b']' | b'{' | b'}' | b',') {
            tokens.push(Token::Punctuation(b as char));
            i += 1;
            continue;
        }

        // Hash — could be unboxed tuple or kind
        if b == b'#' {
            tokens.push(Token::Operator("#".into()));
            i += 1;
            continue;
        }

        // Numeric literal (type-level nats)
        if b.is_ascii_digit() {
            let start = i;
            while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            tokens.push(Token::NumericLiteral(input[start..i].to_string()));
            continue;
        }

        // Operator sequences
        if is_operator_byte(b) {
            let op = consume_operator(input, &mut i);
            tokens.push(Token::Operator(op));
            continue;
        }

        // Identifiers (possibly qualified)
        if b.is_ascii_alphabetic() || b == b'_' {
            let ident = consume_qualified_or_ident(input, &mut i);

            // Check if it's qualified (contains dots between segments)
            if ident.contains('.') {
                tokens.push(Token::QualifiedName(ident));
            } else if KEYWORDS.contains(&ident.as_str()) {
                tokens.push(Token::Keyword(ident));
            } else if ident.as_bytes()[0].is_ascii_uppercase() {
                tokens.push(Token::TypeConstructor(ident));
            } else {
                tokens.push(Token::TypeVariable(ident));
            }
            continue;
        }

        // Backtick operator
        if b == b'`' {
            i += 1;
            let start = i;
            while i < len && bytes[i] != b'`' {
                i += 1;
            }
            let inner = &input[start..i];
            if i < len {
                i += 1; // consume closing backtick
            }
            tokens.push(Token::Operator(format!("`{inner}`")));
            continue;
        }

        // Non-ASCII or truly unknown character: consume one char
        let c = input[i..].chars().next().unwrap();
        tokens.push(Token::Unknown(c.to_string()));
        i += c.len_utf8();
    }

    tokens
}

fn is_operator_byte(b: u8) -> bool {
    matches!(
        b,
        b'-' | b'>'
            | b'='
            | b':'
            | b'.'
            | b'~'
            | b'@'
            | b'!'
            | b'*'
            | b'+'
            | b'/'
            | b'\\'
            | b'|'
            | b'<'
            | b'&'
            | b'^'
            | b'%'
    )
}

fn consume_operator(input: &str, i: &mut usize) -> String {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let start = *i;

    // Special two-char operators
    if *i + 1 < len {
        let two = &input[*i..*i + 2];
        match two {
            "->" | "=>" | "::" | ".." => {
                *i += 2;
                return two.to_string();
            }
            _ => {}
        }
    }

    // Percent for linear types: %1, %m
    if bytes[*i] == b'%' {
        *i += 1;
        // Consume the multiplicity annotation
        while *i < len && (bytes[*i].is_ascii_alphanumeric() || bytes[*i] == b'_') {
            *i += 1;
        }
        // Then consume optional whitespace + ->
        let saved = *i;
        while *i < len && bytes[*i].is_ascii_whitespace() {
            *i += 1;
        }
        if *i + 1 < len && bytes[*i] == b'-' && bytes[*i + 1] == b'>' {
            *i += 2;
        } else {
            *i = saved; // revert if no ->
        }
        return input[start..*i].to_string();
    }

    // General operator: consume consecutive operator chars
    while *i < len && is_operator_byte(bytes[*i]) {
        *i += 1;
    }

    input[start..*i].to_string()
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

/// Consume a potentially qualified identifier like Data.Map.Map
fn consume_qualified_or_ident(input: &str, i: &mut usize) -> String {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let start = *i;

    // Consume first segment
    while *i < len && (bytes[*i].is_ascii_alphanumeric() || bytes[*i] == b'_' || bytes[*i] == b'\'')
    {
        *i += 1;
    }

    // Try to extend with dot-separated segments
    while *i < len && bytes[*i] == b'.' {
        // Peek ahead: must be followed by an alpha char to be a qualified name
        if *i + 1 < len && bytes[*i + 1].is_ascii_alphabetic() {
            let saved = *i;
            *i += 1; // consume dot
            // Consume next segment
            let seg_start = *i;
            while *i < len
                && (bytes[*i].is_ascii_alphanumeric() || bytes[*i] == b'_' || bytes[*i] == b'\'')
            {
                *i += 1;
            }
            if *i == seg_start {
                // No segment after dot — revert
                *i = saved;
                break;
            }
        } else {
            break;
        }
    }

    input[start..*i].to_string()
}

fn consume_string_literal(input: &str, i: &mut usize) -> String {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let start = *i;
    *i += 1; // skip opening quote
    while *i < len && bytes[*i] != b'"' {
        if bytes[*i] == b'\\' && *i + 1 < len {
            *i += 1; // skip escaped char
        }
        *i += 1;
    }
    if *i < len {
        *i += 1; // skip closing quote
    }
    input[start..*i].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token_strings(tokens: &[Token]) -> Vec<String> {
        tokens
            .iter()
            .filter(|t| !matches!(t, Token::Whitespace(_)))
            .map(|t| match t {
                Token::Keyword(s)
                | Token::TypeConstructor(s)
                | Token::TypeVariable(s)
                | Token::Operator(s)
                | Token::QualifiedName(s)
                | Token::StringLiteral(s)
                | Token::NumericLiteral(s)
                | Token::Comment(s)
                | Token::Pragma(s)
                | Token::Unknown(s) => s.clone(),
                Token::Punctuation(c) => c.to_string(),
                Token::Whitespace(n) => " ".repeat(*n),
            })
            .collect()
    }

    #[test]
    fn simple_function() {
        let tokens = tokenize_signature("Int -> Int");
        assert!(tokens.contains(&Token::TypeConstructor("Int".into())));
        assert!(tokens.contains(&Token::Operator("->".into())));
    }

    #[test]
    fn type_variables() {
        let tokens = tokenize_signature("a -> b -> c");
        assert!(tokens.contains(&Token::TypeVariable("a".into())));
        assert!(tokens.contains(&Token::TypeVariable("b".into())));
        assert!(tokens.contains(&Token::TypeVariable("c".into())));
    }

    #[test]
    fn constrained() {
        let tokens = tokenize_signature("Ord k => k -> Map k a -> Maybe a");
        assert!(tokens.contains(&Token::TypeConstructor("Ord".into())));
        assert!(tokens.contains(&Token::Operator("=>".into())));
        assert!(tokens.contains(&Token::TypeConstructor("Map".into())));
        assert!(tokens.contains(&Token::TypeConstructor("Maybe".into())));
        assert!(tokens.contains(&Token::TypeVariable("k".into())));
    }

    #[test]
    fn multi_constraint() {
        let tokens = tokenize_signature("(Monad m, MonadIO m) => m a -> IO a");
        assert!(tokens.contains(&Token::TypeConstructor("Monad".into())));
        assert!(tokens.contains(&Token::TypeConstructor("MonadIO".into())));
        assert!(tokens.contains(&Token::TypeConstructor("IO".into())));
        assert!(tokens.contains(&Token::Punctuation('(')));
        assert!(tokens.contains(&Token::Punctuation(',')));
    }

    #[test]
    fn forall_signature() {
        let tokens = tokenize_signature("forall a b. (a -> b) -> [a] -> [b]");
        assert!(tokens.contains(&Token::Keyword("forall".into())));
        assert!(tokens.contains(&Token::TypeVariable("a".into())));
        assert!(tokens.contains(&Token::Punctuation('[')));
    }

    #[test]
    fn qualified_name() {
        let tokens = tokenize_signature("Data.Map.Strict.Map k v");
        assert!(tokens.contains(&Token::QualifiedName("Data.Map.Strict.Map".into())));
    }

    #[test]
    fn kind_signature() {
        let tokens = tokenize_signature("(Type -> Type) -> Constraint");
        assert!(tokens.contains(&Token::TypeConstructor("Type".into())));
        assert!(tokens.contains(&Token::TypeConstructor("Constraint".into())));
    }

    #[test]
    fn operator_type_in_parens() {
        // The parens are punctuation, the content is parsed normally
        let tokens = tokenize_signature("(++) :: [a] -> [a] -> [a]");
        let strs = token_strings(&tokens);
        assert!(strs.contains(&"++".to_string()));
        assert!(strs.contains(&"::".to_string()));
    }

    #[test]
    fn type_level_string() {
        let tokens = tokenize_signature(r#"Proxy "hello""#);
        assert!(tokens.contains(&Token::TypeConstructor("Proxy".into())));
        assert!(tokens.contains(&Token::StringLiteral("\"hello\"".into())));
    }

    #[test]
    fn type_level_nat() {
        let tokens = tokenize_signature("Vec 3 Int");
        assert!(tokens.contains(&Token::NumericLiteral("3".into())));
    }

    #[test]
    fn promoted_constructor() {
        let tokens = tokenize_signature("Proxy 'True");
        assert!(tokens.contains(&Token::TypeConstructor("'True".into())));
    }

    #[test]
    fn double_colon() {
        let tokens = tokenize_signature("foo :: Int");
        assert!(tokens.contains(&Token::Operator("::".into())));
    }

    #[test]
    fn empty_input() {
        let tokens = tokenize_signature("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn single_identifier() {
        let tokens = tokenize_signature("Int");
        assert_eq!(tokens, vec![Token::TypeConstructor("Int".into())]);
    }

    #[test]
    fn only_punctuation() {
        let tokens = tokenize_signature("()");
        assert_eq!(
            tokens,
            vec![Token::Punctuation('('), Token::Punctuation(')')]
        );
    }

    #[test]
    fn nested_parens() {
        let tokens = tokenize_signature("(a -> (b -> c)) -> d");
        let parens = tokens
            .iter()
            .filter(|t| matches!(t, Token::Punctuation('(' | ')')))
            .count();
        assert_eq!(parens, 4);
    }

    #[test]
    fn arrow_no_spaces() {
        let tokens = tokenize_signature("a->b");
        assert!(tokens.contains(&Token::TypeVariable("a".into())));
        assert!(tokens.contains(&Token::Operator("->".into())));
        assert!(tokens.contains(&Token::TypeVariable("b".into())));
    }

    #[test]
    fn data_keyword() {
        let tokens = tokenize_signature("data Map k a");
        assert!(tokens.contains(&Token::Keyword("data".into())));
        assert!(tokens.contains(&Token::TypeConstructor("Map".into())));
    }

    #[test]
    fn class_keyword() {
        let tokens = tokenize_signature("class Monad m where");
        assert!(tokens.contains(&Token::Keyword("class".into())));
        assert!(tokens.contains(&Token::TypeConstructor("Monad".into())));
        assert!(tokens.contains(&Token::Keyword("where".into())));
    }

    #[test]
    fn linear_types_percent() {
        let tokens = tokenize_signature("%1 -> Int");
        // The %1 -> should be parsed as an operator
        let has_percent = tokens.iter().any(|t| match t {
            Token::Operator(s) => s.contains('%'),
            _ => false,
        });
        assert!(has_percent);
    }

    #[test]
    fn whitespace_preserved() {
        let tokens = tokenize_signature("a  ->  b");
        let ws_count: usize = tokens
            .iter()
            .filter_map(|t| match t {
                Token::Whitespace(n) => Some(*n),
                _ => None,
            })
            .sum();
        assert!(ws_count >= 4);
    }

    #[test]
    fn complex_real_world_sig() {
        let tokens = tokenize_signature(
            "forall k a. (Ord k, Show a) => k -> Map k a -> Either String (Maybe a)",
        );
        assert!(tokens.contains(&Token::Keyword("forall".into())));
        assert!(tokens.contains(&Token::TypeConstructor("Ord".into())));
        assert!(tokens.contains(&Token::TypeConstructor("Show".into())));
        assert!(tokens.contains(&Token::TypeConstructor("Either".into())));
        assert!(tokens.contains(&Token::TypeConstructor("String".into())));
        assert!(tokens.contains(&Token::TypeConstructor("Maybe".into())));
    }
}
