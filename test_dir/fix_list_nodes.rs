    fn handle_list_nodes(state: Arc<AppState>, file_path: &str, _filter: Option<&str>, _max_depth: Option<usize>, _all: bool) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(w) => {
                let label_mgr = LabelManager::load(&state.project_root).ok();
                let mut nodes = Vec::new();
                fn collect(n: &TreeNode, acc: &mut Vec<Value>, fp: &str, lm: &Option<LabelManager>) {
                    let labels = lm.as_ref().map(|mgr| mgr.get_labels(fp, &n.content)).unwrap_or_default();
                    acc.push(json!({"path": n.path, "type": n.node_type, "name": n.get_name(), "start": n.start_line, "labels": labels}));
                    for c in &n.children { collect(c, acc, fp, lm); }
                }
                collect(w.analyze(), &mut nodes, file_path, &label_mgr);
                tool_success(format!("Found {} nodes", nodes.len()), Some(json!({"nodes": nodes})))
            }
            Err(e) => tool_error(format!("IO error: {}", e)),
        }
    }