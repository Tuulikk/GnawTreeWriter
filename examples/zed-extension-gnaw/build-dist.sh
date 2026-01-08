#!/usr/bin/env bash
#
# build-dist.sh
#
# Build and package the Zed dev extension into a distribution tarball/zip under dist/
#
# Usage:
#   ./build-dist.sh           # build release and create tar.gz in ./dist/
#   ./build-dist.sh --zip     # build and create zip instead
#   ./build-dist.sh --clean   # clean existing dist artifacts before building
#   ./build-dist.sh --help    # show help
#
# The script does:
#  - build the extension with `cargo build --release`
#  - collect extension manifest, README, scripts and the compiled cdylib/binary
#  - create a versioned archive in dist/
#
set -euo pipefail
IFS=$'\n\t'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXT_DIR="$SCRIPT_DIR"
DIST_DIR="$EXT_DIR/dist"

FORMAT="tar"
CLEAN=false

usage() {
  cat <<EOF
Usage: $(basename "$0") [--zip] [--clean] [--help]

Options:
  --zip       Create a .zip archive (default: tar.gz)
  --clean     Remove existing dist artifacts before building
  --help      Show this help
EOF
}

# parse args
while [[ $# -gt 0 ]]; do
  case "$1" in
    --zip) FORMAT="zip"; shift ;;
    --clean) CLEAN=true; shift ;;
    --help|-h) usage; exit 0 ;;
    *) echo "Unknown arg: $1"; usage; exit 1 ;;
  esac
done

echo "Extension dir: $EXT_DIR"
echo "Distribution format: $FORMAT"
echo

if [[ "$CLEAN" == "true" ]]; then
  echo "Cleaning dist/..."
  rm -rf "$DIST_DIR"
fi

mkdir -p "$DIST_DIR"

# Extract package name & version from Cargo.toml
if [[ ! -f "$EXT_DIR/Cargo.toml" ]]; then
  echo "ERROR: Cargo.toml not found in extension dir ($EXT_DIR)" >&2
  exit 2
fi

pkg_name=$(sed -n 's/^name *= *"\(.*\)".*$/\1/p' "$EXT_DIR/Cargo.toml" | head -n1 || true)
pkg_ver=$(sed -n 's/^version *= *"\(.*\)".*$/\1/p' "$EXT_DIR/Cargo.toml" | head -n1 || true)

if [[ -z "$pkg_name" || -z "$pkg_ver" ]]; then
  echo "ERROR: Could not determine package name/version from Cargo.toml" >&2
  exit 3
fi

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
DIST_NAME="${pkg_name}-${pkg_ver}-${OS}-${ARCH}"
TMPDIR="$(mktemp -d)"

echo "Package: $pkg_name"
echo "Version: $pkg_ver"
echo "Target: $DIST_NAME"
echo "Temporary staging dir: $TMPDIR"
echo

# Build the extension
echo "Building extension (release)..."
pushd "$EXT_DIR" >/dev/null
cargo build --release
popd >/dev/null
echo "Build finished."
echo

# Locate the compiled artifact(s)
ARTIFACT=""
# Look for common cdylib/binary patterns in target/release
while IFS= read -r f; do
  # prefer non-dbg artifacts
  ARTIFACT="$f"
  break
done < <(find "$EXT_DIR/target/release" -maxdepth 1 -type f \( -name "lib${pkg_name}.*" -o -name "${pkg_name}.*" -o -name "${pkg_name}" \) 2>/dev/null | sort)

if [[ -z "$ARTIFACT" ]]; then
  echo "WARNING: No artifact matching '${pkg_name}' found in target/release. Including entire target/release directory instead."
else
  echo "Found artifact: $ARTIFACT"
fi

# Prepare staging directory structure
STAGING="$TMPDIR/$DIST_NAME"
mkdir -p "$STAGING"

# Files to include
# Always include manifest & README
if [[ -f "$EXT_DIR/extension.toml" ]]; then
  cp "$EXT_DIR/extension.toml" "$STAGING/"
fi
if [[ -f "$EXT_DIR/README.md" ]]; then
  cp "$EXT_DIR/README.md" "$STAGING/"
fi
if [[ -f "$EXT_DIR/Cargo.toml" ]]; then
  cp "$EXT_DIR/Cargo.toml" "$STAGING/"
fi

# Include scripts dir if present
if [[ -d "$EXT_DIR/../..../scripts" ]]; then
  # defensive, do nothing — (this won't normally be true)
  :
fi

# Prefer including the extension's own scripts folder if present
if [[ -d "$EXT_DIR/scripts" ]]; then
  mkdir -p "$STAGING/scripts"
  cp -a "$EXT_DIR/scripts/." "$STAGING/scripts/"
fi

# Copy artifact if found
if [[ -n "$ARTIFACT" && -f "$ARTIFACT" ]]; then
  cp "$ARTIFACT" "$STAGING/"
else
  # fallback: copy all relevant target/release files (non-debug)
  mkdir -p "$STAGING/target_release"
  cp -a "$EXT_DIR/target/release/." "$STAGING/target_release/" || true
fi

# Add a simple manifest describing the package (for consumer convenience)
cat > "$STAGING/extension-package.json" <<JSON
{
  "name": "$pkg_name",
  "version": "$pkg_ver",
  "built_at": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "os": "$OS",
  "arch": "$ARCH"
}
JSON

# Create archive
pushd "$TMPDIR" >/dev/null
ARCHIVE_NAME="$DIST_NAME"
if [[ "$FORMAT" == "tar" ]]; then
  ARCHIVE_PATH="$DIST_DIR/${ARCHIVE_NAME}.tar.gz"
  echo "Creating $ARCHIVE_PATH ..."
  tar -czf "$ARCHIVE_PATH" "$DIST_NAME"
elif [[ "$FORMAT" == "zip" ]]; then
  if ! command -v zip >/dev/null 2>&1; then
    echo "ERROR: 'zip' command not found; cannot create zip archive." >&2
    exit 4
  fi
  ARCHIVE_PATH="$DIST_DIR/${ARCHIVE_NAME}.zip"
  echo "Creating $ARCHIVE_PATH ..."
  zip -r "$ARCHIVE_PATH" "$DIST_NAME" >/dev/null
else
  echo "Unknown format: $FORMAT" >&2
  exit 5
fi
popd >/dev/null

# Generate checksum if available
if command -v sha256sum >/dev/null 2>&1; then
  sha256sum "$ARCHIVE_PATH" > "${ARCHIVE_PATH}.sha256"
  echo "SHA256: $(cut -d' ' -f1 "${ARCHIVE_PATH}.sha256")"
fi

echo
echo "Distribution package created: $ARCHIVE_PATH"
echo "Contents:"
tar -tf "$ARCHIVE_PATH" | sed -n '1,20p' || true
echo
echo "Note: follow Zed docs to install the built extension as a dev extension:"
echo "  https://zed.dev/docs/extensions/mcp-extensions"
echo
echo "Done."
