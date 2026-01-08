#!/usr/bin/env bash
#
# Helper script to run the Rust MCP client example (list / init / analyze / call)
#
# Usage:
#   ./scripts/mcp-client.sh [--url URL] [--token TOKEN] [--release] <command> [args...]
#
# Examples:
#   ./scripts/mcp-client.sh list
#   ./scripts/mcp-client.sh init
#   ./scripts/mcp-client.sh analyze examples/example.rs
#   ./scripts/mcp-client.sh call analyze '{"file_path":"examples/example.rs"}'
#
# Environment:
#   MCP_URL   - default URL if --url not provided (default: http://127.0.0.1:8080/)
#   MCP_TOKEN - default token if --token not provided (default: secret)
#
set -euo pipefail
IFS=$'\n\t'

URL="${MCP_URL:-http://127.0.0.1:8080/}"
TOKEN="${MCP_TOKEN:-secret}"
RELEASE=false

usage() {
  cat <<EOF
Usage: $0 [--url URL] [--token TOKEN] [--release] <command> [args...]

Commands:
  list                    - List available tools
  init                    - Initialize (handshake)
  analyze <file>          - Analyze a file and print summary
  call <tool> [json_args] - Generic tool call, json_args is a JSON string (defaults to {})
  help                    - Show this help

Notes:
  - You can set MCP_URL and MCP_TOKEN environment variables instead of passing flags.
  - Use --release to run with '--release' (faster runtime, but requires a release build).
EOF
}

# Parse global flags
while [[ $# -gt 0 ]]; do
  case "$1" in
    --url)
      URL="$2"; shift 2 ;;
    --token)
      TOKEN="$2"; shift 2 ;;
    --release)
      RELEASE=true; shift ;;
    -h|--help)
      usage; exit 0 ;;
    --)
      shift; break ;;
    -*)
      echo "Unknown option: $1" >&2; usage; exit 1 ;;
    *)
      break ;;
  esac
done

if [[ $# -lt 1 ]]; then
  usage
  exit 1
fi

COMMAND="$1"; shift

# Helper: run the example with the provided args
run_example() {
  local args=()
  if [[ "$RELEASE" == "true" ]]; then
    args+=("--release")
  fi
  args+=("--features" "mcp" "--example" "mcp_client" "--" )
  # append URL & token first, then rest
  args+=("--url" "$URL" "--token" "$TOKEN")
  args+=("$@")

  # Execute
  echo "+ cargo run ${args[*]}"
  cargo run "${args[@]}"
}

case "$COMMAND" in
  list)
    run_example list
    ;;

  init)
    run_example init
    ;;

  analyze)
    if [[ $# -lt 1 ]]; then
      echo "Usage: $0 analyze <file_path>" >&2
      exit 1
    fi
    FILE="$1"
    run_example analyze "$FILE"
    ;;

  call)
    if [[ $# -lt 1 ]]; then
      echo "Usage: $0 call <tool_name> [json_args]" >&2
      exit 1
    fi
    TOOL="$1"
    shift
    if [[ $# -ge 1 ]]; then
      JSON_ARGS="$1"
    else
      JSON_ARGS="{}"
    fi
    run_example call "$TOOL" "$JSON_ARGS"
    ;;

  help|-h|--help)
    usage
    ;;

  *)
    echo "Unknown command: $COMMAND" >&2
    usage
    exit 1
    ;;
esac
