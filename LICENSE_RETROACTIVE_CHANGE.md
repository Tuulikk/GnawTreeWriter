# License Retroactivity Issue and Solution

**Issue:** GnawTreeWriter was developed under MIT/Apache-2.0, but we want to change to MPL-2.0.

**Problem:** Previous commits and their copyright cannot be retroactively changed without permission.

---

## Table of Contents

1. [The Legal Issue](#the-legal-issue)
2. [Copyright Ownership](#copyright-ownership)
3. [Solutions Available](#solutions-available)
4. [Recommended Approach](#recommended-approach)
5. [Implementation Steps](#implementation-steps)
6. [For Contributors](#for-contributors)

---

## The Legal Issue

### What You Can't Do

❌ **You cannot unilaterally change the license of existing code** without permission from all copyright holders.

### Why This Matters

**Scenario:**
```
Commit 1 (2025-12-26): Added Python parser - MIT licensed
Commit 2 (2025-12-27): Improved parser - MIT licensed
Commit 3 (2025-12-28): Added Rust parser - MIT licensed
...
Commit 100 (2025-01-02): Changed to MPL-2.0 ← PROBLEM!
```

**Issue:** Commits 1-99 are still MIT licensed. You can't retroactively change them to MPL-2.0.

### Real-World Impact

**User downloads old commit:**
- They see MIT license in that commit
- They use code under MIT terms
- Later you claim "it's MPL-2.0 now"
- Legal conflict ⚠️

---

## Copyright Ownership

### Who Owns GnawTreeWriter?

Check the git history:

```bash
git log --format='%aN' | sort -u
```

**If all commits are by you (Gnaw Software):**
- ✅ You own 100% of copyright
- ✅ You can relicense
- ✅ No permission needed from others

**If there are external contributors:**
- ⚠️ They own copyright to their contributions
- ⚠️ You need their permission to relicense
- ⚠️ Or use dual-licensing approach

---

## Solutions Available

### Solution 1: Sole Copyright Owner (You)

**If you are the only contributor:**

✅ **You CAN relicense** because you own all copyright.

**Steps:**
1. Verify you're sole author: `git log --format='%aN' | sort -u`
2. Add clear license change notice
3. Update LICENSE to MPL-2.0
4. Add "License History" section to README
5. (Optional) Create new release with clear license change

**Legal basis:** Copyright owner can change license at any time.

**Result:** 
- Old code: Available under MIT (historical)
- New code: MPL-2.0
- Users can choose old MIT version or accept new MPL version

---

### Solution 2: Dual License Going Forward

**Keep old code MIT, new code dual-licensed:**

```
Timeline:
├── Commits 1-99: MIT/Apache-2.0 (historical)
└── Commit 100+:  MPL-2.0 (going forward)
```

**Implementation:**
1. Keep MIT/Apache-2.0 for historical commits
2. Add LICENSE-MPL-2.0 for new code
3. Mark cutoff point clearly

**Pros:**
- ✅ No retroactivity issues
- ✅ Clear transition

**Cons:**
- ⚠️ Complex: two licenses in one repo
- ⚠️ Users confused about which applies

---

### Solution 3: Fork and Relicense

**Create clean break:**

```
gnawtreewriter-mit/     ← Old repo (archived, MIT)
gnawtreewriter/         ← New repo (fresh start, MPL-2.0)
```

**Steps:**
1. Archive current repo as "gnawtreewriter-legacy-mit"
2. Create new repo "gnawtreewriter"
3. Copy all code with MPL-2.0 from start
4. Clear history, fresh commits

**Pros:**
- ✅ Clean license history
- ✅ No confusion

**Cons:**
- ❌ Lose git history
- ❌ Break GitHub stars/forks
- ❌ Confusing for users

---

### Solution 4: License Exception Notice

**Explicitly state transition:**

Add to README and LICENSE:

```markdown
## License History

### Before 2025-01-02
GnawTreeWriter was licensed under MIT OR Apache-2.0.
Historical versions remain available under those terms.

### From 2025-01-02 onwards
GnawTreeWriter is licensed under MPL-2.0.

### Transition
As the sole copyright holder, Gnaw Software has changed
the license to MPL-2.0 for all future releases. Users may
continue to use historical versions (pre-2025-01-02) under
MIT/Apache-2.0 terms.

Git commit hash marking transition: [commit-hash]
```

**This is the most common approach for sole-author projects.**

---

## Recommended Approach

### For GnawTreeWriter (Assuming Sole Author)

**Step 1: Verify Sole Authorship**

```bash
git log --format='%aN <%aE>' | sort -u
```

**Expected output:**
```
Gnaw Software <email@example.com>
Tuulikk <email@example.com>
```

If it's all you → ✅ You can relicense

**Step 2: Add License History Document**

Create clear transition notice (this document).

**Step 3: Update README**

Add section explaining license change:

```markdown
## License History

**Current:** MPL-2.0 (from 2025-01-02)

**Historical:** MIT OR Apache-2.0 (before 2025-01-02)

As the sole copyright holder, Gnaw Software changed the license
to MPL-2.0 starting from commit [hash]. Previous versions remain
available under MIT/Apache-2.0 at tag v0.3.3.

See LICENSE_RETROACTIVE_CHANGE.md for details.
```

**Step 4: Tag Last MIT Version**

```bash
# Tag the last MIT/Apache commit
git tag v0.3.3-mit-final [commit-hash-before-mpl]
git push origin v0.3.3-mit-final
```

**Step 5: Continue with MPL-2.0**

All future work under MPL-2.0.

---

## Implementation Steps

### Step-by-Step Guide

#### 1. Verify Authorship

```bash
cd GnawTreeWriter
git log --format='%aN' | sort -u > authors.txt
cat authors.txt
```

**Check:** Are all names you or your organization?
- ✅ Yes → Proceed
- ❌ No → See "External Contributors" below

#### 2. Tag Transition Point

```bash
# Find commit hash just before MPL change
git log --oneline | grep "license: switch to MPL"
# Note the commit BEFORE this one

# Tag it
git tag -a v0.3.3-mit-final [commit-hash] -m "Last version under MIT/Apache-2.0"
git push origin v0.3.3-mit-final
```

#### 3. Add License History Section to README

```markdown
## License

This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. See LICENSE file for details.

### License History

- **Before January 2, 2025:** MIT OR Apache-2.0
- **January 2, 2025 onwards:** MPL-2.0

As the sole copyright holder, Gnaw Software changed the license to
better protect the project from proprietary forks while still enabling
commercial use. Historical versions (tag: v0.3.3-mit-final) remain
available under the original MIT/Apache-2.0 terms.

For details, see LICENSE_RETROACTIVE_CHANGE.md
```

#### 4. Create GitHub Release for MIT-Final

```bash
gh release create v0.3.3-mit-final \
  --title "v0.3.3 - Final MIT/Apache-2.0 Release" \
  --notes "This is the last version under MIT/Apache-2.0 license.
  
Future versions will be MPL-2.0.

Users who prefer MIT/Apache-2.0 can continue using this version."
```

#### 5. Update CONTRIBUTING.md

```markdown
## License

GnawTreeWriter is now licensed under MPL-2.0.

Historical note: Before January 2, 2025, the project was MIT/Apache-2.0.
The license was changed by the sole copyright holder to better protect
the project while enabling commercial use.
```

---

## For Contributors

### If You Contributed Before

**Your code was contributed under MIT/Apache-2.0:**
- ✅ That contribution remains MIT/Apache-2.0 in historical versions
- ⚠️ If Gnaw Software is sole copyright holder, they can relicense
- ⚠️ If you hold copyright, your permission needed

**Standard practice:**
Most open-source projects include a Contributor License Agreement (CLA)
that allows the project to relicense. If you contributed without a CLA,
your contribution is under the license at time of contribution.

### Going Forward

**New contributions:**
- 📄 Will be under MPL-2.0
- 📄 You agree to MPL-2.0 terms by contributing
- 📄 Standard GitHub contribution terms apply

---

## External Contributors Scenario

### If Others Have Contributed

**Check contributors:**

```bash
git log --format='%aN <%aE>' | sort -u
```

**If external contributors exist:**

1. **Count their contributions:**
   ```bash
   git log --author="their-email" --oneline | wc -l
   ```

2. **Options:**
   
   **Option A: Contact them for permission**
   - Ask to relicense their contributions to MPL-2.0
   - Get written agreement
   - Document in repo
   
   **Option B: Revert their contributions**
   - If minimal contributions, remove their code
   - Rewrite those parts yourself
   - Then relicense
   
   **Option C: Dual license**
   - Keep MIT for their contributions
   - MPL for your new work
   - More complex

3. **Contributor License Agreement (CLA) for future:**
   - Require CLA for future contributions
   - CLA allows relicensing
   - Standard practice for projects that may change license

---

## Legal Disclaimer

**This document is NOT legal advice.**

For important legal decisions:
- ✅ Consult a lawyer
- ✅ Especially if external contributors exist
- ✅ Especially if commercially important

This document reflects common open-source practices but does not
constitute legal counsel.

---

## Practical Outcome

### For Users

**If you're using GnawTreeWriter:**

- **v0.3.3 and earlier:** MIT OR Apache-2.0 ✅
- **v0.3.4 and later:** MPL-2.0 ✅
- **Choice:** Use old version (permissive) or new version (copyleft)

### For the Project

**What this achieves:**

- ✅ Clear license transition
- ✅ Historical versions remain MIT (accessible)
- ✅ Future versions protected by MPL-2.0
- ✅ No legal ambiguity

### Timeline

```
v0.3.0 ──────────────── v0.3.3 ────────────── v0.4.0
    │                      │                     │
    └─ MIT/Apache-2.0 ─────┘                     └─ MPL-2.0
                           │
                           └─ Transition point
                              (2025-01-02)
```

---

## Checklist

Before finalizing license change:

- [ ] Verify sole authorship (`git log --format='%aN' | sort -u`)
- [ ] Tag last MIT version (`v0.3.3-mit-final`)
- [ ] Create GitHub release for MIT-final
- [ ] Add License History to README.md
- [ ] Add this document (LICENSE_RETROACTIVE_CHANGE.md)
- [ ] Update CONTRIBUTING.md
- [ ] (Optional) Announce to users/community
- [ ] Proceed with MPL-2.0 for new work

---

## References

- **Copyright Law:** Owner can relicense their own work
- **Git History:** Commits retain their original license context
- **Best Practice:** Clear documentation of transition
- **Example:** Many projects have changed licenses (Redis, MongoDB, etc.)

---

## Summary

**Bottom Line:**

1. **If you're sole author:** You can relicense ✅
2. **Add clear transition documentation** ✅
3. **Tag historical MIT version** ✅
4. **Users can choose** which version to use ✅
5. **Future work is MPL-2.0** ✅

This approach is legally sound and widely used in open source.

---

*Created: 2025-01-02*
*Purpose: Document license transition from MIT/Apache-2.0 to MPL-2.0*