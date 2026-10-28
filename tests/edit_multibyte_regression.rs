//! Regression (2026-10-28, GTW_MCP_ISSUE_LOG): edit_node panicked with a
//! char-boundary error ("end byte index 80 is not a char boundary; it is
//! inside 'ä'") when editing a file whose EXISTING text contains multibyte
//! characters — å/ä/ö in comments, emoji in string literals. The file was
//! opencode-plugins/tool-guard.js; the identical panic index across two
//! different new_contents proved the byte offset was computed on the OLD
//! text, not the new content.
//!
//! Contract under test: the whole edit write path (resolve → splice →
//! guardian → validate → save) must be UTF-8 safe. Any file GTW can parse
//! must be editable, whatever characters it contains. Multibyte chars are
//! the NORM in this ecosystem's comments — a panic class here makes
//! edit_node unusable on a large share of real files.
use gnawtreewriter::core::EditOperation;

/// Structurally mirrors opencode-plugins/tool-guard.js: multibyte header
/// comments, emoji + å/ä/ö inside template literals, an `export default
/// Plugin.define({...})` block as the last top-level node.
const JS_FILE: &str = r#"// Tool-guard — mekaniskt stopp för §35-filredigeringsbrott i bash.
// AGENTS.md §35: "Källkod och config redigeras ENDAST via dedikerade
// edit-verktyg ... ALDRIG via bash-redirekt, sed, awk, perl -i, dd,
// truncate eller python-heredocs." Denna hook ger regeln teeth (§38/§44):
// blockerar filskrivande mönster och pekar på rätt verktyg.

import { Plugin } from "@opencode/plugin";

const PROJECT_EXT =
  /\.(rs|toml|md|json|js|ts|py)$/i;

export function detect(command) {
  if (typeof command !== "string" || command.length === 0) return null;
  const hasHeredoc = /<<-?\s*['"]?[A-Za-z_]\w*['"]?/.test(command);
  if (hasHeredoc) {
    return {
      reason: "script-heredoc",
      fix: "Skript-exekverande heredoc är §35-klassen 'python-heredocs' — aldrig som script-text i bash.",
    };
  }
  return null;
}

export default Plugin.define({
  id: "tool-guard",
  setup(ctx) {
    return ctx.tool.hook("execute.before", (event) => {
      // V2: tool-ID är "shell" ("bash" är V1-namnet) — matcha båda.
      if (event.tool !== "bash" && event.tool !== "shell") return;

      const command = event.input?.command ?? event.input?.cmd ?? "";
      const hit = detect(command);
      if (!hit) return;

      const err = new Error(
        `🛑 TOOL-GUARD blockerade bash (${hit.reason}):\n` +
          `Kör detta via rätt verktyg i stället:\n  ${hit.fix}\n\n` +
          `Kommandot kördes INTE.`,
      );
      err.name = "ToolGuardBlocked";
      throw err;
    });
  },
});
"#;

/// Walks candidate numeric paths and returns the one whose content is the
/// `export default Plugin.define` block (paths shift with comment nodes —
/// hard-coding "34" would be brittle).
fn find_export_block_path(w: &gnawtreewriter::GnawTreeWriter) -> Option<String> {
    for i in 0..80 {
        let p = i.to_string();
        if let Ok(content) = w.show_node(&p) {
            if content.contains("Plugin.define") {
                return Some(p);
            }
        }
    }
    None
}

#[test]
fn edit_node_on_multibyte_js_file_does_not_panic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tool-guard.js");
    std::fs::write(&path, JS_FILE).unwrap();

    let mut w = gnawtreewriter::GnawTreeWriter::new(path.to_str().unwrap()).unwrap();
    let target = find_export_block_path(&w).expect("export block must resolve to a node path");

    let new_block = "export default Plugin.define({\n  id: \"tool-guard\",\n  async setup(ctx) {\n    // Non-blocking nudge: remind the agent that GTW exists.\n    await ctx.tool.hook(\"execute.after\", async (event) => {\n      if (event.status !== \"completed\") return;\n    });\n    return ctx.tool.hook(\"execute.before\", (event) => {\n      const hit = detect(event.input?.command ?? \"\");\n      if (!hit) return;\n      throw new Error(`🛑 blockerade (${hit.reason}) — kördes INTE.`);\n    });\n  },\n});\n";

    w.edit(
        EditOperation::Edit {
            node_path: target,
            content: new_block.to_string(),
        },
        false,
    )
    .expect("edit on a multibyte file must succeed, not panic");

    let out = std::fs::read_to_string(&path).unwrap();
    assert!(
        out.contains("async setup(ctx)"),
        "new block must be written: {out}"
    );
    assert!(
        out.contains("// Tool-guard — mekaniskt stopp"),
        "untouched multibyte header must survive: {out}"
    );
    assert!(
        out.contains("Skript-exekverande heredoc"),
        "untouched multibyte strings must survive: {out}"
    );
}
