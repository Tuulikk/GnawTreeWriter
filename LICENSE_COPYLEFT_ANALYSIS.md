# Copyleft License Analysis for GnawTreeWriter

**Your Concern:** "A larger entity can take my code, improve it privately, and outcompete me or the open community."

**Status:** VALID CONCERN - MIT/Apache 2.0 allows this. Here are licenses that prevent it.

---

## Table of Contents

1. [The Problem with Permissive Licenses](#the-problem-with-permissive-licenses)
2. [Copyleft Solutions](#copyleft-solutions)
3. [License Comparison for Your Use Case](#license-comparison-for-your-use-case)
4. [Recommended Solution: MPL 2.0](#recommended-solution-mpl-20)
5. [Alternative: GPL 3.0](#alternative-gpl-30)
6. [Strongest Protection: AGPL 3.0](#strongest-protection-agpl-30)
7. [Real-World Examples](#real-world-examples)
8. [Trade-offs](#trade-offs)
9. [Final Recommendation](#final-recommendation)

---

## The Problem with Permissive Licenses

### MIT / Apache 2.0 Scenario (Current)

**What Microsoft/Google/Big Corp Can Do:**

1. ✅ Fork GnawTreeWriter
2. ✅ Add proprietary features (AI integration, cloud sync, premium parsers)
3. ✅ Never share those improvements
4. ✅ Sell "ProTreeWriter Enterprise" for $1000/seat
5. ✅ Outcompete you with their marketing budget
6. ✅ Hire your contributors with those profits

**What They Must Do:**
- 📄 Include your copyright notice (that's it)

**Result:** 
- ❌ You get no improvements back
- ❌ Community gets no improvements back
- ❌ They profit from your work
- ❌ Original project becomes irrelevant

### Historical Example: ElasticSearch

**What Happened:**
1. ElasticSearch was Apache 2.0 licensed
2. AWS took it, created "Amazon Elasticsearch Service"
3. AWS didn't contribute improvements back
4. AWS made millions while Elastic got nothing
5. Elastic was forced to change to SSPL (proprietary-ish license)

**This is exactly what you want to prevent.**

---

## Copyleft Solutions

"Copyleft" = If you modify my code, you must share your modifications under the same license.

### Strength Levels

```
Permissive          Weak Copyleft      Strong Copyleft    Network Copyleft
    MIT    <    MPL 2.0    <    GPL 3.0    <    AGPL 3.0
    
   "Do                "Share              "Share           "Share even
  whatever"         modified files"     everything"      for SaaS"
```

---

## License Comparison for Your Use Case

### Scenario: Microsoft wants to use GnawTreeWriter

| License | Can use? | Must share improvements? | Can sell proprietary version? |
|---------|----------|--------------------------|-------------------------------|
| **MIT** | ✅ Yes | ❌ No | ✅ Yes - outcompetes you |
| **Apache 2.0** | ✅ Yes | ❌ No | ✅ Yes - outcompetes you |
| **MPL 2.0** | ✅ Yes | ✅ Yes (modified files only) | ⚠️ Partially (can add proprietary modules) |
| **GPL 3.0** | ⚠️ Yes (but...) | ✅ Yes (entire codebase) | ❌ No - must open source everything |
| **AGPL 3.0** | ⚠️ Yes (but...) | ✅ Yes (even if SaaS) | ❌ No - even cloud services must be open |

---

## Recommended Solution: MPL 2.0

**Mozilla Public License 2.0 - "Business-Friendly Copyleft"**

### What It Prevents

❌ Microsoft cannot:
- Fork GnawTreeWriter
- Improve the parser engine
- Keep those improvements private
- Sell it as "MS CodeTree Pro"

✅ They MUST share all modifications to MPL-licensed files

### What It Allows

✅ Microsoft CAN:
- Use GnawTreeWriter in their products
- Add proprietary integration modules (separate files)
- Combine with proprietary code
- Sell commercial products that USE it

**Key Difference:** File-level copyleft

- **Modified MPL files** → Must be shared (open source)
- **New separate files** → Can be proprietary

### Example Scenario

**Microsoft wants to add cloud features:**

```
GnawTreeWriter/
├── src/
│   ├── parser.rs          ← MPL 2.0 (your file)
│   ├── core.rs            ← MPL 2.0 (your file)
│   └── microsoft/         ← NEW FILES
│       ├── cloud_sync.rs  ← Can be proprietary! ✅
│       └── azure_auth.rs  ← Can be proprietary! ✅
```

**If they modify `parser.rs`:**
- ❌ Cannot keep private
- ✅ Must share back to community under MPL 2.0

**If they add `cloud_sync.rs`:**
- ✅ Can keep proprietary (it's a new file)
- ✅ Can sell as premium feature

### Benefits of MPL 2.0

✅ **Protects your core work**
- Your parser engine improvements come back to you
- Your core algorithms stay open

✅ **Allows commercial use**
- Companies can build products around it
- They can add proprietary features
- Higher adoption than GPL

✅ **File-level flexibility**
- Clear boundary: modified files = shared
- New files = their choice

✅ **Patent protection**
- Explicit patent grant (like Apache 2.0)

### Who Uses MPL 2.0

- **Firefox** - Browser engine
- **Thunderbird** - Email client
- **LibreOffice** - Office suite
- **Servo** - Rendering engine

**Pattern:** Core technology that others build upon.

---

## Alternative: GPL 3.0

**GNU General Public License 3.0 - "Strong Copyleft"**

### What It Prevents

❌ Microsoft cannot:
- Fork GnawTreeWriter
- Add ANY improvements (core or integration)
- Keep ANYTHING private
- Sell proprietary version

✅ They MUST share EVERYTHING under GPL 3.0

### What "Everything" Means

**GPL is viral:** If they link to your code, their code becomes GPL too.

```
Microsoft Product/
├── gnawtreewriter/   ← GPL 3.0
└── microsoft_code/   ← MUST ALSO BE GPL 3.0 ❗
    ├── proprietary_parser.rs  ← Nope! Must be GPL
    └── cloud_features.rs      ← Nope! Must be GPL
```

**Result:** Most companies won't use it commercially.

### Benefits of GPL 3.0

✅ **Maximum protection**
- ALL improvements come back
- No proprietary forks possible
- Strong community ecosystem

✅ **Ideological alignment**
- Everything stays free software
- Corporate profit → community benefit

✅ **Patent protection**
- Strong patent grant and retaliation

### Drawbacks of GPL 3.0

❌ **Lower adoption**
- Many companies avoid GPL
- Cannot integrate in proprietary products
- Smaller ecosystem

❌ **Contributor friction**
- Companies may not contribute
- Individual devs might avoid it

❌ **Compatibility issues**
- Hard to combine with MIT/Apache code
- Fewer libraries to use

### Who Uses GPL 3.0

- **Linux kernel** (GPL 2.0)
- **Git** - Version control
- **GCC** - Compiler
- **Bash** - Shell

**Pattern:** Core infrastructure, strong community commitment.

---

## Strongest Protection: AGPL 3.0

**Affero GPL 3.0 - "Network Copyleft"**

### What It Adds Beyond GPL

**GPL loophole:** If you run software as a service (SaaS), you don't "distribute" it, so GPL doesn't apply.

**AGPL closes this:**
- ✅ Running GnawTreeWriter as a cloud service = distribution
- ✅ Must share code even for SaaS

### Example Scenario

**AWS wants to offer "GnawTreeWriter as a Service":**

**Under GPL 3.0:**
- ✅ They can run it as a service
- ❌ Don't have to share improvements (no "distribution")
- ✅ Profit from your work

**Under AGPL 3.0:**
- ✅ They can run it as a service
- ✅ MUST share all code (network use = distribution)
- ✅ Improvements come back to you

### Drawbacks of AGPL 3.0

❌ **Extremely low adoption**
- Many companies ban AGPL outright
- Corporate legal departments hate it
- Very small ecosystem

❌ **Contributor chilling effect**
- Many devs avoid AGPL projects
- Hard to get contributions

❌ **May be overkill**
- GnawTreeWriter is a CLI tool, not SaaS
- AGPL makes sense for databases, not dev tools

### Who Uses AGPL 3.0

- **MongoDB** (switched away to SSPL)
- **Grafana** (some components)
- Few others (it's rare)

**Warning:** AGPL is considered "toxic" by many companies.

---

## Real-World Examples

### Mozilla Firefox (MPL 2.0)

**Scenario:**
- Core browser engine: MPL 2.0
- Google, Microsoft, others can use it
- Must share improvements to core engine
- Can add proprietary features on top

**Result:**
- ✅ Chromium uses parts (shared back improvements)
- ✅ Wide adoption
- ✅ Core stays open

### Linux Kernel (GPL 2.0)

**Scenario:**
- Entire kernel: GPL
- Companies must share ALL modifications
- Cannot create proprietary forks

**Result:**
- ✅ All Android improvements contributed back
- ✅ All server improvements contributed back
- ✅ Strong community
- ⚠️ Some companies avoid it (use BSD instead)

### MongoDB (was AGPL, now SSPL)

**Scenario:**
- Started as AGPL 3.0
- AWS created "Amazon DocumentDB" (MongoDB-compatible)
- AWS didn't share improvements
- MongoDB changed to SSPL (not open source)

**Result:**
- ❌ AGPL didn't prevent AWS (they rewrote compatible API)
- ❌ MongoDB lost community trust with SSPL
- ⚠️ Complex legal situation

---

## Trade-offs

### Adoption vs Protection Matrix

```
            High Adoption          |         High Protection
                                   |
    MIT, Apache 2.0                |              AGPL 3.0
         ↓                         |                  ↑
    Easy corporate use             |        SaaS must share
    No improvements back           |        Very low adoption
                                   |
         ↓                         |                  ↑
      MPL 2.0                      |              GPL 3.0
         ↓                         |                  ↑
    File-level copyleft            |        Strong copyleft
    Good balance                   |        Medium adoption
                                   |
         ↓                         |                  ↑
    [Sweet spot for tools]         |     [Ideological choice]
```

### For GnawTreeWriter Specifically

**Your Goals (stated):**
1. ❌ Prevent big companies from taking improvements
2. ✅ Allow community to use freely
3. ✅ Ensure improvements come back
4. ⚠️ Still want some adoption

**Recommendation: MPL 2.0** ✅

**Why:**
- ✅ Core improvements MUST be shared
- ✅ Companies can still build products
- ✅ Better than MIT/Apache (your concern)
- ✅ Not as restrictive as GPL (good adoption)

---

## Final Recommendation

### Best Choice: MPL 2.0 🎯

**Change to MPL 2.0 because:**

1. **Addresses your concern:**
   - ❌ Big companies cannot improve your parser privately
   - ✅ All core improvements come back to community

2. **Still business-friendly:**
   - ✅ Companies can use it
   - ✅ Can add proprietary modules
   - ✅ Better adoption than GPL

3. **Clear boundaries:**
   - Modified files = must share
   - New files = their choice
   - Easy to understand

4. **Patent protection:**
   - Explicit patent grant (like Apache)

5. **Proven track record:**
   - Firefox, LibreOffice use it successfully

### Alternative: GPL 3.0 (If You're Okay with Lower Adoption)

**Choose GPL 3.0 if:**
- ✅ You want MAXIMUM protection
- ✅ You're okay with companies avoiding it
- ✅ Ideological commitment to free software
- ✅ Don't care about commercial adoption

### DON'T Choose: AGPL 3.0

**Reasons:**
- ❌ Too restrictive for a CLI tool
- ❌ Will kill adoption
- ❌ Not necessary (GnawTreeWriter isn't SaaS)

---

## Implementation Steps

### Option 1: Switch to MPL 2.0 (Recommended)

```bash
# Remove current licenses
rm LICENSE-MIT LICENSE-APACHE

# Download MPL 2.0
curl https://www.mozilla.org/media/MPL/2.0/index.txt -o LICENSE

# Update Cargo.toml
sed -i 's/license = "MIT OR Apache-2.0"/license = "MPL-2.0"/' Cargo.toml

# Update README.md
# Change license section to:
# "Licensed under the Mozilla Public License 2.0 - see LICENSE file"

# Add file headers (recommended)
# Add to each .rs file:
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
```

### Option 2: Switch to GPL 3.0 (Maximum Protection)

```bash
# Remove current licenses
rm LICENSE-MIT LICENSE-APACHE

# Download GPL 3.0
curl https://www.gnu.org/licenses/gpl-3.0.txt -o LICENSE

# Update Cargo.toml
sed -i 's/license = "MIT OR Apache-2.0"/license = "GPL-3.0"/' Cargo.toml

# Update README.md
# Add GPL notice and "see LICENSE file"

# Add copyright headers
# Add to each .rs file:
# Copyright (C) 2025 Gnaw Software
# This program is free software: you can redistribute it and/or modify...
```

---

## License Compatibility Check

### MPL 2.0 with Your Dependencies

**Current dependencies are MIT/Apache:**
- tree-sitter: MIT ✅
- clap: MIT OR Apache-2.0 ✅
- serde: MIT OR Apache-2.0 ✅
- anyhow: MIT OR Apache-2.0 ✅

**Verdict:** ✅ MPL 2.0 is compatible with all MIT/Apache dependencies

### GPL 3.0 with Your Dependencies

**Same dependencies:**
- MIT/Apache are compatible with GPL ✅
- BUT: Your project becomes GPL (viral)

**Verdict:** ✅ Compatible, but makes GnawTreeWriter GPL-only

---

## Summary Table

| Concern | MIT/Apache | MPL 2.0 | GPL 3.0 |
|---------|------------|---------|---------|
| **Big company takes code** | ❌ Allowed | ⚠️ Must share core changes | ✅ Cannot keep anything private |
| **Community gets improvements** | ❌ No guarantee | ✅ Yes (modified files) | ✅ Yes (everything) |
| **Commercial use allowed** | ✅ Yes | ✅ Yes (with sharing) | ⚠️ Only if they open source too |
| **Corporate adoption** | ✅ High | ✅ Medium-High | ❌ Low |
| **Patent protection** | ⚠️ (Apache only) | ✅ Yes | ✅ Yes |
| **Your work protected** | ❌ No | ✅ Core work yes | ✅ Everything yes |

---

## My Recommendation for GnawTreeWriter

**Switch to MPL 2.0** 🎯

**Reasoning:**
1. Addresses your valid concern about proprietary forks
2. Ensures core improvements come back to you
3. Still allows commercial use (better adoption than GPL)
4. File-level copyleft is clear and enforceable
5. Compatible with all your dependencies
6. Used by successful projects (Firefox model)

**Next Steps:**
1. Review this document
2. Decide: MPL 2.0 or GPL 3.0
3. I'll help you implement the change
4. Update all documentation
5. Consider adding file headers

**Question for you:**
- Do you want MPL 2.0 (balanced) or GPL 3.0 (maximum protection)?
- Are you okay with potentially lower corporate adoption for stronger protection?

---

*Created: 2025-01-02*
*Purpose: Help choose copyleft license to prevent proprietary forks*