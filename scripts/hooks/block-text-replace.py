#!/usr/bin/env python3
"""
PreToolUse hook: block replace_string_in_file for AST-supported file types.

When the agent tries to use text-replace tools on code files that GnawTreeWriter
can handle, this hook denies the operation and tells the agent to use
mcp_gnawtreewrite_* tools instead.

Exit codes:
  0 = success (response on stdout determines allow/deny)
  2 = blocking error

Input (stdin): JSON with tool call info
Output (stdout): JSON with permission decision
"""

import json
import sys
import os

# File extensions supported by GnawTreeWriter
# Källa: README.md (26+ programming languages)
AST_EXTENSIONS = {
    # Python, Rust, C/C++
    ".py", ".rs", ".c", ".h", ".cpp", ".hpp", ".cc", ".cxx", ".hxx", ".h++",
    # Java, Kotlin, C#, Dart, Swift, Zig
    ".java", ".kt", ".kts", ".cs", ".dart", ".swift", ".zig",
    # TypeScript, JavaScript
    ".ts", ".tsx", ".mts", ".cts", ".d.ts", ".js", ".jsx", ".cjs", ".mjs",
    # Shell
    ".sh", ".bash",
    # PHP, Go, SQL
    ".php", ".go", ".sql",
    # Web
    ".html", ".css", ".svelte", ".vue",
    # QML, YAML, TOML, XML, JSON
    ".qml", ".yaml", ".yml", ".toml", ".xml", ".json",
    # Markdown
    ".md", ".markdown",
}

# Tools to block (VSCode edit/write tools that GnawTreeWriter should replace)
BLOCKED_TOOLS = {
    "replace_string_in_file",
    "insert_edit_into_file",
    "write_file",
    "create_file",
}

# Terminal tools to inspect for text-replace commands
SHELL_TOOLS = {
    "run_in_terminal",
    "send_to_terminal",
    "run_in_shell",
}

# Command patterns that indicate text-replace operations (case-insensitive)
# Agent might run these via terminal to bypass the VSCode tool blocks
BLOCKED_COMMAND_PATTERNS = [
    r'\bsed\s+-i\b',           # sed -i (in-place edit)
    r'\bperl\s+-pi\b',         # perl -pi (in-place perl)
    r'\bawk\s+.*\bfprintf\b',  # awk with fprintf (file write)
    r'\bcat\s+.*>\s*\S+',      # cat redirect to file
    r'\bprintf\b.*>\s*\S+',    # printf redirect to file
    r'\becho\b.*>\s*\S+',      # echo redirect to file
    r'\btee\b\s+\S+',          # tee to file
]

# Mapping from blocked tool/shell to suggested GnawTreeWriter alternative
TOOL_SUGGESTIONS = {
    "replace_string_in_file": "mcp_gnawtreewrite_edit_node or mcp_gnawtreewrite_semantic_edit",
    "insert_edit_into_file": "mcp_gnawtreewrite_insert_node or mcp_gnawtreewrite_semantic_insert",
    "write_file": "mcp_gnawtreewrite_insert_node (to insert new code) or mcp_gnawtreewrite_edit_node (to modify existing)",
    "create_file": "mcp_gnawtreewrite_insert_node (to add new content) — GnawTreeWriter can create new files via insert",
    "run_in_terminal": "mcp_gnawtreewrite_edit_node or mcp_gnawtreewrite_semantic_edit",
    "send_to_terminal": "mcp_gnawtreewrite_edit_node or mcp_gnawtreewrite_semantic_edit",
}


def get_file_path(input_data):
    """Extract file path from various possible input schemas."""
    # Try direct arguments
    if "arguments" in input_data:
        args = input_data["arguments"]
        if isinstance(args, dict):
            # Common patterns for filePath
            for key in ("filePath", "file_path", "filepath", "path"):
                if key in args:
                    return args[key]
            # Check if any value looks like a file path
            for key, val in args.items():
                if isinstance(val, str) and os.path.exists(val):
                    return val

    # Try tool-level properties
    if "filePath" in input_data:
        return input_data["filePath"]
    if "file_path" in input_data:
        return input_data["file_path"]

    return None


def find_ast_files_in_command(command):
    """
    Scan a command string for file arguments with AST-supported extensions.
    Returns the first matching file path, or None.
    """
    import re
    if not command or not isinstance(command, str):
        return None

    # Split into tokens and check each one
    tokens = command.split()
    for token in tokens:
        # Strip quotes and common flags
        clean = token.strip("'\"")
        # Skip flags
        if clean.startswith("-"):
            continue
        # Skip shell operators
        if clean in (">", ">>", "<", "|", "&&", "||", ";"):
            continue
        # Check if it looks like a file path with an AST extension
        _, ext = os.path.splitext(clean)
        if ext.lower() in AST_EXTENSIONS:
            return clean

    return None


