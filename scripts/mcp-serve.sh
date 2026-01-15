#!/usr/bin/env bash
# Start the gnawtreewriter MCP server in a background-friendly way.
# Usage:
#   ./scripts/mcp-serve.sh [--addr ADDR] [--token TOKEN] [--pid PIDFILE] [--log LOGFILE] [--foreground]
#
# Examples:
#   # start in background on default 127.0.0.1:8080
#   ./scripts/mcp-serve.sh
#
#   # start in foreground (useful for debugging)
#   ./scripts/mcp-serve.sh --addr 127.0.0.1:9000 --foreground
#
#   # use ephemeral port (0) and let the script discover the bound port in logs
#   ./scripts/mcp-serve.sh --addr 127.0.0.1:0 --log /tmp/mcp.log
set -euo pipefail

# Diagnostic logging for Zed dev extensions:
# This records invocation attempts (timestamp, cwd, user, argv and a masked token preview)
# into a file named `.mcp-serve-invocations.log` in the extension working directory.
# To disable diagnostic logging, set DIAGFILE="" in the environment when invoking the script.
DIAGFILE="${DIAGFILE:-.mcp-serve-invocations.log}"
if [ -n "$DIAGFILE" ]; then
  {
    printf '---\n'
    printf 'timestamp: %s\n' "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    printf 'cwd: %s\n' "$(pwd -P)"
    printf 'user: %s\n' "$(id -un 2>/dev/null || echo unknown)"
    printf 'uid: %s\n' "$(id -u 2>/dev/null || echo unknown)"
    printf 'pid: %s\n' "$$"
    printf 'argv:'
    for a in "$@"; do printf ' %q' "$a"; done
    printf '\n'
    if [ -n "${MCP_TOKEN:-}" ]; then
      # Mask token for privacy
      token_preview="${MCP_TOKEN:0:4}****"
      printf 'env.MCP_TOKEN: %s\n' "$token_preview"
    else
      printf 'env.MCP_TOKEN: <not set>\n'
    fi
    printf 'script_exists: %s\n' "[ -f ./scripts/mcp-serve.sh ]"
    printf '---\n'
  } >>"$DIAGFILE" 2>/dev/null || true
  # Ensure file permissions are restricted for the user
  chmod 600 "$DIAGFILE" 2>/dev/null || true
fi

# Defaults
ADDR="127.0.0.1:8080"
TOKEN="secret"
PIDFILE=".mcp-server.pid"
LOGFILE=".mcp-server.log"
FOREGROUND=false
TIMEOUT=30

usage() {
  cat <<EOF
Usage: $0 [--addr ADDR] [--token TOKEN] [--pid PIDFILE] [--log LOGFILE] [--foreground] [--help]

Starts the gnawtreewriter MCP server. By default it runs in background and writes:
  PID -> $PIDFILE
  Log -> $LOGFILE

Options:
  --addr <host:port>    Address to bind (default ${ADDR}). Use :0 for ephemeral port.
  --token <token>       Bearer token used for authentication (default ${TOKEN}).
  --pid <file>          PID file path (default ${PIDFILE}).
  --log <file>          Log file path (default ${LOGFILE}).
  --foreground, --fg    Run server in foreground (no PID/log handling).
  -h, --help            Show this help.
EOF
}

# Parse args
# Normalize concatenated/compact args so we accept:
#   --addr127.0.0.1:8080   or  --addr=127.0.0.1:8080
#   --tokensecret          or  --token=secret
#   --pid/path             or  --pid=/some/path
#
# This lets users paste arguments without whitespace and still have them parsed
# correctly. We keep order and leave already-correct args intact.
normalized_args=()
for arg in "$@"; do
  if [[ "$arg" =~ ^--addr=(.+)$ ]]; then
    normalized_args+=(--addr "${BASH_REMATCH[1]}")
  elif [[ "$arg" =~ ^--addr(.+)$ ]]; then
    normalized_args+=(--addr "${BASH_REMATCH[1]}")
  elif [[ "$arg" =~ ^--token=(.+)$ ]]; then
    normalized_args+=(--token "${BASH_REMATCH[1]}")
  elif [[ "$arg" =~ ^--token(.+)$ ]]; then
    normalized_args+=(--token "${BASH_REMATCH[1]}")
  elif [[ "$arg" =~ ^--pid=(.+)$ ]]; then
    normalized_args+=(--pid "${BASH_REMATCH[1]}")
  elif [[ "$arg" =~ ^--pid(.+)$ ]]; then
    normalized_args+=(--pid "${BASH_REMATCH[1]}")
  elif [[ "$arg" =~ ^--log=(.+)$ ]]; then
    normalized_args+=(--log "${BASH_REMATCH[1]}")
  elif [[ "$arg" =~ ^--log(.+)$ ]]; then
    normalized_args+=(--log "${BASH_REMATCH[1]}")
  elif [[ "$arg" =~ ^--foreground=(true|false)$ ]]; then
    # honor --foreground=true/false
    if [[ "${BASH_REMATCH[1]}" == "true" ]]; then
      normalized_args+=(--foreground)
    fi
  else
    normalized_args+=("$arg")
  fi
