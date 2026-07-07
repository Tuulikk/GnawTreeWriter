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
AST_EXTENSIONS = {
    ".py", ".rs", ".c", ".h", ".cpp", ".hpp", ".cc", ".cxx", ".hxx",
    ".h++", ".java", ".zig", ".ts", ".tsx", ".js", ".jsx", ".sh",
    ".bash", ".php", ".html", ".qml", ".go", ".css", ".yaml", ".yml",
    ".toml", ".xml", ".md", ".markdown",
}

# Tools to block (text-replace tools that GnawTreeWriter should replace)
BLOCKED_TOOLS = {
    "replace_string_in_file",
    "insert_edit_into_file",
}

# Mapping from blocked tool to suggested GnawTreeWriter alternative
TOOL_SUGGESTIONS = {
    "replace_string_in_file": "mcp_gnawtreewrite_edit_node or mcp_gnawtreewrite_semantic_edit",
    "insert_edit_into_file": "mcp_gnawtreewrite_insert_node or mcp_gnawtreewrite_semantic_insert",
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

    # Only block text-replace tools
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
