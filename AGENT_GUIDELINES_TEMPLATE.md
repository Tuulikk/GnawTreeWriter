# Agent Guidelines Template

**Template for guiding AI agents to use your CLI tool through dogfooding**

Version: 1.0 | Last Updated: 2025-01-10

---

## Overview

This template provides a structured approach for guiding AI agents to contribute to your project by using your tool to develop itself ("dogfooding"). Replace `[BRACKETS]` with project-specific information.

---

## 🎯 Purpose

AI agents working on this project **MUST** use `[PROJECT_NAME]` as their primary editing tool. This practice:

- Validates tool functionality through real usage
- Discovers edge cases and usability issues
- Improves tool design based on agent feedback
- Ensures contributions are consistent with project architecture

---

## 🚨 CRITICAL REQUIREMENTS

### 1. Primary Tool Usage

**AI Agents MUST:**

- Use `[PROJECT_NAME]` for ALL edits to `[PROJECT_TYPE]` files
- Use `[PROJECT_NAME]` for ALL multi-file operations
- Use `[PROJECT_NAME]` for ALL refactoring operations
- Validate changes with `[PROJECT_NAME]` before applying
- Log all operations using `[PROJECT_NAME]`'s session tracking

**AI Agents MUST NOT:**

- Fall back to standard text editing tools (`edit`, `write`) for `[PROJECT_TYPE]` files without explicit justification
- Perform multi-file edits without using `[PROJECT_NAME]`'s batch/atomic operations
- Edit project files directly without previewing changes first
- Skip validation steps provided by `[PROJECT_NAME]`

### 2. Required Workflow

Before making ANY changes:

```bash
# 1. ALWAYS analyze the code structure first
[COMMAND_PREFIX] analyze <file>

# 2. ALWAYS list relevant nodes/types
[COMMAND_PREFIX] list <file> --filter-type <type>

# 3. ALWAYS preview changes before applying
[COMMAND_PREFIX] [edit_command] <file> <params> --preview

# 4. Apply the previewed change
[COMMAND_PREFIX] [edit_command] <file> <params>

# 5. ALWAYS validate after changes
[COMMAND_PREFIX] validate <file>
```

### 3. Session Management

**REQUIRED for all editing sessions:**

```bash
# Start session before work
[COMMAND_PREFIX] session-start

# After making changes
[COMMAND_PREFIX] history

# Rollback if needed
[COMMAND_PREFIX] undo --steps <n>
```

---

## 📋 Valid Exceptions

The only cases where AI agents MAY use standard editing tools:

1. **Non-[PROJECT_TYPE] files**: Documentation (`.md`, `.txt`), config files (`.json`, `.yaml`, `.toml`) unless `[PROJECT_NAME]` supports them
2. **New files**: Creating files that don't exist yet
3. **Emergency fixes**: Explicitly marked with `# EMERGENCY FIX` comment explaining why `[PROJECT_NAME]` couldn't be used
4. **Tool bootstrap**: When the tool itself doesn't work and needs fixing (document with `# BOOTSTRAP FIX`)

**Note**: All exceptions MUST include a comment explaining why `[PROJECT_NAME]` wasn't used.

---

## 🧪 Validation Requirements

### Self-Validation Checklist

After making changes, AI agents MUST verify:

```bash
# 1. Check that operations were logged
[COMMAND_PREFIX] history

# 2. Verify syntax is valid
[COMMAND_PREFIX] validate <file>

# 3. Run project tests
[TEST_COMMAND]

# 4. Build if applicable
[BUILD_COMMAND]
```

### Transaction Log Verification

```bash
# Verify all edits are in the log
cat [TRANSACTION_LOG_PATH]

# Count operations - should match expectations
grep -c "operation_type" [TRANSACTION_LOG_PATH]
```

---

## 📚 Agent Scenarios

### Scenario 1: Adding a New Feature

**Requirement**: Use `[PROJECT_NAME]` for ALL code changes

```bash
# Analyze relevant files
[COMMAND_PREFIX] analyze src/module.rs

# List existing patterns
[COMMAND_PREFIX] list src/module.rs --filter-type function_definition

# Make changes (preview first!)
[COMMAND_PREFIX] edit src/module.rs <path> <content> --preview
[COMMAND_PREFIX] edit src/module.rs <path> <content>

# Validate
[COMMAND_PREFIX] validate src/module.rs
[TEST_COMMAND]
```

### Scenario 2: Multi-File Refactoring

**Requirement**: MUST use batch/atomic operations

```bash
# Create batch specification
cat > refactor_batch.json << 'EOF'
[BATCH_SPEC_TEMPLATE]
EOF

# Preview batch
[COMMAND_PREFIX] batch refactor_batch.json --preview

# Apply atomically
[COMMAND_PREFIX] batch refactor_batch.json

# Verify
[COMMAND_PREFIX] history
```

