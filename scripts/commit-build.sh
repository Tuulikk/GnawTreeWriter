#!/usr/bin/env bash
# commit-build.sh — GTW:s säker-kedja: commit → test → build → identitets-facit
#
# Samma §28.1-ordning som Motor2/gnawdriver: commit FÖRE bygget (binärer
# bäddar in bygg-tids-state; fel ordning = artefakt som ljuger om sitt ursprung).
#
# GTW:s två konsumtionsvägar (skriptet påminner om båda):
#   1. PATH-DEP: Motor2 bygger GTW:s KÄLLKOD in i sig — Motor2:s
#      commit-build-deploy.sh plockar upp ändringar vid nästa Motor2-bygge.
#   2. MCP-BINÄR: klienten (Cline/opencode) startar `gnawtreewriter` från
#      ~/.cargo/bin vid spawn och behåller den bilden — därför installerar
#      steg 4, och därför måste daemonen bytas ut för att nya grepp ska nå den.
#
# Användning:
#   scripts/commit-build.sh "<commit-msg>"     # commit → test → build → install
#   scripts/commit-build.sh --skip-commit      # test + build av nuvarande HEAD
#
# Facit (§28.1): exit-kod = testens rc, sedan cargo builds rc. Sedan 0.25.x
# bäddar build.rs in git-sha + byggtid (`GTW_GIT_SHA`), och `doctor` jämför den
# mot HEAD för att svara "kör jag den här källkoden?" (`src/version.rs`).
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

MSG="${1:-}"
SKIP_COMMIT=false
[[ "${1:-}" == "--skip-commit" ]] && SKIP_COMMIT=true

# 1. Commit FÖRE test/bygg (§28.1).
if $SKIP_COMMIT; then
    # code-index.md är en GENERERAD artefakt: hooken skriver om den vid varje
    # commit men stage:ar den aldrig (P4), så den är alltid "dirty" direkt efter
    # en commit. Räknas den som ocommittat arbete blir --skip-commit oanvändbar
    # för agenter — som committar själva och sedan bara vill bygga.
    DIRTY="$(git status --porcelain | grep -vE '^( M|\?\?) code-index\.md$' || true)"
    if [[ -n "$DIRTY" ]]; then
        echo "FEL: --skip-commit men trädet har ocommittade ändringar:" >&2
        echo "$DIRTY" >&2
        echo "[commit-build] RESULT fail:dirty" >&2
        exit 1
    fi
else
    if [[ -z "$MSG" ]]; then
        echo "Användning: $0 \"<commit-msg>\" | $0 --skip-commit" >&2
        echo "[commit-build] RESULT fail:usage" >&2
        exit 1
    fi
    git add -A
    if ! git commit -m "$MSG"; then
        echo "FEL: commit misslyckades (rc=$?)." >&2
        echo "[commit-build] RESULT fail:commit" >&2
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
    echo "[commit-build] RESULT fail:test" >&2
    exit $rc
fi

# 3. Bygg release.
cargo build --release
rc=$?
if [[ $rc -ne 0 ]]; then
    echo "FEL: cargo build --release misslyckades (RC=$rc)." >&2
    echo "[commit-build] RESULT fail:build" >&2
    exit $rc
fi

BIN="$CARGO_TARGET_DIR/release/gnawtreewriter"
if [[ ! -x "$BIN" ]]; then
    echo "FEL: binär saknas: $BIN" >&2
    echo "[commit-build] RESULT fail:bin" >&2
    exit 1
fi

# 4. Installera — ENDA anledningen: MCP-klienten startar
#    ~/.cargo/bin/gnawtreewriter, inte target-katalogen. Utan detta steg
#    bygger vi en binär som ingen klient någonsin startar (uppmätt 2026-11-10:
#    en hel session där "fixen inte tog" berodde på exakt detta glapp).
#    Ingen omkompilering — samma features (default = modernbert + mcp) och
#    samma CARGO_TARGET_DIR, så det är en kopiering.
cargo install --path . --features modernbert,mcp
rc=$?
if [[ $rc -ne 0 ]]; then
    echo "FEL: cargo install misslyckades (RC=$rc) — MCP ser fortfarande den gamla binären." >&2
    echo "[commit-build] RESULT fail:install" >&2
    exit $rc
