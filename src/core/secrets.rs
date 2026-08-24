//! Secret detection for safe AI context sharing.
//!
//! Combines custom regex patterns with the `secrets_scanner` crate for
//! comprehensive credential detection. Two layers:
//! 1. Custom patterns (AWS, GitHub, GitLab, Stripe, JWT, etc.)
//! 2. secrets_scanner (Aho-Corasick + regex + entropy gating)

use regex::Regex;
use serde::Serialize;
use std::sync::LazyLock;

static SCANNER: LazyLock<Option<secrets_scanner::Scanner>> = LazyLock::new(|| {
    secrets_scanner::Scanner::from_bundled().ok()
});

/// A detected secret.
#[derive(Debug, Clone, Serialize)]
pub struct DetectedSecret {
    /// Line number (1-based).
    pub line: usize,
    /// Pattern name that matched.
    pub pattern: String,
    /// Preview of the matched content (redacted).
    pub preview: String,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f64,
}

/// Scan source code for secrets using both custom patterns and secrets_scanner.
pub fn scan_for_secrets(source: &str) -> Vec<DetectedSecret> {
    let mut secrets = Vec::new();

    // Layer 1: Custom patterns
    for (line_num, line) in source.lines().enumerate() {
        for pattern in SECRET_PATTERNS.iter() {
            let re = pattern.get_regex();
            if let Some(mat) = re.find(line) {
                let matched = mat.as_str();
                if is_likely_safe(matched) {
                    continue;
                }
                secrets.push(DetectedSecret {
                    line: line_num + 1,
                    pattern: pattern.name.to_string(),
                    preview: redact_secret(matched),
                    confidence: pattern.confidence,
                });
            }
        }
    }

    // Layer 2: secrets_scanner (entropy + pattern based)
    if let Some(scanner) = SCANNER.as_ref() {
        let result = scanner.scan_bytes_detailed("source", source.as_bytes());

        for finding in &result.findings {
            let line_num = finding.line;

            // Check if we already detected this on this line
            let already_found = secrets.iter().any(|s| s.line == line_num);
            if already_found {
                continue;
            }

            // Extract the matched text from the line
            let line_start = source.lines().take(line_num - 1).map(|l| l.len() + 1).sum::<usize>();
            let line_text = source.lines().nth(line_num - 1).unwrap_or("");
            let col = finding.col.saturating_sub(1);
            let end_col = finding.end_col.saturating_sub(1);
            let matched = if col < line_text.len() && end_col <= line_text.len() {
                &line_text[col..end_col]
            } else {
                continue;
            };

            if is_likely_safe(matched) {
                continue;
            }

            secrets.push(DetectedSecret {
                line: line_num,
                pattern: finding.rule_id.clone(),
                preview: redact_secret(matched),
                confidence: 0.6,
            });
        }
    }

    secrets
}

/// Redact secrets in source code, replacing matches with `<REDACTED>`.
pub fn redact_secrets(source: &str) -> (String, usize) {
    let mut result = source.to_string();
    let mut count = 0;

    for pattern in SECRET_PATTERNS.iter() {
        let re = pattern.get_regex();
        let source_clone = result.clone();
        let matches: Vec<_> = re.find_iter(&source_clone).collect();
        for mat in matches.into_iter().rev() {
            let matched = mat.as_str();
            if is_likely_safe(matched) {
                continue;
            }
            result.replace_range(mat.start()..mat.end(), "<REDACTED>");
            count += 1;
        }
    }

    (result, count)
}

struct SecretPattern {
    name: &'static str,
    regex_str: &'static str,
    confidence: f64,
}

impl SecretPattern {
    fn get_regex(&self) -> Regex {
        Regex::new(self.regex_str).unwrap()
    }
}

macro_rules! secret_pattern {
    ($name:expr, $regex:expr, $confidence:expr) => {
        SecretPattern {
            name: $name,
            regex_str: $regex,
            confidence: $confidence,
        }
    };
}

