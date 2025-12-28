use crate::parser::{ParserEngine, TreeNode};
use anyhow::Result;
use xmltree::{Element, XMLNode};

pub struct XmlParser;

impl XmlParser {
    pub fn new() -> Self {
        Self
    }
}

impl ParserEngine for XmlParser {
    fn parse(&self, code: &str) -> Result<TreeNode> {
        // Collect top-level constructs (declaration, doctype, comments)
        // before parsing the root element. We keep the original `code`
        // so we can map byte offsets back to line numbers.
        let mut remaining = code;
        let mut top_children: Vec<TreeNode> = Vec::new();

        // Consume leading declarations/comments (simple, line-oriented)
        loop {
            let s = remaining.trim_start();
            if s.is_empty() {
                break;
            }

            if s.starts_with("<?xml") {
                if let Some(pos) = s.find("?>") {
                    let decl = &s[..pos + 2];
                    top_children.push(TreeNode {
                        id: format!("{}", top_children.len()),
                        path: format!("{}", top_children.len()),
                        node_type: "xml_declaration".to_string(),
                        content: decl.to_string(),
                        start_line: 1,
                        end_line: 1,
                        children: vec![],
                    });
                    remaining = &s[pos + 2..];
                    continue;
                }
            }

            if s.starts_with("<!DOCTYPE") {
                if let Some(pos) = s.find('>') {
                    let doctype = &s[..pos + 1];
                    top_children.push(TreeNode {
                        id: format!("{}", top_children.len()),
                        path: format!("{}", top_children.len()),
                        node_type: "doctype".to_string(),
                        content: doctype.to_string(),
                        start_line: 1,
                        end_line: 1,
                        children: vec![],
                    });
                    remaining = &s[pos + 1..];
                    continue;
                }
            }

            if s.starts_with("<!--") {
                if let Some(pos) = s.find("-->") {
                    let comment = &s[..pos + 3];
                    top_children.push(TreeNode {
                        id: format!("{}", top_children.len()),
                        path: format!("{}", top_children.len()),
                        node_type: "comment".to_string(),
                        content: comment.to_string(),
                        start_line: 1,
                        end_line: 1,
                        children: vec![],
                    });
                    remaining = &s[pos + 3..];
                    continue;
                }
            }

            // No more leading top-level constructs to consume
            break;
        }

        // Compute base offset of `remaining` inside the full `code`
        let base_offset = code.find(remaining).unwrap_or(0);

        // Parse root element with xmltree
        let elem = Element::parse(&mut std::io::Cursor::new(remaining.as_bytes()))
            .map_err(|e| anyhow::anyhow!("XML parse error: {}", e))?;

        // Try to locate the root element byte-span inside the remaining source
        if let Some(rel_open) = remaining.find(&format!("<{}", elem.name)) {
            if let Some(rel_close) =
                Self::find_matching_close_in_slice(remaining, rel_open, &elem.name)
            {
                let abs_start = base_offset + rel_open;
                let abs_end = base_offset + rel_close;
                top_children.push(self.element_to_treenode_with_span(
                    &elem,
                    "0".to_string(),
                    code,
                    abs_start,
                    abs_end,
                ));
            } else {
                // Fallback: use remaining as span if no close match found
                let abs_start = base_offset + rel_open;
                let abs_end = base_offset + remaining.len();
                top_children.push(self.element_to_treenode_with_span(
                    &elem,
                    "0".to_string(),
                    code,
                    abs_start,
                    abs_end,
                ));
            }
        } else {
            // Fallback: if we cannot find opening tag text, try to attach the parsed element to the whole remainder
            top_children.push(self.element_to_treenode_with_span(
                &elem,
                "0".to_string(),
                code,
                base_offset,
                base_offset + remaining.len(),
            ));
        }

        Ok(TreeNode {
            id: "".to_string(),
            path: "".to_string(),
            node_type: "document".to_string(),
            content: String::new(),
            start_line: 1,
            end_line: code.lines().count().max(1),
            children: top_children,
        })
    }

    fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec!["xml", "svg", "xsl", "xsd", "rss", "atom"]
    }
}

