# MPL 2.0 Quick Reference Guide for GnawTreeWriter

**Document Purpose:** Explain what MPL 2.0 means for GnawTreeWriter users and contributors.

---

## For Users: What You Can Do

### ✅ You CAN:

1. **Use GnawTreeWriter commercially**
   - Build products that use it
   - Sell software that includes it
   - Use it in your company
   - Charge for services built with it

2. **Modify GnawTreeWriter**
   - Fix bugs
   - Add features
   - Customize for your needs
   - Fork the project

3. **Combine with proprietary code**
   - Add your own proprietary modules
   - Integrate with closed-source products
   - Keep your integration code private

4. **Distribute modified versions**
   - Share your improvements
   - Create derivative works

### 📋 You MUST:

1. **Share modifications to GnawTreeWriter files**
   - If you modify any existing `.rs` file, you must share those changes
   - Share under MPL 2.0 license
   - Include source code with your distribution

2. **Keep license notices**
   - Include the MPL 2.0 license text
   - Keep copyright notices in files
   - Provide access to source code of modified MPL files

3. **Document your changes**
   - Note what you changed in NOTICE file (recommended)
   - Make source available (same as or similar to original distribution)

---

## File-Level Copyleft: The Key Concept

MPL 2.0 works at the **file level**, not project level.

### Example Scenario

```
YourProduct/
├── gnawtreewriter/           ← MPL 2.0 files
│   ├── src/
│   │   ├── parser.rs         ← You modified this
│   │   ├── core.rs           ← You modified this
│   │   └── cli.rs            ← You didn't touch this
└── your_code/                ← Your proprietary code
    ├── integration.rs        ← NEW FILE - can be proprietary! ✅
    ├── cloud_sync.rs         ← NEW FILE - can be proprietary! ✅
    └── business_logic.rs     ← NEW FILE - can be proprietary! ✅
```

**What you must share:**
- ✅ Modified `parser.rs` (you changed it)
- ✅ Modified `core.rs` (you changed it)
- ❌ NOT `integration.rs` (new file, your code)
- ❌ NOT `cloud_sync.rs` (new file, your code)
- ❌ NOT `business_logic.rs` (new file, your code)

**Key Rule:** If you modify an MPL file, share it. If you create a new file, it's yours.

---

## Common Use Cases

### Case 1: Use As-Is

**Scenario:** You use GnawTreeWriter without modifications.

**Requirements:**
- ✅ Include LICENSE file
- ✅ That's it!

**Example:**
```bash
# Your product includes GnawTreeWriter binary
./your_product --use-gnawtreewriter
```

