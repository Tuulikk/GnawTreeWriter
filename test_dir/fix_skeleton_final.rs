    fn handle_get_skeleton(file_path: &str, max_depth: usize) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(w) => {
                let mut s = String::new();
                fn build(n: &TreeNode, out: &mut String, d: usize, md: usize) {
                    if d > md { return; }
                    out.push_str(&format!("{}{} [{}] {}
", "  ".repeat(d), n.path, n.node_type, n.get_name().unwrap_or_default()));
                    for c in &n.children { build(c, out, d + 1, md); }
                }
                build(w.analyze(), &mut s, 0, max_depth);
                tool_success(format!("Skeleton of {}", file_path), Some(json!({{"skeleton": s}})))
            }
            Err(e) => tool_error(format!("IO error: {}", e)),
        }
    }