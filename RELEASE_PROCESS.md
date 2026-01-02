# Release Process for GnawTreeWriter

**Complete guide for creating releases and managing versions**

Version: 1.0 | Last Updated: 2025-01-02

---

## Table of Contents

1. [Overview](#overview)
2. [Version Strategy](#version-strategy)
3. [Release Types](#release-types)
4. [Step-by-Step Release Process](#step-by-step-release-process)
5. [Automation Scripts](#automation-scripts)
6. [Troubleshooting](#troubleshooting)
7. [Post-Release Checklist](#post-release-checklist)

---

## Overview

This document describes the complete process for releasing new versions of GnawTreeWriter. Following this process ensures:

- ✅ Consistent version numbering across all files
- ✅ Proper Git tags and GitHub Releases
- ✅ Complete changelog documentation
- ✅ No version drift between components

**Key Principle**: Every release must synchronize 5 key places:
1. `Cargo.toml` version field
2. `CHANGELOG.md` release entry
3. Git tag (`vX.Y.Z`)
4. GitHub Release
5. CLI `--version` output

---

## Version Strategy

GnawTreeWriter follows [Semantic Versioning 2.0.0](https://semver.org/):

```
MAJOR.MINOR.PATCH
  │     │     │
  │     │     └─── Bug fixes, docs (backward compatible)
  │     └─────── New features (backward compatible)
  └───────────── Breaking changes
```

### Examples

- `0.3.3 → 0.3.4` — Patch: Documentation fixes, minor bug fixes
- `0.3.4 → 0.4.0` — Minor: New commands, new language support
- `0.9.0 → 1.0.0` — Major: Breaking CLI changes, API redesign

### Pre-1.0 Guidelines

While in `0.x.y` versions:
- Minor versions (`0.x`) can include breaking changes
- Still aim for backward compatibility when possible
- Document breaking changes clearly in CHANGELOG

---

## Release Types

### Patch Release (X.Y.Z → X.Y.Z+1)

**When to use:**
- Bug fixes only
- Documentation improvements
- Performance optimizations (no API changes)
- Test additions

**Example:** `0.3.3 → 0.3.4`

### Minor Release (X.Y.Z → X.Y+1.0)

**When to use:**
- New features (backward compatible)
- New CLI commands
- New language support
- Deprecations (with backward compatibility)

**Example:** `0.3.4 → 0.4.0`

### Major Release (X.Y.Z → X+1.0.0)

**When to use:**
- Breaking changes to CLI
- Removal of deprecated features
- Major architectural changes
- API redesign

**Example:** `0.9.0 → 1.0.0`

---

## Step-by-Step Release Process

### Phase 1: Preparation

#### 1.1 Determine Next Version

```bash
# Check current version
cargo metadata --format-version 1 | grep '"version"' | head -1

# Check last git tag
git describe --tags --abbrev=0

# Check GitHub releases
gh release list | head -1
```

Decide on next version based on changes since last release.

#### 1.2 Review Changes

```bash
# List commits since last release
git log v0.3.3..HEAD --oneline

# Review changed files
git diff v0.3.3..HEAD --stat
```

#### 1.3 Run Full Test Suite

```bash
# Clean build environment
cargo clean

# Run all tests
cargo test --all

# Check for warnings
cargo clippy -- -D warnings

# Test release build
cargo build --release

# Verify binary works
./target/release/gnawtreewriter --version
./target/release/gnawtreewriter examples
```

### Phase 2: Version Update

#### 2.1 Update Cargo.toml

```bash
# Current version
OLD_VERSION="0.3.3"
NEW_VERSION="0.3.4"

# Update using GnawTreeWriter itself (dogfooding!)
gnawtreewriter edit Cargo.toml "version" "\"$NEW_VERSION\""

# Or manually edit:
# version = "0.3.4"
```

#### 2.2 Update CHANGELOG.md

Add new section at the top of CHANGELOG.md:

```markdown
## [0.3.4] - 2025-01-02

### Added
- New feature X
- New command Y

### Changed
- Improved Z performance
- Updated documentation

### Fixed
- Fixed issue with A
- Resolved bug in B
```

**Important**: If previous version was marked as "Unreleased", change it to released with date.

#### 2.3 Create Release Notes

Create `RELEASE_NOTES_v0.3.4.md`:

```markdown
# GnawTreeWriter — Release Notes (v0.3.4)

**Date:** 2025-01-02  
**Type:** Patch Release

## Summary

Brief description of what this release includes.

## Highlights

### Feature Name
Description of the feature with examples.

## Changes

### Added
- Feature X
- Command Y

### Changed
- Improvement Z

### Fixed
- Bug A
- Issue B

## Upgrade Instructions

\`\`\`bash
cd GnawTreeWriter
git pull origin master
cargo install --path .
gnawtreewriter --version  # Should show 0.3.4
\`\`\`

## Testing

All tests passing:
\`\`\`
cargo test
# test result: ok. N passed; 0 failed
\`\`\`

## Acknowledgments

Thanks to contributors and testers.
```

### Phase 3: Verification

#### 3.1 Rebuild and Test

```bash
# Clean rebuild
cargo clean
cargo build --release

# Verify version in binary
./target/release/gnawtreewriter --version
# Must show: gnawtreewriter 0.3.4

# Run all tests
cargo test

# Manual smoke test
./target/release/gnawtreewriter analyze README.md
```

#### 3.2 Check All Files

Verify these files are updated:
- [ ] `Cargo.toml` - version field
- [ ] `Cargo.lock` - automatically updated
- [ ] `CHANGELOG.md` - new version section
- [ ] `RELEASE_NOTES_vX.Y.Z.md` - created

### Phase 4: Commit and Tag

#### 4.1 Stage Changes

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md RELEASE_NOTES_v0.3.4.md
```

#### 4.2 Commit

```bash
git commit -m "chore(release): bump version to 0.3.4"
```

**Commit message format**:
```
chore(release): bump version to X.Y.Z

- Update Cargo.toml to X.Y.Z
- Update CHANGELOG.md with release notes
- Add RELEASE_NOTES_vX.Y.Z.md
```

#### 4.3 Push to Master

```bash
git push origin master
```

#### 4.4 Create and Push Tag

```bash
# Create annotated tag
git tag -a v0.3.4 -m "Release v0.3.4: Brief description"

# Push tag
git push origin v0.3.4
```

**Note**: Always use annotated tags (`-a` flag), not lightweight tags.

### Phase 5: GitHub Release

#### 5.1 Create Release (GitHub CLI)

```bash
gh release create v0.3.4 \
  --title "v0.3.4 - Release Title" \
  --notes-file RELEASE_NOTES_v0.3.4.md
```

#### 5.2 Create Release (Manual)

1. Go to https://github.com/Tuulikk/GnawTreeWriter/releases/new
2. Select tag: `v0.3.4`
3. Release title: `v0.3.4 - Release Title`
4. Copy content from `RELEASE_NOTES_v0.3.4.md` into description
5. Click "Publish release"

#### 5.3 Verify Release

```bash
# Check release list
gh release list

# Should show:
# TITLE                TYPE    TAG NAME  PUBLISHED
# v0.3.4 - Title      Latest  v0.3.4    less than a minute ago
```

**Critical**: Verify that the new release is marked as "Latest"

---

## Automation Scripts

### Quick Version Bump Script

Save as `scripts/bump-version.sh`:

```bash
#!/bin/bash
set -e

if [ -z "$1" ]; then
  echo "Usage: $0 <new_version>"
  echo "Example: $0 0.3.4"
  exit 1
fi

NEW_VERSION=$1
DATE=$(date +%Y-%m-%d)

echo "Bumping version to $NEW_VERSION..."

# Update Cargo.toml
sed -i "s/^version = .*/version = \"$NEW_VERSION\"/" Cargo.toml

# Add to CHANGELOG
echo -e "\n## [$NEW_VERSION] - $DATE\n\n### Added\n- \n\n### Changed\n- \n\n### Fixed\n- \n" | cat - CHANGELOG.md > temp && mv temp CHANGELOG.md

echo "✅ Version bumped to $NEW_VERSION"
echo "⚠️  Now edit CHANGELOG.md and create RELEASE_NOTES_v$NEW_VERSION.md"
```

### Release Checklist Script

Save as `scripts/pre-release-check.sh`:

```bash
#!/bin/bash
set -e

echo "🔍 Pre-release checklist..."

# Check if on master
BRANCH=$(git branch --show-current)
if [ "$BRANCH" != "master" ]; then
  echo "❌ Not on master branch (current: $BRANCH)"
  exit 1
fi
echo "✅ On master branch"

# Check for uncommitted changes
if ! git diff-index --quiet HEAD --; then
  echo "❌ Uncommitted changes detected"
  exit 1
fi
echo "✅ No uncommitted changes"

# Run tests
echo "🧪 Running tests..."
cargo test --all
echo "✅ All tests passed"

# Check for clippy warnings
echo "🔍 Running clippy..."
cargo clippy -- -D warnings
echo "✅ No clippy warnings"

# Build release
echo "🔨 Building release..."
cargo build --release
echo "✅ Release build successful"

# Check version sync
CARGO_VERSION=$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
BINARY_VERSION=$(./target/release/gnawtreewriter --version | awk '{print $2}')

if [ "$CARGO_VERSION" != "$BINARY_VERSION" ]; then
  echo "❌ Version mismatch: Cargo.toml=$CARGO_VERSION, binary=$BINARY_VERSION"
  exit 1
fi
echo "✅ Version synchronized: $CARGO_VERSION"

echo ""
echo "🎉 All pre-release checks passed!"
echo "📦 Ready to release version $CARGO_VERSION"
```

Make scripts executable:
```bash
chmod +x scripts/*.sh
```

---

## Troubleshooting

### Version Shows Old Number

**Problem**: `gnawtreewriter --version` shows old version after update.

**Solution**:
```bash
cargo clean
cargo build --release
./target/release/gnawtreewriter --version
```

The binary caches the version from `Cargo.toml` at compile time.

### GitHub Release Not Showing as Latest

**Problem**: New release exists but old one still shows as "Latest".

**Solution**:
1. Check that you created a proper GitHub Release (not just a tag)
2. Verify tag format is `vX.Y.Z` (with 'v' prefix)
3. Release might be marked as "Pre-release" - edit and uncheck that box

### Tag Already Exists

**Problem**: `git push origin v0.3.4` fails with "tag already exists".

**Solution**:
```bash
# Delete local tag
git tag -d v0.3.4

# Delete remote tag (careful!)
git push origin :refs/tags/v0.3.4

# Recreate tag
git tag -a v0.3.4 -m "Release v0.3.4"
git push origin v0.3.4
```

### CHANGELOG Merge Conflicts

**Problem**: Multiple contributors updating CHANGELOG simultaneously.

**Solution**:
- Always add new entries at the top
- Use descriptive section headers
- Pull and rebase before adding entries

---

## Post-Release Checklist

After creating a release:

- [ ] Verify GitHub Release shows as "Latest"
- [ ] Check that release notes are properly formatted
- [ ] Test installation from source works:
  ```bash
  git clone https://github.com/Tuulikk/GnawTreeWriter.git
  cd GnawTreeWriter
  cargo install --path .
  gnawtreewriter --version
  ```
- [ ] Update project README if needed
- [ ] Announce release (if major/minor):
  - GitHub Discussions
  - Project documentation
  - Social media (if applicable)
- [ ] Close related GitHub issues/milestones
- [ ] Update roadmap (ROADMAP.md) with completed features

---

## Quick Reference

### Version Update Commands

```bash
# 1. Update version
vim Cargo.toml  # Change version = "X.Y.Z"

# 2. Update changelog
vim CHANGELOG.md

# 3. Create release notes
vim RELEASE_NOTES_vX.Y.Z.md

# 4. Test
cargo clean && cargo test && cargo build --release

# 5. Commit
git add Cargo.toml Cargo.lock CHANGELOG.md RELEASE_NOTES_vX.Y.Z.md
git commit -m "chore(release): bump version to X.Y.Z"
git push origin master

# 6. Tag
git tag -a vX.Y.Z -m "Release vX.Y.Z: Description"
git push origin vX.Y.Z

# 7. Release
gh release create vX.Y.Z --title "vX.Y.Z - Title" --notes-file RELEASE_NOTES_vX.Y.Z.md
```

### Files That Must Be Updated

Every release must update:
1. ✅ `Cargo.toml` → `version = "X.Y.Z"`
2. ✅ `Cargo.lock` → (auto-updated by cargo)
3. ✅ `CHANGELOG.md` → `## [X.Y.Z] - DATE`
4. ✅ `RELEASE_NOTES_vX.Y.Z.md` → (new file)

Every release must create:
1. ✅ Git tag → `vX.Y.Z`
2. ✅ GitHub Release → "vX.Y.Z - Title"

---

## Best Practices

### Do ✅

- Always test before releasing
- Use semantic versioning correctly
- Write clear, detailed changelog entries
- Create annotated git tags
- Verify all version numbers match
- Test the installation process
- Keep release notes user-focused

### Don't ❌

- Skip the test suite
- Forget to update CHANGELOG.md
- Use lightweight tags (use `-a` flag)
- Release with uncommitted changes
- Push directly without testing
- Reuse version numbers
- Leave versions as "Unreleased" indefinitely

---

## Emergency Rollback

If a release has critical issues:

### Option 1: Quick Patch Release

```bash
# Fix the issue
git commit -m "fix: critical issue in vX.Y.Z"

# Release X.Y.Z+1 immediately
# Follow normal release process
```

### Option 2: Mark Release as Pre-release

1. Go to GitHub Release page
2. Edit the problematic release
3. Check "Set as a pre-release"
4. Add warning to description

### Option 3: Delete Release (Last Resort)

```bash
# Delete GitHub release
gh release delete vX.Y.Z

# Delete tag
git tag -d vX.Y.Z
git push origin :refs/tags/vX.Y.Z

# Fix issues, then re-release
```

**Warning**: Only use Option 3 if absolutely necessary and immediately after release.

---

## Version History Template

Keep track of releases:

```markdown
| Version | Date       | Type  | Highlights                    |
|---------|------------|-------|-------------------------------|
| 0.3.4   | 2025-01-02 | Patch | Documentation improvements    |
| 0.3.3   | 2025-01-02 | Patch | Installation section in README|
| 0.3.2   | 2025-12-31 | Patch | Diff-to-batch, quick command  |
| 0.3.1   | 2025-12-31 | Patch | Code quality (clippy fixes)   |
| 0.3.0   | 2025-12-28 | Minor | Batch ops, tags, sessions     |
```

---

**Remember**: Consistency is key. Following this process ensures that users, contributors, and automation tools all see the same version information across the entire project.

---

*Last Updated: 2025-01-02*
*Process Version: 1.0*