done
# Replace positional args with normalized list for the existing parser
set -- "${normalized_args[@]}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --addr) ADDR="$2"; shift 2 ;;
    --token) TOKEN="$2"; shift 2 ;;
    --pid) PIDFILE="$2"; shift 2 ;;
    --log) LOGFILE="$2"; shift 2 ;;
    --foreground|--fg) FOREGROUND=true; shift ;;
    -h|--help) usage; exit 0 ;;
    *)
      echo "Unknown argument: $1"
      usage
      exit 1
      ;;
  esac
done

# Prevent starting if a running server PID is already present
if [[ -f "$PIDFILE" ]]; then
  oldpid=$(cat "$PIDFILE" || true)
  if [[ -n "$oldpid" ]] && kill -0 "$oldpid" 2>/dev/null; then
    echo "A server is already running (pid=$oldpid). Stop it first or remove $PIDFILE."
    exit 1
  else
    # stale pidfile - remove it
    rm -f "$PIDFILE"
  fi
fi

# Pick executable: prefer installed binary, otherwise fallback to cargo run
if command -v gnawtreewriter >/dev/null 2>&1; then
  CMD=(gnawtreewriter mcp serve --addr "$ADDR" --token "$TOKEN")
else
  CMD=(cargo run --features mcp -- mcp serve --addr "$ADDR" --token "$TOKEN")
fi

echo "Starting gnawtreewriter MCP server on http://$ADDR (token='$TOKEN')"

if [[ "$FOREGROUND" == "true" ]]; then
  # Run in foreground (logs to terminal)
  "${CMD[@]}"
  exit $?
fi

# Start server in background and write log + pidfile
"${CMD[@]}" >"$LOGFILE" 2>&1 &
PID=$!
echo "$PID" > "$PIDFILE"
echo "Server started (pid=$PID). Logs: $LOGFILE"

# Determine server URL:
# - If user passed :0, parse the bound address from log output.
SERVER_URL=""
if [[ "$ADDR" == *":0" ]]; then
  echo "Waiting for server to advertise bound address in logs..."
  for i in $(seq 1 $TIMEOUT); do
    if grep -q 'Starting MCP server on http' "$LOGFILE" 2>/dev/null; then
      SERVER_URL=$(grep -m1 'Starting MCP server on http' "$LOGFILE" | sed -E 's/.*(http:\/\/[^[:space:]]+).*/\1/')
      break
    fi
    sleep 0.5
  done
  if [[ -z "$SERVER_URL" ]]; then
    echo "Could not determine server URL from $LOGFILE (timed out)."
  fi
else
  SERVER_URL="http://${ADDR}/"
fi

# Wait for server readiness (initialize)
if [[ -n "$SERVER_URL" ]]; then
  echo "Waiting for server readiness at $SERVER_URL (timeout ${TIMEOUT}s)..."
  for i in $(seq 1 $TIMEOUT); do
    if curl -s --fail -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
         -d '{"jsonrpc":"2.0","method":"initialize","id":1}' "$SERVER_URL" >/dev/null 2>&1; then
      echo "Server is ready at $SERVER_URL"
      echo "PID: $PID (stored in $PIDFILE)"
      echo "To stop: ./scripts/mcp-stop.sh (or kill $PID)"
      exit 0
    fi
    sleep 0.5
  done
  echo "Server did not become ready within ${TIMEOUT}s; check $LOGFILE for details." >&2
  exit 1
else
  echo "Server started (pid=$PID) but URL is unknown (addr was :0 and no log line found)."
  echo "Check $LOGFILE to find the bound URL."
  exit 0
fi
