#!/usr/bin/env bash
# Build a CUDA-enabled gnawtreewriter and install it with GPU runtime libs.
#
# WHAT THIS GIVES YOU: `ai index` may then use the GPU (opt-in — set
# `indexing.device: auto` in gnawtreewriter.yaml, or run with --gpu).
# The 20%-VRAM safety gate still decides at every run.
#
# REQUIREMENTS (any Linux + NVIDIA GPU, nothing distro-specific):
#   - podman (or docker: CONTAINER_CMD=docker)
#   - NVIDIA driver (nvidia-smi works)
#   - network on first run (rustup + crates; caches live in named volumes)
#
# The build runs inside nvidia/cuda devel — the host does NOT need the
# CUDA toolkit (Fedora has none; that is the point).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CONTAINER_CMD="${CONTAINER_CMD:-podman}"
CUDA_IMAGE="${CUDA_IMAGE:-docker.io/nvidia/cuda:12.8.1-devel-ubuntu24.04}"
LIBDIR="$HOME/.local/lib/gnawtreewriter-cuda"
REALBIN="$HOME/.cargo/libexec/gnawtreewriter"
WRAPPER="$HOME/.cargo/bin/gnawtreewriter"

die() { echo "ERROR: $*" >&2; exit 1; }

command -v "$CONTAINER_CMD" >/dev/null || die "need podman (or CONTAINER_CMD=docker)"
command -v nvidia-smi >/dev/null || die "need an NVIDIA driver (nvidia-smi missing) — use the normal CPU build instead"

# Compute capability of GPU0 (bindgen_cuda cannot probe inside the container,
# so we take it from the host): e.g. 8.9 -> 89
CAP_RAW="$(nvidia-smi --query-gpu=compute_cap --format=csv,noheader | head -1 | tr -d ' ')"
[[ "$CAP_RAW" =~ ^[0-9]+\.[0-9]+$ ]] || die "could not read compute_cap (got: '$CAP_RAW')"
CUDA_COMPUTE_CAP="${CAP_RAW/./}"
echo "== GPU: compute cap $CAP_RAW -> CUDA_COMPUTE_CAP=$CUDA_COMPUTE_CAP"

echo "== Building in $CUDA_IMAGE (cached: rustup, crates, objects) =="
"$CONTAINER_CMD" run --rm -i --security-opt label=disable \
    -v "$REPO_ROOT:/src:ro" \
    -v gtw-cargo:/usr/local/cargo \
    -v gtw-rustup:/usr/local/rustup \
    -v gtw-target:/build/target \
    -v gtw-cuda-libs:/libs \
    -e CARGO_HOME=/usr/local/cargo \
    -e RUSTUP_HOME=/usr/local/rustup \
    -e CARGO_TARGET_DIR=/build/target \
    -e CUDA_HOME=/usr/local/cuda \
    -e "CUDA_COMPUTE_CAP=$CUDA_COMPUTE_CAP" \
    "$CUDA_IMAGE" bash -s <<'BUILD'
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive
command -v curl >/dev/null || { apt-get update -qq && apt-get install -y -qq curl pkg-config libssl-dev >/dev/null; }
if [ ! -x "$CARGO_HOME/bin/cargo" ]; then
    echo "== installing rustup =="
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --profile minimal --default-toolchain stable --no-modify-path
fi
export PATH="$CARGO_HOME/bin:$PATH"
echo "== $(rustc --version)"
rm -rf /work && cp -a /src /work && cd /work
cargo build --release --features cuda
cp "$CARGO_TARGET_DIR/release/gnawtreewriter" /libs/gnawtreewriter-cuda
echo "== BUILD_OK =="
BUILD

echo "== Checking which CUDA runtime libs the HOST lacks =="
OUT_TMP="$(mktemp -d)"
trap 'rm -rf "$OUT_TMP"' EXIT
"$CONTAINER_CMD" run --rm --security-opt label=disable \
    -v gtw-cuda-libs:/in -v "$OUT_TMP:/out" "$CUDA_IMAGE" \
    cp /in/gnawtreewriter-cuda /out/

# The host has the driver (libcuda.so) but usually no CUDA toolkit: ship
# exactly the runtime libs that are missing HERE — resolved iteratively so
# transitive deps (cublas -> cublasLt -> ...) are caught too.
mkdir -p "$LIBDIR"
for _pass in 1 2 3 4; do
    missing="$(LD_LIBRARY_PATH="$LIBDIR" ldd "$OUT_TMP/gnawtreewriter-cuda" | awk '/not found/{print $1}')"
    [ -z "$missing" ] && break
    echo "== pass $_pass: shipping: $missing"
    "$CONTAINER_CMD" run --rm --security-opt label=disable \
        -e "MISSING=$missing" -v "$LIBDIR:/out" "$CUDA_IMAGE" bash -c '
            for lib in $MISSING; do
                found=$(find /usr/local/cuda/lib64 -name "${lib}*" | head -1)
                [ -n "$found" ] || { echo "FATAL: $lib is neither on the host nor in the image" >&2; exit 1; }
                cp -a "$(dirname "$found")/${lib}"* /out/
                echo "== shipped $lib"
            done'
done
LD_LIBRARY_PATH="$LIBDIR" ldd "$OUT_TMP/gnawtreewriter-cuda" | grep -q "not found" \
    && die "unresolved libraries remain — see above"

echo "== Installing (binary -> $REALBIN, wrapper -> $WRAPPER) =="
mkdir -p "$(dirname "$REALBIN")" "$(dirname "$WRAPPER")"
install -m 755 "$OUT_TMP/gnawtreewriter-cuda" "$REALBIN"
cat > "$WRAPPER" <<WRAP
#!/bin/sh
# GTW CUDA build ($CUDA_IMAGE, compute cap $CUDA_COMPUTE_CAP) — runtime libs in $LIBDIR
LD_LIBRARY_PATH="$LIBDIR\${LD_LIBRARY_PATH:+:\$LD_LIBRARY_PATH}" exec "$REALBIN" "\$@"
WRAP
chmod +x "$WRAPPER"

echo
echo "== DONE. Verify: gnawtreewriter --version"
echo "== GPU indexing is opt-in:"
echo "     persistent: set 'indexing: { device: auto }' in gnawtreewriter.yaml (see AGENTS.md)"
echo "     one-off:    gnawtreewriter ai index --gpu"
echo "   (20%-VRAM safety gate applies; without the flag everything stays CPU)"
