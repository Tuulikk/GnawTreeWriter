    fn handle_search_nodes(file_path: &str, pattern: &str) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(w) => {
                let mut m = Vec::new();
                fn find(n: &TreeNode, acc: &mut Vec<Value>, p: &str) {
                    if n.content.contains(p) {
                        acc.push(json!({"path": n.path, "type": n.node_type, "name": n.get_name()}));
                    }
                    for c in &n.children { find(c, acc, p); }
                }
                find(w.analyze(), &mut m, pattern);
                tool_success(format!("Found {} matches", m.len()), Some(json!({"matches": m})))
            }
            Err(e) => tool_error(format!("IO error: {}", e)),
        }
    }