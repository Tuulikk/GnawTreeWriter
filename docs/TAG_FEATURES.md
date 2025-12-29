# Named References Feature

**Status**: Implemented | **Priority**: #4 (Complete) | **Last Updated**: 2025-12-28

---

## 🎯 Purpose

Allow users to assign memorable names to tree node paths. This makes scripts more robust when tree structure changes, as names remain stable while numerical paths like `1.2.0.5` may shift when nodes are added/removed above.

---

## 📋 Problem Statement

**Current Issue**: Numerical paths are brittle
```bash
# Edit a function at path 1.2.0
gnawtreewriter edit main.rs "1.2.0" 'def new_func(): pass'

# If we add a new import at the top (path 1), now the function is at 1.3.0!
# Our script breaks because the path changed!
```

**Solution**: Named references that persist across structural changes
```bash
# Assign a name to the function path
gnawtreewriter tag add main.rs "1.2.0" "my_function"

# Edit using the name (even if path changes!)
gnawtreewriter edit main.rs --tag "my_function" 'def new_func(): pass'

# Works even if path is now 1.3.0 or 1.2.1 - the name stays the same!
```

---

## 🏗️ Architecture

### 1. Tag Storage
**Location**: `.gnawtreewriter-tags.toml` in project root

**Structure**:
```toml
[files."main.rs"]
tags."my_function" = "1.2.0.5"
tags."main_struct" = "1.2"
tags."helper_fn" = "0.1.3"
```

### 2. Command Line Interface

```bash
# Add a named reference
gnawtreewriter tag add <file> <path> <name>

# Edit using a named reference
gnawtreewriter edit <file> --tag <name> <content>
gnawtreewriter insert <file> --tag <name> <content>
gnawtreewriter delete <file> --tag <name>

# List all tags for a file
gnawtreewriter tag list <file>

# Remove a named reference
gnawtreewriter tag remove <file> <name>
```

### 3. Name Resolution

When using `--tag <name>`:
1. Load `.gnawtreewriter-tags.toml`
2. Look up `files.<file>.tags.<name>` → get current path
3. Use that path for the edit/insert/delete operation
4. Success! The operation works regardless of what the actual numeric path is now

---

## 🔧 Implementation Steps

### Step 1: Add TagManager Module
**File**: `src/core/tag_manager.rs`

```rust
pub struct TagManager {
    tag_file: PathBuf,
    tags: HashMap<String, HashMap<String, String>>, // file -> (name -> path)
}

impl TagManager {
    pub fn load(project_root: &Path) -> Result<Self>;
    pub fn save(&self) -> Result<()>;
    pub fn add_tag(&mut self, file: &str, name: &str, path: &str);
    pub fn get_path(&self, file: &str, name: &str) -> Option<String>;
    pub fn list_tags(&self, file: &str) -> Vec<(String, String)>; // (name, path)
    pub fn remove_tag(&mut self, file: &str, name: &str);
}
```

### Step 2: Update GnawTreeWriter Core
**File**: `src/core/mod.rs`

```rust
pub struct GnawTreeWriter {
    file_path: String,
    source_code: String,
    tree: TreeNode,
    transaction_log: TransactionLog,
    tag_manager: TagManager, // NEW
}
```

### Step 3: Add CLI Commands
**File**: `src/cli.rs`

```rust
enum Commands {
    // ... existing commands ...
    
    // NEW: Tag management
    Tag {
        #[arg(subcommand)]
        command: TagSubcommands,
    },
}

enum TagSubcommands {
    Add { file_path: String, node_path: String, name: String },
    List { file_path: String },
    Remove { file_path: String, name: String },
}

// Add --tag flag to Edit, Insert, Delete
Edit {
    file_path: String,
    node_path: Option<String>, // Can be --tag
    #[arg(short, long)]
    tag: Option<String>,
    content: Option<String>,
    // ...
}
```

### Step 4: Implement Tag Resolution Logic

In `handle_edit`, `handle_insert`, `handle_delete`:

