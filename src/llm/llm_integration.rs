use crate::core::{EditOperation, GnawTreeWriter};
use crate::parser::TreeNode;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// LLM-friendly request format for code modifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMEditRequest {
    pub file_path: String,
    pub intent: EditIntent,
}

/// The intent behind edit request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EditIntent {
    ReplaceNode {
        description: String,
        node_path: String,
        new_content: String,
    },
    AddProperty {
        description: String,
        component_path: String,
        property_name: String,
        property_value: String,
    },
    InsertBefore {
        description: String,
        node_path: String,
        content: String,
    },
    InsertAfter {
        description: String,
        node_path: String,
        content: String,
    },
    DeleteNode {
        description: String,
        node_path: String,
    },
}

/// Response from LLM analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMAnalysis {
    pub summary: String,
    pub suggested_edits: Vec<EditIntent>,
    pub confidence: f32,
}

/// Context information for LLM to understand code structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeContext {
    pub path: String,
    pub node_type: String,
    pub content: String,
    pub parent_path: Option<String>,
    pub children_count: usize,
    pub sibling_context: Vec<String>,
}

/// Process LLM edit request
pub fn process_llm_request(request: LLMEditRequest) -> Result<LLMResponse> {
    let mut writer = GnawTreeWriter::new(&request.file_path)?;

    match request.intent {
        EditIntent::ReplaceNode {
            description,
            node_path,
            new_content,
        } => {
            writer.edit(EditOperation::Edit {
                node_path: node_path.clone(),
                content: new_content,
            }, false)?;
            Ok(LLMResponse::success(format!(
                "Replaced node at {}: {}",
                node_path, description
            )))
        }
        EditIntent::InsertBefore {
            description,
            node_path,
            content,
        } => {
            let tree = writer.analyze();
            let parent_path = find_parent_path(tree, &node_path)
                .ok_or_else(|| anyhow::anyhow!("Could not find parent for node: {}", node_path))?;
            writer.edit(EditOperation::Insert {
                parent_path,
                position: 0,
                content,
            }, false)?;
            Ok(LLMResponse::success(format!(
                "Inserted before node {}: {}",
                node_path, description
            )))
        }
        EditIntent::InsertAfter {
            description,
            node_path,
            content,
        } => {
            let tree = writer.analyze();
            let parent_path = find_parent_path(tree, &node_path)
                .ok_or_else(|| anyhow::anyhow!("Could not find parent for node: {}", node_path))?;
            writer.edit(EditOperation::Insert {
                parent_path,
                position: 1,
                content,
            }, false)?;
            Ok(LLMResponse::success(format!(
                "Inserted after node {}: {}",
                node_path, description
            )))
        }
        EditIntent::DeleteNode {
            description,
            node_path,
        } => {
            let node_path_clone = node_path.clone();
            writer.edit(EditOperation::Delete { node_path }, false)?;
            Ok(LLMResponse::success(format!(
                "Deleted node {}: {}",
                node_path_clone, description
            )))
        }
        EditIntent::AddProperty {
            description,
            component_path,
            property_name,
            property_value,
        } => {
            let content = format!("{}: {}", property_name, property_value);
            writer.edit(EditOperation::Insert {
                parent_path: component_path.clone(),
                position: 1,
                content,
            }, false)?;
            Ok(LLMResponse::success(format!(
                "Added property {} to {}: {}",
                property_name, component_path, description
            )))
        }
    }
}

/// Get context for a specific node (for LLM understanding)
pub fn get_node_context(file_path: &str, node_path: &str) -> Result<NodeContext> {
    let writer = GnawTreeWriter::new(file_path)?;
    let tree = writer.analyze();

    let node = find_node(tree, node_path)
        .ok_or_else(|| anyhow::anyhow!("Node not found: {}", node_path))?;

    let parent_path = find_parent_path(tree, node_path);
    let sibling_context = get_sibling_content(tree, node_path);

    Ok(NodeContext {
        path: node_path.to_string(),
        node_type: node.node_type.clone(),
        content: node.content.clone(),
        parent_path,
        children_count: node.children.len(),
        sibling_context,
    })
}

/// Suggest edits based on LLM analysis
pub fn suggest_edits(file_path: &str, analysis: LLMAnalysis) -> Result<Vec<LLMResponse>> {
    let mut responses = Vec::new();

    for edit in analysis.suggested_edits {
        let request = LLMEditRequest {
            file_path: file_path.to_string(),
            intent: edit.clone(),
        };
        responses.push(process_llm_request(request)?);
    }

    Ok(responses)
}

/// LLM response format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub success: bool,
    pub message: String,
    pub modified_nodes: Vec<String>,
}

impl LLMResponse {
    fn success(message: String) -> Self {
        Self {
            success: true,
            message,
            modified_nodes: Vec::new(),
        }
    }
}

fn find_node<'a>(tree: &'a TreeNode, path: &str) -> Option<&'a TreeNode> {
    if tree.path == path {
        return Some(tree);
    }
    for child in &tree.children {
        if let Some(node) = find_node(child, path) {
            return Some(node);
        }
    }
    None
}

fn find_parent_path(tree: &TreeNode, node_path: &str) -> Option<String> {
    for child in &tree.children {
        if child.path == node_path {
            return Some(tree.path.clone());
        }
        if let Some(found) = find_parent_path(child, node_path) {
            return Some(found);
        }
    }
    None
}

fn get_sibling_content(tree: &TreeNode, node_path: &str) -> Vec<String> {
    if let Some(parent_path) = find_parent_path(tree, node_path) {
        if let Some(parent) = find_node(tree, &parent_path) {
            return parent
                .children
                .iter()
                .filter(|c| c.path != node_path)
                .map(|c| c.content.clone())
                .collect();
        }
    }
    Vec::new()
}

/// Create a map of all nodes in tree for easy lookup
pub fn create_node_map(tree: &TreeNode) -> HashMap<String, TreeNode> {
    let mut map = HashMap::new();
    build_node_map(tree, &mut map);
    map
}

fn build_node_map(node: &TreeNode, map: &mut HashMap<String, TreeNode>) {
    map.insert(node.path.clone(), node.clone());
    for child in &node.children {
        build_node_map(child, map);
    }
}
