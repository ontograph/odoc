//! Tokenizer shared by indexing and queries (tokenizer version 1).
//!
//! Words are maximal runs of Unicode alphanumerics and `_`. A word that splits
//! into several parts at `_` or camelCase boundaries emits the whole word,
//! lowercased, followed by its parts; otherwise it emits one lowercased token.
//! No stemming and no stop words.

/// Tokens of `text` in order, duplicates kept.
pub fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for word in text.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
        let parts = split_parts(word);
        if parts.len() > 1 {
            out.push(word.to_lowercase());
        }
        out.extend(parts);
    }
    out
}

/// Unique query terms in first-occurrence order.
pub fn query_terms(query: &str) -> Vec<String> {
    let mut terms: Vec<String> = Vec::new();
    for token in tokenize(query) {
        if !terms.contains(&token) {
            terms.push(token);
        }
    }
    terms
}

/// Lowercased parts split at `_`, lower/digit→Upper, and Upper→Upper+lower.
fn split_parts(word: &str) -> Vec<String> {
    let chars: Vec<char> = word.chars().collect();
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut flush = |current: &mut String| {
        if !current.is_empty() {
            parts.push(current.to_lowercase());
            current.clear();
        }
    };
    for (i, &c) in chars.iter().enumerate() {
        if c == '_' {
            flush(&mut current);
            continue;
        }
        if i > 0 && c.is_uppercase() {
            let prev = chars[i - 1];
            let next_lower = chars.get(i + 1).is_some_and(|n| n.is_lowercase());
            if prev.is_lowercase() || prev.is_numeric() || (prev.is_uppercase() && next_lower) {
                flush(&mut current);
            }
        }
        current.push(c);
    }
    flush(&mut current);
    parts
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn splits_camel_and_snake_case_and_lowercases_unicode() {
        assert_eq!(
            tokenize("getUserName snake_case HTTPServer, Straße ÜBER v2Api x"),
            vec![
                "getusername",
                "get",
                "user",
                "name",
                "snake_case",
                "snake",
                "case",
                "httpserver",
                "http",
                "server",
                "straße",
                "über",
                "v2api",
                "v2",
                "api",
                "x",
            ]
        );
        assert_eq!(tokenize("__init__ --- _x"), vec!["init", "x"]);
        assert!(tokenize("  !? ").is_empty());
    }

    #[test]
    fn query_terms_are_unique_in_order() {
        assert_eq!(query_terms("b a B a_c"), vec!["b", "a", "a_c", "c"]);
    }
}