static SECRET_PATTERNS: &[SecretPattern] = &[
    // AWS
    secret_pattern!(
        "AWS Access Key",
        r"AKIA[0-9A-Z]{16}",
        0.95
    ),
    // AWS Secret Key
    secret_pattern!(
        "AWS Secret Key",
        r"(?i)(?:aws[_]?secret[_]?access[_]?key|secret[_]?key)\s*[:=]\s*[A-Za-z0-9/+=]{40}",
        0.90
    ),
    // GitHub
    secret_pattern!(
        "GitHub Personal Access Token",
        r"ghp_[A-Za-z0-9_]{36,}",
        0.95
    ),
    secret_pattern!(
        "GitHub Fine-grained PAT",
        r"github_pat_[A-Za-z0-9_]{22,}",
        0.95
    ),
    // GitLab
    secret_pattern!(
        "GitLab Personal Access Token",
        r"(?:^|[^A-Za-z0-9])(?:glpat|glpat-)[A-Za-z0-9\-_]{20,}(?:[^A-Za-z0-9\-_]|$)",
        0.90
    ),
    // Google API
    secret_pattern!(
        "Google API Key",
        r"(?:^|[^A-Za-z0-9])(?:AIza)[A-Za-z0-9_\-]{35}(?:[^A-Za-z0-9_\-]|$)",
        0.90
    ),
    // Private keys
    secret_pattern!(
        "RSA Private Key",
        r"-----BEGIN\s+(?:RSA\s+)?PRIVATE\s+KEY-----",
        0.99
    ),
    secret_pattern!(
        "EC Private Key",
        r"-----BEGIN\s+EC\s+PRIVATE\s+KEY-----",
        0.99
    ),
    secret_pattern!(
        "DSA Private Key",
        r"-----BEGIN\s+DSA\s+PRIVATE\s+KEY-----",
        0.99
    ),
    // SSH
    secret_pattern!(
        "SSH Private Key",
        r"-----BEGIN\s+OPENSSH\s+PRIVATE\s+KEY-----",
        0.99
    ),
    // JWT
    secret_pattern!(
        "JWT Token",
        r"eyJ[A-Za-z0-9_\-]+\.eyJ[A-Za-z0-9_\-]+\.[A-Za-z0-9_\-]+",
        0.85
    ),
    // Bearer tokens
    secret_pattern!(
        "Bearer Token",
        r"(?i)(?:bearer|token|api[_]?key|apikey|api[_]?secret)\s*[:=]\s*[A-Za-z0-9_\-\.]{20,}",
        0.75
    ),
    // Passwords
    secret_pattern!(
        "Password Assignment",
        r"(?i)(?:password|passwd|pwd)\s*[:=]\s*[^\s]{8,}",
        0.70
    ),
    // Base64-encoded secrets (long strings)
    secret_pattern!(
        "Base64 Secret",
        r"(?i)(?:secret|key|token|password|credential)\s*[:=]\s*[A-Za-z0-9+/]{40,}={0,2}",
        0.65
    ),
    // Connection strings
    secret_pattern!(
        "Database Connection String",
        r"(?i)(?:mysql|postgresql|mongodb|redis|amqp)://[^\s]{20,}",
        0.80
    ),
    // Stripe
    secret_pattern!(
        "Stripe API Key",
        r"(?:^|[^A-Za-z0-9])(?:sk_live|sk_test|pk_live|pk_test)_[A-Za-z0-9]{20,}(?:[^A-Za-z0-9]|$)",
        0.90
    ),
    // Slack
    secret_pattern!(
        "Slack Token",
        r"(?:^|[^A-Za-z0-9])(?:xox[baprs]-[A-Za-z0-9\-]{10,})(?:[^A-Za-z0-9]|$)",
        0.85
    ),
    // Twilio
    secret_pattern!(
        "Twilio API Key",
        r"(?:^|[^A-Za-z0-9])(?:SK)[a-f0-9]{32}(?:[^A-Za-z0-9]|$)",
        0.70
    ),
    // Generic hex secrets (long strings)
    secret_pattern!(
        "Generic Hex Secret",
        r"(?i)(?:secret|key|token|password|salt|hash)\s*[:=]\s*[0-9a-f]{32,}",
        0.60
    ),
];

