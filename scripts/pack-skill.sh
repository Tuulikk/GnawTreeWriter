#!/bin/bash
# Skill Packer — one source of truth for the agent skill.
#
#   SKILL.md (repo root)                    <- CANONICAL, edit only this
#     ├── .agents/skills/gnawtreewriter/SKILL.md   symlink (Codex reads it there)
#     └── gnawtreewriter-crush.skill               zip package (Crush needs a copy)
#
# The zip cannot be a symlink, so it is GENERATED here — and this script's
# --check mode makes drift LOUD instead of silent (a stale package once shipped
# v0.9.2 instructions to Crush while the repo was at 0.25.x).
#
# Usage:
#   scripts/pack-skill.sh           # regenerate gnawtreewriter-crush.skill
#   scripts/pack-skill.sh --check   # exit 1 if the package or symlink is stale
set -euo pipefail

cd "$(dirname "$0")/.."

CANON="SKILL.md"
PKG="gnawtreewriter-crush.skill"
LINK=".agents/skills/gnawtreewriter/SKILL.md"

[ -f "$CANON" ] || { echo "ERROR: missing $CANON" >&2; exit 1; }

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

if [ "${1:-}" = "--check" ]; then
    status=0

    # 1. The .skill package must carry exactly the canonical content.
    if [ ! -f "$PKG" ]; then
        echo "STALE: $PKG is missing — run scripts/pack-skill.sh" >&2
        status=1
    elif ! unzip -p "$PKG" SKILL.md > "$tmp/inside.md" 2>/dev/null; then
        echo "STALE: $PKG contains no SKILL.md — run scripts/pack-skill.sh" >&2
        status=1
    elif ! cmp -s "$CANON" "$tmp/inside.md"; then
        echo "STALE: $PKG differs from $CANON (package: $(wc -l < "$tmp/inside.md") lines, source: $(wc -l < "$CANON") lines)" >&2
        echo "       run scripts/pack-skill.sh" >&2
        status=1
    else
        echo "OK: $PKG matches $CANON"
    fi

    # 2. The Codex path must resolve to the canonical file (catches a copy
    #    that replaced the symlink — the exact drift this script exists for).
    if [ ! -L "$LINK" ]; then
        echo "STALE: $LINK is not a symlink — run scripts/pack-skill.sh" >&2
        status=1
    elif [ ! -f "$LINK" ]; then
        echo "STALE: $LINK is a dangling symlink (-> $(readlink "$LINK"))" >&2
        status=1
    elif ! cmp -s "$CANON" "$LINK"; then
        echo "STALE: $LINK does not resolve to $CANON" >&2
        status=1
    else
        echo "OK: $LINK -> $CANON"
    fi

    exit $status
fi

# Regenerate the package from the canonical skill.
cp "$CANON" "$tmp/SKILL.md"
rm -f "$PKG"
( cd "$tmp" && zip -q -X "$PKG" SKILL.md )
mv "$tmp/$PKG" "$PKG"
echo "wrote $PKG from $CANON ($(wc -l < "$CANON") lines)"

# Keep the Codex path in lockstep.
if [ ! -L "$LINK" ]; then
    mkdir -p "$(dirname "$LINK")"
    rm -f "$LINK"
    ln -s ../../../SKILL.md "$LINK"
    echo "linked $LINK -> ../../../SKILL.md"
fi
