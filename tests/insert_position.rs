/// Regression tests for insert position logic.
///
/// Covers the bug where `insert position=1` (append) would land inside the
/// last function instead of at file level when the file lacked a trailing newline.
/// Also tests normal operation with trailing newlines and block-level inserts.

use std::io::Write;

use gnawtreewriter::core::EditOperation;

fn make_file(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

/// Helper: read file as string, normalizing to unix line endings.
fn read_file(path: &std::path::Path) -> String {
    std::fs::read_to_string(path)
        .unwrap()
        .replace("\r\n", "\n")
}

// ── Helpers ──────────────────────────────────────────────────────────

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

/// Returns (TempDir, path) for a Rust file *with* trailing newline.
fn rust_file_with_newline() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = temp_dir();
    let src = "fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n\nfn main() {\n    let x = 1;\n}\n";
    let path = make_file(dir.path(), "calc.rs", src);
    (dir, path)
}

/// Returns (TempDir, path) for a Rust file *without* trailing newline.
fn rust_file_without_newline() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = temp_dir();
    let src = "fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n\nfn main() {\n    let x = 1;\n}";
    let path = make_file(dir.path(), "calc.rs", src);
    (dir, path)
}

// ── source_file insert position=1 (append at end of file) ────────────

#[test]
fn insert_source_file_pos1_with_trailing_newline() {
    let (_dir, path) = rust_file_with_newline();
    let writer = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();

    let result = writer
        .preview_edit(EditOperation::Insert {
            parent_path: String::new(), // source_file root
            position: 1,
            content: "fn extra() {}\n".to_string(),
        })
        .unwrap();

    // The new function must appear AFTER main's closing brace.
    let lines: Vec<&str> = result.lines().collect();
    // Find the line with "fn extra"
    let extra_idx = lines.iter().position(|l| l.contains("fn extra")).unwrap();
    let main_idx = lines.iter().position(|l| l.contains("fn main")).unwrap();

    assert!(
        extra_idx > main_idx,
        "fn extra ({}) should appear after fn main ({})",
        extra_idx,
        main_idx
    );
    assert!(
        lines[main_idx + 1..extra_idx].iter().any(|l| l.contains('}')),
        "main's closing brace should appear before fn extra"
    );
}

#[test]
fn insert_source_file_pos1_without_trailing_newline() {
    let (_dir, path) = rust_file_without_newline();
    let writer = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();

    let result = writer
        .preview_edit(EditOperation::Insert {
            parent_path: String::new(),
            position: 1,
            content: "fn extra() {}".to_string(),
        })
        .unwrap();

    // The new function must appear AFTER main's closing brace.
    let lines: Vec<&str> = result.lines().collect();
    let extra_idx = lines.iter().position(|l| l.contains("fn extra")).unwrap();
    let main_idx = lines.iter().position(|l| l.contains("fn main")).unwrap();

    assert!(
        extra_idx > main_idx,
        "fn extra ({}) should appear after fn main ({})",
        extra_idx,
        main_idx
    );

    // Crucially: "fn extra" must NOT appear inside main's body.
    // The last line of main should be "}" and extra should come after it.
    let last_main_line = lines[main_idx..extra_idx]
        .iter()
        .rposition(|l| l.contains('}'))
        .unwrap();
    assert!(
        last_main_line < extra_idx,
        "closing brace should be before fn extra"
    );
}

#[test]
fn insert_source_file_pos1_actually_writes_correctly() {
    let (_dir, path) = rust_file_without_newline();
    let mut writer = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();

    writer
        .edit(
            EditOperation::Insert {
                parent_path: String::new(),
                position: 1,
                content: "fn extra() {}".to_string(),
            },
            false,
        )
        .unwrap();

    let written = read_file(&path);
    assert!(
        written.contains("fn extra()"),
        "file should contain the inserted function"
    );
    // The inserted function should be after main's closing brace
    let main_close = written.rfind("fn main").unwrap();
    let main_brace = written[main_close..].find('}').unwrap() + main_close;
    let extra_pos = written.find("fn extra").unwrap();
    assert!(
        extra_pos > main_brace,
        "fn extra should be after main's closing brace"
    );
}

