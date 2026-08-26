//! Deterministic prompt templates for the LFM2.5 pipeline commands.
//!
//! One builder per step. Each is a pure function `Input -> String` so the
//! prompts are unit-testable and stable across runs.

/// Prompt for `explain`: explain a single code node.
pub fn explain_prompt(file_path: &str, node_type: &str, node_content: &str) -> String {
    format!(
        "You are an expert programmer. Explain the following code from {file_path} \
         ({node_type}) in plain, concise language. Describe what it does, its inputs \
         and outputs, and any notable details. Keep it under 150 words.\n\n\
         ```\n{node_content}\n```\n\nExplanation:"
    )
}

/// Prompt for `summarize` step 1: summarize a single file.
pub fn summarize_file_prompt(file_path: &str, content: &str, max_words: usize) -> String {
    format!(
        "You are a codebase summarizer. Summarize what this file does in at most \
         {max_words} words. Focus on purpose and key functions/structs. Output only \
         the summary, no preamble.\n\n\
         FILE: {file_path}\n\n\
         ```\n{content}\n```\n\nSummary:"
    )
}

/// Prompt for `summarize` step 2: summarize a directory from file summaries.
pub fn summarize_dir_prompt(dir: &str, file_summaries: &[(String, String)]) -> String {
    let body = file_summaries
        .iter()
        .map(|(name, summary)| format!("- {name}: {summary}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "You are a codebase summarizer. Summarize the directory '{dir}' in at most \
         120 words based on these per-file summaries. Focus on the directory's role \
         in the project. Output only the summary.\n\n{body}\n\nDirectory summary:"
    )
}

/// Prompt for `summarize` step 3: summarize the project.
pub fn summarize_project_prompt(dir_summaries: &[(String, String)]) -> String {
    let body = dir_summaries
        .iter()
        .map(|(name, summary)| format!("- {name}: {summary}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "You are a codebase summarizer. Summarize this project in at most 150 words \
         based on these directory summaries. Give an overview of the whole codebase. \
         Output only the summary.\n\n{body}\n\nProject summary:"
    )
}

/// Prompt for `investigate` step 1: expand a question into search terms.
pub fn expand_query_prompt(question: &str) -> String {
    format!(
        "Given the question: \"{question}\"\n\n\
         List 3-6 search terms that would find relevant code for answering it. \
         Output ONLY a JSON array of strings, e.g. [\"backup\", \"restore\"]."
    )
}

/// Prompt for `investigate` step 3: rank candidate files by relevance.
pub fn rank_candidates_prompt(question: &str, candidates: &[(String, String)]) -> String {
    let body = candidates
        .iter()
        .enumerate()
        .map(|(i, (path, summary))| format!("{i}. {path}: {summary}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Question: \"{question}\"\n\n\
         Here are candidate files with brief summaries:\n{body}\n\n\
         Output ONLY a JSON array of the 1-3 most relevant file indices, e.g. [2, 0]."
    )
}

/// Prompt for `investigate` step 4: synthesize the final answer.
pub fn synthesize_answer_prompt(
    question: &str,
    evidence: &[(String, String)],
) -> String {
    let body = evidence
        .iter()
        .map(|(path, excerpt)| format!("--- {path} ---\n{excerpt}\n"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Answer the question: \"{question}\"\n\n\
         Use ONLY the following code excerpts as evidence. Reference files by path \
         in the answer. Be concise (under 200 words).\n\n{body}\n\nAnswer:"
    )
}

/// Prompt for `edit --ask`: propose a line-based change.
/// The model gives the line number to change and the new line content; GTW
/// finds the containing AST node and validates before applying. Line-based
/// output avoids fragile JSON escaping of code snippets (a small-model pain).
/// `issues` is an optional annotation of known rule violations in the file.
pub fn edit_ask_prompt(
    file_path: &str,
    request: &str,
    file_preview: &str,
    issues: &str,
) -> String {
    let issues_block = if issues.trim().is_empty() {
        String::new()
    } else {
        format!("\n{issues}\n")
    };
    format!(
        "You are editing the file {file_path}. The user wants: \"{request}\"\n\
         {issues_block}\
         Here is the relevant part of the file (line numbers shown):\n\n{file_preview}\n\n\
         Respond with EXACTLY this JSON, no other text:\n\
         {{\"line\": <line number>, \"new\": \"<new content for that line>\"}}\n\n\
         Rules:\n\
         - line: the 1-based line number from the preview that must change\n\
         - new: the FULL replacement text for that line (no line number prefix)\n\
         - Keep the change minimal; do not change other lines\n\
         - If known issues are listed above, your new line must not introduce or worsen them\n\
         - escape double quotes in new as \\\"; keep it on one JSON line\n\
         JSON:"
    )
}

/// Prompt for `edit --ask --all`: the model gives a recurring snippet (old)
/// and its replacement (new); GTW applies it to every occurrence. `old` must
/// appear verbatim multiple times.
pub fn edit_ask_all_prompt(
    file_path: &str,
    request: &str,
    file_preview: &str,
    issues: &str,
) -> String {
    let issues_block = if issues.trim().is_empty() {
        String::new()
    } else {
        format!("\n{issues}\n")
    };
    format!(
        "You are editing the file {file_path}. The user wants: \"{request}\"\n\
         {issues_block}\
         Here is the relevant part of the file (line numbers shown):\n\n{file_preview}\n\n\
         The change should apply to MULTIPLE occurrences. Respond with EXACTLY this JSON:\n\
         {{\"old\": \"<exact recurring snippet>\", \"new\": \"<its replacement>\"}}\n\n\
         Rules:\n\
         - old: a SHORT snippet that appears VERBATIM multiple times (e.g. \".unwrap()\")\n\
         - new: its replacement (e.g. \".expect(\\\"failed\\\")\")\n\
         - Do NOT include line numbers or full lines unless they are identical\n\
         - escape double quotes as \\\"; keep each on one JSON line\n\
         JSON:"
    )
}

/// Prompt for `lint --discover`: propose project-specific lint rules from a
/// code sample. The model returns a JSON array of rules.
pub fn discover_rules_prompt(code_sample: &str) -> String {
    format!(
        "You are analyzing a codebase to find project-specific lint rules.\n\
         Here are code samples:\n\n{code_sample}\n\n\
         Propose 2-4 semgrep-style rules that capture recurring problems or \
         project-specific anti-patterns visible in this code.\n\n\
         A rule's pattern is a SMALL, VALID code fragment with $X placeholders \
         standing in for any expression. Examples of good patterns:\n\
         - Rust: \"$X.unwrap()\", \"let _ = $X;\", \"return $X;\\nreturn $X;\"\n\
         - Python: \"except:\\n    pass\", \"eval($X)\", \"$X == None\"\n\
         - JS: \"console.log($X)\", \"$X == $Y\"\n\n\
         Respond with EXACTLY a JSON array, no other text:\n\
         [{{\"id\": \"unique_id\", \"language\": \"rust|python|javascript|...\", \
         \"severity\": \"error|warning|info\", \
         \"message\": \"why this is a problem\", \
         \"pattern\": \"$X.unwrap()\"}}]\n\n\
         Rules:\n\
         - id: short lowercase id (e.g. proj_never_ignore)\n\
         - language: must match the sample language\n\
         - pattern: SMALL and VALID (a statement or expression with $X)\n\
         - Prefer patterns that actually appear in the samples\n\
         JSON:"
    )
}
