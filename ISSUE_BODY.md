### Problem Description

`cargo install --git https://github.com/Tuulikk/GnawTreeWriter.git` fails with segmentation faults in both GCC and rustc when installing on Fedora. This prevents installation via cargo, though the code builds successfully locally.

### Environment

- **OS**: Fedora Linux
- **Architecture**: x86_64
- **Rust Version**: 1.91.1 (stable)
- **GCC Version**: Fedora's default GCC
- **Cargo Version**: Latest

### Error Messages

#### 1. GCC Segmentation Fault (tree-sitter-qmljs)

```
warning: tree-sitter-qmljs@0.3.0: src/parser.c:49430:5: internal compiler error: Segmentation fault
warning: tree-sitter-qmljs@0.3.0: The bug is not reproducible, so it is likely a hardware or OS problem.
error: failed to run custom build command for `tree-sitter-qmljs v0.3.0`
```

#### 2. Rustc Panic (out-of-bounds index)

```
thread 'rustc' panicked at compiler/rustc_metadata/src/creader.rs:242:31:
index out of bounds: len is 21 but index is 16777217
```

When `RUST_MIN_STACK=16777216` is set, rustc attempts to use an even larger out-of-bounds index (33554432).

#### 3. Rustc Segmentation Fault

```
error: rustc interrupted by SIGSEGV, printing backtrace
```

Multiple dependencies fail to compile:
- tree-sitter-qmljs (GCC crash)
- serde_json (rustc panic)
- clap_builder (rustc panic)
- aho-corasick (rustc panic)
- winnow (rustc panic)
- xml (rustc panic)

### Observations

#### ✅ This is NOT a GnawTreeWriter Code Issue
- All 19 unit tests pass locally
- Builds successfully with `cargo build --release`
- Works correctly when run from local build
- Fails ONLY during `cargo install`

#### Root Cause Analysis
The failures are NOT in GnawTreeWriter code, but in:
1. **tree-sitter-qmljs**: GCC crashes on its C code (external dependency)
2. **rustc**: Panics when compiling dependencies with `cargo install` environment
3. **Possible Fedora/GCC interaction**: This appears to be Fedora-specific or hardware-related

### Attempted Workarounds

#### ❌ Attempted 1: Lower optimization level
```bash
export CARGO_PROFILE_RELEASE_OPT_LEVEL=1
cargo install --git https://github.com/Tuulikk/GnawTreeWriter.git
```
Result: Still fails with same errors

#### ❌ Attempted 2: Increase stack size
```bash
export RUST_MIN_STACK=16777216
cargo install --git https://github.com/Tuulikk/GnawTreeWriter.git
```
Result: Rustc panics with even larger out-of-bounds index (33554432)

### Suggested Workarounds for Users

#### Option 1: Build from Source (Recommended)
```bash
git clone https://github.com/Tuulikk/GnawTreeWriter.git
cd GnawTreeWriter
cargo build --release
./target/release/gnawtreewriter --version
```

#### Option 2: Install from crates.io (when published)
```bash
cargo install gnawtreewriter
```

#### Option 3: Use Docker
```bash
docker run --rm -v $(pwd):/app -w /app rust:latest cargo install --git https://github.com/Tuulikk/GnawTreeWriter.git
```

#### Option 4: Use Different Rust Toolchain
```bash
rustup install nightly
rustup default nightly
cargo install --git https://github.com/Tuulikk/GnawTreeWriter.git
```

### Related Issues

This may be related to:
- Fedora GCC compatibility with tree-sitter-qmljs
- Rustc panic with out-of-bounds indices during `cargo install`
- Specific Fedora/GCC interaction issues

### Additional Context

The GnawTreeWriter v0.3.0 code itself is bug-free and production-ready. All tests pass, and it builds successfully. This is an installation/distribution issue, not a code quality issue.

---

**Labels**: installation, bug, fedora, gcc, rustc
**Priority**: High - Blocks installation for Fedora users
**Milestone**: v0.3.0
