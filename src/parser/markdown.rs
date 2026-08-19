use crate::parser::{TreeNode, ParserEngineLegacy};
use anyhow::Result;
use regex::Regex;

pub struct MarkdownParser;

impl Default for MarkdownParser {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownParser {
    pub fn new() -> Self {
        Self
    }
}

impl ParserEngineLegacy for MarkdownParser {
    fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode> {
        let root = self.parse_document(code)?;
        Ok(root)
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["md", "markdown"]
    }
}

impl MarkdownParser {
    fn parse_document(&self, code: &str) -> Result<TreeNode> {
        let mut children = Vec::new();
        let lines: Vec<&str> = code.lines().collect();
        let mut i = 0;
        let mut line_num = 1;

        let header_regex = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap();
        let code_block_regex = Regex::new(r"^```(\w*)\s*$").unwrap();
        let list_regex = Regex::new(r"^(\s*)([-*+]|\d+\.)\s+(.+)$").unwrap();
        let block_quote_regex = Regex::new(r"^>\s*(.+)$").unwrap();
        let hr_regex = Regex::new(r"^[-*_]{3,}\s*$").unwrap();

        while i < lines.len() {
            let line = lines[i];

            // Skip empty lines
            if line.trim().is_empty() {
                i += 1;
                line_num += 1;
                continue;
            }

            // Code blocks
            if let Some(caps) = code_block_regex.captures(line) {
                let lang = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let start_line = line_num;
                i += 1;
                line_num += 1;

                let mut code_lines = Vec::new();
                while i < lines.len() && !lines[i].trim().starts_with("```") {
                    code_lines.push(lines[i]);
                    i += 1;
                    line_num += 1;
                }
                i += 1; // Skip closing ```
                line_num += 1;

                children.push(TreeNode { start_col: 0, end_col: 0, 
                    id: format!("{}", children.len()),
                    path: format!("{}", children.len()),
                    node_type: "code_block".to_string(),
                    content: code_lines.join("\n"),
                    start_line,
                    end_line: line_num,
                    children: vec![TreeNode { start_col: 0, end_col: 0, 
                        id: format!("{}.lang", children.len()),
                        path: format!("{}.lang", children.len()),
                        node_type: "language".to_string(),
                        content: lang.to_string(),
                        start_line,
                        end_line: start_line,
                        children: vec![],
                    }],
                });
                continue;
            }

            // Headers
            if let Some(caps) = header_regex.captures(line) {
                let level = caps.get(1).unwrap().as_str().len();
                let text = caps.get(2).unwrap().as_str();
                // Kolumn-span = HELA radens text (1..len+1). Utan korrekta
                // kolumner (0,0) använder edit_node fallback-grenen som byter
                // hela raden och tappar "### "-prefixet — "span-överlapp" vid
                // efterföljande list-edits (2026-08-19, roadmap.md).
                let line_len = line.chars().count();
                children.push(TreeNode { start_col: 1, end_col: line_len + 1,
                    id: format!("{}", children.len()),
                    path: format!("{}", children.len()),
                    node_type: format!("heading_{}", level),
                    content: text.to_string(),
                    start_line: line_num,
                    end_line: line_num,
                    children: vec![TreeNode { start_col: 1, end_col: line_len + 1,
                        id: format!("{}.level", children.len()),
                        path: format!("{}.level", children.len()),
                        node_type: "level".to_string(),
                        content: level.to_string(),
                        start_line: line_num,
                        end_line: line_num,
                        children: vec![],
                    }],
                });

                i += 1;
                line_num += 1;
                continue;
            }

            // Block quotes
            if let Some(_caps) = block_quote_regex.captures(line) {
                let start_line = line_num;
                let mut quote_lines = Vec::new();

                while i < lines.len() {
                    if lines[i].trim().is_empty() {
                        break;
                    }
                    if let Some(c) = block_quote_regex.captures(lines[i]) {
                        quote_lines.push(c.get(1).unwrap().as_str());
                        i += 1;
                        line_num += 1;
                    } else {
                        break;
                    }
                }

                children.push(TreeNode { start_col: 0, end_col: 0, 
                    id: format!("{}", children.len()),
                    path: format!("{}", children.len()),
                    node_type: "block_quote".to_string(),
                    content: quote_lines.join("\n"),
                    start_line,
                    end_line: line_num,
                    children: vec![],
                });
                continue;
            }

            // Horizontal rules
            if hr_regex.is_match(line) {
                children.push(TreeNode { start_col: 0, end_col: 0, 
                    id: format!("{}", children.len()),
                    path: format!("{}", children.len()),
                    node_type: "horizontal_rule".to_string(),
                    content: "---".to_string(),
                    start_line: line_num,
                    end_line: line_num,
                    children: vec![],
                });

                i += 1;
                line_num += 1;
                continue;
            }

            // Lists
            if let Some(caps) = list_regex.captures(line) {
                let is_ordered = caps.get(2).unwrap().as_str().contains('.');
                let list_type = if is_ordered { "ordered" } else { "unordered" };
                let start_line = line_num;

                let mut list_items = Vec::new();
                while i < lines.len() {
                    if lines[i].trim().is_empty() {
                        i += 1;
                        line_num += 1;
                        continue;
                    }
                    if let Some(c) = list_regex.captures(lines[i]) {
                        list_items.push(c.get(2).unwrap().as_str());
                        i += 1;
                        line_num += 1;
                    } else {
                        break;
                    }
                }

                let mut item_nodes = Vec::new();
                for (idx, item) in list_items.iter().enumerate() {
                    let parsed_inline = self.parse_inline(item);
                    // Kolumn-span = hela radens text (1..len+1) — annars används
                    // fallback-grenen som tappar "- "/"1. "-prefixet vid edit.
                    let item_line = lines.get(start_line + idx).map(|l| l.chars().count()).unwrap_or(item.chars().count());
                    let item_children = vec![TreeNode { start_col: 1, end_col: item_line + 1,
                        id: format!("{}.{}.text", children.len(), idx),
                        path: format!("{}.{}.text", children.len(), idx),
                        node_type: "text".to_string(),
                        content: item.to_string(),
                        start_line: start_line + idx,
                        end_line: start_line + idx,
                        children: parsed_inline,
                    }];

                    item_nodes.push(TreeNode { start_col: 1, end_col: item_line + 1,
                        id: format!("{}.{}", children.len(), idx),
                        path: format!("{}.{}", children.len(), idx),
                        node_type: "list_item".to_string(),
                        content: item.to_string(),
                        start_line: start_line + idx,
                        end_line: start_line + idx,
                        children: item_children,
                    });
                }

                children.push(TreeNode { start_col: 1, end_col: lines.get(start_line).map(|l| l.chars().count() + 1).unwrap_or(1),
                    id: format!("{}", children.len()),
                    path: format!("{}", children.len()),
                    node_type: format!("list_{}", list_type),
                    content: String::new(),
                    start_line,
                    end_line: line_num,
                    children: item_nodes,
                });
                continue;
            }

            // Paragraphs
            let start_line = line_num;
            let mut para_lines = Vec::new();

            while i < lines.len() {
                if lines[i].trim().is_empty() {
                    break;
                }
                if header_regex.is_match(lines[i])
                    || code_block_regex.is_match(lines[i])
                    || list_regex.is_match(lines[i])
                    || block_quote_regex.is_match(lines[i])
                    || hr_regex.is_match(lines[i])
                {
                    break;
                }
                para_lines.push(lines[i]);
                i += 1;
                line_num += 1;
            }

            if !para_lines.is_empty() {
                let para_text = para_lines.join("\n");
                let inline_nodes = self.parse_inline(&para_text);

                children.push(TreeNode { start_col: 0, end_col: 0, 
                    id: format!("{}", children.len()),
                    path: format!("{}", children.len()),
                    node_type: "paragraph".to_string(),
                    content: para_text,
                    start_line,
                    end_line: line_num,
                    children: inline_nodes,
                });
            }
        }

        Ok(TreeNode { start_col: 0, end_col: 0, 
            id: "".to_string(),
            path: "".to_string(),
            node_type: "document".to_string(),
            content: String::new(),
            start_line: 1,
            end_line: line_num,
            children, 
        })
    }

