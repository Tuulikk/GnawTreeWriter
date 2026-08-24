#!/bin/bash
# GTW AI-Features Performance Benchmark
# Kör från projektroten: bash scripts/benchmark_ai.sh

GTW="/home/tuulikk/.cargo/builds/release/gnawtreewriter"
SRC="src"

echo "=== GTW AI-Features Benchmark ==="
echo ""

bench() {
    local label="$1"
    shift
    local start=$(date +%s%N)
    "$@" > /dev/null 2>&1
    local end=$(date +%s%N)
    local ms=$(( (end - start) / 1000000 ))
    printf "  %-45s %6d ms\n" "$label" "$ms"
}

mcp() {
    echo "$1" | timeout 30 $GTW mcp stdio 2>/dev/null > /dev/null
}

echo "── explore ──"
bench "overview (level 0)" $GTW explore "" --level 0 --format json
bench "directory (level 1)" $GTW explore src/core --level 1 --format json
bench "file (level 2)" $GTW explore src/core/compress.rs --level 2 --format json
bench "full (level 3)" $GTW explore src/core/compress.rs --level 3 --format json
echo ""

echo "── compress ──"
bench "compress single file" $GTW compress src/core/compress.rs --stats
echo ""

echo "── pack ──"
bench "pack src/ (no compress)" $GTW pack $SRC --format json --no-redact
bench "pack src/ (with compress)" $GTW pack $SRC --compress --format json --no-redact
echo ""

echo "── stats ──"
bench "stats summary" $GTW stats $SRC --format json
echo ""

echo "── save_state ──"
bench "save state" mcp '{"jsonrpc":"2.0","method":"tools/call","id":1,"params":{"name":"save_state","arguments":{}}}'
echo ""

echo "── diff_since ──"
bench "diff since HEAD~5" mcp '{"jsonrpc":"2.0","method":"tools/call","id":1,"params":{"name":"diff_since","arguments":{"since_commit":"HEAD~5","include_uncommitted":true}}}'
bench "diff since saved state" mcp '{"jsonrpc":"2.0","method":"tools/call","id":1,"params":{"name":"diff_since","arguments":{"include_uncommitted":true}}}'
echo ""

echo "── index_entities ──"
bench "1 file" mcp '{"jsonrpc":"2.0","method":"tools/call","id":1,"params":{"name":"index_entities","arguments":{"file_path":"src/core/compress.rs"}}}'
bench "5 files" mcp '{"jsonrpc":"2.0","method":"tools/call","id":1,"params":{"name":"index_entities","arguments":{"file_paths":["src/core/compress.rs","src/core/pack.rs","src/core/curator.rs","src/core/stats.rs","src/core/state.rs"]}}}'
bench "20 files" mcp '{"jsonrpc":"2.0","method":"tools/call","id":1,"params":{"name":"index_entities","arguments":{"file_paths":["src/core/alf.rs","src/core/anchor.rs","src/core/batch.rs","src/core/blast.rs","src/core/blueprint.rs","src/core/compress.rs","src/core/curator.rs","src/core/diagnostics.rs","src/core/diff_parser.rs","src/core/explore.rs","src/core/file_walker.rs","src/core/gnaw_diff.rs","src/core/gnaw_find.rs","src/core/gnaw_graph.rs","src/core/gnaw_refactor.rs","src/core/guardian.rs","src/core/healer.rs","src/core/index_entities.rs","src/core/index_relations.rs","src/core/inspect.rs"]}}}'
echo ""

echo "── index_relations ──"
bench "1 file" mcp '{"jsonrpc":"2.0","method":"tools/call","id":1,"params":{"name":"index_relations","arguments":{"file_path":"src/core/compress.rs"}}}'
bench "5 files" mcp '{"jsonrpc":"2.0","method":"tools/call","id":1,"params":{"name":"index_relations","arguments":{"file_paths":["src/core/compress.rs","src/core/pack.rs","src/core/curator.rs","src/core/stats.rs","src/core/state.rs"]}}}'
bench "20 files" mcp '{"jsonrpc":"2.0","method":"tools/call","id":1,"params":{"name":"index_relations","arguments":{"file_paths":["src/core/alf.rs","src/core/anchor.rs","src/core/batch.rs","src/core/blast.rs","src/core/blueprint.rs","src/core/compress.rs","src/core/curator.rs","src/core/diagnostics.rs","src/core/diff_parser.rs","src/core/explore.rs","src/core/file_walker.rs","src/core/gnaw_diff.rs","src/core/gnaw_find.rs","src/core/gnaw_graph.rs","src/core/gnaw_refactor.rs","src/core/guardian.rs","src/core/healer.rs","src/core/index_entities.rs","src/core/index_relations.rs","src/core/inspect.rs"]}}}'
echo ""

echo "=== Benchmark complete ==="