impl XmlParser {
    fn element_to_treenode_with_span(
        &self,
        el: &Element,
        path: String,
        source: &str,
        abs_start: usize,
        abs_end: usize,
    ) -> TreeNode {
        // Map byte offsets to line numbers (1-based)
        let start_line = source[..abs_start].chars().filter(|c| *c == '\n').count() + 1;
        let end_line = source[..abs_end].chars().filter(|c| *c == '\n').count() + 1;

        // Build opening tag text for convenience (name + attributes)
        let mut opening = format!("<{}", el.name);
        for (k, v) in el.attributes.iter() {
            opening.push_str(&format!(" {}=\"{}\"", k, v));
        }
        opening.push('>');

        // Attributes container (if any)
        let mut children: Vec<TreeNode> = Vec::new();
        if !el.attributes.is_empty() {
            let mut attrs: Vec<TreeNode> = Vec::new();
            for (i, (k, v)) in el.attributes.iter().enumerate() {
                let attr_path = format!("{}.attributes.{}", path, i);
                attrs.push(TreeNode {
                    id: format!("{}.name", attr_path),
                    path: format!("{}.name", attr_path),
                    node_type: "name".to_string(),
                    content: k.clone(),
                    start_line,
                    end_line,
                    children: vec![],
                });
                attrs.push(TreeNode {
                    id: format!("{}.value", attr_path),
                    path: format!("{}.value", attr_path),
                    node_type: "value".to_string(),
                    content: v.clone(),
                    start_line,
                    end_line,
                    children: vec![],
                });
            }
            children.push(TreeNode {
                id: format!("{}.attributes", path),
                path: format!("{}.attributes", path),
                node_type: "attributes".to_string(),
                content: String::new(),
                start_line,
                end_line,
                children: attrs,
            });
        }

        // Walk children and map them to spans (searching within the element's source window)
        let mut search_pos = abs_start;
        for (i, node) in el.children.iter().enumerate() {
            let child_path = format!("{}.{}", path, i);
            match node {
                XMLNode::Element(child_el) => {
                    // Try to find the child's opening tag within the parent span
                    if let Some(rel_open) =
                        source[search_pos..abs_end].find(&format!("<{}", child_el.name))
                    {
                        let child_abs_start = search_pos + rel_open;
                        // Try to find matching closing tag inside the [child_abs_start..abs_end] slice
                        if let Some(rel_close_in_slice) = Self::find_matching_close_in_slice(
                            &source[child_abs_start..abs_end],
                            0,
                            &child_el.name,
                        ) {
                            let child_abs_end = child_abs_start + rel_close_in_slice;
                            // Recurse with absolute positions
                            let child_node = self.element_to_treenode_with_span(
                                child_el,
                                child_path.clone(),
                                source,
                                child_abs_start,
                                child_abs_end,
                            );
                            children.push(child_node);
                            search_pos = child_abs_end;
                        } else if let Some(gt_rel) = source[child_abs_start..abs_end].find('>') {
                            // Self-closing or single-tag fallback: capture opening tag substring
                            let gt_abs = child_abs_start + gt_rel;
                            let full_tag = &source[child_abs_start..=gt_abs];
                            let s_line = source[..child_abs_start]
                                .chars()
                                .filter(|c| *c == '\n')
                                .count()
                                + 1;
                            let e_line =
                                source[..gt_abs + 1].chars().filter(|c| *c == '\n').count() + 1;
                            children.push(TreeNode {
                                id: child_path.clone(),
                                path: child_path.clone(),
                                node_type: "element".to_string(),
                                content: full_tag.to_string(),
                                start_line: s_line,
                                end_line: e_line,
                                children: vec![],
                            });
                            search_pos = gt_abs + 1;
                        } else {
                            // Last resort: no '>' found, fallback to name-only node
                            children.push(TreeNode {
                                id: child_path.clone(),
                                path: child_path.clone(),
                                node_type: "element".to_string(),
                                content: child_el.name.clone(),
                                start_line,
                                end_line,
                                children: vec![],
                            });
                        }
                    } else {
                        // No reliable match: attempt to find any self-closing tag in the remaining parent range
                        if let Some(rel_open2) =
                            source[search_pos..abs_end].find(&format!("<{}", child_el.name))
                        {
                            let child_abs_start = search_pos + rel_open2;
                            if let Some(gt_rel2) = source[child_abs_start..abs_end].find('>') {
                                let gt_abs2 = child_abs_start + gt_rel2;
                                let full_tag = &source[child_abs_start..=gt_abs2];
                                let s_line2 = source[..child_abs_start]
                                    .chars()
                                    .filter(|c| *c == '\n')
                                    .count()
                                    + 1;
                                let e_line2 =
                                    source[..gt_abs2 + 1].chars().filter(|c| *c == '\n').count()
                                        + 1;
                                children.push(TreeNode {
                                    id: child_path.clone(),
                                    path: child_path.clone(),
                                    node_type: "element".to_string(),
                                    content: full_tag.to_string(),
                                    start_line: s_line2,
                                    end_line: e_line2,
                                    children: vec![],
                                });
                                search_pos = gt_abs2 + 1;
                            } else {
                                children.push(TreeNode {
                                    id: child_path.clone(),
                                    path: child_path.clone(),
                                    node_type: "element".to_string(),
                                    content: child_el.name.clone(),
                                    start_line,
                                    end_line,
                                    children: vec![],
                                });
                            }
                        } else {
                            // No match at all, fallback to name-only node
                            children.push(TreeNode {
                                id: child_path.clone(),
                                path: child_path.clone(),
                                node_type: "element".to_string(),
                                content: child_el.name.clone(),
                                start_line,
                                end_line,
                                children: vec![],
                            });
                        }
                    }
                }
                XMLNode::Text(t) => {
                    let text = t.trim();
                    if !text.is_empty() {
                        if let Some(rel_pos) = source[search_pos..abs_end].find(text) {
                            let t_abs_start = search_pos + rel_pos;
                            let t_abs_end = t_abs_start + text.len();
                            let s_line =
                                source[..t_abs_start].chars().filter(|c| *c == '\n').count() + 1;
                            let e_line =
                                source[..t_abs_end].chars().filter(|c| *c == '\n').count() + 1;
                            children.push(TreeNode {
                                id: child_path.clone(),
                                path: child_path.clone(),
                                node_type: "text".to_string(),
                                content: text.to_string(),
                                start_line: s_line,
                                end_line: e_line,
                                children: vec![],
                            });
                            search_pos = t_abs_end;
                        } else {
                            children.push(TreeNode {
                                id: child_path.clone(),
                                path: child_path.clone(),
                                node_type: "text".to_string(),
                                content: text.to_string(),
                                start_line,
                                end_line,
                                children: vec![],
                            });
                        }
                    }
                }
                XMLNode::CData(c) => {
                    let cdata = c.to_string();
                    if let Some(rel_pos) = source[search_pos..abs_end].find(&cdata) {
                        let c_abs_start = search_pos + rel_pos;
                        let c_abs_end = c_abs_start + cdata.len();
                        let s_line =
                            source[..c_abs_start].chars().filter(|c| *c == '\n').count() + 1;
                        let e_line = source[..c_abs_end].chars().filter(|c| *c == '\n').count() + 1;
                        children.push(TreeNode {
                            id: child_path.clone(),
                            path: child_path.clone(),
                            node_type: "cdata".to_string(),
                            content: cdata,
                            start_line: s_line,
                            end_line: e_line,
                            children: vec![],
                        });
                        search_pos = c_abs_end;
                    } else {
                        children.push(TreeNode {
                            id: child_path.clone(),
                            path: child_path.clone(),
                            node_type: "cdata".to_string(),
                            content: cdata,
                            start_line,
                            end_line,
                            children: vec![],
                        });
                    }
                }
                XMLNode::Comment(c) => {
                    let comment = c.to_string();
                    if let Some(rel_pos) = source[search_pos..abs_end].find(&comment) {
                        let c_abs_start = search_pos + rel_pos;
                        let c_abs_end = c_abs_start + comment.len();
                        let s_line =
                            source[..c_abs_start].chars().filter(|c| *c == '\n').count() + 1;
                        let e_line = source[..c_abs_end].chars().filter(|c| *c == '\n').count() + 1;
                        children.push(TreeNode {
                            id: child_path.clone(),
                            path: child_path.clone(),
                            node_type: "comment".to_string(),
                            content: comment,
                            start_line: s_line,
                            end_line: e_line,
                            children: vec![],
                        });
                        search_pos = c_abs_end;
                    } else {
                        children.push(TreeNode {
                            id: child_path.clone(),
                            path: child_path.clone(),
                            node_type: "comment".to_string(),
                            content: comment,
                            start_line,
                            end_line,
                            children: vec![],
                        });
                    }
                }
                _ => {
                    // ignore other node variants for now
                }
            }
        }

        TreeNode {
            id: path.clone(),
            path,
            node_type: "element".to_string(),
            content: opening,
            start_line,
            end_line,
            children,
        }
    }

