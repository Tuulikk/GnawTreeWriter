#!/usr/bin/env bash
# Test orchestration script for MCP:
# - starts a gnawtreewriter MCP server
# - waits for readiness
# - runs client checks (list, init, analyze)
# - stops the server unless --keep is specified
#
# Usage:
#   ./scripts/test-mcp.sh [--addr HOST:PORT] [--token TOKEN] [--keep] [--timeout SECONDS]
#
# Examples:
#   ./scripts/test-mcp.sh
#   ./scripts/test-mcp.sh --addr 127.0.0.1:8081 --token secret
#   ./scripts/test-mcp.sh --keep
#
set -euo pipefail
IFS=$'\n\t'

# Helpers
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root_dir="$(cd "$script_dir/.." && pwd)"

usage() {
  cat <<EOF
Usage: $0 [options]

Options:
  --addr HOST:PORT   Address to bind server to (default: 127.0.0.1:8080)
  --token TOKEN      Bearer token used by server & client (default: testtoken)
  --keep             Keep server running after test (do not stop it)
  --timeout SECS     Timeout in seconds for server readiness (default: 30)
  -h,--help          Show this help
EOF
}

# Defaults
ADDR="127.0.0.1:8080"
TOKEN="testtoken"
KEEP=false
TIMEOUT=30

# Parse arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    --addr) ADDR="$2"; shift 2 ;;
    --token) TOKEN="$2"; shift 2 ;;
    --keep) KEEP=true; shift ;;
    --timeout) TIMEOUT="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
  esac
done

# Workdir and files
WORKDIR="$(mktemp -d -t gnaw-mcp-XXXX)"
PIDFILE="$WORKDIR/server.pid"
LOGFILE="$WORKDIR/server.log"
TESTFILE="$WORKDIR/test.py"

cleanup() {
  rc=$?
  if [[ $rc -ne 0 ]]; then
    echo "=== Test failed (exit code $rc) — server log (tail) ==="
    if [[ -f "$LOGFILE" ]]; then
      tail -n 200 "$LOGFILE" || true
    else
      echo "(no log file: $LOGFILE)"
    fi
  fi

  if [[ "$KEEP" == "false" ]]; then
    if [[ -f "$PIDFILE" ]]; then
      echo "Stopping server..."
      "$root_dir/scripts/mcp-stop.sh" --pid "$PIDFILE" --no-tail --timeout 5 || true
    fi
  else
    echo "Keeping server running. PID file: $PIDFILE, log: $LOGFILE"
  fi

  # Don't remove log if KEEP requested (for debugging)
  if [[ "$KEEP" == "false" ]]; then
    rm -rf "$WORKDIR" || true
  fi

  exit $rc
}
trap cleanup EXIT

# Ensure helper scripts are available
if [[ ! -x "$root_dir/scripts/mcp-serve.sh" ]]; then
  echo "Missing or non-executable helper: $root_dir/scripts/mcp-serve.sh" >&2
  exit 2
fi
if [[ ! -x "$root_dir/scripts/mcp-stop.sh" ]]; then
  echo "Missing or non-executable helper: $root_dir/scripts/mcp-stop.sh" >&2
  exit 2
fi
if [[ ! -x "$root_dir/scripts/mcp-client.sh" ]]; then
  echo "Missing or non-executable helper: $root_dir/scripts/mcp-client.sh" >&2
  exit 2
fi

echo "Temporary workdir: $WORKDIR"
echo "Server PID file: $PIDFILE"
echo "Server log file: $LOGFILE"
echo "Server addr: $ADDR"
echo "Token: $TOKEN"
echo

# Create a tiny test file
echo "def foo(): return 42" > "$TESTFILE"
echo "Created test file: $TESTFILE"
echo

# Start server (the serve script will background the server and wait for readiness)
echo "Starting server..."
"$root_dir/scripts/mcp-serve.sh" --addr "$ADDR" --token "$TOKEN" --pid "$PIDFILE" --log "$LOGFILE" --timeout "$TIMEOUT"

# Determine effective server URL:
SERVER_URL=""
if [[ "$ADDR" == *":0" ]]; then
  # Parse actual bound URL from log
  for i in $(seq 1 "$TIMEOUT"); do
    if grep -q 'Starting MCP server on http' "$LOGFILE" 2>/dev/null; then
      SERVER_URL=$(grep -m1 'Starting MCP server on http' "$LOGFILE" | sed -E 's/.*(http:\/\/[^[:space:]]+).*/\1/')
      break
    fi
    sleep 0.5
  done
  if [[ -z "$SERVER_URL" ]]; then
    echo "Failed to discover server URL from log $LOGFILE" >&2
    exit 3
  fi
else
  # Ensure trailing slash
  if [[ "$ADDR" == */ ]]; then
    SERVER_URL="http://$ADDR"
  else
    SERVER_URL="http://$ADDR/"
  fi
fi

echo "Using server URL: $SERVER_URL"
echo

# Wrapper to run client command and fail fast
run_client() {
  echo "=> Client: $*"
  if ! "$root_dir/scripts/mcp-client.sh" --url "$SERVER_URL" --token "$TOKEN" "$@"; then
    echo "Client command failed: $*" >&2
    return 1
  fi
  return 0
}

# Run checks
echo "Running client: list"
run_client list

echo
echo "Running client: init"
run_client init

echo
echo "Running client: analyze (test file)"
run_client analyze "$TESTFILE"

echo
echo "All MCP checks passed successfully."

# If we reach here, script will exit 0 and cleanup() will stop the server unless --keep
exit 0
