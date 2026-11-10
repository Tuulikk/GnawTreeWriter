use crate::parser::TreeNode;
use colored::*;
use std::collections::HashSet;

pub struct TreeVisualizer {
    pub draw_distance: usize,
    pub use_colors: bool,
}

impl TreeVisualizer {
    pub fn new(draw_distance: usize, use_colors: bool) -> Self {
        Self {
            draw_distance,
            use_colors,
        }
    }

    pub fn generate_sparkline(&self, root: &TreeNode) -> String {
        let mut depths: Vec<usize> = Vec::new();
        fn collect(n: &TreeNode, d: usize, acc: &mut Vec<usize>) {
            acc.push(d);
            for c in &n.children {
                collect(c, d + 1, acc);
            }
        }
        collect(root, 0, &mut depths);
        if depths.is_empty() {
            return "DNA: [empty]".to_string();
        }
        // A single-node tree has max depth 0. That is the normal case when a
        // LEAF is edited (the visual diff renders the edited node as its own
        // root), and the index math below then divided by zero: the CLI
        // panicked AFTER the edit had been written and verified on disk, so the
        // caller saw a crash and had no way to know the file had changed.
        let max = (*depths.iter().max().unwrap_or(&1)).max(1);
        let chars = [" ", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
        let s: String = depths
            .iter()
            .take(50)
            .map(|&d| chars[(d * (chars.len() - 1)) / max])
            .collect();
        format!("DNA: [{}] (Complexity: {})", s.cyan(), max)
    }

    pub fn render_with_diff(
        &self,
        root: &TreeNode,
        focus_path: &str,
        old_node: Option<&TreeNode>,
    ) -> String {
        let mut out = String::new();
        let mut ancestors = HashSet::new();
        let parts: Vec<&str> = focus_path.split(".").collect();
        let mut curr = String::new();
        for p in parts {
            if !curr.is_empty() {
                curr.push('.');
            }
            curr.push_str(p);
            ancestors.insert(curr.clone());
        }
        self.render_node_internal(root, 0, &ancestors, focus_path, old_node, &mut out);
        out
    }

    fn render_node_internal(
        &self,
        n: &TreeNode,
        d: usize,
        anc: &HashSet<String>,
        fp: &str,
        old: Option<&TreeNode>,
        out: &mut String,
    ) {
        let is_focus = n.path == fp;
        let is_anc = anc.contains(&n.path);
        let is_desc = n.path.starts_with(fp) && n.path.len() > fp.len();
        let dist = self.calculate_distance(&n.path, fp);

        if !is_anc && !is_desc && dist > self.draw_distance {
            return;
        }

        let indent = "  ".repeat(d);
        if is_focus {
            if let Some(o) = old {
                let mut line = format!("{}[-] └─ {} [{}]", indent, o.path, o.node_type);
                if let Some(nm) = o.get_name() {
                    line.push_str(&format!(" \"{}\"", nm));
                }
                out.push_str(&line.red().dimmed().to_string());
                out.push('\n');
            }
        }

        let marker = if is_focus {
            "[+] "
        } else if is_desc {
            "[*] "
        } else {
            "    "
        };
        let connector = if d == 0 { "" } else { "└─ " };
        let mut line = format!(
            "{}{}{}{} [{}]",
            indent, marker, connector, n.path, n.node_type
        );
        if let Some(nm) = n.get_name() {
            line.push_str(&format!(" \"{}\"", nm));
        }

        let color_line = if is_focus {
            line.green().bold()
        } else if is_desc {
            line.cyan()
        } else {
            line.white()
        };
        out.push_str(&color_line.to_string());
        if is_focus {
            out.push_str(" <--- MODIFIED ✨");
        }
        out.push('\n');

        for c in &n.children {
            self.render_node_internal(c, d + 1, anc, fp, None, out);
        }
    }

    fn calculate_distance(&self, a: &str, b: &str) -> usize {
        let ap: Vec<&str> = a.split(".").collect();
        let bp: Vec<&str> = b.split(".").collect();
        let mut common = 0;
        for (x, y) in ap.iter().zip(bp.iter()) {
            if x == y {
                common += 1;
            } else {
                break;
            }
        }
        (ap.len() - common) + (bp.len() - common)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::TreeNode;

    /// A LEAF node rendered as its own root has max depth 0. The sparkline
    /// index math used to divide by that zero and panic — after the edit had
    /// already been written and verified on disk, so the caller believed the
    /// operation had failed. Editing any leaf (a CSS value, a JSON string,
    /// an identifier) went through this path.
    #[test]
    fn sparkline_of_a_single_node_does_not_divide_by_zero() {
        let leaf = TreeNode {
            id: "0".to_string(),
            path: "0".to_string(),
            node_type: "value".to_string(),
            content: "red".to_string(),
            start_line: 1,
            end_line: 1,
            start_col: 1,
            end_col: 4,
            children: Vec::new(),
        };
        let viz = TreeVisualizer::new(2, false);
        let out = viz.generate_sparkline(&leaf);
        assert!(out.starts_with("DNA: ["), "unexpected sparkline: {out}");
    }

    #[test]
    fn sparkline_of_a_deep_tree_still_scales() {
        let leaf = TreeNode {
            id: "0.0".to_string(),
            path: "0.0".to_string(),
            node_type: "identifier".to_string(),
            content: "x".to_string(),
            start_line: 1,
            end_line: 1,
            start_col: 1,
            end_col: 2,
            children: Vec::new(),
        };
        let root = TreeNode {
            id: "0".to_string(),
            path: "0".to_string(),
            node_type: "root".to_string(),
            content: String::new(),
            start_line: 1,
            end_line: 1,
            start_col: 1,
            end_col: 2,
            children: vec![leaf],
        };
        let viz = TreeVisualizer::new(2, false);
        assert!(viz.generate_sparkline(&root).starts_with("DNA: ["));
    }
}
