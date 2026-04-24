# GTW Backup & Time Machine — Feedback & Observations

> Based on a real recovery scenario in GnawFlow (2026-04-16), where an AI agent reverted files via `git checkout`, losing 45 lines in server.js and 461 lines in index.html. GTW's backup system was used to restore the project.

## What Worked Well

### 1. Backups Exist When You Need Them
The `.gnawtreewriter_backups/` directory contained full file snapshots from every GTW edit session. When the agent's git checkout wiped changes, the backups were still there — untouched by git.

**This is the killer feature.** Git `checkout` silently destroys work. GTW's backups survive because they're outside git's scope.

### 2. History Gives Immediate Context
```
gnawtreewriter history
```
Showed every edit with timestamps and descriptions. Made it easy to see:
- What features were added (restart endpoint, ping/pong, memory tab)
- When they were added
- Which files were affected

### 3. Full File Snapshots, Not Just Diffs
Each backup is the complete file state. This meant:
- Easy line-count comparison (1262 vs 1302 lines → 40 lines missing)
- Direct extraction and overwriting without merge conflicts
- No dependency on a chain of patches

### 4. Restoration Was Straightforward
```bash
# Extract backup content
python3 -c "
import json
with open('.gnawtreewriter_backups/server.js_backup_20260416_201758_061.json') as f:
    d = json.load(f)
with open('server.js', 'w') as f:
    f.write(d['source_code'])
"
```
Three lines of Python, file restored. No git reflog archaeology needed.

---

## What Could Be Better

### 1. 🔴 UTC vs Local Time Mismatch (Critical)

**Problem:** Backup filenames use UTC (`20260416_201758`) but `gnawtreewriter history` shows local time (`20:17`). This makes cross-referencing painful.

**Example:**
- History says: `04-16 20:17:58  Edit  server.js`
- Backup file: `server.js_backup_20260416_201758_061.json` ← this is actually 22:17 local time

**Suggestion:** Use consistent timezone everywhere. Prefer local time in filenames, or always show UTC in history with explicit timezone label.

### 2. 🟡 No "Diff Backup vs Current" Command

**Problem:** To find what was missing, I had to write custom Python code:
```python
import json, subprocess, tempfile
with open('backup.json') as f:
    d = json.load(f)
# write to temp, diff, parse output...
```

**Suggestion:** Add a command like:
```bash
gnawtreewriter diff-backup <file> [--latest | --timestamp "2026-04-16 20:00"]
```
That shows the diff between a backup and the current file state.

### 3. 🟡 No Awareness of Non-GTW Changes

**Problem:** The agent used `git checkout` which reverted files. GTW's history didn't show this because it only tracks GTW operations. The backup directory had the correct files, but nothing flagged "hey, the current file is older than your latest backup."

**Suggestion:** Add a `gnawtreewriter verify` or `gnawtreewriter drift-check` command that:
- Compares current file hashes against the latest backup
- Warns if current file is smaller/older than backup
- Shows which files have drifted from GTW's last-known state

### 4. 🟡 Backup Directory Gets Large

**Observation:** The GnawFlow backup directory had dozens of snapshots per file, some at 4MB+ each (index.html). Over time this adds up.

**Suggestion:**
- Option to compress old backups (gzip)
- Configurable retention policy (keep last N snapshots per file)
- Or store only diffs after the first full snapshot (like git does internally)

### 5. 🟢 Minor: `restore-project` Timestamp Format

**Problem:** First attempt with `"04-16 20:00:00"` failed silently with confusing error. Had to guess the format `"2026-04-16 20:00:00"`.

**Suggestion:** Accept more formats, or show the expected format in the error message with an example from the user's actual history.

### 6. 🟢 Minor: No "Restore Preview" in History

The `history` command shows operations but not what the file state was at each point. A `--show-state` flag that includes file size/line count at each checkpoint would help identify when things went wrong.

---

## Proposed Improvement: `gnawtreewriter doctor`

A single command that combines recovery diagnostics:

```bash
gnawtreewriter doctor [--fix]
```

Would:
1. Scan `.gnawtreewriter_backups/` for all tracked files
2. Compare current file state against latest backup
3. Report files that have drifted (smaller, older, missing content)
4. With `--fix`: offer to restore from the latest backup

This would catch issues like the git checkout problem automatically.

---

## Summary

| Aspect | Rating | Notes |
|--------|--------|-------|
| Backup existence | ⭐⭐⭐⭐⭐ | Files were there when needed |
| History readability | ⭐⭐⭐⭐ | Good, but timezone confusion |
| Restoration ease | ⭐⭐⭐⭐ | Simple once you know the format |
| Drift detection | ⭐⭐ | Missing — no way to know files were reverted |
| Cross-tool awareness | ⭐⭐ | Only tracks GTW edits, not git/bash changes |
| Backup size management | ⭐⭐⭐ | Gets large over time |

**Bottom line:** GTW's backup system saved this project. The core idea — keeping backups outside git's control — is exactly right. The main gaps are around detecting when non-GTW tools have modified files, and making the backup ↔ current comparison easier.
