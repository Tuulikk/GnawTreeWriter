#!/bin/bash
# MCP daemon hygiene — list and reap stale gnawtreewriter MCP daemons.
#
# WHY: stdio MCP daemons are spawned per client and keep running the binary
# they started with. After `cargo install`, `readlink /proc/<pid>/exe` shows
# "(deleted)" — the process lives on old code, so a fix "does not take effect"
# until that process is replaced (AGENTS.md lesson 4). Field case 2026-11-10:
# eight daemons aged 95-285 minutes, all on a deleted binary.
#
# Usage:
#   scripts/daemon-hygiene.sh                       # list only
#   scripts/daemon-hygiene.sh reap                  # DRY RUN: what would die
#   scripts/daemon-hygiene.sh reap --yes            # kill stale ones
#   scripts/daemon-hygiene.sh reap --older-than 60 --yes   # only idlers
#
# WARNING: the invoking client's own daemon looks exactly like the others.
# Reaping it disconnects this session's MCP until the client reconnects
# (observed 2026-11-10: "MCP server is not connected" and it stayed that way).
# Use --older-than, or reap between sessions.
set -uo pipefail

PATTERN='gnawtreewriter mcp'
MODE="${1:-list}"
shift || true
DRY_RUN=1
MIN_AGE=0
while [ $# -gt 0 ]; do
    case "$1" in
        --yes) DRY_RUN=0 ;;
        --dry-run) DRY_RUN=1 ;;
        --older-than) MIN_AGE="${2:-0}"; shift ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
    shift
done

if [ ! -d /proc/self ]; then
    echo "ERROR: this script needs /proc (Linux). On macOS use 'pgrep -fl \"$PATTERN\"'." >&2
    exit 2
fi

now=$(date +%s)
pids=$(pgrep -f "$PATTERN" 2>/dev/null | grep -vx "$$")
[ -n "$pids" ] || { echo "No gnawtreewriter MCP daemons running."; exit 0; }

printf '%-9s %-7s %-46s %-10s %s\n' PID AGE CWD BINARY RSS
stale=""
count=0
for p in $pids; do
    [ -d "/proc/$p" ] || continue
    cmd=$(tr '\0' ' ' < "/proc/$p/cmdline" 2>/dev/null) || continue
    case "$cmd" in *gnawtreewriter*) ;; *) continue ;; esac

    cwd=$(readlink "/proc/$p/cwd" 2>/dev/null || echo '(gone)')
    exe=$(readlink "/proc/$p/exe" 2>/dev/null || echo '(gone)')
    start=$(stat -c %Y "/proc/$p" 2>/dev/null || echo "$now")
    age=$(( (now - start) / 60 ))
    rss=$(awk '{printf "%dM", $2*4/1024}' "/proc/$p/statm" 2>/dev/null || echo '?')
    case "$exe" in
        *'(deleted)'*) state='(deleted)' ;;
        *) state='on-disk' ;;
    esac
    printf '%-9s %-7s %-46s %-10s %s\n' "$p" "${age}m" "$cwd" "$state" "$rss"

    if [ "$state" = '(deleted)' ] && [ "$age" -ge "$MIN_AGE" ]; then
        stale="$stale $p"
        count=$((count + 1))
    fi
done

echo
if [ "$count" -eq 0 ]; then
    echo "Nothing stale${MIN_AGE:+ (age filter: ${MIN_AGE}m)}."
    exit 0
fi

if [ "$DRY_RUN" -eq 1 ]; then
    echo "DRY RUN — would reap $count stale daemon(s):$stale"
    echo "Re-run with --yes to kill them. Expect this session's MCP to drop if its daemon is among them."
    exit 0
fi

echo "Reaping $count stale daemon(s):$stale"
for p in $stale; do
    kill "$p" 2>/dev/null && echo "  killed $p"
done
sleep 1
left=$(for p in $stale; do [ -d "/proc/$p" ] && echo "$p"; done | wc -w)
[ "$left" -eq 0 ] && echo "All reaped." || echo "WARNING: $left still alive — try: kill -9$stale"
