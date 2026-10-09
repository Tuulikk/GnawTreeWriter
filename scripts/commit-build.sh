#!/usr/bin/env bash
# commit-build.sh — GTW:s säker-kedja: commit → test → build → identitets-facit
#
# Samma §28.1-ordning som Motor2/gnawdriver: commit FÖRE bygget (binärer
# bäddar in bygg-tids-state; fel ordning = artefakt som ljuger om sitt ursprung).
#
# GTW:s två konsumtionsvägar (skriptet påminner om båda):
#   1. PATH-DEP: Motor2 bygger GTW:s KÄLLKOD in i sig — Motor2:s
#      commit-build-deploy.sh plockar upp ändringar vid nästa Motor2-bygge.
#   2. MCP-BINÄR: opencode-sessioner laddar gnawtreewriter-binären vid
#      session-start (binärfäst per session) — omstart krävs för nya grepp.
#
# Användning:
#   scripts/commit-build.sh "<commit-msg>"     # commit → test → build → facit
#   scripts/commit-build.sh --skip-commit      # test + build av nuvarande HEAD
#
# Facit (§28.1): exit-kod = testens rc, sedan cargo builds rc. GTW har ingen
# inbäddad git-sha i binären (ingen build.rs ännu) — identitetskontroll är
# därför rev+mtime, ALDRIG påstådd sha-jämförelse.
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

MSG="${1:-}"
SKIP_COMMIT=false
[[ "${1:-}" == "--skip-commit" ]] && SKIP_COMMIT=true

# 1. Commit FÖRE test/bygg (§28.1).
if $SKIP_COMMIT; then
    if [[ -n "$(git status --porcelain)" ]]; then
        echo "FEL: --skip-commit men trädet har ocommittade ändringar." >&2
        exit 1
    fi
else
    if [[ -z "$MSG" ]]; then
        echo "Användning: $0 \"<commit-msg>\" | $0 --skip-commit" >&2
        exit 1
    fi
    git add -A
    if ! git commit -m "$MSG"; then
        echo "FEL: commit misslyckades (rc=$?)." >&2
        exit 1
    fi
fi

REV="$(git rev-parse --short HEAD)"
echo "[commit-build] HEAD = $REV"

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/.cargo/builds}"

# 2. Tester (GTW:s enda gate — ingen deploy-grind finns).
cargo test --release
rc=$?
if [[ $rc -ne 0 ]]; then
    echo "FEL: cargo test misslyckades (RC=$rc) — bygges avbruten." >&2
    exit $rc
fi

# 3. Bygg release.
cargo build --release
rc=$?
if [[ $rc -ne 0 ]]; then
    echo "FEL: cargo build --release misslyckades (RC=$rc)." >&2
    exit $rc
fi

BIN="$CARGO_TARGET_DIR/release/gnawtreewriter"
if [[ ! -x "$BIN" ]]; then
    echo "FEL: binär saknas: $BIN" >&2
    exit 1
fi

echo "[commit-build] KLART: rev $REV → $BIN (mtime $(date -r "$BIN" '+%F %T'))"
echo "[commit-build] OBS: (1) Motor2 plockar upp GTW-källkoden vid nästa Motor2-bygge (path-dep)."
echo "[commit-build] OBS: (2) opencode-sessioner laddar MCP-binären vid start — omstart krävs för nya grepp."