// ── block-level insert position=0 (after opening brace) ─────────────

#[test]
fn insert_block_pos0_adds_after_opening_brace() {
    let (_dir, path) = rust_file_with_newline();
    let writer = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();

    // Find the main function's block node
    let tree = writer.analyze();
    let main_block = tree.children
        .iter()
        .find(|c| c.node_type == "function_item" && c.content.contains("main"))
        .unwrap()
        .children
        .iter()
        .find(|c| c.node_type == "block")
        .unwrap();

    let writer = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();
    let result = writer
        .preview_edit(EditOperation::Insert {
            parent_path: main_block.path.clone(),
            position: 0,
            content: "    let inserted_first = true;".to_string(),
        })
        .unwrap();

    // The inserted line should appear before "let x = 1;"
    let lines: Vec<&str> = result.lines().collect();
    let inserted_idx = lines
        .iter()
        .position(|l| l.contains("inserted_first"))
        .unwrap();
    let x_idx = lines.iter().position(|l| l.contains("let x = 1")).unwrap();

    assert!(
        inserted_idx < x_idx,
        "inserted_first ({}) should appear before let x ({})",
        inserted_idx,
        x_idx
    );
}

// ── block-level insert position=1 (before closing brace) ─────────────

#[test]
fn insert_block_pos1_adds_before_closing_brace() {
    let (_dir, path) = rust_file_with_newline();
    let writer = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();

    let tree = writer.analyze();
    let main_block = tree.children
        .iter()
        .find(|c| c.node_type == "function_item" && c.content.contains("main"))
        .unwrap()
        .children
        .iter()
        .find(|c| c.node_type == "block")
        .unwrap();

    let writer = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();
    let result = writer
        .preview_edit(EditOperation::Insert {
            parent_path: main_block.path.clone(),
            position: 1,
            content: "    let inserted_last = true;".to_string(),
        })
        .unwrap();

    let lines: Vec<&str> = result.lines().collect();
    let inserted_idx = lines
        .iter()
        .position(|l| l.contains("inserted_last"))
        .unwrap();
    let x_idx = lines.iter().position(|l| l.contains("let x = 1")).unwrap();

    assert!(
        inserted_idx > x_idx,
        "inserted_last ({}) should appear after let x ({})",
        inserted_idx,
        x_idx
    );

    // The inserted line should be inside the function (before its closing brace)
    let lines_from_insert: String = lines[inserted_idx..].join("\n");
    assert!(
        lines_from_insert.contains('}'),
        "closing brace should appear after inserted line"
    );
}

// ── block-level insert without trailing newline ──────────────────────

#[test]
fn insert_block_pos1_without_trailing_newline() {
    let (_dir, path) = rust_file_without_newline();
    let writer = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();

    let tree = writer.analyze();
    let main_block = tree.children
        .iter()
        .find(|c| c.node_type == "function_item" && c.content.contains("main"))
        .unwrap()
        .children
        .iter()
        .find(|c| c.node_type == "block")
        .unwrap();

    let writer = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();
    let result = writer
        .preview_edit(EditOperation::Insert {
            parent_path: main_block.path.clone(),
            position: 1,
            content: "    let inserted = true;".to_string(),
        })
        .unwrap();

    let lines: Vec<&str> = result.lines().collect();
    let inserted_idx = lines.iter().position(|l| l.contains("inserted")).unwrap();
    let x_idx = lines.iter().position(|l| l.contains("let x = 1")).unwrap();

    assert!(
        inserted_idx > x_idx,
        "inserted ({}) should appear after let x ({})",
        inserted_idx,
        x_idx
    );

    // Must still be inside the function (before closing brace)
    let lines_from_insert: String = lines[inserted_idx..].join("\n");
    assert!(
        lines_from_insert.contains('}'),
        "closing brace should appear after inserted line"
    );
}

// ── Python file (different language, same logic) ─────────────────────

