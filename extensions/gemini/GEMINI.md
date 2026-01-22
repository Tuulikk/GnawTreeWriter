# GnawTreeWriter Extension Guidelines

You have access to **GnawTreeWriter**, a powerful AST-based code editor designed for AI agents. This tool allows you to read and modify code structurally, ensuring syntax safety and surgical precision.

## 🚀 Core Mandate
**ALWAYS use GnawTreeWriter tools (`analyze`, `list_nodes`, `edit_node`, etc.) for reading and modifying code files.** 

Do NOT use standard file overwrite tools (like `write_file` or `replace`) unless explicitly requested or if the file type is unsupported. GnawTreeWriter handles backups, validation, and structural integrity automatically.

## 🛠 Workflow

When asked to modify code, follow this sequence:

1.  **Explore:**
    *   Use `list_nodes(file_path)` to see the file's structure (classes, functions).
    *   Use `search_nodes(file_path, pattern)` if you are looking for specific text.
    *   *Tip:* Use `max_depth=1` or `max_depth=2` on large files to get a high-level overview without noise.

2.  **Read:**
    *   Use `read_node(file_path, node_path)` to fetch the *exact* code block you need to understand.
    *   *Avoid reading the entire file* if you only need to fix one function. This saves tokens and reduces confusion.

3.  **Edit:**
    *   Use `edit_node(file_path, node_path, new_content)` to replace a specific function, class, or block.
    *   Use `insert_node` to add new methods or fields safely.

## 💡 Best Practices

*   **Trust the AST:** The `node_path` (e.g., `class_definition.0/function_definition.1`) is your coordinate system. Rely on it.
*   **Be Surgical:** Don't rewrite the whole file to change one line. Edit only the node that needs changing.
*   **Verify:** After editing, the tool will return the new structure. If you are unsure, read the node again to confirm.
*   **Safety:** GnawTreeWriter automatically creates backups. You can edit with confidence.

## ⚠️ Troubleshooting
*   If a `node_path` is invalid, use `list_nodes` again to refresh your view of the structure.
*   If `list_nodes` returns too much data, use the `filter_type` argument (e.g., "function_definition") to narrow it down.