def has_blocked_command_pattern(command):
    """
    Check if a command contains a blocked text-replace pattern.
    """
    import re
    if not command or not isinstance(command, str):
        return False
    cmd_lower = command.lower()
    for pattern in BLOCKED_COMMAND_PATTERNS:
        if re.search(pattern, cmd_lower):
            return True
    return False


def is_ast_supported(file_path):
    """Check if the file extension is supported by GnawTreeWriter."""
    if not file_path or not isinstance(file_path, str):
        return False
    _, ext = os.path.splitext(file_path)
    return ext.lower() in AST_EXTENSIONS


def handle_pre_tool_use(input_data):
    """Process a PreToolUse event and return permission decision."""
    tool_name = input_data.get("toolName") or input_data.get("tool", {}).get("name", "")
    arguments = input_data.get("arguments") or input_data.get("tool", {}).get("arguments", {})

    # ── Block shell text-replace commands ────────────────────────────
    if tool_name in SHELL_TOOLS and isinstance(arguments, dict):
        command = arguments.get("command") or arguments.get("text") or ""
        if command and has_blocked_command_pattern(command):
            ast_file = find_ast_files_in_command(command)
            if ast_file:
                return {
                    "hookSpecificOutput": {
                        "permissionDecision": "deny",
                        "permissionDecisionReason": (
                            f"Terminal text-replace command blocked for {ast_file}. "
                            f"GnawTreeWriter supports this file type. "
                            f"Use mcp_gnawtreewrite_edit_node instead."
                        ),
                    },
                    "systemMessage": (
                        f"CRITICAL: You attempted to use a terminal text-replace "
                        f"command ({command[:80]}...) on {ast_file}. "
                        f"This was BLOCKED. "
                        f"Use mcp_gnawtreewrite_edit_node or "
                        f"mcp_gnawtreewrite_semantic_edit instead."
                    ),
                }
        # Not a blocked shell command — allow
        return {"hookSpecificOutput": {"permissionDecision": "allow"}}

    # ── Block VSCode text-replace tools ──────────────────────────────
    if tool_name not in BLOCKED_TOOLS:
        return {"hookSpecificOutput": {"permissionDecision": "allow"}}

    # Extract file path from arguments
    file_path = None
    if isinstance(arguments, dict):
        file_path = arguments.get("filePath") or arguments.get("file_path")

    # If no file path, try to find it elsewhere
    if not file_path:
        file_path = get_file_path(input_data)

    # If we can't determine the file, allow (don't block unknown usage)
    if not file_path or not is_ast_supported(file_path):
        return {"hookSpecificOutput": {"permissionDecision": "allow"}}

    suggestion = TOOL_SUGGESTIONS.get(tool_name, "mcp_gnawtreewrite_* tools")

    return {
        "hookSpecificOutput": {
            "permissionDecision": "deny",
            "permissionDecisionReason": (
                f"{tool_name} is blocked for {file_path} because GnawTreeWriter "
                f"supports this file type. Use {suggestion} instead for "
                f"syntax-validated, surgical edits."
            ),
        },
        "systemMessage": (
            f"CRITICAL: You attempted to use {tool_name} on a file type that "
            f"GnawTreeWriter supports. This operation was BLOCKED. "
            f"You MUST use GnawTreeWriter MCP tools "
            f"(prefixed mcp_gnawtreewrite_) for code modifications in "
            f"AST-supported files. "
            f"Valid alternatives: {suggestion}. "
            f"Use mcp_gnawtreewrite_list_nodes first to find the correct node path."
        ),
    }


def main():
    try:
        raw_input = sys.stdin.read()
        if not raw_input.strip():
            # No input — this is a probe/initialization, allow
            print(json.dumps({"hookSpecificOutput": {"permissionDecision": "allow"}}))
            sys.exit(0)

        input_data = json.loads(raw_input)

        # Determine event type
        event_type = input_data.get("type") or input_data.get("event") or "PreToolUse"

        if event_type == "PreToolUse" or "toolName" in input_data or "tool" in input_data:
            result = handle_pre_tool_use(input_data)
        else:
            result = {"hookSpecificOutput": {"permissionDecision": "allow"}}

        print(json.dumps(result))
        sys.exit(0)

    except json.JSONDecodeError:
        # Invalid JSON — don't block, just warn via stderr
        print(json.dumps({"hookSpecificOutput": {"permissionDecision": "allow"}}))
        sys.exit(0)
    except Exception as e:
        # Unexpected error — don't block, log to stderr
        print(f"Hook error: {e}", file=sys.stderr)
        print(json.dumps({"hookSpecificOutput": {"permissionDecision": "allow"}}))
        sys.exit(0)


if __name__ == "__main__":
    main()
