# Known Issues and Limitations

**Current as of GnawTreeWriter v0.9.8** *(gått igenom och verifierad mot koden 2026-10-04)*

This document tracks known issues, limitations, and workarounds based on real-world usage and AI agent testing feedback.

> **Aktuella utvecklingsluckor** (vad som ska byggas/fixas härnäst) finns i
> [ROADMAP.md](ROADMAP.md) **Phase 9: Agent Adoption & MCP Parity** (inplockad ur
> `GAP-REPORT-2026-10-04.md`). Detta dokument täcker kända begränsningar i befintlig funktionalitet.

---

## 📋 **Directory Analysis Issues**

### **Issue**: Directory paths without `--recursive` require explicit flag
**Status**: Intentional design decision (not a bug)  
**Behavior**: `gnawtreewriter analyze src/` gives helpful message: "Directory 'src/' requires --recursive flag for safety"  
**Solution**: Use `gnawtreewriter analyze src/ --recursive`  
**Rationale**: Prevents accidental analysis of large directory trees and makes intent explicit

### **Issue**: No glob expansion in recursive mode
**Status**: Shell-dependent  
**Symptom**: `gnawtreewriter analyze *.py --recursive` may not work as expected  
**Workaround**: Use shell glob expansion or specify directories explicitly  

---

## ⚙️ **Command Interface Issues**

### **Issue**: Missing `--version` in older documentation
**Status**: Fixed in v0.2.1  
**Symptom**: Documentation referenced version flag that didn't exist  
**Solution**: `gnawtreewriter --version` now works  

### **Issue**: `lint` command missing from CLI
**Status**: Fixed in v0.2.1  
**Symptom**: Documentation referenced `lint` command that wasn't implemented  
**Solution**: `gnawtreewriter lint` now available as wrapper around analyze  

---

## 🔄 **Restoration System Issues**

### **Issue**: Hash matching occasionally fails
**Status**: Has fallback mechanism  
**Symptom**: "Backup not found for hash" errors during restoration  
**Workaround**: System automatically falls back to timestamp-based restoration  
**Impact**: Restoration still works, but may take slightly longer  

### **Issue**: Session restoration requires exact session IDs
**Status**: By design  
**Symptom**: Need to copy-paste long session IDs from history  
**Workaround**: Use `gnawtreewriter history --format json | grep session_id` to find IDs  
**Future**: Consider shorter session aliases  

---

## 🎯 **QML-Specific Issues**

### **Issue**: Parse success ≠ instantiation success
**Status**: Limitation of static analysis  
**Symptom**: QML files parse correctly but fail at runtime  
**Impact**: Tool validates syntax but not semantic correctness  
**Recommendation**: Always test QML changes in actual Qt environment  

### **Issue**: Complex QML property paths
**Status**: Parser limitation  
**Symptom**: Nested QML components sometimes have unexpected node paths  
**Workaround**: Use `list` command to verify exact paths before editing  

---

## 📊 **Performance Issues**

### **Issue**: Large project analysis is slow
**Status**: ✅ Mitigated in v0.9.7 — file processing is parallelized (rayon): pack ~50% faster, explore ~59% faster, indexing 40–52% faster  
**Symptom**: Very large projects (1000+ files) still take time on first full analysis  
**Workaround**: Use `explore` / `analyze --format summary` for cheap overviews, or target specific files  
**Future**: Progress indicators for long-running scans

### **Issue**: Memory usage on very large files
**Status**: TreeSitter limitation (still current)  
**Symptom**: Files over 10MB may consume significant memory  
**Workaround**: Consider splitting very large files  

---

## 🔧 **Integration Issues**

### **Issue**: Exit codes not documented
**Status**: Documentation gap  
**Impact**: CI/CD integration may be unclear about success/failure  
**Workaround**: Check for error output on stderr  
**Future**: Document standard exit codes  

### **Issue**: No MCP integration yet
**Status**: ✅ RESOLVED — MCP integration shipped in Phase 2 (stdio & HTTP transports, ~30 tools)  
**Note**: Kvar som utvecklingslucka är MCP-*parity* — vissa CLI-funktioner (t.ex. `lint`) saknar
MCP-post och vissa schemas är tomma. Se ROADMAP.md Phase 9.1/9.3.  

---

## 🧪 **Testing and Verification Issues**

### **Issue**: Examples in documentation not CI-tested
**Status**: Manual verification only  
**Risk**: Documentation may become stale as features change  
**Mitigation**: Community feedback and manual testing  
**Future**: Automated documentation testing planned  

### **Issue**: No automated regression testing for AI workflows
**Status**: Reliance on manual AI agent testing  
**Impact**: Changes could break AI agent workflows without notice  
**Mitigation**: Comprehensive test scenarios provided for AI agents  

---

## ⚡ **Quick Reference: Common Error Messages**

| Error Message | Likely Cause | Quick Fix |
|---------------|--------------|-----------|
| `Directory requires --recursive flag` | Used directory path without `--recursive` (intentional safety feature) | Add `--recursive` flag |
| `Node not found at path` | Path changed since last analysis | Re-run `analyze` or `list` to get current paths |
| `Backup not found for hash` | Hash mismatch in restoration | System auto-retries with timestamp method |
| `Validation failed` | Syntax error in edit content | Check syntax of new content |
| `unrecognized subcommand` | Typo in command name | Check `gnawtreewriter --help` for correct commands |

---

## 🔄 **Workaround Patterns**

### **Safe Analysis Pattern**
```bash
# Directories require explicit --recursive flag (by design)
gnawtreewriter analyze . --recursive --format summary | head -20

# This is intentional - prevents accidental large scans
# gnawtreewriter analyze .  # Will require --recursive flag
```

### **Reliable Restoration Pattern**
```bash
# Always preview first, especially for project-wide restoration
gnawtreewriter restore-project "timestamp" --preview
# Review output, then run without --preview if acceptable
```

### **Error Recovery Pattern**
```bash
# If edit fails, check current structure
gnawtreewriter list file.py
gnawtreewriter analyze file.py
# Find correct path and retry
```

---

## 📞 **Reporting New Issues**

### **Before Reporting**
1. Check this document for known issues
2. Verify you're using latest version: `gnawtreewriter --version`  
3. Try the suggested workarounds
4. Include your exact command and error output

### **Where to Report**
- **GitHub Issues**: https://github.com/Tuulikk/GnawTreeWriter/issues
- **Include**: Version, OS, exact command, error output
- **For AI Agents**: Note your AI type (Claude, GPT, Gemini, etc.)

### **What Helps**
- Minimal reproduction case
- Exact file content that triggers the issue  
- Expected vs actual behavior
- Impact on your workflow

---

*This document is maintained based on real user feedback, particularly from AI agent testing. Last reviewed against code: 2026-10-04 (v0.9.8)*