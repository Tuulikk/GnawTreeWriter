use crate::parser::SyntaxError;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealingAction {
    pub description: String,
    pub fix: String, // Code to append or prepend
    pub line: usize,
}

pub struct Healer;

/// What `repair_file` decided. The CLI prints it; the MCP tool serialises it.
/// Deliberately explicit: "proposal" and "applied" are different outcomes, and
/// neither is allowed to look like the other.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum HealOutcome {
    AlreadyParses,
    NoRecipe {
        error: String,
    },
    /// A repair that would DELETE code. Proposed with its line, never applied.
    ProposalOnly {
        description: String,
        line: usize,
    },
    /// An appendable repair, shown but not written (no `--apply`).
    Preview {
        description: String,
        append: String,
        line: usize,
    },
    Applied {
        description: String,
        bytes: usize,
    },
}

/// Read → parse → propose (→ verify → write → log) one file.
///
/// Rules that make this safe: a healthy file is never touched, a repair that
/// does not itself parse is never written, and code is never deleted.
pub fn repair_file(path: &Path, apply: bool) -> anyhow::Result<HealOutcome> {
    let code = std::fs::read_to_string(path)?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();
    let parser = crate::parser::get_parser(Path::new(&format!("x.{ext}")))
        .map_err(|_| anyhow::anyhow!("no parser for .{ext} — cannot judge or heal it"))?;

    let error = match parser.parse(&code) {
        Ok(_) => return Ok(HealOutcome::AlreadyParses),
        Err(e) => e,
    };

    let Some(action) = Healer::new().suggest_fix(&code, &error, &ext) else {
        return Ok(HealOutcome::NoRecipe {
            error: error.message,
        });
    };
    if action.fix.is_empty() {
        return Ok(HealOutcome::ProposalOnly {
            description: action.description,
            line: action.line,
        });
    }

    let mut healed = format!("{code}{}", action.fix);
    if !healed.ends_with('\n') {
        healed.push('\n');
    }
    if let Err(e) = parser.parse(&healed) {
        return Ok(HealOutcome::NoRecipe {
            error: format!("the proposed repair does not parse either: {e}"),
        });
    }
    if !apply {
        return Ok(HealOutcome::Preview {
            description: action.description,
            append: action.fix,
            line: action.line,
        });
    }

    std::fs::write(path, &healed)?;
    let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    if let Ok(mut log) = crate::core::transaction_log::TransactionLog::new(&root) {
        use crate::core::transaction_log::{calculate_content_hash, OperationType};
        let mut meta = std::collections::HashMap::new();
        meta.insert("operation".to_string(), "heal".to_string());
        let _ = log.log_transaction(
            OperationType::Edit,
            path.to_path_buf(),
            None,
            Some(calculate_content_hash(&code)),
            Some(calculate_content_hash(&healed)),
            format!("heal: {}", action.description),
            meta,
        );
    }
    Ok(HealOutcome::Applied {
        description: action.description,
        bytes: healed.len(),
    })
}

/// Measured advice for a file that failed to parse: IS this a `heal` case?
///
/// Shared by `validate` (CLI and MCP) so the two surfaces cannot drift, and
/// measured rather than assumed — telling a model to run `heal` on a parse
/// error that is *not* a brace problem wastes the turn (the failure mode this
/// project keeps paying for: a conclusion drawn from one observation).
pub fn heal_advice(path: &Path) -> String {
    match repair_file(path, false) {
        Ok(HealOutcome::Preview { description, .. })
        | Ok(HealOutcome::ProposalOnly { description, .. }) => format!(
            "brace balance is off ({description}) — this IS a heal case: `gnawtreewriter heal {}` previews the repair, `--apply` writes it after proving the result parses",
            path.display()
        ),
        Ok(_) => "braces are balanced, so this is NOT a heal case — the error is somewhere else; read the position above".to_string(),
        Err(e) => format!("heal could not inspect the file ({e})"),
    }
}

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
