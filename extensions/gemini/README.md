# GnawTreeWriter Extension for Gemini CLI

This extension integrates GnawTreeWriter directly into Gemini CLI using the Model Context Protocol (MCP).

## Features
- **Analyze Code:** `analyze(file_path)` gives you the AST structure.
- **List Nodes:** `list_nodes(file_path)` lists all edit targets.
- **Surgical Edits:** `edit_node` and `insert_node` modify code safely with backups.

## Installation
Run this command from the project root:

```bash
gemini extensions link ./gemini-extension
```

## Usage
Once installed, you can ask Gemini to:
- "Analyze `src/main.rs` and tell me what functions exist."
- "Rename the function `foo` to `bar` in `app.py` using GnawTreeWriter."
- "Add a logging statement to the `process_data` function."
