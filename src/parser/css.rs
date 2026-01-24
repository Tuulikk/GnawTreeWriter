use crate::parser::{TreeNode, ParserEngineLegacy};
use anyhow::Result;
use regex::Regex;

pub struct CssParser;

impl Default for CssParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CssParser {
    pub fn new() -> Self {
        Self
    }
}

impl ParserEngineLegacy for CssParser {
    fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode> {
        // Remove comments first
        let clean_code = self.remove_comments(code);

        // Parse the CSS into a tree structure
        let root = self.parse_css(&clean_code, "".to_string(), 1)?;
        Ok(root)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["css"]
    }
}

impl CssParser {
    fn remove_comments(&self, code: &str) -> String {
        // Remove CSS comments /* ... */
        let re = Regex::new(r"/\*.*?\*/").unwrap();
        re.replace_all(code, "").to_string()
    }

    fn parse_css(&self, code: &str, path: String, start_line: usize) -> Result<TreeNode> {
        let mut children = Vec::new();
        let mut current_pos = 0;
        let mut line_num = start_line;

        // Parse at-rules (@media, @keyframes, etc.)
        let at_rule_regex = Regex::new(r"@([a-zA-Z-]+)\s*([^{]*)\s*\{").unwrap();

        // Parse regular rules (selector { ... })
        let rule_regex = Regex::new(r"([^{]+)\s*\{").unwrap();

        while current_pos < code.len() {
            let remaining = &code[current_pos..];
            let _remaining_start = current_pos;

            // Skip whitespace and newlines
            if remaining.trim().is_empty() {
                current_pos += 1;
                continue;
            }

            // Try to find an at-rule
            if let Some(caps) = at_rule_regex.captures(remaining) {
                let full_match = caps.get(0).unwrap();
                let at_name = caps.get(1).unwrap().as_str().trim();
                let at_value = caps.get(2).unwrap().as_str().trim();

                // Find matching closing brace (relative to remaining)
                let brace_start = full_match.start();
                let brace_pos = self.find_matching_brace(remaining, brace_start, '{', '}')?;
                let block_content = &remaining[full_match.end()..brace_pos];
                let rule_content = &remaining[full_match.start()..=brace_pos];

                let child_path = if path.is_empty() {
                    format!("{}", children.len())
                } else {
                    format!("{}.{}", path, children.len())
                };

                let mut at_rule_children = Vec::new();

                // Add at-rule name as child
                at_rule_children.push(TreeNode {
                    id: format!("{}.name", child_path),
                    path: format!("{}.name", child_path),
                    node_type: "at_rule_name".to_string(),
                    content: at_name.to_string(),
                    start_line: line_num,
                    end_line: line_num,
                    children: vec![],
                });

                // Add at-rule value as child if exists
                if !at_value.is_empty() {
                    at_rule_children.push(TreeNode {
                        id: format!("{}.value", child_path),
                        path: format!("{}.value", child_path),
                        node_type: "at_rule_value".to_string(),
                        content: at_value.to_string(),
                        start_line: line_num,
                        end_line: line_num,
                        children: vec![],
                    });
                }

                // Parse nested content
                let nested_tree =
                    self.parse_css(block_content, format!("{}.content", child_path), line_num)?;
                at_rule_children.push(nested_tree);

                let block_lines = rule_content.lines().count();
                children.push(TreeNode {
                    id: child_path.clone(),
                    path: child_path.clone(),
                    node_type: "at_rule".to_string(),
                    content: rule_content.to_string(),
                    start_line: line_num,
                    end_line: line_num + block_lines,
                    children: at_rule_children,
                });

                line_num += block_lines;
                current_pos += brace_pos + 1;
                continue;
            }

            // Try to find a regular rule
            if let Some(caps) = rule_regex.captures(remaining) {
                let full_match = caps.get(0).unwrap();
                let selector = caps.get(1).unwrap().as_str().trim();

                // Find matching closing brace (relative to remaining)
                let brace_start = full_match.start();
                let brace_pos = self.find_matching_brace(remaining, brace_start, '{', '}')?;
                let block_content = &remaining[full_match.end()..brace_pos];
                let rule_content = &remaining[full_match.start()..=brace_pos];

                let child_path = if path.is_empty() {
                    format!("{}", children.len())
                } else {
                    format!("{}.{}", path, children.len())
                };

                // Add selector
                let mut rule_children = vec![TreeNode {
                    id: format!("{}.selector", child_path),
                    path: format!("{}.selector", child_path),
                    node_type: "selector".to_string(),
                    content: selector.to_string(),
                    start_line: line_num,
                    end_line: line_num,
                    children: vec![],
                }];

                // Parse declarations
                let declarations = self.parse_declarations(
                    block_content,
                    format!("{}.declarations", child_path),
                    line_num,
                )?;
                rule_children.push(declarations);

                let block_lines = rule_content.lines().count();
                children.push(TreeNode {
                    id: child_path.clone(),
                    path: child_path.clone(),
                    node_type: "rule".to_string(),
                    content: rule_content.to_string(),
                    start_line: line_num,
                    end_line: line_num + block_lines,
                    children: rule_children,
                });

                line_num += block_lines;
                current_pos += brace_pos + 1;
                continue;
            }

            // If no rules found, move to next character
            current_pos += 1;
        }

        Ok(TreeNode {
            id: path.clone(),
            path,
            node_type: "stylesheet".to_string(),
            content: String::new(),
            start_line,
            end_line: line_num,
            children,
        })
    }

