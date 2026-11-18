#!/usr/bin/env bash
# claim-scan gate (T4 §3.1): doc claims the code-index store cannot verify.
#
# WHY a gate: the scan found its own motivating case on day one (a doc
# comment promising relation types the extractor never emits), and the
# documented failure class in this repo is exactly "a gate nobody runs is
# decoration" (code-index --check had that fate, uppslag #17).
#
# CONTRACT: HIGH severity findings fail the build (a qualified path the
# store cannot verify is almost always a lie); MEDIUM are reported as a
# ranked summary and never fail (they include parameter names, env vars
# and test fixtures - legitimate prose). The supply is visible either way.
#
# Usage: scripts/claim-scan-gate.sh [ROOT]
# Exit: 0 = no HIGH findings, 1 = HIGH findings (printed with anchors),
#       2 = the store or the binary is missing (the gate cannot be judged).
set -uo pipefail

ROOT="${1:-.}"
BIN="${GTW_BIN:-gnawtreewriter}"

if ! command -v "$BIN" >/dev/null 2>&1; then
    echo "claim-scan gate: '$BIN' not on PATH — build or point GTW_BIN at it" >&2
    exit 2
fi

REPORT="$("$BIN" claim-scan "$ROOT" 2>/dev/null)"
if [[ -z "$REPORT" ]]; then
    echo "claim-scan gate: no report (store missing? run 'gnawtreewriter code-index $ROOT --db' first)" >&2
    exit 2
fi

TOTAL="$(printf '%s' "$REPORT" | sed -n 's/.*"findings_total": *\([0-9]*\).*/\1/p' | head -1)"
CLAIMS="$(printf '%s' "$REPORT" | sed -n 's/.*"claims_total": *\([0-9]*\).*/\1/p' | head -1)"
VERIFIED="$(printf '%s' "$REPORT" | sed -n 's/.*"verified": *\([0-9]*\).*/\1/p' | head -1)"

# HIGH findings: identifier lines immediately followed by a HIGH severity.
HIGH_COUNT="$(printf '%s' "$REPORT" | grep -c '"severity": "HIGH"' || true)"
MEDIUM_COUNT="$((${TOTAL:-0} - ${HIGH_COUNT:-0}))"

echo "claim-scan gate: ${CLAIMS:-0} claims, ${VERIFIED:-0} verified, ${TOTAL:-0} lie-CANDIDATES (HIGH ${HIGH_COUNT:-0}, MEDIUM ${MEDIUM_COUNT:-0})"
echo "  candidates, not verdicts — grep to disambiguate; MEDIUM never fail the gate."

if [[ "${HIGH_COUNT:-0}" -gt 0 ]]; then
    echo ""
    echo "✗ HIGH-severity claim candidates (qualified paths the store cannot verify):"
    printf '%s' "$REPORT" \
        | grep -B3 '"severity": "HIGH"' \
        | grep -E '"identifier"|"file"|"line"' \
        | paste - - - 2>/dev/null | sed 's/  */ /g' | head -20
    echo ""
    echo "Fix the doc wording or add the fact to the store, then re-run."
    exit 1
fi

exit 0