fi

echo "[commit-build] KLART: rev $REV → $BIN (mtime $(date -r "$BIN" '+%F %T'))"
echo "[commit-build] installerad: $(command -v gnawtreewriter) ($(~/.cargo/bin/gnawtreewriter --version 2>/dev/null || echo '?') )"
echo "[commit-build] OBS: (1) Motor2 plockar upp GTW-källkoden vid nästa Motor2-bygge (path-dep)."
echo "[commit-build] OBS: (2) MCP-klienten (Cline/opencode) kör den binär den startade med —"
echo "                  nya grepp når den först när daemonen bytts ut:"
echo "                  scripts/daemon-hygiene.sh reap --yes   (dödar även din egen daemon)"
echo "                  kör det mellan sessioner, eller starta om klienten."

# 5. Fråga om reap — men ALDRIG när stdin inte är en terminal.
#    En agent som kör det här skriptet skulle annars blockera på `read` för
#    evigt (uppmätt mönster: agenter kör skript icke-interaktivt). Styr utan
#    fråga med GTW_REAP=yes|no.
ask_reap() {
    local svar
    case "${GTW_REAP:-ask}" in
        yes) scripts/daemon-hygiene.sh reap --yes ;;
        no) echo "[commit-build] reap hoppad (GTW_REAP=no)." ;;
        *)
            if [[ -t 0 ]]; then
                read -r -p "Döda gamla MCP-daemoner nu? (dödar även denna sessions) [j/N] " svar || svar=""
                case "$svar" in
                    j|J|ja|JA|y|Y) scripts/daemon-hygiene.sh reap --yes ;;
                    *) echo "[commit-build] hoppar reap — kör scripts/daemon-hygiene.sh reap --yes när du vill." ;;
                esac
            else
                echo "[commit-build] icke-interaktiv körning: frågar inte (GTW_REAP=yes tvingar)."
            fi
            ;;
    esac
}

ask_reap

# 6. Artifact gate: CI runs `code-index . --check`, so the tree we leave behind
#    must pass it. The artifact is refreshed HERE (after install, with the NEW
#    binary) because rendering it with the previous binary can differ by the
#    version stamp and by items the new build just learned to extract.
#    Before this step the artifact was refreshed by nothing at all in a repo
#    without a git hook, so commits touching src/ went to CI red (2026-11-11).
if [[ -f code-index.md && -x "$HOME/.cargo/bin/gnawtreewriter" ]]; then
    "$HOME/.cargo/bin/gnawtreewriter" code-index . --check --fix >/dev/null 2>&1 || true
    # Cargo.lock is generated too, and a version bump dirties it: leaving it
    # behind produced a tag whose lockfile still named the previous version
    # (2026-11-11). Stage both, commit once, keep the tree clean.
    if [[ -n "$(git status --porcelain code-index.md Cargo.lock)" ]]; then
        git add code-index.md
        [[ -f Cargo.lock ]] && git add Cargo.lock
        if git commit -q -m "chore(code-index): refresh artifact + lockfile after build"; then
            echo "[commit-build] artefakten/lockfilen var inaktuell — committad separat (trädet förblir rent)."
        fi
    fi
    if ! "$HOME/.cargo/bin/gnawtreewriter" code-index . --check >/dev/null 2>&1; then
        echo "[commit-build] VARNING: code-index.md är fortfarande inaktuell — CI kommer att faila." >&2
        echo "[commit-build] RESULT fail:code-index" >&2
        exit 1
    fi
fi

# Maskinläsbart facit SIST — en agent läser `tail -1`, inte prosan ovanför.
echo "[commit-build] RESULT ok"
