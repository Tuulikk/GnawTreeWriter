use crate::parser::TreeNode;
use colored::*; // Corrected: Removed unnecessary space after *
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

    /// Renders a tree with focus on a specific node path.
    /// Nodes far from the path or deeper than draw_distance from the focus are simplified.
    pub fn render(&self, root: &TreeNode, focus_path: &str) -> String {
        let mut output = String::new();
        let focus_parts: Vec<&str> = focus_path.split('.').collect();
        let mut ancestors = HashSet::new();
        
        // Pre-calculate ancestors of the focus node to know what to keep open
        let mut current = String::new();
        for part in &focus_parts {
            if !current.is_empty() { current.push('.'); }
            current.push_str(part);
            ancestors.insert(current.clone());
        }

        self.render_node(root, 0, &ancestors, focus_path, &mut output);
        output
    }

    fn render_node(
        &self,
        node: &TreeNode,
        depth: usize,
        ancestors: &HashSet<String>,
        focus_path: &str,
        output: &mut String,
    ) {
        let is_focus = node.path == focus_path;
        let is_ancestor = ancestors.contains(&node.path);
        let distance_to_focus = self.calculate_distance(&node.path, focus_path);

        // Draw distance logic:
        // 1. Always show ancestors
        // 2. Show nodes within draw_distance
        // 3. Collapse others
        if !is_ancestor && distance_to_focus > self.draw_distance {
            // If it's a sibling of an ancestor but far from focus, we might skip or simplify
            if depth > 0 && distance_to_focus == self.draw_distance + 1 {
                let indent = "  ".repeat(depth);
                output.push_str(&format!("{}
", indent)); // Simplified: removed '...' and '(hidden)' as per original code logic
            }
            return;
        }

        let indent = "  ".repeat(depth);
        let connector = if depth == 0 { "" } else { "└─ " };
        
        // Corrected: Removed unnecessary quotes around node.path.blue()
        let mut line = format!("{}{}{}", indent, connector, node.path.blue());
        
        line.push_str(&format!(" [{}]", node.node_type.yellow()));

        if let Some(name) = node.get_name() {
            // Corrected: Removed unnecessary quotes around name.bold()
            line.push_str(&format!(" {}", name.bold()));
        }

        if is_focus {
            // Corrected: Removed unnecessary quotes around '<--- MODIFIED ✨'.green().bold()
            line.push_str(&format!(" {}", "<--- MODIFIED ✨".green().bold()));
        }

        output.push_str(&line);
        output.push('\n');

        // Sort children to keep focus path visible
        for child in &node.children {
            self.render_node(child, depth + 1, ancestors, focus_path, output);
        }
    }

    fn calculate_distance(&self, path_a: &str, path_b: &str) -> usize {
        let a: Vec<&str> = path_a.split('.').collect();
        let b: Vec<&str> = path_b.split('.').collect();
        
        let mut common = 0;
        for (part_a, part_b) in a.iter().zip(b.iter()) {
            if part_a == part_b { common += 1; } else { break; }
        }

        (a.len() - common) + (b.len() - common)
    }
}