/// Check if a matched string is likely safe (not a real secret).
fn is_likely_safe(matched: &str) -> bool {
    let lower = matched.to_lowercase();

    // Common placeholder values
    let placeholders = [
        "your_api_key",
        "your-api-key",
        "your_api_secret",
        "xxx",
        "example",
        "placeholder",
        "todo",
        "changeme",
        "replace_me",
        "insert_key_here",
        "sk-xxx",
        "none",
        "null",
        "empty",
    ];

    for p in &placeholders {
        if lower.contains(p) {
            return true;
        }
    }

    // Test/example values
    if lower.contains("test") || lower.contains("example") || lower.contains("dummy") {
        return true;
    }

    // All same character (e.g., "aaaaaaa")
    let chars: Vec<char> = matched.chars().filter(|c| !c.is_whitespace()).collect();
    if chars.len() > 4 && chars.windows(2).all(|w| w[0] == w[1]) {
        return true;
    }

    false
}

/// Redact a secret string, showing only first and last few characters.
fn redact_secret(secret: &str) -> String {
    let len = secret.len();
    if len <= 8 {
        "*".repeat(len)
    } else {
        format!("{}...{}", &secret[..4], &secret[len - 4..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_aws_key() {
        let source = "aws_access_key_id = \"AKIAIOSFODNN7QWERTYUI\"";
        let secrets = scan_for_secrets(source);
        assert!(!secrets.is_empty(), "Should detect AWS key");
        assert_eq!(secrets[0].pattern, "AWS Access Key");
    }

    #[test]
    fn test_detect_github_token() {
        let source = "const token = \"ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij\"";
        let secrets = scan_for_secrets(source);
        assert!(!secrets.is_empty(), "Should detect GitHub token");
        assert_eq!(secrets[0].pattern, "GitHub Personal Access Token");
    }

    #[test]
    fn test_detect_private_key() {
        let source = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAK...";
        let secrets = scan_for_secrets(source);
        assert!(!secrets.is_empty(), "Should detect private key");
        assert_eq!(secrets[0].pattern, "RSA Private Key");
    }

    #[test]
    fn test_detect_jwt() {
        let source = "token = \"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U\"";
        let secrets = scan_for_secrets(source);
        assert!(!secrets.is_empty(), "Should detect JWT");
        assert_eq!(secrets[0].pattern, "JWT Token");
    }

    #[test]
    fn test_detect_stripe_key() {
        let source = "stripe_secret_key = \"sk_live_FAKEKEY1234567890ABCDEF\"";
        let secrets = scan_for_secrets(source);
        assert!(!secrets.is_empty(), "Should detect Stripe key");
        assert_eq!(secrets[0].pattern, "Stripe API Key");
    }

    #[test]
    fn test_ignores_placeholders() {
        let source = "api_key = \"your_api_key_here\"";
        let secrets = scan_for_secrets(source);
        assert!(secrets.is_empty(), "Should not flag placeholders");
    }

    #[test]
    fn test_ignores_test_values() {
        let source = "token = \"test_token_12345\"";
        let secrets = scan_for_secrets(source);
        assert!(secrets.is_empty(), "Should not flag test values");
    }

    #[test]
    fn test_redact_secrets() {
        let source = "key = \"AKIAIOSFODNN7QWERTYUI\"";
        let (redacted, count) = redact_secrets(source);
        assert!(count > 0);
        assert!(!redacted.contains("AKIAIOSFODNN7QWERTYUI"));
        assert!(redacted.contains("<REDACTED>"));
    }

    #[test]
    fn test_redact_preserves_safe_code() {
        let source = "fn main() {\n    let x = 42;\n    println!(\"{}\", x);\n}";
        let (redacted, count) = redact_secrets(source);
        assert_eq!(count, 0);
        assert_eq!(redacted, source);
    }

    #[test]
    fn test_multiple_secrets() {
        let source = r#"
aws_key = "AKIAIOSFODNN7QWERTYUI"
token = "ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij"
password = "supersecretpassword123"
"#;
        let secrets = scan_for_secrets(source);
        assert!(secrets.len() >= 2, "Should detect multiple secrets, found: {}", secrets.len());
    }

    #[test]
    fn test_no_false_positives_on_common_code() {
        let source = r#"use std::collections::HashMap;

fn main() {
    let mut map = HashMap::new();
    map.insert("key", "value");
    let config = Config::new();
}"#;
        let secrets = scan_for_secrets(source);
        assert!(secrets.is_empty(), "Should not flag common code patterns");
    }
}