#[test]
fn insert_python_source_file_pos1() {
    let dir = temp_dir();
    let src = "def add(a, b):\n    return a + b\n\ndef main():\n    print(add(1, 2))\n";
    let path = make_file(dir.path(), "calc.py", src);

    let writer = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();

    let result = writer
        .preview_edit(EditOperation::Insert {
            parent_path: String::new(),
            position: 1,
            content: "def extra():\n    pass\n".to_string(),
        })
        .unwrap();

    let lines: Vec<&str> = result.lines().collect();
    let extra_idx = lines.iter().position(|l| l.contains("def extra")).unwrap();
    let main_idx = lines.iter().position(|l| l.contains("def main")).unwrap();

    assert!(
        extra_idx > main_idx,
        "def extra ({}) should appear after def main ({})",
        extra_idx,
        main_idx
    );
}

#[test]
fn insert_python_source_file_pos1_no_trailing_newline() {
    let dir = temp_dir();
    let src = "def add(a, b):\n    return a + b\n\ndef main():\n    print(add(1, 2))";
    let path = make_file(dir.path(), "calc.py", src);

    let writer = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();

    let result = writer
        .preview_edit(EditOperation::Insert {
            parent_path: String::new(),
            position: 1,
            content: "def extra():\n    pass".to_string(),
        })
        .unwrap();

    let lines: Vec<&str> = result.lines().collect();
    let extra_idx = lines.iter().position(|l| l.contains("def extra")).unwrap();
    let main_idx = lines.iter().position(|l| l.contains("def main")).unwrap();

    assert!(
        extra_idx > main_idx,
        "def extra ({}) should appear after def main ({})",
        extra_idx,
        main_idx
    );
}

// ── TypeScript file ──────────────────────────────────────────────────

#[test]
fn insert_typescript_source_file_pos1_no_trailing_newline() {
    let dir = temp_dir();
    let src = "function add(a: number, b: number): number {\n    return a + b;\n}\n\nfunction main(): void {\n    console.log(add(1, 2));\n}";
    let path = make_file(dir.path(), "calc.ts", src);

    let writer = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();

    let result = writer
        .preview_edit(EditOperation::Insert {
            parent_path: String::new(),
            position: 1,
            content: "function extra(): void {}".to_string(),
        })
        .unwrap();

    let lines: Vec<&str> = result.lines().collect();
    let extra_idx = lines.iter().position(|l| l.contains("function extra")).unwrap();
    let main_idx = lines.iter().position(|l| l.contains("function main")).unwrap();

    assert!(
        extra_idx > main_idx,
        "function extra ({}) should appear after function main ({})",
        extra_idx,
        main_idx
    );
}

// ── Empty file edge case ────────────────────────────────────────────

#[test]
fn insert_source_file_pos1_empty_file() {
    let dir = temp_dir();
    let path = make_file(dir.path(), "empty.rs", "");

    let writer = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();
    let result = writer
        .preview_edit(EditOperation::Insert {
            parent_path: String::new(),
            position: 1,
            content: "fn main() {}".to_string(),
        })
        .unwrap();

    assert!(
        result.contains("fn main()"),
        "content should be inserted into empty file"
    );
}

// ── Single-function file without trailing newline ────────────────────

#[test]
fn insert_source_file_pos1_single_function_no_newline() {
    let dir = temp_dir();
    let src = "fn only_fn() -> i32 {\n    42\n}";
    let path = make_file(dir.path(), "single.rs", src);

    let writer = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();

    let result = writer
        .preview_edit(EditOperation::Insert {
            parent_path: String::new(),
            position: 1,
            content: "fn another() {}".to_string(),
        })
        .unwrap();

    let lines: Vec<&str> = result.lines().collect();
    let only_idx = lines.iter().position(|l| l.contains("only_fn")).unwrap();
    let another_idx = lines.iter().position(|l| l.contains("another")).unwrap();

    assert!(
        another_idx > only_idx,
        "fn another ({}) should appear after fn only_fn ({})",
        another_idx,
        only_idx
    );
}
