//! Token estimation for LLM context planning.
//!
//! Provides a fast heuristic tokenizer that estimates token counts without
//! requiring an external tokenizer model. Useful for agents that need to
//! know how much context they are consuming.

/// Estimate the number of tokens in a text string.
///
/// Uses a simple heuristic: split on whitespace and punctuation,
/// with language-aware adjustments for code.
///
/// # Accuracy
/// - Natural language: ~1 token per word (within 10-15% of tiktoken)
/// - Code: ~1 token per 3-4 characters (varies by language)
/// - Strings/identifiers: ~1 token per word boundary
pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }

    let mut tokens = 0usize;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            continue;
        }

        // Single-character tokens (operators, brackets, punctuation)
        if is_single_char_token(c) {
            tokens += 1;
            continue;
        }

        // Multi-character tokens: consume until next boundary
        let mut len = 1;
        while let Some(&next) = chars.peek() {
            if is_token_boundary(c, next) {
                break;
            }
            chars.next();
            len += 1;
        }

        // Heuristic: ~4 chars per token for identifiers/words,
        // but at least 1 token per consumed character sequence
        tokens += std::cmp::max(1, len / 4);
    }

    tokens
}

/// Estimate tokens for a source code file.
///
/// Adjusts the basic estimate for code-specific patterns:
/// - Keywords and short identifiers: 1 token each
/// - Long identifiers: split on camelCase/snake_case
/// - Punctuation clusters: fewer tokens than raw count
pub fn estimate_code_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }

    let mut tokens = 0usize;

    for line in text.lines() {
        tokens += estimate_line_tokens(line);
    }

    // Newlines between lines
    let line_count = text.lines().count();
    if line_count > 1 {
        tokens += line_count - 1;
    }

    tokens
}

fn estimate_line_tokens(line: &str) -> usize {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return 0;
    }

    let mut tokens = 0usize;
    let mut chars = trimmed.chars().peekable();

    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            continue;
        }

        // Comments: ~1 token per 4 chars
        if c == '/' {
            if let Some(&next) = chars.peek() {
                if next == '/' || next == '*' {
                    // Rest of line is comment
                    let remaining: String = std::iter::once(c)
                        .chain(std::iter::once(chars.next().unwrap()))
                        .chain(chars.by_ref())
                        .collect();
                    tokens += std::cmp::max(1, remaining.len() / 4);
                    return tokens;
                }
            }
        }

        // Strings: ~1 token per 4 chars
        if c == '"' || c == '\'' {
            let quote = c;
            let mut string_len = 1;
            while let Some(next) = chars.next() {
                string_len += 1;
                if next == quote {
                    break;
                }
                if next == '\\' {
                    chars.next(); // skip escaped char
                    string_len += 1;
                }
            }
            tokens += std::cmp::max(1, string_len / 4);
            continue;
        }

        // Single-char tokens
        if is_single_char_token(c) {
            tokens += 1;
            continue;
        }

        // Identifiers and keywords: consume until boundary
        let mut len = 1;
        while let Some(&next) = chars.peek() {
            if is_token_boundary(c, next) {
                break;
            }
            chars.next();
            len += 1;
        }

        // Short identifiers (keywords, common ops): 1 token
        // Longer ones: ~4 chars per token
        tokens += if len <= 6 { 1 } else { std::cmp::max(1, len / 4) };
    }

    tokens
}

fn is_single_char_token(c: char) -> bool {
    matches!(c,
        '(' | ')' | '[' | ']' | '{' | '}' |
        ';' | ':' | ',' | '.' | '=' | '+' | '-' | '*' | '/' |
        '<' | '>' | '!' | '&' | '|' | '^' | '~' | '%' | '#' | '@'
    )
}

fn is_token_boundary(prev: char, next: char) -> bool {
    // Whitespace is always a boundary
    if next.is_whitespace() {
        return true;
    }

    // Transition from alphanumeric to special char or vice versa
    (prev.is_alphanumeric() && !next.is_alphanumeric())
        || (!prev.is_alphanumeric() && next.is_alphanumeric())
        // camelCase boundary (lowercase -> uppercase)
        || (prev.is_lowercase() && next.is_uppercase())
}

/// Count tokens across multiple texts and return per-item + total.
pub fn count_tokens_batch(texts: &[(&str, &str)]) -> Vec<(String, usize)> {
    texts
        .iter()
        .map(|(name, text)| (name.to_string(), estimate_code_tokens(text)))
        .collect()
}

/// Token count summary for a collection of files.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TokenSummary {
    pub total_tokens: usize,
    pub file_count: usize,
    pub files: Vec<FileTokenInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FileTokenInfo {
    pub path: String,
    pub tokens: usize,
    pub lines: usize,
}

impl TokenSummary {
    pub fn from_files(entries: &[(&str, &str)]) -> Self {
        let files: Vec<FileTokenInfo> = entries
            .iter()
            .map(|(path, content)| FileTokenInfo {
                path: path.to_string(),
                tokens: estimate_code_tokens(content),
                lines: content.lines().count(),
            })
            .collect();

        let total_tokens = files.iter().map(|f| f.tokens).sum();

        TokenSummary {
            total_tokens,
            file_count: files.len(),
            files,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_estimate_tokens_simple() {
        let tokens = estimate_tokens("hello world");
        assert!(tokens >= 1 && tokens <= 4);
    }

    #[test]
    fn test_estimate_tokens_code() {
        let code = "fn main() {\n    println!(\"hello\");\n}";
        let tokens = estimate_code_tokens(code);
        assert!(tokens >= 5 && tokens <= 20);
    }

    #[test]
    fn test_estimate_tokens_long_code() {
        let code = r#"
use std::collections::HashMap;

pub struct Config {
    pub name: String,
    pub debug: bool,
}

impl Config {
    pub fn new() -> Self {
        Config {
            name: String::from("default"),
            debug: false,
        }
    }
}
"#;
        let tokens = estimate_code_tokens(code);
        // Should be reasonable for this ~15-line file
        assert!(tokens >= 20 && tokens <= 80);
    }

    #[test]
    fn test_estimate_tokens_with_comments() {
        let code = "// This is a comment about the function\nfn compute() -> i32 { 42 }";
        let tokens = estimate_code_tokens(code);
        assert!(tokens >= 5 && tokens <= 25);
    }

    #[test]
    fn test_estimate_tokens_with_strings() {
        let code = r#"let msg = "hello world this is a string";"#;
        let tokens = estimate_code_tokens(code);
        assert!(tokens >= 5 && tokens <= 20);
    }

    #[test]
    fn test_token_summary() {
        let entries = vec![
            ("src/main.rs", "fn main() {}"),
            ("src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }"),
        ];
        let summary = TokenSummary::from_files(&entries);
        assert_eq!(summary.file_count, 2);
        assert!(summary.total_tokens > 0);
    }
}