    // Finds the end (byte index relative to slice) of the matching closing tag for `tag`,
    // starting search at `rel_open` (relative index within `slice`). Returns index
    // of the byte just after the closing '>' of the matching closing tag (i.e., exclusive end).
    fn find_matching_close_in_slice(slice: &str, rel_open: usize, tag: &str) -> Option<usize> {
        let open_pat = format!("<{}", tag);
        let close_pat = format!("</{}", tag);

        // Start after the initial open
        let mut pos = rel_open + open_pat.len();
        let mut depth: i32 = 1;

        while pos < slice.len() {
            let next_open = slice[pos..].find(&open_pat).map(|p| pos + p);
            let next_close = slice[pos..].find(&close_pat).map(|p| pos + p);

            match (next_open, next_close) {
                (Some(o), Some(c)) => {
                    if o < c {
                        depth += 1;
                        pos = o + open_pat.len();
                    } else {
                        // found a close at `c`
                        if let Some(gt) = slice[c..].find('>') {
                            let end_pos = c + gt + 1;
                            depth -= 1;
                            if depth == 0 {
                                return Some(end_pos);
                            }
                            pos = end_pos;
                        } else {
                            return None;
                        }
                    }
                }
                (Some(o), None) => {
                    depth += 1;
                    pos = o + open_pat.len();
                }
                (None, Some(c)) => {
                    if let Some(gt) = slice[c..].find('>') {
                        let end_pos = c + gt + 1;
                        depth -= 1;
                        if depth == 0 {
                            return Some(end_pos);
                        }
                        pos = end_pos;
                    } else {
                        return None;
                    }
                }
                (None, None) => break,
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sample_xml() {
        let xml = r#"<?xml version="1.0"?>
<!DOCTYPE note SYSTEM "Note.dtd">
<!-- Sample XML -->
<note>
  <to>Tove</to>
  <from>Jani</from>
  <heading reminder="true">Reminder</heading>
  <body><![CDATA[Don't forget me this weekend!]]></body>
</note>"#;

        let parser = XmlParser::new();
        let doc = parser.parse(xml).expect("parse should succeed");
        assert_eq!(doc.node_type, "document");
        assert!(doc
            .children
            .iter()
            .any(|c| c.node_type == "xml_declaration"));
        assert!(doc.children.iter().any(|c| c.node_type == "doctype"));
        assert!(doc.children.iter().any(|c| c.node_type == "comment"));
        assert!(doc.children.iter().any(|c| c.node_type == "element"));
        let root_elem = doc
            .children
            .iter()
            .find(|n| n.node_type == "element")
            .expect("Should have root element");

        // Root element should be <note>
        assert!(root_elem.content.starts_with("<note"));
        // The element should map to the correct source lines (note starts on line 4 and ends on line 9)
        assert_eq!(root_elem.start_line, 4);
        assert_eq!(root_elem.end_line, 9);

        // Check that `to` element exists with text child "Tove"
        let to_elem = root_elem
            .children
            .iter()
            .find(|c| c.node_type == "element" && c.content.starts_with("<to"))
            .expect("Should have <to> element");

        // `to` should be on its own line (line 5)
        assert_eq!(to_elem.start_line, 5);
        assert_eq!(to_elem.end_line, 5);

        let found_text = to_elem
            .children
            .iter()
            .find(|c| c.node_type == "text" && c.content == "Tove")
            .is_some();
        assert!(found_text, "Expected text 'Tove' inside <to>");

        // Check CDATA body was captured as cdata or text, and that the cdata node has correct lines
        let body_elem = root_elem
            .children
            .iter()
            .find(|c| c.node_type == "element" && c.content.starts_with("<body"))
            .expect("Should have <body> element");

        let body_has_cdata = body_elem.children.iter().any(|c| c.node_type == "cdata");
        assert!(body_has_cdata, "Expected CDATA inside <body>");
        let cdata_node = body_elem
            .children
            .iter()
            .find(|c| c.node_type == "cdata")
            .expect("Expected cdata node");
        // CDATA should be on the expected line (line 8)
        assert_eq!(cdata_node.start_line, 8);
        assert_eq!(cdata_node.end_line, 8);
    }

    #[test]
    fn parse_self_closing_and_attrs() {
        let xml = r#"<root><img src="logo.png" alt="Logo" /><meta name='x' value='y'/></root>"#;

        let parser = XmlParser::new();
        let doc = parser.parse(xml).expect("Should parse simple XML");

        let root = doc
            .children
            .iter()
            .find(|n| n.node_type == "element")
            .expect("root element");
        // It should have children for img and meta as elements
        assert!(root
            .children
            .iter()
            .any(|c| c.node_type == "element" && c.content.starts_with("<img")));
        assert!(root
            .children
            .iter()
            .any(|c| c.node_type == "element" && c.content.starts_with("<meta")));
    }
}