```rust
// If --tag is provided, look up the actual path
let actual_path = match tag {
    Some(tag_name) => {
        let project_root = find_project_root(&file_path)?;
        let tag_manager = TagManager::load(&project_root)?;
        tag_manager.get_path(&file_path, &tag_name)
            .ok_or_else(|| anyhow!("Tag '{}' not found for file {}", tag_name, file_path))?
    },
    None => {
        node_path.ok_or_else(|| anyhow!("Either --tag or node_path must be specified"))?
    }
};

// Then use actual_path for the operation
let op = EditOperation::Edit { node_path: actual_path, content };
writer.edit(op)?;
```

---

## 📊 File Changes Needed

### Create New Files:
1. `src/core/tag_manager.rs` - Tag storage and management

### Modify Existing Files:
1. `src/core/mod.rs` - Add TagManager to GnawTreeWriter
2. `src/cli.rs` - Add Tag commands and --tag flags
3. `Cargo.toml` - No changes needed (using std collections)

### Update Documentation:
1. Update `AGENTS.md` with tag feature documentation
2. Add tag examples to `README.md`
3. Update `docs/RECIPES.md` with tag usage recipes

---

## ✅ Testing Plan

```bash
# Test 1: Create tags
gnawtreewriter tag add test.py "1.2.0" "my_function"
gnawtreewriter tag add test.py "0" "root"

# Test 2: List tags
gnawtreewriter tag list test.py
# Expected: Output showing my_function -> 1.2.0, root -> 0

# Test 3: Edit using tag
gnawtreewriter edit test.py --tag "my_function" 'new code'
# Expected: Works even if path has changed!

# Test 4: Remove tag
gnawtreewriter tag remove test.py "my_function"

# Test 5: Missing tag error handling
gnawtreewriter edit test.py --tag "nonexistent" 'code'
# Expected: Error message "Tag 'nonexistent' not found"
```

---

## 🚀 Benefits

1. **Script Robustness**: Scripts don't break when tree structure changes
2. **Readability**: Names like `main_function` are more readable than `1.2.0.5`
3. **Maintainability**: Easy to understand what a script edits without tracing paths
4. **Future-Proof**: Tags persist across sessions and refactoring
5. **Backward Compatible**: Numeric paths still work, --tag is optional

---

## 📝 Implementation Notes

### Tag Storage Format Choice

**Why TOML?**
- Human-readable (easy to edit manually if needed)
- Well-tested in Rust ecosystem (serde + toml already dependencies)
- Supports nested structure (file → tags)
- Better than JSON for config files

### Tag Naming Conventions

**Recommendations:**
- Use snake_case: `my_function`, `helper_class`
- Be descriptive but concise
- Avoid special characters
- Use underscores for spaces: `my_helper_function`

### Error Handling

**Tag Conflicts**:
```rust
if tag_manager.tag_exists(file, name) {
    return Err(anyhow!("Tag '{}' already exists for file {}. Use --force to overwrite.", name, file));
}
```

**Invalid Paths**:
```rust
// Verify the path exists before creating tag
let writer = GnawTreeWriter::new(&file_path)?;
if !writer.node_exists(&path) {
    return Err(anyhow!("Path '{}' does not exist in file {}", path, file_path));
}
```

---

## 🔮 Future Enhancements

1. **Tag Groups**: Organize related tags (e.g., "tests", "config")
2. **Tag Descriptions**: Add optional descriptions to tags
3. **Tag Search**: Find tags by name pattern or description
4. ✅ **Tag Rename**: Implemented in v0.3.0 — `gnawtreewriter tag rename <file> <old_name> <new_name> [--force]`
5. **Tag Export/Import**: Share tags between projects
6. **Wildcards**: `gnawtreewriter edit --tag "test_*"` for multiple matching tags

---

## 📚 References

- Similar to Git aliases but for AST paths instead of commits
- Inspired by Docker volume naming conventions
- Alternative to symbolic links in version control

---

**Status**: Planned | **Priority**: Future | **Last Updated**: 2025-12-28
