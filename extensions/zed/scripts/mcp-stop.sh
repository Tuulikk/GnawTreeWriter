#!/usr/bin/env bash
# Stop a running gnawtreewriter MCP server started by scripts/mcp-serve.sh
#
# Usage:
#   ./scripts/mcp-stop.sh
#   ./scripts/mcp-stop.sh --pid .mcp-server.pid
#   ./scripts/mcp-stop.sh --pid /tmp/foo.pid --log /tmp/mcp.log --force
#
# By default it reads PID from .mcp-server.pid and shows the tail of .mcp-server.log.
set -euo pipefail
IFS=$'\n\t'

PIDFILE=".mcp-server.pid"
LOGFILE=".mcp-server.log"
FORCE=false
TAIL=true
TIMEOUT=10

usage() {
  cat <<EOF
Usage: $0 [options]

Options:
  --pid <file>       PID file to read (default: ${PIDFILE})
  --log <file>       Log file to tail after shutdown (default: ${LOGFILE})
  --force            If running process doesn't look like gnawtreewriter, kill it anyway
  --no-tail          Do not print log tail after stopping
  --timeout <secs>   Seconds to wait for graceful shutdown (default: ${TIMEOUT})
  -h, --help         Show this help
EOF
}

# Parse args
while [[ $# -gt 0 ]]; do
  case "$1" in
    --pid)
      PIDFILE="$2"; shift 2 ;;
    --log)
      LOGFILE="$2"; shift 2 ;;
    --force)
      FORCE=true; shift ;;
    --no-tail)
      TAIL=false; shift ;;
    --timeout)
      TIMEOUT="$2"; shift 2 ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ ! -f "$PIDFILE" ]]; then
  echo "PID file not found: $PIDFILE" >&2
  exit 1
fi

pid=$(cat "$PIDFILE" 2>/dev/null || true)
if [[ -z "$pid" ]]; then
  echo "PID file is empty or unreadable: $PIDFILE" >&2
  rm -f "$PIDFILE"
  exit 1
fi

# Validate PID is numeric
if ! printf '%s' "$pid" | grep -qE '^[0-9]+$'; then
  echo "PID file does not contain a numeric PID: $PIDFILE" >&2
  echo "Contents: $(cat "$PIDFILE")" >&2
  exit 1
fi

# Check if process exists
if ! kill -0 "$pid" 2>/dev/null; then
  echo "No process with PID $pid appears to be running. Removing stale PID file."
  rm -f "$PIDFILE"
  exit 0
fi

# Try to inspect the running process command (best-effort)
proc_cmd="$(ps -p "$pid" -o comm= 2>/dev/null || true)"
if [[ -n "$proc_cmd" ]]; then
  echo "Found running process PID $pid (${proc_cmd})"
else
  echo "Found running process PID $pid (command unknown)"
fi

# If the process doesn't look like `gnawtreewriter`, require --force
if [[ "$FORCE" != "true" ]]; then
  if [[ -n "$proc_cmd" ]] && [[ "$proc_cmd" != *gnawtreewriter* ]] && [[ "$proc_cmd" != *cargo* ]]; then
    echo "Warning: process command '$proc_cmd' does not look like gnawtreewriter." >&2
    echo "Use --force to proceed anyway." >&2
    exit 3
  fi
fi

echo "Stopping process $pid ..."
# Send SIGTERM
kill "$pid" 2>/dev/null || true

# Wait for graceful shutdown
count=0
while kill -0 "$pid" 2>/dev/null; do
  if [[ "$count" -ge "$TIMEOUT" ]]; then
    echo "Process $pid did not stop within ${TIMEOUT}s."
    if [[ "$FORCE" == "true" ]]; then
      echo "Sending SIGKILL to $pid (force mode)."
      kill -9 "$pid" 2>/dev/null || true
      # small wait
      sleep 1
      if kill -0 "$pid" 2>/dev/null; then
        echo "Process $pid still running after SIGKILL; manual intervention required." >&2
        exit 4
      else
        break
      fi
    else
      echo "Use --force to send SIGKILL." >&2
      exit 4
    fi
  fi
  sleep 1
  count=$((count + 1))
done

# Clean up pidfile
if [[ -f "$PIDFILE" ]]; then
  rm -f "$PIDFILE" || true
fi

echo "Stopped process $pid."

if [[ "$TAIL" == "true" ]] && [[ -f "$LOGFILE" ]]; then
  echo
  echo "=== Last lines of ${LOGFILE} ==="
  tail -n 40 "$LOGFILE" || true
fi

exit 0
