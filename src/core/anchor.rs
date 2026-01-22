//! Anchor detection for LLM partial outputs.
//! Ported from Comparative-Writer.
//!
//! Detects patterns like `// ...`, `# ...`, `/* ... */` that indicate
//! "keep existing code here".

use regex::Regex;

/// An anchor in the code that represents "existing code"
#[derive(Debug, Clone, PartialEq)]
pub struct Anchor {
    /// The original anchor text (e.g., "// ... existing imports ...")
    pub text: String,
    /// Start position in the source
    pub start: usize,
    /// End position in the source
    pub end: usize,
    /// Optional hint about what the anchor represents
    pub hint: Option<String>,
    /// The anchor style
    pub style: AnchorStyle,
}

/// Style of anchor comment
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnchorStyle {
    /// `// ...` or `// ... text ...`
    SlashSlash,
    /// `/* ... */` or `/* ... text ... */`
    SlashStar,
    /// `# ...` or `# ... text ...`
    Hash,
    /// `<!-- ... -->` or `<!-- ... text ... -->`
    Html,
    /// `""" ... """` or `''' ... '''`
    TripleQuote,
}

/// Detects anchors in code
pub struct AnchorDetector {
    patterns: Vec<(Regex, AnchorStyle)>,
}

impl Default for AnchorDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl AnchorDetector {
    /// Create a new anchor detector with default patterns
    pub fn new() -> Self {
        let patterns = vec![
            // // ... or // ... text ...
            (
                Regex::new(r"//\s*\.{3,}[^\n]*").unwrap(),
                AnchorStyle::SlashSlash,
            ),
            // // existing code, // rest of file, etc.
            (
                Regex::new(r"//\s*(?:existing|rest of|previous|remaining|other)[^\n]*").unwrap(),
                AnchorStyle::SlashSlash,
            ),
            // /* ... */ or /* ... text ... */
            (
                Regex::new(r"/\*\s*\.{3,}[^*]*\*/").unwrap(),
                AnchorStyle::SlashStar,
            ),
            // # ... or # ... text ...
            (
                Regex::new(r"#\s*\.{3,}[^\n]*").unwrap(),
                AnchorStyle::Hash,
            ),
            // # existing code, # rest of file, etc.
            (
                Regex::new(r"#\s*(?:existing|rest of|previous|remaining|other)[^\n]*").unwrap(),
                AnchorStyle::Hash,
            ),
            // <!-- ... --> or <!-- ... text ... -->
            (
                Regex::new(r"<!--\s*\.{3,}[^-]*-->").unwrap(),
                AnchorStyle::Html,
            ),
            // ... (standalone ellipsis line)
            (
                Regex::new(r"^\s*\.{3,}\s*$").unwrap(),
                AnchorStyle::SlashSlash, // Default style
            ),
        ];
        
        Self { patterns }
    }
    
    /// Detect all anchors in the given code
    pub fn detect(&self, code: &str) -> Vec<Anchor> {
        let mut anchors = Vec::new();
        
        for (pattern, style) in &self.patterns {
            for m in pattern.find_iter(code) {
                let text = m.as_str().to_string();
                let hint = self.extract_hint(&text, *style);
                
                anchors.push(Anchor {
                    text: text.clone(),
                    start: m.start(),
                    end: m.end(),
                    hint,
                    style: *style,
                });
            }
        }
        
        // Sort by position and deduplicate overlapping
        anchors.sort_by_key(|a| a.start);
        self.deduplicate_overlapping(anchors)
    }
    
    /// Extract a hint from the anchor text
    fn extract_hint(&self, text: &str, style: AnchorStyle) -> Option<String> {
        // Remove comment markers
        let clean = match style {
            AnchorStyle::SlashSlash => text.trim_start_matches('/').trim(),
            AnchorStyle::SlashStar => text
                .trim_start_matches("/*")
                .trim_end_matches("*/")
                .trim(),
            AnchorStyle::Hash => text.trim_start_matches('#').trim(),
            AnchorStyle::Html => text
                .trim_start_matches("<!--")
                .trim_end_matches("-->")
                .trim(),
            AnchorStyle::TripleQuote => text
                .trim_start_matches("\"\"\"")
                .trim_end_matches("\"\"\"")
                .trim_start_matches("'''")
                .trim_end_matches("'''")
                .trim(),
        };
        
        // Remove ellipsis
        let hint = clean
            .trim_start_matches('.')
            .trim_end_matches('.')
            .trim();
        
        if hint.is_empty() {
            None
        } else {
            Some(hint.to_string())
        }
    }
    
    /// Remove overlapping anchors, keeping the more specific one
    fn deduplicate_overlapping(&self, anchors: Vec<Anchor>) -> Vec<Anchor> {
        let mut result: Vec<Anchor> = Vec::new();
        
        for anchor in anchors {
            // Check if this overlaps with the last added anchor
            if let Some(last) = result.last() {
                if anchor.start < last.end {
                    // Overlapping - keep the more specific one (with hint)
                    if anchor.hint.is_some() && last.hint.is_none() {
                        result.pop();
                        result.push(anchor);
                    }
                    // Otherwise keep the existing one
                    continue;
                }
            }
            result.push(anchor);
        }
        
        result
    }
}