    fn find_matching_brace(
        &self,
        code: &str,
        start: usize,
        open: char,
        close: char,
    ) -> Result<usize> {
        let mut depth = 0;
        let mut pos = start;
        let chars: Vec<char> = code.chars().collect();

        while pos < chars.len() {
            if chars[pos] == open {
                depth += 1;
            } else if chars[pos] == close {
                depth -= 1;
                if depth == 0 {
                    return Ok(pos);
                }
            }
            pos += 1;
        }

        Err(anyhow::anyhow!("Unmatched brace in CSS"))
    }

    #[allow(dead_code)]
    fn count_lines(&self, text: &str) -> usize {
        text.lines().count().max(1)
    }

    fn parse_declarations(&self, code: &str, path: String, start_line: usize) -> Result<TreeNode> {
        let mut children = Vec::new();
        let mut line_num = start_line;

        // Parse property: value declarations
        let decl_regex = Regex::new(r"([a-zA-Z-]+)\s*:\s*([^;]+)\s*;").unwrap();

        for (i, caps) in decl_regex.captures_iter(code).enumerate() {
            let property = caps.get(1).unwrap().as_str().trim();
            let value = caps.get(2).unwrap().as_str().trim();

            let child_path = format!("{}.{}", path, i);

            let decl_children = vec![
                TreeNode {
                    id: format!("{}.property", child_path),
                    path: format!("{}.property", child_path),
                    node_type: "property".to_string(),
                    content: property.to_string(),
                    start_line: line_num,
                    end_line: line_num,
                    children: vec![],
                },
                TreeNode {
                    id: format!("{}.value", child_path),
                    path: format!("{}.value", child_path),
                    node_type: "value".to_string(),
                    content: value.to_string(),
                    start_line: line_num,
                    end_line: line_num,
                    children: vec![],
                },
            ];

            children.push(TreeNode {
                id: child_path.clone(),
                path: child_path,
                node_type: "declaration".to_string(),
                content: format!("{}: {};", property, value),
                start_line: line_num,
                end_line: line_num,
                children: decl_children,
            });

            line_num += 1;
        }

        Ok(TreeNode {
            id: path.clone(),
            path,
            node_type: "declarations".to_string(),
            content: String::new(),
            start_line,
            end_line: line_num,
            children,
        })
    }
}