No code sharing required (you didn't modify anything).

---

### Case 2: Modify Core Files

**Scenario:** You improve the Python parser in `src/parser/python.rs`.

**Requirements:**
- ✅ Share your modified `python.rs` under MPL 2.0
- ✅ Make source available to your users
- ✅ Include LICENSE and NOTICE

**Example:**
```rust
// src/parser/python.rs (your improved version)
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub struct PythonParser {
    // Your improvements here
}
```

You must share this file, but not your entire product.

---

### Case 3: Add Proprietary Integration

**Scenario:** You create `src/integration/enterprise_features.rs` (new file).

**Requirements:**
- ❌ NO requirement to share this file
- ✅ It's a new file, you own it
- ✅ Can be proprietary

**Example:**
```
src/
├── parser/
│   └── python.rs          ← MPL 2.0 (existing)
└── integration/           ← NEW DIRECTORY
    └── enterprise.rs      ← Can be proprietary! ✅
```

The new file can have any license you want.

---

### Case 4: Build Commercial Product

**Scenario:** You build "ProTreeWriter Enterprise" using GnawTreeWriter.

**Allowed:**
- ✅ Sell your product
- ✅ Add proprietary features (in new files)
- ✅ Charge for support
- ✅ Keep business logic private

**Required:**
- ✅ Share any modifications to original GnawTreeWriter files
- ✅ Include MPL 2.0 license
- ✅ Provide source for modified MPL files

**Example:**
```
ProTreeWriter Enterprise/
├── gnawtreewriter/        ← MPL 2.0 (must share if modified)
└── enterprise/
    ├── cloud_sync.rs      ← Proprietary ✅
    ├── team_features.rs   ← Proprietary ✅
    └── ai_assistant.rs    ← Proprietary ✅
```

You can sell this! Just share modifications to GnawTreeWriter files.

---

## For Contributors

### What Happens to Your Contributions?

When you contribute code to GnawTreeWriter:

1. ✅ Your contribution becomes MPL 2.0
2. ✅ You retain copyright
3. ✅ Everyone can use your code under MPL 2.0 terms
4. ✅ Others must share their modifications to your code

### Example Contribution

```rust
// src/parser/new_language.rs
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2025 Your Name

pub struct NewLanguageParser {
    // Your contribution
}
```

---

## Comparison with Other Licenses

### vs MIT/Apache (what we had before)

| Aspect | MIT/Apache | MPL 2.0 |
|--------|------------|---------|
| **Modify and keep private?** | ✅ Yes | ❌ No (must share modified files) |
| **Use commercially?** | ✅ Yes | ✅ Yes |
| **Add proprietary modules?** | ✅ Yes | ✅ Yes (new files) |
| **Patent protection?** | ⚠️ (Apache only) | ✅ Yes |
| **Core improvements shared?** | ❌ No | ✅ Yes |

**MPL 2.0 is better for the project:** Core improvements come back to community.

### vs GPL 3.0

| Aspect | GPL 3.0 | MPL 2.0 |
|--------|---------|---------|
| **Modify and keep private?** | ❌ No (must share everything) | ⚠️ No (must share modified files) |
| **Use commercially?** | ⚠️ Only if you open source | ✅ Yes |
| **Add proprietary modules?** | ❌ No | ✅ Yes (new files) |
| **Scope** | Project-wide (viral) | File-level |
| **Corporate adoption** | Low | Medium-High |

**MPL 2.0 is more flexible:** File-level vs project-level copyleft.

---

## How to Comply

### For Users

1. **Include LICENSE file**
   ```bash
   cp LICENSE /path/to/your/distribution/
   ```

2. **If you modified MPL files, create NOTICE**
   ```
   This product includes modified files from GnawTreeWriter:
   - src/parser/python.rs (added async support)
   - src/core/mod.rs (improved error handling)
   
   Source code available at: https://github.com/yourcompany/gnawtreewriter-fork
   ```

3. **Make modified source available**
   - GitHub repository (easiest)
   - Download link on your website
   - Include with your distribution

### For Contributors

1. **Add license header to new files** (optional but recommended):
   ```rust
   // This Source Code Form is subject to the terms of the Mozilla Public
   // License, v. 2.0. If a copy of the MPL was not distributed with this
   // file, You can obtain one at https://mozilla.org/MPL/2.0/.
   ```

2. **Contribute via GitHub Pull Request**
   - Your contribution automatically becomes MPL 2.0
   - You agree to MPL 2.0 terms by contributing

---

## Real-World Examples

### Firefox (MPL 2.0)

- Core browser engine: MPL 2.0
- Companies can use it (Chromium does)
- Must share improvements to core files
- Can add proprietary features on top

### LibreOffice (MPL 2.0)

- Office suite core: MPL 2.0
- Companies build products with it
- Extensions can be proprietary
- Core improvements are shared

### GnawTreeWriter (MPL 2.0)

- Parser engine, core functionality: MPL 2.0
- You can build commercial products
- Integrations can be proprietary
- Parser improvements must be shared

---

## FAQ

### Q: Can I use GnawTreeWriter in my commercial product?
**A:** Yes! Absolutely.

### Q: Do I have to open-source my entire product?
**A:** No! Only modifications to GnawTreeWriter files. Your code stays private.

### Q: Can I sell a product that uses GnawTreeWriter?
**A:** Yes! You can charge money for products built with GnawTreeWriter.

### Q: What if I improve the parser?
**A:** You must share that improvement under MPL 2.0.

### Q: What if I add a new cloud-sync module?
**A:** New files can be proprietary. You don't have to share them.

### Q: Can I fork GnawTreeWriter?
**A:** Yes! But your fork must also be MPL 2.0 for the original files.

### Q: Can I create a competing product?
**A:** Yes, but you must share improvements to GnawTreeWriter's files.

### Q: What about my proprietary AI features?
**A:** If they're in new files, they can be proprietary.

### Q: Do I need a lawyer?
**A:** For commercial use, consulting a lawyer is recommended. This guide is not legal advice.

### Q: How do I provide "source code"?
**A:** GitHub repo, download link, or include with distribution. Make it easy to access.

---

## Why This License?

**Gnaw Software chose MPL 2.0 because:**

1. ✅ **Protects core work** - Parser improvements come back to community
2. ✅ **Enables commercial use** - Build products without fear
3. ✅ **Clear boundaries** - File-level = easy to understand
4. ✅ **Proven model** - Firefox, LibreOffice use it successfully
5. ✅ **Patent protection** - Protects everyone

**What we're preventing:**
- ❌ Big company takes code, improves it, never shares
- ❌ Proprietary fork that outcompetes original
- ❌ Community loses access to improvements

**What we're allowing:**
- ✅ Commercial products built with GnawTreeWriter
- ✅ Proprietary integrations and features
- ✅ Selling services and support
- ✅ Building businesses around it

---

## Resources

- **Official MPL 2.0 Text:** https://www.mozilla.org/MPL/2.0/
- **MPL 2.0 FAQ:** https://www.mozilla.org/MPL/2.0/FAQ/
- **Mozilla License Policy:** https://www.mozilla.org/MPL/
- **GnawTreeWriter LICENSE:** See LICENSE file in repository

---

## Summary

**MPL 2.0 in one sentence:**

> You can use GnawTreeWriter in commercial products and keep your code private, but if you improve GnawTreeWriter itself, share those improvements.

**The balance:**
- Your business logic: Private ✅
- Your integrations: Private ✅
- GnawTreeWriter improvements: Shared ✅

This protects the project while enabling commercial success for everyone.

---

*Last Updated: 2025-01-02*
*License: This document is CC0 (public domain)*