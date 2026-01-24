use crate::parser::SyntaxError;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealingAction {
    pub description: String,
    pub fix: String, // Code to append or prepend
    pub line: usize,
}

pub struct Healer;

impl Healer {
    pub fn new() -> Self {
        Self
    }

    /// Analyze a syntax error and suggest a fix if possible
    pub fn suggest_fix(&self, code: &str, error: &SyntaxError, extension: &str) -> Option<HealingAction> {
        match extension {
            "rs" | "c" | "cpp" | "java" | "js" | "ts" | "qml" => self.heal_brace_languages(code, error),
            "py" => self.heal_python(code, error),
            _ => None,
        }
    }

    fn heal_brace_languages(&self, _code: &str, error: &SyntaxError) -> Option<HealingAction> {
        // Recipe A: Missing closing brace at end of file
        if error.message.contains("Syntax error") || error.message.contains("unexpected") {
            // Very simple heuristic for now: if we have more { than }
            let open_braces = _code.chars().filter(|&c| c == '{').count();
            let close_braces = _code.chars().filter(|&c| c == '}').count();
            
            if open_braces > close_braces {
                return Some(HealingAction {
                    description: format!("Added missing closing brace ({} missing)", open_braces - close_braces),
                    fix: "}".repeat(open_braces - close_braces),
                    line: error.line,
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
               && !error_line.trim().ends_with(':') {
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
