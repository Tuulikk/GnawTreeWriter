use crate::parser::TreeNode;
use colored::::*;
use std::collections::HashSet;

pub struct TreeVisualizer {
    pub draw_distance: usize,
    pub use_colors: bool,
}

impl TreeVisualizer {
    pub fn new(draw_distance: usize, use_colors: bool) -> Self {
        Self { draw_distance, use_colors }
    }

    pub fn generate_sparkline(&self, root: &TreeNode) -> String {
        let mut depths: Vec<usize> = Vec::new();
        fn collect(n: &TreeNode, d: usize, acc: &mut Vec<usize>) {
            acc.push(d);
            for c in &n.children { collect(c, d + 1, acc); }
        }
        collect(root, 0, &mut depths);
        if depths.is_empty() { return "DNA: [empty]".to_string(); }
        let max = *depths.iter().max().unwrap_or(&1);
        let chars = [" ", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
        let s: String = depths.iter().take(50).map(|&d| chars[(d * (chars.len()-1)) / max]).collect();
        format!("DNA: [{}] (Complexity: {})", s.cyan(), max)
    }

    pub fn render_with_diff(&self, root: &TreeNode, focus_path: &str, old_node: Option<&TreeNode>) -> String {
        let mut out = String::new();
        let mut ancestors = HashSet::new();
        let parts: Vec<&str> = focus_path.split('.').collect();
        let mut curr = String::new();
        for p in parts {
            if !curr.is_empty() { curr.push('.'); }
            curr.push_str(p);
            ancestors.insert(curr.clone());
        }
        self.render_node_internal(root, 0, &ancestors, focus_path, old_node, &mut out);
        out
    }

    fn render_node_internal(&self, n: &TreeNode, d: usize, anc: &HashSet<String>, fp: &str, old: Option<&TreeNode>, out: &mut String) {
        let is_focus = n.path == fp;
        let is_anc = anc.contains(&n.path);
        let is_desc = n.path.starts_with(fp) && n.path.len() > fp.len();
        let dist = self.calculate_distance(&n.path, fp);

        if !is_anc && !is_desc && dist > self.draw_distance { return; }

        let indent = "  ".repeat(d);
        if is_focus {
            if let Some(o) = old {
                let mut line = format!("{}[-] └─ {} [{}]", indent, o.path, o.node_type);
                if let Some(nm) = o.get_name() { line.push_str(&format!(" \"{}\"", nm)); }
                out.push_str(&line.red().dimmed().to_string());
                out.push('\n');
            }
        }

        let marker = if is_focus { "[+] " } else if is_desc { "[*] " } else { "    " };
        let connector = if d == 0 { "" } else { "└─ " };
        let mut line = format!("{}{}{} [{}]", indent, marker, connector, n.path, n.node_type);
        if let Some(nm) = n.get_name() { line.push_str(&format!(" \"{}\"", nm)); }
        
        let color_line = if is_focus { line.green().bold() } else if is_desc { line.cyan() } else { line.white() };
        out.push_str(&color_line.to_string());
        if is_focus { out.push_str(" <--- MODIFIED ✨"); }
        out.push('\n');

        for c in &n.children { self.render_node_internal(c, d + 1, anc, fp, None, out); }
    }

    fn calculate_distance(&self, a: &str, b: &str) -> usize {
        let ap: Vec<&str> = a.split('.').collect();
        let bp: Vec<&str> = b.split('.').collect();
        let mut common = 0;
        for (x, y) in ap.iter().zip(bp.iter()) { if x == y { common += 1; } else { break; } }
        (ap.len() - common) + (bp.len() - common)
    }
}