### Scenario 3: Bug Fix

**Requirement**: Use `[PROJECT_NAME]` for precise edits

```bash
# Find the bug location
[COMMAND_PREFIX] list <file> --filter-type <type>

# Make targeted edit
[COMMAND_PREFIX] [quick_edit] <file> --node <path> --content <fix> --preview
[COMMAND_PREFIX] [quick_edit] <file> --node <path> --content <fix>

# Verify fix
[COMMAND_PREFIX] validate <file>
[TEST_COMMAND]
```

---

## 🎯 Agent Performance Metrics

Agents will be evaluated on:

### Primary Metrics (70%)

1. **Tool Adoption Rate**
   - % of edits made using `[PROJECT_NAME]` vs. fallback tools
   - Target: > 95% usage of `[PROJECT_NAME]`

2. **Session Consistency**
   - % of sessions with proper session-start/session-end
   - Target: 100%

3. **Preview Discipline**
   - % of edits with --preview flag before applying
   - Target: > 90%

### Secondary Metrics (30%)

1. **Validation Compliance**
   - % of changes followed by validation
   - Target: > 95%

2. **Test Coverage**
   - % of changes with corresponding tests
   - Target: > 80%

3. **Error Rate**
   - % of changes requiring rollback
   - Target: < 10%

---

## ⚠️ Consequences for Non-Compliance

### Level 1: Warning (First Offense)

- Explicit reminder in agent feedback
- Review of the specific issue
- Opportunity to correct

### Level 2: Performance Impact

- Lower performance scores in evaluation
- Additional verification steps required
- Detailed review of subsequent changes

### Level 3: Rejection

- Changes that don't use `[PROJECT_NAME]` where required will be rejected
- Agent may be restricted from future sessions
- Manual intervention required

---

## 🚀 Project-Specific Configuration

### Project Details

```yaml
project_name: "[PROJECT_NAME]"
version: "[CURRENT_VERSION]"
primary_language: "[LANGUAGE/RUST/PYTHON/ETC]"
supported_file_types:
  - "[EXTENSION1]"
  - "[EXTENSION2]"
```

### Tool Commands

```bash
# Installation
[INSTALL_COMMAND]

# Available commands
[COMMAND_PREFIX] analyze <file>              # Analyze file structure
[COMMAND_PREFIX] list <file> --filter-type  # List nodes by type
[COMMAND_PREFIX] edit <file> <path>         # Edit node by path
[COMMAND_PREFIX] batch <file>                # Multi-file batch operations
[COMMAND_PREFIX] quick <file>               # Quick single-file edits
[COMMAND_PREFIX] validate <file>            # Validate syntax
[COMMAND_PREFIX] session-start              # Start editing session
[COMMAND_PREFIX] history                    # Show operation history
[COMMAND_PREFIX] undo                       # Undo operations
```

### File Types Requiring `[PROJECT_NAME]`

- **MUST use**: `[LIST_EXTENSIONS]`
- **MAY use**: `[LIST_EXTENSIONS]`
- **Use fallback**: `[LIST_EXTENSIONS]`

### Testing

```bash
# Run all tests
[TEST_COMMAND]

# Run specific test
[TEST_COMMAND] <test_name>

# Build command
[BUILD_COMMAND]
```

---

## 💡 Best Practices for Agents

### 1. Think Before You Act

- Analyze the code structure first
- Understand the existing patterns
- Plan your changes before making them

### 2. Always Preview

- NEVER apply changes without preview
- Review the preview output carefully
- Verify the changes match your intent

### 3. Use Sessions

- Start sessions for all work
- Track your operations
- Use undo when things go wrong

### 4. Validate Everything

- Check syntax after edits
- Run tests after changes
- Verify the build still works

### 5. Document Your Work

- Leave clear comments
- Update relevant documentation
- Explain why you made changes

---

## 📝 Change History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-01-10 | Initial template |

---

## 🔧 Adaptation Guide

To adapt this template for your project:

1. **Replace all `[BRACKETED]` sections** with project-specific information
2. **Update command examples** to match your tool's CLI
3. **Adjust file type lists** to match your supported formats
4. **Customize the scenarios** to reflect common workflows in your project
5. **Add project-specific rules** in the exceptions section if needed
6. **Update metrics** to align with your quality standards

---

## 🤝 Contributing

This is a template meant to be adapted. Share your improvements back to the community to help other projects benefit from agent dogfooding!

---

*This template is inspired by the GnawTreeWriter project's dogfooding approach to AI agent development.*