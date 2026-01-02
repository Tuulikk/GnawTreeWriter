# License Comparison for GnawTreeWriter

**Document Purpose:** Help you understand different open-source licenses and choose the right one for GnawTreeWriter.

**Current Status:** Project declares MIT License in `Cargo.toml` but LICENSE file is missing.

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Current License: MIT](#current-license-mit)
3. [License Comparison Matrix](#license-comparison-matrix)
4. [Popular Alternatives](#popular-alternatives)
5. [Commercial Considerations](#commercial-considerations)
6. [Recommendations](#recommendations)
7. [Action Items](#action-items)

---

## Executive Summary

**TL;DR for GnawTreeWriter:**

- ✅ **MIT License (Current)** - Best for maximum adoption and commercial use
- 🟡 **Apache 2.0** - Better patent protection, more corporate-friendly
- 🟡 **MPL 2.0** - Good for code you want kept open while allowing proprietary use
- ❌ **GPL/AGPL** - Restrictive, may limit adoption in commercial settings
- ❌ **Proprietary** - Limits community contributions and trust

**Recommendation:** Stick with MIT for a developer tool, or consider Apache 2.0 for better patent clarity.

---

## Current License: MIT

### What You Declared

```toml
# Cargo.toml
license = "MIT"
```

```markdown
# README.md
MIT License - see LICENSE file for details
```

### What MIT Means

**Permissions:**
- ✅ Commercial use
- ✅ Modification
- ✅ Distribution
- ✅ Private use

**Conditions:**
- 📄 License and copyright notice must be included

**Limitations:**
- ❌ No liability
- ❌ No warranty

### Plain English

MIT is the most permissive license. Anyone can:
- Use GnawTreeWriter in commercial products
- Modify it and not share changes
- Include it in proprietary software
- Sell products that use it

They only need to:
- Include your copyright notice
- Include the MIT license text

They don't need to:
- Share their modifications
- Make their code open source
- Pay you anything

### Example Use Cases Under MIT

**✅ Allowed:**
- Company X builds a proprietary AI coding assistant using GnawTreeWriter
- Company Y modifies GnawTreeWriter and sells it as "ProTreeWriter" (with your license notice)
- Startup Z integrates GnawTreeWriter into their $10,000/month SaaS product
- Developer A forks it, improves it privately, never shares changes

**❌ Not Allowed:**
- Removing your copyright notice
- Claiming they wrote GnawTreeWriter
- Suing you if it breaks

---

## License Comparison Matrix

| Feature                          | MIT | Apache 2.0 | MPL 2.0 | GPL 3.0 | AGPL 3.0 | Proprietary |
|----------------------------------|-----|------------|---------|---------|----------|-------------|
| **Commercial Use**               | ✅   | ✅          | ✅       | ✅       | ✅        | ⚠️ (Your terms) |
| **Modify & Keep Private**        | ✅   | ✅          | ✅       | ❌       | ❌        | ❌          |
| **Include in Proprietary**       | ✅   | ✅          | ⚠️ (File-level) | ❌ | ❌ | ❌ |
| **Patent Grant**                 | ❌   | ✅          | ✅       | ✅       | ✅        | ⚠️          |
| **Trademark Protection**         | ❌   | ✅          | ❌       | ❌       | ❌        | ✅          |
| **Requires Sharing Changes**    | ❌   | ❌          | ✅ (Modified files) | ✅ (All) | ✅ (All + Network) | N/A |
| **License Compatibility**        | High | High       | Medium  | Low     | Very Low | None        |
| **Corporate-Friendly**           | ✅   | ✅          | ✅       | ⚠️       | ❌        | ✅          |
| **Community Adoption**           | Very High | High | Medium | Medium | Low | Low |
| **Length (approx lines)**        | 20  | 200        | 400     | 600     | 650      | Varies      |

Legend:
- ✅ Yes / Allowed / Good
- ❌ No / Not Allowed / Bad
- ⚠️ Conditional / Limited

---

## Popular Alternatives

### 1. Apache License 2.0

**Best for:** Corporate-friendly projects with patent concerns

**Key Differences from MIT:**
- ✅ Explicit patent grant (users can't sue you for patent infringement)
- ✅ Trademark protection (they can't use your name/logo)
- ✅ More legally tested and explicit
- ❌ Longer, more complex (11 pages vs. 1 paragraph)
- ❌ Incompatible with GPL 2.0

**Who Uses It:**
- Rust language
- Android OS
- Apache HTTP Server
- Kubernetes
- TensorFlow

**Example Scenario:**
Your code accidentally implements a patent you hold. Under MIT, users might worry about patent trolling. Under Apache 2.0, you grant them patent rights explicitly.

### 2. Mozilla Public License 2.0 (MPL)

**Best for:** "Copyleft but business-friendly" approach

**Key Differences from MIT:**
- ✅ File-level copyleft (modifications to MPL files must be shared)
- ✅ Can combine with proprietary code (as long as MPL files separate)
- ✅ Patent grant included
- ❌ More complex to understand
- ❌ Less common in tooling ecosystem

**Who Uses It:**
- Firefox
- Thunderbird
- LibreOffice

**Example Scenario:**
Company X can use GnawTreeWriter in proprietary software, but if they modify `parser.rs`, they must share those changes. Their proprietary integration code can stay private.

### 3. GNU GPL 3.0 (Copyleft)

**Best for:** Ensuring all derivatives stay open source

**Key Differences from MIT:**
- ✅ All modifications must be open-sourced
- ✅ Strong copyleft (viral license)
- ✅ Patent protection
- ❌ Cannot be used in proprietary software
- ❌ Less corporate adoption
- ❌ License compatibility issues

**Who Uses It:**
- Linux kernel (GPL 2.0)
- Git
- GCC
- Bash

**Example Scenario:**
Company X wants to use GnawTreeWriter in their proprietary AI tool. They can't, because GPL requires their entire tool to be open-sourced. They look for an alternative.

### 4. GNU AGPL 3.0 (Network Copyleft)

**Best for:** SaaS products you want to keep open

**Key Differences from GPL:**
- ✅ Everything GPL has
- ✅ "Network use is distribution" clause
- ❌ Even MORE restrictive
- ❌ Very low corporate adoption

**Who Uses It:**
- MongoDB (relicensed to SSPL later)
- Grafana (some components)

**Example Scenario:**
Company X runs GnawTreeWriter as a cloud service. Under GPL, they could keep modifications private (no distribution). Under AGPL, they must open-source it because users access it over the network.

**Note:** AGPL is considered toxic by many companies and will limit adoption significantly.

### 5. Dual License (MIT + Commercial)

**Best for:** Open source with paid commercial option

**How It Works:**
- Open source version: MIT (or GPL)
- Commercial version: Proprietary license with support/features
- You own the copyright, can sell exceptions

**Who Uses It:**
- MySQL (GPL + Commercial)
- Qt (LGPL + Commercial)
- GitLab (MIT + Enterprise)

**Example Scenario:**
GnawTreeWriter is MIT for individuals and open source projects. Companies wanting support, indemnification, or proprietary features pay for a commercial license.

### 6. Business Source License (BSL)

**Best for:** "Eventually open source" approach

**How It Works:**
- Restrictive initially (no commercial use without license)
- Converts to open source (e.g., Apache 2.0) after time period
- Protects business model while promising future openness

**Who Uses It:**
- MariaDB
- Sentry
- Couchbase

**Example Scenario:**
GnawTreeWriter released as BSL, converts to Apache 2.0 after 2 years. Prevents competitors from immediately using it commercially while building market position.

---

## Commercial Considerations

### For GnawTreeWriter Specifically

**Current Position:**
- Developer tool (not end-user product)
- Targets LLMs and developers
- Competes with proprietary IDEs and AI tools
- Benefits from community contributions

**Scenarios to Consider:**

#### Scenario 1: Maximum Adoption (MIT)
- ✅ Everyone can use it freely
- ✅ Companies integrate without legal review
- ✅ High adoption, high visibility
- ❌ Competitors can fork and commercialize
- ❌ No patent protection
- ❌ Hard to monetize directly

**Best if:** You want maximum impact and aren't worried about commercialization.

#### Scenario 2: Patent Protection (Apache 2.0)
- ✅ Companies feel safer (explicit patent grant)
- ✅ Trademark protection
- ✅ Still very permissive
- ❌ Slightly less simple than MIT
- ❌ More legal text to read

**Best if:** You want MIT-like freedom with better legal clarity.

#### Scenario 3: Keep Improvements Open (MPL 2.0)
- ✅ Modifications must be shared
- ✅ Can still be used in proprietary products
- ✅ Patents protected
- ❌ More complex to understand
- ❌ Less common in Rust ecosystem

**Best if:** You want to see improvements while allowing commercial use.

#### Scenario 4: Force Everything Open (GPL 3.0)
- ✅ All derivatives must be open source
- ✅ Community-driven development
- ❌ Companies will avoid it
- ❌ Limits adoption significantly
- ❌ May get fewer contributions

**Best if:** You're ideologically committed to open source above all else.

#### Scenario 5: Future Monetization (Dual License)
- ✅ Open source version for community
- ✅ Paid version for companies
- ⚠️ Requires CLA (Contributor License Agreement)
- ❌ Complex to manage
- ❌ May discourage contributions

**Best if:** You have a clear monetization strategy.

---

## Recommendations

### Recommendation 1: Keep MIT ✅ (Easiest)

**Why:**
- Already declared in Cargo.toml
- Most permissive and widely accepted
- Highest adoption potential
- Simple and well-understood
- Perfect for developer tools

**Action:**
1. Add MIT LICENSE file
2. Add copyright headers to key files (optional)
3. Done!

**Risks:**
- No patent protection
- Competitors can fork freely
- Hard to monetize later

### Recommendation 2: Switch to Apache 2.0 ⚠️ (Better Legal Protection)

**Why:**
- Explicit patent grant
- Trademark protection
- Still very permissive
- Corporate-friendly
- Common in Rust ecosystem

**Action:**
1. Update `Cargo.toml`: `license = "Apache-2.0"`
2. Add Apache 2.0 LICENSE file
3. Add NOTICE file (optional)
4. Update README.md

**Risks:**
- Slightly more complex
- Need to do license change PR

### Recommendation 3: Dual License MIT + Apache 2.0 🎯 (Best of Both)

**Why:**
- Common in Rust projects (e.g., Rust itself)
- Users choose which license they prefer
- Maximum compatibility
- Shows you've thought about it

**Action:**
1. Update `Cargo.toml`: `license = "MIT OR Apache-2.0"`
2. Add both LICENSE-MIT and LICENSE-APACHE files
3. Update README.md

**Who Does This:**
- Rust compiler
- Tokio
- Serde
- Many Rust crates

---

## Action Items

### Option A: Keep MIT (Recommended for Now)

```bash
# Create LICENSE file
cat > LICENSE << 'EOF'
MIT License

Copyright (c) 2025 Gnaw Software

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
EOF
```

### Option B: Switch to Apache 2.0

```bash
# Update Cargo.toml
sed -i 's/license = "MIT"/license = "Apache-2.0"/' Cargo.toml

# Download Apache 2.0 license
curl https://www.apache.org/licenses/LICENSE-2.0.txt -o LICENSE

# Create NOTICE file
cat > NOTICE << 'EOF'
GnawTreeWriter
Copyright 2025 Gnaw Software

This product includes software developed at Gnaw Software.
EOF
```

### Option C: Dual License (Rust Standard)

```bash
# Update Cargo.toml
sed -i 's/license = "MIT"/license = "MIT OR Apache-2.0"/' Cargo.toml

# Create MIT license
cat > LICENSE-MIT << 'EOF'
[MIT License text - see Option A]
EOF

# Create Apache license
curl https://www.apache.org/licenses/LICENSE-2.0.txt -o LICENSE-APACHE

# Update README
# Add: "Licensed under either MIT or Apache 2.0 at your option"
```

---

## Dependency License Compatibility

Check your dependencies' licenses:

```bash
cargo tree --prefix none | sort -u
```

**Current Dependencies (from Cargo.toml):**
- tree-sitter: MIT
- tree-sitter-* (languages): MIT
- clap: MIT OR Apache-2.0
- serde: MIT OR Apache-2.0
- anyhow: MIT OR Apache-2.0
- tokio: MIT

**Verdict:** All dependencies are MIT or dual MIT/Apache-2.0 compatible. ✅

**This means:**
- ✅ You can use MIT
- ✅ You can use Apache 2.0
- ✅ You can use dual MIT/Apache-2.0
- ❌ GPL/AGPL would create conflicts

---

## Questions to Ask Yourself

Before deciding, consider:

1. **Do you plan to commercialize GnawTreeWriter directly?**
   - Yes → Consider dual license or BSL
   - No → MIT or Apache 2.0

2. **Are you worried about patent trolls?**
   - Yes → Apache 2.0
   - No → MIT is fine

3. **Do you want derivatives to stay open source?**
   - Yes → MPL 2.0 or GPL 3.0
   - No → MIT or Apache 2.0

4. **Do you care if companies profit from your work?**
   - Yes, I want a cut → Dual license (MIT + Commercial)
   - Yes, but just credit → MIT or Apache 2.0
   - No, that's fine → MIT

5. **Do you want maximum adoption?**
   - Yes → MIT
   - Corporate adoption → Apache 2.0
   - Ideological reasons → GPL

6. **Will you accept contributions from others?**
   - Yes → Keep permissive (MIT/Apache)
   - Maybe → Consider CLA + dual license
   - No → Doesn't matter much

---

## Final Recommendation for GnawTreeWriter

**🎯 Recommended: Dual License (MIT OR Apache-2.0)**

**Why:**
1. ✅ Standard in Rust ecosystem
2. ✅ Maximum compatibility
3. ✅ Corporate-friendly (Apache side)
4. ✅ Simple for individuals (MIT side)
5. ✅ Patent protection available
6. ✅ All dependencies compatible
7. ✅ Easy to implement (just add both files)

**This gives users the choice and shows maturity of the project.**

---

## Additional Resources

- [Choose a License](https://choosealicense.com/) - GitHub's license picker
- [tl;drLegal](https://tldrlegal.com/) - Plain English license summaries
- [Rust Book - Appendix E](https://doc.rust-lang.org/book/appendix-07-newest-features.html) - Licensing in Rust
- [Apache vs MIT](https://www.apache.org/licenses/LICENSE-2.0.html#contributions)

---

**Next Steps:**
1. Review this document
2. Choose a license (recommendation: MIT OR Apache-2.0)
3. Add LICENSE file(s)
4. Update README if needed
5. Commit and push before crates.io publication

---

*Document created: 2025-01-02*
*For: GnawTreeWriter project licensing decision*