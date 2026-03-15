use crate::tokens::Token;

const KEYWORDS: &[&str] = &[
    "forall", "where", "type", "data", "class", "newtype", "family", "instance", "deriving",
    "infixl", "infixr", "infix",
];

/// Tokenize a Haskell type signature into a sequence of tokens.
pub fn tokenize_signature(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];

        // Whitespace
        if c.is_ascii_whitespace() {
            let start = i;
            while i < len && chars[i].is_ascii_whitespace() {
                i += 1;
            }
            tokens.push(Token::Whitespace(i - start));
            continue;
        }

        // String literal (type-level)
        if c == '"' {
            let s = consume_string_literal(&chars, &mut i);
            tokens.push(Token::StringLiteral(s));
            continue;
        }

        // Promoted constructor: 'True, 'Just (but not ' for char literals in sigs)
        if c == '\'' && i + 1 < len && chars[i + 1].is_ascii_uppercase() {
            i += 1; // skip the tick
            let ident = consume_ident(&chars, &mut i);
            tokens.push(Token::TypeConstructor(format!("'{ident}")));
            continue;
        }

        // Punctuation
        if matches!(c, '(' | ')' | '[' | ']' | '{' | '}' | ',') {
            // Check for unboxed tuple syntax (# inside parens)
            tokens.push(Token::Punctuation(c));
            i += 1;
            continue;
        }

        // Hash — could be unboxed tuple or kind
        if c == '#' {
            tokens.push(Token::Operator("#".into()));
            i += 1;
            continue;
        }

        // Numeric literal (type-level nats)
        if c.is_ascii_digit() {
            let start = i;
            while i < len && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            tokens.push(Token::NumericLiteral(chars[start..i].iter().collect()));
            continue;
        }

        // Operator sequences
        if is_operator_char(c) {
            let op = consume_operator(&chars, &mut i);
            tokens.push(Token::Operator(op));
            continue;
        }

        // Identifiers (possibly qualified)
        if c.is_alphabetic() || c == '_' {
            let ident = consume_qualified_or_ident(&chars, &mut i);

            // Check if it's qualified (contains dots between uppercase segments)
            if ident.contains('.') {
                tokens.push(Token::QualifiedName(ident));
            } else if KEYWORDS.contains(&ident.as_str()) {
                tokens.push(Token::Keyword(ident));
            } else if ident.starts_with(|c: char| c.is_ascii_uppercase()) {
                tokens.push(Token::TypeConstructor(ident));
            } else {
                tokens.push(Token::TypeVariable(ident));
            }
            continue;
        }

        // Backtick operator
        if c == '`' {
            i += 1;
            let start = i;
            while i < len && chars[i] != '`' {
                i += 1;
            }
            let inner: String = chars[start..i].iter().collect();
            if i < len {
                i += 1; // consume closing backtick
            }
            tokens.push(Token::Operator(format!("`{inner}`")));
            continue;
        }

        // Unknown
        tokens.push(Token::Unknown(c.to_string()));
        i += 1;
    }

    tokens
}

fn is_operator_char(c: char) -> bool {
    matches!(
        c,
        '-' | '>'
            | '='
            | ':'
            | '.'
            | '~'
            | '@'
            | '!'
            | '*'
            | '+'
            | '/'
            | '\\'
            | '|'
            | '<'
            | '&'
            | '^'
            | '%'
    )
}

fn consume_operator(chars: &[char], i: &mut usize) -> String {
    let start = *i;
    let len = chars.len();

    // Special two-char operators
    if *i + 1 < len {
        let two: String = chars[*i..*i + 2].iter().collect();
        match two.as_str() {
            "->" | "=>" | "::" | ".." => {
                *i += 2;
                return two;
            }
            _ => {}
        }
    }

    // Percent for linear types: %1, %m
    if chars[*i] == '%' {
        *i += 1;
        // Consume the multiplicity annotation
        while *i < len && (chars[*i].is_alphanumeric() || chars[*i] == '_') {
            *i += 1;
        }
        // Then consume optional whitespace + ->
        let saved = *i;
        while *i < len && chars[*i].is_ascii_whitespace() {
            *i += 1;
        }
        if *i + 1 < len && chars[*i] == '-' && chars[*i + 1] == '>' {
            *i += 2;
        } else {
            *i = saved; // revert if no ->
        }
        return chars[start..*i].iter().collect();
    }

    // General operator: consume consecutive operator chars
    while *i < len && is_operator_char(chars[*i]) {
        *i += 1;
    }

    chars[start..*i].iter().collect()
}

fn consume_ident(chars: &[char], i: &mut usize) -> String {
    let start = *i;
    let len = chars.len();
    while *i < len && (chars[*i].is_alphanumeric() || chars[*i] == '_' || chars[*i] == '\'') {
        *i += 1;
    }
    chars[start..*i].iter().collect()
}

/// Consume a potentially qualified identifier like Data.Map.Map
fn consume_qualified_or_ident(chars: &[char], i: &mut usize) -> String {
    let start = *i;
    let len = chars.len();

    // Consume first segment
    while *i < len && (chars[*i].is_alphanumeric() || chars[*i] == '_' || chars[*i] == '\'') {
        *i += 1;
    }

    // Try to extend with dot-separated uppercase segments
    while *i < len && chars[*i] == '.' {
        // Peek ahead: must be followed by an alpha char to be a qualified name
        if *i + 1 < len && chars[*i + 1].is_alphabetic() {
            let saved = *i;
            *i += 1; // consume dot
                     // Consume next segment
            let seg_start = *i;
            while *i < len && (chars[*i].is_alphanumeric() || chars[*i] == '_' || chars[*i] == '\'')
            {
                *i += 1;
            }
            if *i == seg_start {
                // No segment after dot — revert
                *i = saved;
                break;
            }
            // If the segment after the dot started lowercase and we already
            // have qualified parts, this is the final segment (e.g. Data.Map.lookup)
            // Keep it as part of the qualified name
        } else {
            break;
        }
    }

    chars[start..*i].iter().collect()
}

fn consume_string_literal(chars: &[char], i: &mut usize) -> String {
    let start = *i;
    let len = chars.len();
    *i += 1; // skip opening quote
    while *i < len && chars[*i] != '"' {
        if chars[*i] == '\\' && *i + 1 < len {
            *i += 1; // skip escaped char
        }
        *i += 1;
    }
    if *i < len {
        *i += 1; // skip closing quote
    }
    chars[start..*i].iter().collect()
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
