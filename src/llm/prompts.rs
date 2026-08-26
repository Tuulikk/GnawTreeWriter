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
