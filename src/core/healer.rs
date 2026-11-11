use crate::parser::SyntaxError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealingAction {
    pub description: String,
    pub fix: String, // Code to append or prepend
    pub line: usize,
}

pub struct Healer;

impl Default for Healer {
    fn default() -> Self {
        Self::new()
    }
}

impl Healer {
    pub fn new() -> Self {
        Self
    }

    /// Analyze a syntax error and suggest a fix if possible
    pub fn suggest_fix(
        &self,
        code: &str,
        error: &SyntaxError,
        extension: &str,
    ) -> Option<HealingAction> {
        match extension {
            "rs" | "c" | "cpp" | "java" | "js" | "ts" | "qml" => {
                self.heal_brace_languages(code, error)
            }
            "py" => self.heal_python(code, error),
            _ => None,
        }
    }

    fn heal_brace_languages(&self, code: &str, _error: &SyntaxError) -> Option<HealingAction> {
        /// Braces as a LEXER sees them: string literals, char literals and
        /// comments open and close nothing.
        ///
        /// The first version counted raw characters, so a file containing
        /// `"{".to_string()` looked unbalanced and the "fix" appended a stray
        /// brace — a wrong answer that made the file worse. Returns
        /// `(opens - closes, line of the first unmatched '}')`.
        fn balance(code: &str) -> (i64, Option<usize>) {
            let mut opens: i64 = 0;
            let mut closes: i64 = 0;
            let mut depth: i64 = 0;
            let mut first_unmatched_close: Option<usize> = None;
            let mut line = 1usize;
            let mut chars = code.chars().peekable();
            let mut in_string: Option<char> = None;
            let mut escape = false;
            let mut in_line_comment = false;
            let mut in_block_comment = false;

            while let Some(c) = chars.next() {
                if c == '\n' {
                    line += 1;
                    in_line_comment = false;
                    escape = false;
                    continue;
                }
                if in_line_comment {
                    continue;
                }
                if in_block_comment {
                    if c == '*' && chars.peek() == Some(&'/') {
                        chars.next();
                        in_block_comment = false;
                    }
                    continue;
                }
                if let Some(quote) = in_string {
                    if escape {
                        escape = false;
                    } else if c == '\\' {
                        escape = true;
                    } else if c == quote {
                        in_string = None;
                    }
                    continue;
                }
                if c == '/' && chars.peek() == Some(&'/') {
                    chars.next();
                    in_line_comment = true;
                    continue;
                }
                if c == '/' && chars.peek() == Some(&'*') {
                    chars.next();
                    in_block_comment = true;
                    continue;
                }
                if c == '"' {
                    in_string = Some('"');
                    continue;
                }
                if c == '\'' {
                    // A char literal only when it closes one character later
                    // ('x' or '\n'); otherwise it is a Rust lifetime like 'a,
                    // which opens nothing.
                    let mut probe = chars.clone();
                    let first = probe.next();
                    let after = if first == Some('\\') {
                        probe.next();
                        probe.next()
                    } else {
                        probe.next()
                    };
                    if after == Some('\'') {
                        let skip = if first == Some('\\') { 3 } else { 2 };
                        for _ in 0..skip {
                            chars.next();
                        }
                    }
                    continue;
                }
                match c {
                    '{' => {
                        opens += 1;
                        depth += 1;
                    }
                    '}' => {
                        closes += 1;
                        depth -= 1;
                        if depth < 0 && first_unmatched_close.is_none() {
                            first_unmatched_close = Some(line);
                        }
                    }
                    _ => {}
                }
            }
            (opens - closes, first_unmatched_close)
        }

        let (net, unmatched_close) = balance(code);
        if net > 0 {
            return Some(HealingAction {
                description: format!("Added {} missing closing brace(s) at end of file", net),
                fix: "}".repeat(net as usize),
                line: code.lines().count().max(1),
            });
        }
        if net < 0 {
            if let Some(line) = unmatched_close {
                return Some(HealingAction {
                    description: format!(
                        "{} closing brace(s) without an opener; the first is on line {}. \
                         Deleting a brace is destructive, so GTW proposes but does not apply it — \
                         inspect that line.",
                        -net, line
                    ),
                    fix: String::new(),
                    line,
                });
            }
        }
        None
    }

    fn heal_python(&self, code: &str, error: &SyntaxError) -> Option<HealingAction> {
        // Recipe B: Missing colon after def or if
        let lines: Vec<&str> = code.lines().collect();
        if error.line <= lines.len() {
            let error_line = lines[error.line - 1];
            if (error_line.trim().starts_with("def ") || error_line.trim().starts_with("if "))
                && !error_line.trim().ends_with(':')
            {
                return Some(HealingAction {
                    description: "Added missing colon at end of line".into(),
                    fix: ":".into(),
                    line: error.line,
                });
            }
        }
        None
    }
}
