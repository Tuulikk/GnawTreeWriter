# AI Agent Test Scenarios: The Architect's Journey

This document defines scenarios for evaluating GnawTreeWriter within the **TCARV 1.0** methodology. These tasks are designed to test precision, logic-first thinking, and structural integrity.

## Test Levels: The Path to Verification

### Level 1: The Draftsman (Hypothesis & Precision)
- **Objective**: Map existing logic to text and perform surgical edits.
- **Goal**: Analyze a file, create a "Text-App" description of a function, and then update a single value or rename a local variable.
- **Success Criteria**: The logic description matches the code, and the edit applies without breaking the AST.

### Level 2: The Mason (Building Blocks)
- **Objective**: Structural expansion following TCARV Step 2.
- **Goal**: Add a new method to a class or a new field to a struct by first defining the "Kloss" (block) in pseudocode.
- **Success Criteria**: Valid syntax post-insertion and correct integration with the "Shell".

### Level 3: The Master Builder (Coordinated Systems)
- **Objective**: Coordinated multi-file changes (TCARV Step 4).
- **Goal**: Refactor a shared interface or rename a symbol across multiple files using a batch operation.
- **Success Criteria**: All files remain syntactically valid and the "Text-App" remains the single source of truth.

### Level 4: The Restorer (Recursive Verification)
- **Objective**: Recovery and refactoring of legacy code.
- **Goal**: Take a "Legacy-monolit", destistill its logic into a Text-App, and break out one part into a verified "Kloss".
- **Success Criteria**: The new modular kloss is verified by tests, and the old code is successfully updated to use it.

---

## Active Bounties (The TCARV Challenge)
- [ ] **Bounty #1**: Create a Retroactive Text-App for `src/mcp/mod.rs` and implement a health-check endpoint (Level 2).
- [ ] **Bounty #2**: Extract the error handling logic from `src/core/` into an isolated "Kloss" with its own tests (Level 3).
- [ ] **Bounty #3**: Refactor a complex module by first creating a full backup (`.full_bak`) and then rebuilding it piece-by-piece (Level 4).