    fn parse_inline(&self, text: &str) -> Vec<TreeNode> {
        let mut children = Vec::new();
        let bold_regex = Regex::new(r"\*\*(.+?)\*\*").unwrap();
        let italic_regex = Regex::new(r"\*(.+?)\*").unwrap();
        let code_regex = Regex::new(r"`(.+?)`").unwrap();
        let link_regex = Regex::new(r"\[(.+?)\]\((.+?)\)").unwrap();

        let mut remaining = text;
        let _pos = 0;

        while !remaining.is_empty() {
            let mut found = false;
            let start_pos = 0;

            // Check for links first (they may contain other inline elements)
            if let Some(caps) = link_regex.captures(remaining) {
                if let Some(m) = caps.get(0) {
                    let before = &remaining[start_pos..m.start()];
                    if !before.is_empty() {
                        children.push(TreeNode { start_col: 0, end_col: 0, 
                            id: format!("inline_{}", children.len()),
                            path: format!("inline_{}", children.len()),
                            node_type: "text".to_string(),
                            content: before.to_string(),
                            start_line: 1,
                            end_line: 1,
                            children: vec![],
                        });
                    }

                    let link_text = caps.get(1).unwrap().as_str();
                    let link_url = caps.get(2).unwrap().as_str();

                    children.push(TreeNode { start_col: 0, end_col: 0, 
                        id: format!("inline_{}", children.len()),
                        path: format!("inline_{}", children.len()),
                        node_type: "link".to_string(),
                        content: link_text.to_string(),
                        start_line: 1,
                        end_line: 1,
                        children: vec![TreeNode { start_col: 0, end_col: 0, 
                            id: format!("inline_{}.url", children.len()),
                            path: format!("inline_{}.url", children.len()),
                            node_type: "url".to_string(),
                            content: link_url.to_string(),
                            start_line: 1,
                            end_line: 1,
                            children: vec![],
                        }],
                    });

                    remaining = &remaining[m.end()..];
                    found = true;
                }
            }

            if found {
                continue;
            }

            // Check for bold
            if let Some(caps) = bold_regex.captures(remaining) {
                if let Some(m) = caps.get(0) {
                    let before = &remaining[start_pos..m.start()];
                    if !before.is_empty() {
                        children.push(TreeNode { start_col: 0, end_col: 0, 
                            id: format!("inline_{}", children.len()),
                            path: format!("inline_{}", children.len()),
                            node_type: "text".to_string(),
                            content: before.to_string(),
                            start_line: 1,
                            end_line: 1,
                            children: vec![],
                        });
                    }

                    let bold_text = caps.get(1).unwrap().as_str();

                    children.push(TreeNode { start_col: 0, end_col: 0, 
                        id: format!("inline_{}", children.len()),
                        path: format!("inline_{}", children.len()),
                        node_type: "bold".to_string(),
                        content: bold_text.to_string(),
                        start_line: 1,
                        end_line: 1,
                        children: vec![],
                    });

                    remaining = &remaining[m.end()..];
                    found = true;
                }
            }

            if found {
                continue;
            }

            // Check for code
            if let Some(caps) = code_regex.captures(remaining) {
                if let Some(m) = caps.get(0) {
                    let before = &remaining[start_pos..m.start()];
                    if !before.is_empty() {
                        children.push(TreeNode { start_col: 0, end_col: 0, 
                            id: format!("inline_{}", children.len()),
                            path: format!("inline_{}", children.len()),
                            node_type: "text".to_string(),
                            content: before.to_string(),
                            start_line: 1,
                            end_line: 1,
                            children: vec![],
                        });
                    }

                    let code_text = caps.get(1).unwrap().as_str();

                    children.push(TreeNode { start_col: 0, end_col: 0, 
                        id: format!("inline_{}", children.len()),
                        path: format!("inline_{}", children.len()),
                        node_type: "inline_code".to_string(),
                        content: code_text.to_string(),
                        start_line: 1,
                        end_line: 1,
                        children: vec![],
                    });

                    remaining = &remaining[m.end()..];
                    found = true;
                }
            }

            if found {
                continue;
            }

            // Check for italic (but not matching ** as bold)
            if let Some(caps) = italic_regex.captures(remaining) {
                if let Some(m) = caps.get(0) {
                    // Make sure it's not part of bold
                    if m.start() == 0 || !remaining[m.start() - 1..m.start()].contains('*') {
                        let before = &remaining[start_pos..m.start()];
                        if !before.is_empty() {
                            children.push(TreeNode { start_col: 0, end_col: 0, 
                                id: format!("inline_{}", children.len()),
                                path: format!("inline_{}", children.len()),
                                node_type: "text".to_string(),
                                content: before.to_string(),
                                start_line: 1,
                                end_line: 1,
                                children: vec![],
                            });
                        }

                        let italic_text = caps.get(1).unwrap().as_str();

                        children.push(TreeNode { start_col: 0, end_col: 0, 
                            id: format!("inline_{}", children.len()),
                            path: format!("inline_{}", children.len()),
                            node_type: "italic".to_string(),
                            content: italic_text.to_string(),
                            start_line: 1,
                            end_line: 1,
                            children: vec![],
                        });

                        remaining = &remaining[m.end()..];
                        found = true;
                    }
                }
            }

            if found {
                continue;
            }

            // No more inline elements found, add remaining text
            if !remaining.is_empty() {
                children.push(TreeNode { start_col: 0, end_col: 0, 
                    id: format!("inline_{}", children.len()),
                    path: format!("inline_{}", children.len()),
                    node_type: "text".to_string(),
                    content: remaining.to_string(),
                    start_line: 1,
                    end_line: 1,
                    children: vec![],
                });
                remaining = "";
            }
        }

        children
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// KOLUMN-REG.RESSION (2026-08-19): rubrik- och list-noder hade start_col/end_col
    /// = 0, vilket fick edit_node att använda fallback-grenen (hel-radsbyte) som
    /// tappade "### "/"- "-prefixet — "span-överlapp" vid roadmap-edits. Med
    /// korrekta kolumner används den kirurgiska grenen som bevarar prefix/suffix.
    #[test]
    fn markdown_nodes_have_real_column_spans() {
        let parser = MarkdownParser::new();
        let md = "### Funktioner\n- Punkt ett\n- Punkt två\n";
        let root = parser.parse_legacy(md).unwrap();

        let heading = root.children.iter().find(|n| n.node_type.starts_with("heading_")).expect("rubrik-nod");
        assert!(heading.start_col > 0, "rubrik start_col ska vara > 0 (var 0 → fallback-edit): {}", heading.start_col);
        assert!(heading.end_col > heading.start_col, "rubrik end_col ska täcka raden");

        let list = root.children.iter().find(|n| n.node_type.starts_with("list_")).expect("list-nod");
        assert!(list.start_col > 0, "list start_col ska vara > 0: {}", list.start_col);
        assert!(!list.children.is_empty(), "list ska ha items");
        let first_item = &list.children[0];
        assert!(first_item.start_col > 0, "list_item start_col ska vara > 0: {}", first_item.start_col);
        assert!(
            first_item.end_col > first_item.start_col,
            "list_item end_col ska täcka raden (annars tappas '- '-prefixet vid edit)"
        );
    }
}