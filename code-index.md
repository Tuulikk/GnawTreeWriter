# CODE-INDEX — GnawTreeWriter

> Genererad 2026-11-10 20:03 UTC · gnawtreewriter 0.25.1 · git `cdff41c`
> Återskapa: `gnawtreewriter code-index <root> -o code-index.md --intro <introfil>`
> Inaktuellt om `git`-rev ≠ HEAD — kör om kommandot, redigera ALDRIG det genererade innehållet för hand.
> Merge-konflikt i code-index.md: tag valfri sida och kör om verktyget — hand-merga aldrig (P3).
> Body-hash: 0da2275fdfcd13ed4625061290179aae

## Träd

```
./
  build.rs (40 rader)
  color_test.rs (1 rader)
  demo_workflow.sh (53 rader)
  sync-docs.sh (146 rader)
  test.go (5 rader)
  test.py (1 rader)
  test.qml (12 rader)
  test_add_prop.qml (17 rader)
  test_preview.rs (1 rader)
  test_sense.rs (7 rader)
  test_transaction.py (2 rader)
  test_transaction2.py (2 rader)
examples/
  HelloWorld.java (48 rader)
  complex_test.qml (50 rader)
  debug_loading.rs (17 rader)
  demo.py (48 rader)
  example.c (83 rader)
  example.cpp (179 rader)
  example.php (20 rader)
  example.py (15 rader)
  example.qml (13 rader)
  example.rs (4 rader)
  example.sh (181 rader)
  example.ts (15 rader)
  hello.c (14 rader)
  hello.cpp (33 rader)
  hello.sh (31 rader)
  hello.zig (49 rader)
  mcp_client.rs (300 rader)
  node_mcp_client.js (287 rader)
  python_complex.py (20 rader)
  python_mcp_client.py (275 rader)
  qml_complex.qml (41 rader)
  typescript_complex.ts (47 rader)
examples/temporal-demo/
  notes.py (8 rader)
  restore_demo.sh (20 rader)
extensions/gemini/
  run_mcp.sh (14 rader)
extensions/vscode/src/
  extension.js (135 rader)
extensions/zed/
  build-dist.sh (215 rader)
extensions/zed/scripts/
  mcp-client.sh (127 rader)
  mcp-serve.sh (168 rader)
  mcp-stop.sh (141 rader)
  test-mcp.sh (172 rader)
extensions/zed/src/
  lib.rs (34 rader)
scripts/
  benchmark_ai.sh (66 rader)
  build-gpu.sh (107 rader)
  check-license.sh (14 rader)
  commit-build.sh (76 rader)
  daemon-hygiene.sh (89 rader)
  gtw_adoption_report.py (118 rader)
  mcp-client.sh (127 rader)
  mcp-serve.sh (198 rader)
  mcp-stop.sh (141 rader)
  pack-skill.sh (77 rader)
  test-mcp.sh (172 rader)
scripts/hooks/
  block-text-replace.py (258 rader)
src/
  auth_demo.rs (19 rader)
  cli.rs (6678 rader)
  lib.rs (11 rader)
  main.rs (77 rader)
  network.rs (14 rader)
  version.rs (236 rader)
src/core/
  alf.rs (122 rader)
  anchor.rs (166 rader)
  backup.rs (260 rader)
  batch.rs (897 rader)
  blast.rs (310 rader)
  blueprint.rs (182 rader)
  code_graph.rs (525 rader)
  code_index.rs (683 rader)
  code_index_coverage.rs (385 rader)
  code_index_db.rs (1322 rader)
  compress.rs (487 rader)
  curator.rs (709 rader)
  diagnostics.rs (1077 rader)
  diff_parser.rs (586 rader)
  edit_delta.rs (749 rader)
  explore.rs (567 rader)
  file_walker.rs (143 rader)
  gnaw_diff.rs (267 rader)
  gnaw_find.rs (380 rader)
  gnaw_graph.rs (366 rader)
  gnaw_refactor.rs (369 rader)
  guardian.rs (332 rader)
  healer.rs (78 rader)
  index_entities.rs (1011 rader)
  index_relations.rs (839 rader)
  inspect.rs (514 rader)
  label_manager.rs (90 rader)
  macro_dispatcher.rs (249 rader)
  mod.rs (2270 rader)
  pack.rs (749 rader)
  parse_cache.rs (119 rader)
  report.rs (86 rader)
  restoration_engine.rs (446 rader)
  rules.rs (1289 rader)
  scaffold.rs (274 rader)
  secrets.rs (401 rader)
  state.rs (164 rader)
  stats.rs (335 rader)
  tag_manager.rs (297 rader)
  token_count.rs (150 rader)
  transaction_log.rs (626 rader)
  undo_redo.rs (1107 rader)
  visualizer.rs (200 rader)
  visualizer_test.rs (82 rader)
src/llm/
  ai_manager.rs (1401 rader)
  batch.rs (96 rader)
  gnaw_sense.rs (876 rader)
  harmony.rs (364 rader)
  impact_analyzer.rs (197 rader)
  llm_integration.rs (266 rader)
  mod.rs (28 rader)
  pipeline.rs (893 rader)
  project_indexer.rs (209 rader)
  prompts.rs (174 rader)
  recall_eval.rs (240 rader)
  relational_index.rs (684 rader)
  semantic_index.rs (1247 rader)
src/mcp/
  mod.rs (4259 rader)
src/parser/
  bash.rs (84 rader)
  c.rs (84 rader)
  cmake.rs (33 rader)
  cpp.rs (84 rader)
  csharp.rs (82 rader)
  css.rs (33 rader)
  dart.rs (82 rader)
  error.rs (35 rader)
  generic.rs (56 rader)
  go.rs (80 rader)
  html.rs (82 rader)
  java.rs (84 rader)
  javascript.rs (82 rader)
  json.rs (98 rader)
  kotlin.rs (80 rader)
  make.rs (33 rader)
  markdown.rs (33 rader)
  mod.rs (438 rader)
  php.rs (82 rader)
  powershell.rs (33 rader)
  python.rs (117 rader)
  qml.rs (104 rader)
  qml_tree_sitter.rs (96 rader)
  ruby.rs (33 rader)
  rust.rs (267 rader)
  slint.rs (117 rader)
  sql.rs (82 rader)
  svelte.rs (82 rader)
  swift.rs (80 rader)
  text.rs (52 rader)
  toml.rs (33 rader)
  typescript.rs (82 rader)
  xml.rs (609 rader)
  yaml.rs (33 rader)
  zig.rs (84 rader)
tag_test/
  test.rs (1 rader)
test_dir/
  file2.css (2 rader)
  fix_list_nodes.rs (16 rader)
  fix_search.rs (16 rader)
  fix_skeleton.rs (16 rader)
  fix_skeleton_final.rs (16 rader)
  test1.py (1 rader)
  test2.rs (1 rader)
tests/
  ai_modernbert_tests.rs (184 rader)
  b1_path_validation.rs (106 rader)
  cli_behaviour.rs (194 rader)
  cli_contract.rs (20 rader)
  code_index_check.rs (117 rader)
  code_index_db.rs (189 rader)
  code_index_fixtures.rs (231 rader)
  doc_comment_handling.rs (123 rader)
  edit_multibyte_regression.rs (113 rader)
  edit_node_insert_regression.rs (99 rader)
  edit_node_midline_block_regression.rs (152 rader)
  gnaw_sense_integration.rs (72 rader)
  insert_position.rs (555 rader)
  legacy_parser_surgical_edit.rs (194 rader)
  mcp_integration.rs (2508 rader)
  phase3_graph.rs (106 rader)
  phase4_runners.rs (161 rader)
  relational_integration.rs (55 rader)
  t6_canonical_ids.rs (206 rader)
```

## ./

### build.rs (40 rader)
- `fn main()` :12
- `fn run(cmd: &str, args: &[&str]) -> Option<String>` :33

### color_test.rs (1 rader)
_(inga toppnivå-items)_

### demo_workflow.sh (53 rader)
_(inga toppnivå-items)_

### sync-docs.sh (146 rader)
_(inga toppnivå-items)_

### test.go (5 rader)
- `func main()` :3

### test.py (1 rader)
_(inga toppnivå-items)_

### test.qml (12 rader)
_(inga toppnivå-items)_

### test_add_prop.qml (17 rader)
_(inga toppnivå-items)_

### test_preview.rs (1 rader)
- `fn main() { println!("Hello"); }` :1

### test_sense.rs (7 rader)
- `fn hello()` :1
- `fn goodbye()` :5

### test_transaction.py (2 rader)
- `def hello()` :1

### test_transaction2.py (2 rader)
- `def goodbye()` :1

## examples/

### HelloWorld.java (48 rader)
- `public class HelloWorld` :5
  - `public static void main(String[] args)` :7
  - `public static void greet(String name)` :16
  - `public static int add(int a, int b)` :20
- `class Calculator` :25
  - `public int multiply(int a, int b)` :27
  - `public int divide(int a, int b)` :31
- `interface Greeter` :39
  - `void greet(String name)` :40
- `class SimpleGreeter implements Greeter` :43
  - `@Override` :44

### complex_test.qml (50 rader)
_(inga toppnivå-items)_

### debug_loading.rs (17 rader)
- `fn main()` :5

### demo.py (48 rader)
- `class DatabaseManager` :4
  - `def __init__(self, connection_string)` :7
  - `def connect(self)` :11
  - `def disconnect(self)` :17
- `def authenticate_user(username, password)` :24
- `def main()` :38

### example.c (83 rader)
- `int add(int a, int b)` :25
- `Account* create_account(int id, const char *name)` :30
- `void print_account(Account *acc)` :46
- `int main(int argc, char *argv[])` :58

### example.cpp (179 rader)
- `int main()` :129

### example.php (20 rader)
- `function greet($name)` :3
- `class Person` :7
  - `public function __construct($name)` :10
  - `public function sayHello()` :14

### example.py (15 rader)
- `def greet(name)` :3
- `class Person` :6
  - `def __init__(self, name)` :7
  - `def say_hello(self)` :10

### example.qml (13 rader)
_(inga toppnivå-items)_

### example.rs (4 rader)
- `fn main()` :1

### example.sh (181 rader)
- `print_message()` :20
- `log()` :27
- `command_exists()` :34
- `add()` :39
- `process_file()` :46
- `print_array()` :68
- `main()` :78
- `cleanup()` :171

### example.ts (15 rader)
- `interface Person` :1
- `function greet(person: Person): string` :6

### hello.c (14 rader)
- `int main()` :3
- `int add(int a, int b)` :8
- `void greet(char *name)` :12

### hello.cpp (33 rader)
- `class Greeter` :4
  - `Greeter(const std::string& n) : name(n) {}` :9
  - `void greet()` :11
- `int main()` :16
- `int add(int a, int b)` :25

### hello.sh (31 rader)
- `greet()` :5
- `add()` :10
- `main()` :16

### hello.zig (49 rader)
- `pub fn main() !void` :3
- `fn greet(name: []const u8) !void` :13
- `fn add(a: i32, b: i32) i32` :18
- `fn multiply(a: i32, b: i32) i32` :22

### mcp_client.rs (300 rader)
- `struct JsonRpcRequest<'a>` :29
- `struct JsonRpcResponse` :40
- `struct JsonRpcError` :52
- `struct Args` :63
- `fn build_request(method: &str, id: u64, params: Option<Value>) -> JsonRpcRequest<'_>` :81
- `async fn send_request(` :91
- `async fn wait_for_server(` :121
- `fn pretty_print_result(result: &Value)` :149
- `async fn main() -> Result<()>` :170

### node_mcp_client.js (287 rader)
- `async function getFetch()` :34
- `function parseArgs()` :62
- `function printHelp()` :109
- `async function callRpc(fetchFn, url, token, method, params = {}, id = 1)` :128
- `async function waitForServer(fetchFn, url, token, tries = 40, delayMs = 250)` :145
- `function prettyResult(result)` :168
- `async function main()` :187

### python_complex.py (20 rader)
- `def greet(name)` :1
- `class Calculator` :4
  - `def __init__(self)` :5
  - `def add(self, x, y)` :8
  - `def multiply(self, x, y)` :12

### python_mcp_client.py (275 rader)
- `def build_rpc_payload(` :47
- `def post_rpc(` :55
- `def check_rpc_result(body: Dict[str, Any]) -> Dict[str, Any]` :76
- `def wait_for_server(` :93
- `def rpc_initialize(url: str, token: Optional[str] = None) -> Dict[str, Any]` :116
- `def rpc_tools_list(url: str, token: Optional[str] = None) -> Dict[str, Any]` :121
- `def rpc_tools_call(` :126
- `def pretty_print_result(result: Dict[str, Any]) -> None` :142
- `def cli_list(args: argparse.Namespace)` :162
- `def cli_analyze(args: argparse.Namespace)` :181
- `def cli_call(args: argparse.Namespace)` :193
- `def build_arg_parser() -> argparse.ArgumentParser` :206
- `def main(argv=None)` :240

### qml_complex.qml (41 rader)
_(inga toppnivå-items)_

### typescript_complex.ts (47 rader)
- `interface User` :1
- `interface AppState` :8
- `function processUserData(users: User[]): User[]` :14
- `async function fetchUsers(): Promise<User[]>` :18
- `class UserService` :23
  - `async loadUsers(): Promise<void>` :30
  - `getCurrentUser(): User | null` :41

## examples/temporal-demo/

### notes.py (8 rader)
- `def save_note(text)` :2

### restore_demo.sh (20 rader)
_(inga toppnivå-items)_

## extensions/gemini/

### run_mcp.sh (14 rader)
_(inga toppnivå-items)_

## extensions/vscode/src/

### extension.js (135 rader)
- `function findBinary()` :11
- `class GnawTreeWriterMcpProvider` :63
  - `constructor()` :64
  - `get onDidChangeMcpServerDefinitions()` :70
  - `provideMcpServerDefinitions(token)` :78
- `function activate(context)` :116
- `function deactivate() {}` :133

## extensions/zed/

### build-dist.sh (215 rader)
- `usage()` :28

## extensions/zed/scripts/

### mcp-client.sh (127 rader)
- `usage()` :25
- `run_example()` :70

### mcp-serve.sh (168 rader)
- `usage()` :25

### mcp-stop.sh (141 rader)
- `usage()` :19

### test-mcp.sh (172 rader)
- `usage()` :23
- `cleanup()` :60
- `run_client()` :147

## extensions/zed/src/

### lib.rs (34 rader)
- `pub struct GnawExtension {}` :4
- `impl GnawExtension` :6
  - `pub fn new() -> Self` :7
- `impl zed::Extension for GnawExtension` :12
  - `fn new() -> Self` :13
  - `fn context_server_command(` :18

## scripts/

### benchmark_ai.sh (66 rader)
- `bench()` :11
- `mcp()` :21

### build-gpu.sh (107 rader)
- `die() { echo "ERROR: $*" >&2; exit 1; }` :24

### check-license.sh (14 rader)
_(inga toppnivå-items)_

### commit-build.sh (76 rader)
_(inga toppnivå-items)_

### daemon-hygiene.sh (89 rader)
_(inga toppnivå-items)_

### gtw_adoption_report.py (118 rader)
- `def rows_old_era(cur: sqlite3.Cursor) -> dict[str, dict[str, int]]` :32
- `def rows_new_era(cur: sqlite3.Cursor) -> dict[str, dict[str, int]]` :50
- `def main() -> int` :70

### mcp-client.sh (127 rader)
- `usage()` :25
- `run_example()` :70

### mcp-serve.sh (198 rader)
- `usage()` :55

### mcp-stop.sh (141 rader)
- `usage()` :19

### pack-skill.sh (77 rader)
_(inga toppnivå-items)_

### test-mcp.sh (172 rader)
- `usage()` :23
- `cleanup()` :60
- `run_client()` :147

## scripts/hooks/

### block-text-replace.py (258 rader)
- `def get_file_path(input_data)` :80
- `def find_ast_files_in_command(command)` :104
- `def has_blocked_command_pattern(command)` :132
- `def is_ast_supported(file_path)` :146
- `def handle_pre_tool_use(input_data)` :154
- `def main()` :225

## src/

### auth_demo.rs (19 rader)
_(inga toppnivå-items)_

### cli.rs (6678 rader)
- `enum McpSubcommands` :26
- `pub struct Cli` :71
- `enum Commands` :84
- `enum AiSubcommands` :731
- `enum RuleSubcommands` :779
- `enum TagSubcommands` :804
- `impl Cli` :842
  - `pub async fn run(self) -> Result<()>` :843
  - `fn handle_gnaw_blast(` :1747
  - `fn handle_tag_add(file_path: &str, node_path: &str, name: &str, force: bool) -> Result<()>` :1770
  - `fn handle_tag_list(file_path: &str) -> Result<()>` :1797
  - `fn handle_tag_remove(file_path: &str, name: &str) -> Result<()>` :1813
  - `fn handle_tag_rename(` :1825
  - `fn handle_undo(steps: usize, preview: bool) -> Result<()>` :1844
  - `fn handle_redo(steps: usize) -> Result<()>` :1911
  - `fn handle_history(limit: usize, format: &str) -> Result<()>` :1944
  - `fn handle_restore(file_path: &str, transaction_id: &str, preview: bool) -> Result<()>` :1989
  - `fn handle_batch(file: &str, preview: bool) -> Result<()>` :2028
  - `fn handle_code_index_coverage(path: &Path) -> Result<()>` :2045
  - `fn handle_code_graph(` :2059
  - `fn handle_diff_to_batch(diff_file: &str, output: Option<&str>, preview: bool) -> Result<()>` :2174
  - `fn handle_search(` :2219
  - `fn handle_code_index(` :2309
  - `fn hook_block() -> String` :2412
  - `fn handle_code_index_hook(path: &Path, install: bool, uninstall: bool) -> Result<()>` :2443
  - `fn handle_skeleton(file_path: &str, max_depth: usize, raw: bool) -> Result<()>` :2545
  - `fn handle_compress(file_path: &str, output: Option<&str>, show_stats: bool) -> Result<()>` :2571
  - `fn handle_pack(` :2598
  - `fn handle_curate(` :2661
  - `fn handle_stats(path: &str, format: &str) -> Result<()>` :2723
  - `fn handle_explore(target: &str, level_str: &str, format: &str) -> Result<()>` :2780
  - `fn print_explore_tree(node: &crate::core::explore::ExploreNode, depth: usize)` :2810
  - `async fn handle_semantic_report(file_path: &str) -> Result<()>` :2846
  - `fn sense_degraded_cli(` :2885
  - `async fn handle_sense(` :2937
  - `async fn handle_sense_insert(` :3097
  - `fn handle_version() -> Result<()>` :3170
  - `fn err_modernbert_disabled() -> Result<()>` :3178
  - `fn err_mamba_disabled() -> Result<()>` :3195
  - `fn print_budget(budget: &crate::llm::TokenBudget)` :3213
  - `fn handle_edit_ask(file_path: &str, request: &str, preview: bool, force: bool) -> Result<()>` :3236
  - `fn handle_edit_ask(` :3331
  - `fn handle_edit_ask_all(` :3344
  - `fn handle_edit_ask_all(` :3408
  - `fn handle_explain(file: &PathBuf, node: Option<&str>, resolution: &str) -> Result<()>` :3419
  - `fn handle_explain(_file: &PathBuf, _node: Option<&str>, _resolution: &str) -> Result<()>` :3438
  - `fn handle_summarize(path: &PathBuf, max_files: usize, resolution: &str) -> Result<()>` :3444
  - `fn handle_summarize(_path: &PathBuf, _max_files: usize, _resolution: &str) -> Result<()>` :3470
  - `fn handle_investigate(question: &str, resolution: &str) -> Result<()>` :3476
  - `fn handle_investigate(_question: &str, _resolution: &str) -> Result<()>` :3497
  - `fn handle_blueprint(output_path: Option<&str>) -> Result<()>` :3501
  - `async fn handle_semantic_edit(` :3523
  - `fn handle_gnaw_find(` :3587
  - `fn handle_gnaw_inspect(` :3630
  - `fn handle_gnaw_refactor(` :3667
  - `fn handle_validate(file: &std::path::Path) -> Result<()>` :3704
  - `fn handle_gnaw_diff(` :3724
  - `fn handle_gnaw_graph(` :3750
  - `fn handle_scaffold(file_path: &PathBuf, schema: &str) -> Result<()>` :3773
  - `fn handle_alf(` :3802
  - `async fn handle_ai_setup(force: bool) -> Result<()>` :3876
  - `fn handle_ai_status() -> Result<()>` :3903
  - `fn handle_ai_calibrate() -> Result<()>` :3936
  - `fn handle_ai_calibrate() -> Result<()>` :3953
  - `fn device_mode_from_config_text(text: &str) -> Option<String>` :3961
  - `fn resolve_index_device_from_config() -> Option<String>` :3975
  - `async fn handle_ai_index(path: Option<PathBuf>, gpu: bool) -> Result<()>` :3981
  - `async fn handle_ai_report(limit: usize, output: Option<PathBuf>) -> Result<()>` :4023
  - `fn handle_session_start(name: Option<String>) -> Result<()>` :4041
  - `fn handle_restore_project(timestamp: &str, preview: bool) -> Result<()>` :4058
  - `fn handle_restore_files(since: &str, file_patterns: &[String], preview: bool) -> Result<()>` :4105
  - `fn handle_restore_session(session_id: &str, preview: bool) -> Result<()>` :4170
  - `fn handle_debug_hash(content: &str) -> Result<()>` :4215
  - `fn handle_commands(json: bool) -> Result<()>` :4233
  - `fn handle_examples(topic: Option<&str>) -> Result<()>` :4341
  - `fn handle_wizard(task: Option<&str>) -> Result<()>` :4678
  - `fn show_visual_pulse(writer: &GnawTreeWriter, focus_path: &str, narrative: Option<&str>)` :4897
  - `fn show_visual_diff(` :4923
  - `fn handle_analyze(paths: &[String], format: &str, recursive: bool) -> Result<()>` :4997
  - `fn handle_quick_insert(` :5093
  - `fn find_supported_files(dir: &std::path::Path) -> Result<Vec<String>>` :5198
  - `fn handle_quick_replace(` :5227
  - `fn handle_rename(` :5333
  - `fn handle_clone(` :5345
  - `fn handle_move(` :5422
  - `fn find_node_by_path<'a>(tree: &'a TreeNode, path: &str) -> Option<&'a TreeNode>` :5501
  - `fn handle_lint(` :5514
  - `fn handle_rules_add(` :5639
  - `fn handle_rules_list() -> Result<()>` :5683
  - `fn handle_lint_discover(paths: &[String]) -> Result<()>` :5710
  - `fn handle_lint_discover(_paths: &[String]) -> Result<()>` :5843
  - `fn parse_rule_proposals(s: &str) -> Result<Vec<serde_json::Value>>` :5851
  - `fn handle_doctor(format: Option<&str>) -> Result<()>` :5863
  - `async fn handle_health_check() -> Result<()>` :5926
- `fn print_diff(old: &str, new: &str)` :6154
- `fn show_hint()` :6167
- `fn list_nodes(` :6197
- `fn resolve_content(` :6258
- `fn parse_user_timestamp(timestamp: &str) -> Result<chrono::DateTime<chrono::Utc>>` :6295

### lib.rs (11 rader)
_(inga toppnivå-items)_

### main.rs (77 rader)
- `async fn main() -> Result<()>` :10

### network.rs (14 rader)
- `pub struct Config` :8
- `pub fn start()` :12

### version.rs (236 rader)
- `pub struct BuildInfo` :19
- `pub fn current() -> BuildInfo` :31
- `fn is_stale(built_sha: &str, built_epoch: u64) -> bool` :57
- `fn code_head_moved(built_sha: &str) -> bool` :70
- `const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR")` :103
- `fn src_newer_than(built_epoch: u64) -> Option<bool>` :107
- `fn walk(dir: &std::path::Path, newest: &mut u64, found_any: &mut bool)` :127
- `fn run_git(args: &[&str]) -> Option<String>` :144
- `pub fn format_epoch_utc(secs: u64) -> String` :159

## src/core/

### alf.rs (122 rader)
- `pub enum AlfType` :9
- `pub struct AlfEntry` :19
- `pub struct AlfManager` :30
- `impl AlfManager` :36
  - `pub fn load(project_root: &Path) -> Result<Self>` :37
  - `pub fn set_actor(&mut self, actor: &str)` :58
  - `pub fn log(` :62
  - `pub fn add_tag(&mut self, alf_id: &str, tag: &str) -> Result<()>` :85
  - `pub fn update_message(&mut self, alf_id: &str, new_message: &str) -> Result<()>` :97
  - `pub fn find_by_txn(&self, txn_id: &str) -> Option<&AlfEntry>` :107
  - `pub fn list(&self, limit: usize) -> Vec<AlfEntry>` :113
  - `fn save(&self) -> Result<()>` :117

### anchor.rs (166 rader)
- `pub struct Anchor` :11
- `pub enum AnchorStyle` :26
- `pub struct AnchorDetector` :40
- `impl Default for AnchorDetector` :44
  - `fn default() -> Self` :45
- `impl AnchorDetector` :50
  - `pub fn new() -> Self` :52
  - `pub fn detect(&self, code: &str) -> Vec<Anchor>` :92
  - `fn extract_hint(&self, text: &str, style: AnchorStyle) -> Option<String>` :116
  - `fn deduplicate_overlapping(&self, anchors: Vec<Anchor>) -> Vec<Anchor>` :145

### backup.rs (260 rader)
- `pub struct BackupFile` :18
- `pub fn list_backup_files<P: AsRef<Path>>(backup_dir: P) -> Result<Vec<BackupFile>>` :32
- `pub fn parse_backup_file<P: AsRef<Path>>(backup_path: P) -> Result<BackupFile>` :66
- `pub fn find_backup_by_content_hash<P: AsRef<Path>>(` :108
- `pub fn find_backup_by_content_hash_for_file<P: AsRef<Path>>(` :125
- `pub fn restore_from_backup<P: AsRef<Path>, Q: AsRef<Path>>(` :152

### batch.rs (897 rader)
- `pub enum BatchOp` :31
- `pub enum AnchorPosition` :64
- `pub enum BatchEdit` :71
- `pub struct BatchFile` :87
- `pub struct FileDiff` :93
- `pub struct Batch` :100
- `impl Batch` :119
  - `pub fn new() -> Self` :121
  - `pub fn with_file(file_path: String, operations: Vec<BatchEdit>) -> Self` :126
  - `pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self>` :162
  - `pub fn from_json(s: &str) -> Result<Self>` :168
  - `pub fn preview(&self) -> Result<Vec<FileDiff>>` :180
  - `pub fn apply(&self) -> Result<()>` :394
  - `fn description_or_ops(&self) -> String` :476
  - `pub fn preview_text(&self) -> Result<String>` :485
- `fn insert_text_splice(` :505
- `fn format_diff(before: &str, after: &str) -> String` :542

### blast.rs (310 rader)
- `pub enum ImpactLevel` :11
- `impl std::fmt::Display for ImpactLevel` :18
  - `fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` :19
- `pub struct BlastResult` :30
- `pub struct CodeRelation` :43
- `pub struct ImpactSummary` :52
- `pub fn blast(` :59
- `fn collect_all_nodes(node: &TreeNode) -> Vec<&TreeNode>` :120
- `pub fn find_callers_in_file(tree: &TreeNode, target: &str, file: &str) -> Vec<CodeRelation>` :128
- `fn find_all_functions(tree: &TreeNode) -> Vec<&TreeNode>` :168
- `fn find_parent_function<'a>(node: &'a TreeNode, tree: &'a TreeNode) -> Option<&'a TreeNode>` :181
- `pub fn find_callers_project(target: &str, directory: Option<&str>) -> Result<Vec<CodeRelation>>` :200
- `pub fn find_callees(target: &TreeNode) -> Vec<CodeRelation>` :221
- `fn collect_callees_recursive(node: &TreeNode, results: &mut Vec<CodeRelation>)` :227
- `fn calculate_risk(` :243
- `pub fn format_blast_text(result: &BlastResult) -> String` :255

### blueprint.rs (182 rader)
- `pub struct BlueprintEngine` :7
- `pub struct ProjectBlueprint` :11
- `impl BlueprintEngine` :18
  - `pub fn new(project_root: &Path) -> Self` :19
  - `pub fn generate(&self) -> Result<ProjectBlueprint>` :25
  - `pub fn render_to_terminal(&self, blueprint: &ProjectBlueprint)` :103
  - `pub fn render_to_markdown(&self, blueprint: &ProjectBlueprint) -> String` :145

### code_graph.rs (525 rader)
- `pub struct Reach` :27
- `pub struct Cycle` :33
- `pub fn reachable(conn: &Connection, start: &str, max_depth: usize) -> Result<Vec<Reach>>` :44
- `pub fn cycles(conn: &Connection, max_depth: usize) -> Result<Vec<Cycle>>` :78
- `fn rotate_to_smallest(c: &mut [String])` :142
- `pub fn chain(conn: &Connection, from: &str, to: &str) -> Result<Option<Vec<String>>>` :154
- `fn files_of(conn: &Connection, nodes: &[String]) -> Result<Vec<String>>` :194
- `pub struct SymbolSeed` :219
- `pub struct ImpactReport` :228
- `pub fn resolve_symbols(conn: &Connection, symbol: &str) -> Result<Vec<SymbolSeed>>` :237
- `pub fn impact(conn: &Connection, symbol: &str, max_depth: usize) -> Result<ImpactReport>` :252

### code_index.rs (683 rader)
- `fn item_category(kind: &str) -> Option<&'static str>` :30
- `fn is_container(kind: &str) -> bool` :81
- `pub struct IndexItem` :89
- `pub(crate) const STORED_SIG_CAP: usize = 512` :107
- `fn truncate_sig(sig: &str, max_len: usize) -> String` :111
- `fn signature(node: &TreeNode) -> String` :121
- `pub fn extract_items(root: &TreeNode) -> Vec<IndexItem>` :136
- `pub fn parse_intros(text: &str) -> (BTreeMap<String, String>, Vec<String>)` :192
- `pub struct CodeIndexOptions` :230
- `impl Default for CodeIndexOptions` :237
  - `fn default() -> Self` :238
- `pub(crate) fn git_facts(root: &Path) -> (String, bool)` :259
- `pub(crate) struct FileIndex` :279
- `pub(crate) fn canonical_rel_path(root: &Path, path: &Path) -> String` :288
- `pub(crate) fn render_doc(` :299
- `fn render_item(i: &IndexItem, max_sig: usize) -> IndexItem` :444
- `pub fn generate_index(root: &Path, opts: &CodeIndexOptions) -> anyhow::Result<String>` :457
- `pub fn body_hash(body: &str) -> String` :505
- `fn split_header_body(doc: &str) -> Option<(&str, &str)>` :511
- `pub struct CheckResult` :517
- `pub fn check_index(` :528

### code_index_coverage.rs (385 rader)
- `pub struct TypeCoverage` :27
- `pub struct SourceClass` :40
- `pub struct TargetClass` :48
- `pub struct SymbolSummary` :54
- `pub struct ProseSummary` :64
- `pub struct CoverageReport` :74
- `pub fn measure(conn: &Connection) -> Result<CoverageReport>` :95
- `fn relations_total(conn: &Connection) -> anyhow::Result<i64>` :253

### code_index_db.rs (1322 rader)
- `pub const EXTRACTOR_VERSION: &str = "3"` :35
- `const SCHEMA_VERSION: i32 = 4` :37
- `const BUSY_TIMEOUT_MS: u64 = 5000` :38
- `const SCHEMA_V1: &str = r#"` :40
- `const SCHEMA_V2: &str = r#"` :77
- `const SCHEMA_V3: &str = r#"` :109
- `pub(crate) const IDENTITY_SCHEME_VERSION: &str = "1"` :118
- `const SCHEMA_V4: &str = r#"` :125
- `pub struct RefreshStats` :191
- `pub struct DbRun` :200
- `pub fn db_path(root: &Path) -> PathBuf` :210
- `pub fn resolve_index_root(root: &Path) -> PathBuf` :217
- `fn sidecar(path: &Path, suffix: &str) -> PathBuf` :237
- `fn open_and_migrate(path: &Path) -> Result<Connection>` :243
- `pub(crate) fn migrate_connection(conn: &Connection) -> Result<()>` :255
- `fn repair_prose_columns(conn: &Connection) -> Result<()>` :292
- `pub fn open_or_recreate(root: &Path) -> Result<(Connection, bool)>` :331
- `pub fn open_existing(root: &Path) -> Result<Connection>` :352
- `pub struct StoreStatus` :375
- `pub fn store_status(root: &Path) -> Option<StoreStatus>` :382
- `fn store_is_newer(store: &str, mine: &str) -> bool` :405
- `fn generation_key(opts: &CodeIndexOptions) -> String` :412
- `fn meta_get(conn: &Connection, key: &str) -> Result<Option<String>>` :423
- `fn meta_set(conn: &Connection, key: &str, value: &str) -> Result<()>` :432
- `fn upsert_file(` :443
- `pub fn refresh(` :498
- `pub(crate) fn record_revision(conn: &mut Connection, root: &Path, source: &str) -> Result<i64>` :654
- `pub fn upsert_prose(` :692
- `pub(crate) fn sweep_prose_freshness(conn: &mut Connection) -> Result<()>` :722
- `pub(crate) fn load_index(conn: &Connection) -> Result<Vec<FileIndex>>` :828
- `fn load_warnings(conn: &Connection) -> Result<Vec<String>>` :893
- `pub fn render(` :906
- `pub fn generate_with_db(root: &Path, opts: &CodeIndexOptions) -> Result<DbRun>` :920
- `fn store_entities_relations(` :969
- `fn resolve_unresolved_in_db(conn: &Connection) -> Result<()>` :1035
- `pub fn mark_stale(root: &Path, file_path: &str)` :1115
- `pub fn content_hash(path: &Path) -> Option<String>` :1137
- `pub fn is_stale(root: &Path, file: &str) -> bool` :1146
- `pub fn stale_files(root: &Path) -> std::collections::BTreeSet<String>` :1165
- `pub fn load_entities(conn: &Connection, prefix: Option<&str>) -> Result<Vec<serde_json::Value>>` :1186
- `pub fn load_items(conn: &Connection, prefix: Option<&str>) -> Result<Vec<serde_json::Value>>` :1211
- `pub fn load_relations(conn: &Connection, prefix: Option<&str>) -> Result<Vec<serde_json::Value>>` :1237
- `pub fn load_symbols(conn: &Connection) -> Result<Vec<serde_json::Value>>` :1261
- `pub fn is_busy_error(err: &anyhow::Error) -> bool` :1283

### compress.rs (487 rader)
- `pub struct CompressedOutput` :12
- `pub fn compress_file(file_path: &str) -> Result<CompressedOutput, anyhow::Error>` :26
- `pub fn compress_source(source: &str, tree: &TreeNode) -> CompressedOutput` :34
- `fn collect_body_replacements(node: &TreeNode, replacements: &mut Vec<(usize, usize)>)` :99
- `fn should_compress_body(node: &TreeNode) -> bool` :115
- `fn is_body_node(node: &TreeNode) -> bool` :161

### curator.rs (709 rader)
- `pub enum CurationStrategy` :22
- `impl CurationStrategy` :33
  - `pub fn parse(s: &str) -> Self` :34
- `pub struct CuratedFile` :46
- `pub struct CuratedContext` :61
- `pub fn curate_context(` :73
- `fn score_by_relevance(files: &[PathBuf], root: &Path, task: &str) -> Vec<CuratedFile>` :188
- `fn score_by_recent_changes(files: &[PathBuf], root: &Path) -> Vec<CuratedFile>` :256
- `fn score_by_dependencies(files: &[PathBuf], root: &Path, target: &str) -> Vec<CuratedFile>` :303
- `fn check_if_definition(tree: &TreeNode, target: &str) -> bool` :382

### diagnostics.rs (1077 rader)
- `pub struct DiagnosticReport` :7
- `pub struct DiagnosticError` :22
- `pub struct VerboseTrace` :36
- `pub struct VerboseStep` :41
- `pub struct AstDiff` :51
- `pub struct StructuralChange` :63
- `pub struct DoctorReport` :76
- `fn semantic_model_present(project_root: &std::path::Path) -> bool` :94
- `pub fn capability_level(project_root: &std::path::Path) -> &'static str` :108
- `pub struct DoctorCheck` :117
- `pub fn run_parser_smoke(extension: &str, code: &str) -> DoctorCheck` :129
- `impl DiagnosticReport` :159
  - `pub fn ok(tool: &str, file: &str, language: &str) -> Self` :160
  - `pub fn err(tool: &str, file: &str, language: &str, error: DiagnosticError) -> Self` :173
  - `pub fn with_verbose(&mut self, trace: VerboseTrace) -> &mut Self` :186
  - `pub fn with_ast_diff(&mut self, diff: AstDiff) -> &mut Self` :191
- `impl DiagnosticError` :197
  - `pub fn from_anyhow(err: &anyhow::Error, language: &str) -> Self` :198
  - `pub fn with_context_line(mut self, source: &str) -> Self` :233
- `impl Default for VerboseTrace` :244
  - `fn default() -> Self` :245
- `impl VerboseTrace` :250
  - `pub fn new() -> Self` :251
  - `pub fn step(&mut self, phase: &str, message: &str) -> &mut Self` :255
  - `pub fn step_with_detail(&mut self, phase: &str, message: &str, detail: &str) -> &mut Self` :265
- `impl AstDiff` :276
  - `pub fn compare(before: &TreeNode, after: &TreeNode) -> Self` :278
  - `fn compare_children(before: &TreeNode, after: &TreeNode, changes: &mut Vec<StructuralChange>)` :347
- `impl Default for DoctorReport` :386
  - `fn default() -> Self` :387
- `impl DoctorReport` :392
  - `pub fn new() -> Self` :393
  - `pub fn pass(&mut self, category: &str, name: &str, message: &str)` :405
  - `pub fn fail(&mut self, category: &str, name: &str, message: &str)` :417
  - `pub fn warn(&mut self, category: &str, name: &str, message: &str)` :430
  - `pub fn record_check(&mut self, check: DoctorCheck)` :445
  - `pub fn check_parser(&mut self, extension: &str, code: &str)` :460
  - `pub fn check_backups(&mut self, project_root: &std::path::Path)` :465
  - `pub fn check_transaction_log(&mut self, project_root: &std::path::Path)` :529
  - `pub fn check_semantic_model(&mut self, project_root: &std::path::Path)` :562
  - `pub fn check_knowledge_graph(&mut self, project_root: &std::path::Path)` :582
  - `pub fn check_code_index_store(&mut self, project_root: &std::path::Path)` :651
  - `pub fn check_binary_freshness(&mut self, project_root: &std::path::Path)` :689
- `fn extract_line_number(msg: &str) -> Option<usize>` :732
- `fn generate_suggestion(error_type: &str, language: &str, _msg: &str) -> Option<String>` :754
- `fn collect_node_types(node: &TreeNode) -> std::collections::HashMap<String, usize>` :787
- `fn count_nodes(node: &TreeNode) -> usize` :799
- `fn max_depth(node: &TreeNode, current: usize) -> usize` :803
- `fn is_important_node_type(ntype: String) -> bool` :815
- `fn is_gnawtreewriter_manifest(text: &str) -> bool` :834
- `fn cargo_toml_version(text: &str) -> Option<String>` :845
- `fn compare_version_triples(a: &str, b: &str) -> std::cmp::Ordering` :872
- `fn installed_binary_date() -> String` :895
- `pub fn run_full_doctor(project_root: &std::path::Path) -> DoctorReport` :908

### diff_parser.rs (586 rader)
- `pub struct DiffHunk` :20
- `pub enum DiffLine` :37
- `pub struct ParsedDiff` :48
- `pub struct DiffMetadata` :57
- `pub fn parse_unified_diff(diff: &str) -> Result<ParsedDiff>` :65
- `pub fn diff_to_batch(diff: &ParsedDiff) -> Result<Batch>` :168
- `fn resolve_line_to_node(tree: &TreeNode, line: usize, source: &str) -> String` :307
- `fn compute_insert_position(root: &TreeNode, parent: &TreeNode, line: usize) -> usize` :361
- `fn find_editable_ancestor<'a>(tree: &'a TreeNode, target_path: &str) -> Option<&'a TreeNode>` :396
- `fn normalize_path(path: &str) -> PathBuf` :433
- `pub fn parse_diff_file<P: AsRef<Path>>(diff_file: P) -> Result<ParsedDiff>` :441
- `pub fn preview_diff(diff: &ParsedDiff) -> String` :448

### edit_delta.rs (749 rader)
- `pub enum DeltaKind` :15
- `pub struct EditDelta` :28
- `const MAX_DEPTH: usize = 24` :35
- `const MAX_DELTAS: usize = 16` :36
- `fn is_critical(node_type: &str) -> bool` :40
- `fn is_definition(node_type: &str) -> bool` :54
- `fn is_noise(content: &str) -> bool` :63
- `fn meaningful_children(node: &TreeNode) -> Vec<&TreeNode>` :68
- `fn is_operator_token(node_type: &str) -> bool` :75
- `fn is_number_token(node_type: &str, content: &str) -> bool` :85
- `fn count_word(content: &str, word: &str) -> usize` :100
- `fn multiset_removed(old: &[String], new: &[String]) -> Vec<String>` :123
- `fn header(content: &str) -> String` :139
- `pub fn unwrap_root(tree: &TreeNode) -> &TreeNode` :152
- `pub fn diff_node_trees(old: &TreeNode, new: &TreeNode) -> Vec<EditDelta>` :166
- `fn diff_walk(old: &TreeNode, new: &TreeNode, depth: usize, deltas: &mut Vec<EditDelta>)` :172
- `fn diff_operators(old: &TreeNode, new: &TreeNode, deltas: &mut Vec<EditDelta>)` :271
- `fn diff_logic_operators(old: &TreeNode, new: &TreeNode, deltas: &mut Vec<EditDelta>)` :314
- `fn diff_numbers(old: &TreeNode, new: &TreeNode, deltas: &mut Vec<EditDelta>)` :333
- `fn diff_error_handling(old: &TreeNode, new: &TreeNode, deltas: &mut Vec<EditDelta>)` :356
- `fn diff_tokens_fallback(old: &TreeNode, new: &TreeNode, deltas: &mut Vec<EditDelta>)` :395
- `pub struct Contract` :425
- `pub fn extract_contract(node: &TreeNode) -> Contract` :435
- `fn guard_lines(content: &str) -> Vec<String>` :456
- `pub fn check_contract(old: &Contract, new_content: &str) -> Vec<EditDelta>` :475
- `fn classify_removed(gone: &TreeNode, parent_type: &str, deltas: &mut Vec<EditDelta>)` :566

### explore.rs (567 rader)
- `pub enum ZoomLevel` :20
- `impl ZoomLevel` :31
  - `pub fn parse(s: &str) -> Self` :32
- `pub struct ExploreNode` :45
- `pub struct ExploreResult` :66
- `pub fn explore(root: &Path, target: &str, level: ZoomLevel) -> Result<ExploreResult>` :78
- `fn explore_overview(root: &Path) -> Result<ExploreNode>` :123
- `fn explore_directory(target: &Path) -> Result<ExploreNode>` :236
- `fn explore_file(target: &Path) -> Result<ExploreNode>` :306
- `fn extract_signatures(node: &TreeNode, children: &mut Vec<ExploreNode>, depth: usize)` :334
- `fn explore_full(target: &Path) -> Result<ExploreNode>` :391
- `struct DirStats` :417
- `struct FileEntry` :424
- `fn get_file_summary(path: &Path) -> Vec<ExploreNode>` :432

### file_walker.rs (143 rader)
- `pub fn walk_source_files(root: &Path) -> Vec<PathBuf>` :15
- `pub fn walk_source_files_filtered(root: &Path, extensions: &[&str]) -> Vec<PathBuf>` :39
- `pub fn has_extension(path: &Path, extensions: &[&str]) -> bool` :56

### gnaw_diff.rs (267 rader)
- `pub struct DiffResult` :9
- `pub struct DiffChange` :18
- `pub struct DiffModification` :27
- `pub struct DiffSummary` :36
- `impl Clone for DiffSummary` :43
  - `fn clone(&self) -> Self` :44
- `pub fn diff(old_file: &str, new_file: &str, _format: &str) -> Result<DiffResult>` :55
- `fn compare_trees<'a>(` :81
- `fn find_modifications(` :124
- `impl TreeNode` :160
  - `fn get_all_paths(&self) -> Vec<String>` :161
  - `fn get_all_nodes(&self) -> Vec<&TreeNode>` :169
- `pub fn format_diff_text(result: &DiffResult) -> String` :179
- `pub fn diff_nodes(file_path: &str, old_path: &str, new_path: &str) -> Result<String>` :228

### gnaw_find.rs (380 rader)
- `pub fn find_nodes(` :11
- `fn collect_by_type(` :70
- `fn collect_by_text(` :99
- `pub struct FindResult` :125
- `pub fn format_results_text(results: &[FindResult], total: usize, max: usize) -> String` :134
- `pub fn format_results_summary(results: &[FindResult], total: usize) -> String` :157
- `pub struct LexicalHit` :173
- `fn lexical_terms(query: &str) -> Vec<String>` :183
- `pub fn lexical_degrade(` :198
- `fn collect_lexical(` :243
- `fn first_line_preview(content: &str) -> String` :300

### gnaw_graph.rs (366 rader)
- `pub struct GraphResult` :9
- `pub struct GraphNode` :17
- `pub struct GraphEdge` :26
- `pub struct GraphSummary` :33
- `pub fn graph(` :41
- `fn collect_functions_and_calls(` :85
- `fn find_parent_function_id(node: &TreeNode, nodes: &Vec<GraphNode>) -> Option<String>` :160
- `fn get_call_target(node: &TreeNode) -> Option<String>` :173
- `fn find_function_id(name: &str, nodes: &Vec<GraphNode>) -> Option<String>` :183
- `pub fn format_graph_text(result: &GraphResult) -> String` :201
- `pub fn format_graph_mermaid(result: &GraphResult) -> String` :256
- `pub fn format_graph_dot(result: &GraphResult) -> String` :285
- `pub fn format_graph_tree(result: &GraphResult) -> String` :311

### gnaw_refactor.rs (369 rader)
- `pub enum RefactorKind` :10
- `impl std::fmt::Display for RefactorKind` :23
  - `fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` :24
- `pub struct RefactorResult` :36
- `pub struct Change` :46
- `pub struct RefactorSummary` :55
- `pub fn refactor(` :63
- `fn rename_symbol(` :95
- `fn find_and_rename_references(old_name: &str, new_name: &str) -> Result<Vec<Change>>` :149
- `fn collect_nodes_matching<'a>(tree: &'a TreeNode, pattern: &str) -> Vec<&'a TreeNode>` :175
- `fn extract_function(` :186
- `fn move_code(` :231
- `fn change_signature(` :267
- `fn inline_function(file_path: &str, node_path: &str, _preview: bool) -> Result<RefactorResult>` :303
- `pub fn format_refactor_text(result: &RefactorResult) -> String` :335

### guardian.rs (332 rader)
- `pub enum IntegrityLevel` :6
- `pub struct IntegrityReport` :14
- `pub struct GuardianEngine` :25
- `impl Default for GuardianEngine` :27
  - `fn default() -> Self` :28
- `impl GuardianEngine` :33
  - `pub fn new() -> Self` :34
  - `pub fn audit_edit(&self, old_node: &TreeNode, new_content: &str) -> IntegrityReport` :39
  - `pub fn audit_edit_with_language(` :48
  - `fn estimate_complexity(&self, content: &str) -> usize` :157

### healer.rs (78 rader)
- `pub struct HealingAction` :5
- `pub struct Healer` :11
- `impl Default for Healer` :13
  - `fn default() -> Self` :14
- `impl Healer` :19
  - `pub fn new() -> Self` :20
  - `pub fn suggest_fix(` :25
  - `fn heal_brace_languages(&self, _code: &str, error: &SyntaxError) -> Option<HealingAction>` :40
  - `fn heal_python(&self, code: &str, error: &SyntaxError) -> Option<HealingAction>` :61

### index_entities.rs (1011 rader)
- `pub struct Entity` :13
- `pub struct EntityIndex` :42
- `pub fn index_entities(file_path: &str, include_private: bool) -> Result<EntityIndex>` :56
- `pub fn index_entities_in(` :64
- `pub fn extract_entities_from_tree(` :78
- `pub fn extract_entities_from_tree_in(` :88
- `pub fn canonical_path_within(file_path: &str, boundary: &std::path::Path) -> String` :110
- `pub fn canonical_entity_path_in(file_path: &str, boundary: Option<&std::path::Path>) -> String` :123
- `pub fn canonical_entity_path(file_path: &str) -> String` :134
- `fn disambiguate_ids(entities: &mut [Entity])` :159
- `fn collect_entities(` :179
- `fn get_node_source(node: &TreeNode, lines: &[&str]) -> String` :296
- `fn get_first_line(node: &TreeNode, lines: &[&str]) -> String` :302
- `fn detect_visibility(node: &TreeNode) -> String` :311
- `fn extract_doc_comment(node: &TreeNode, lines: &[&str]) -> Option<String>` :343
- `fn extract_function(` :368
- `fn extract_struct(` :410
- `fn extract_enum(` :487
- `fn extract_impl(` :562
- `fn extract_trait(` :633
- `fn extract_type_alias(` :688
- `fn extract_const(` :719
- `fn extract_static(` :749
- `fn extract_module(` :779

### index_relations.rs (839 rader)
- `pub struct Relation` :14
- `pub struct RelationIndex` :43
- `pub fn index_relations(file_path: &str) -> Result<RelationIndex>` :53
- `pub fn index_relations_in(` :59
- `pub fn extract_relations_from_tree(` :72
- `pub fn resolve_relations(relations: &mut [Relation], entities: &[Entity])` :112
- `pub(crate) fn is_method_entity_id(id: &str) -> bool` :181
- `fn get_line(lines: &[&str], line_num: usize) -> String` :264
- `fn extract_imports(` :273
- `fn extract_import_target(import_line: &str) -> Option<String>` :346
- `fn extract_calls(tree: &TreeNode, file_path: &str, lines: &[&str], relations: &mut Vec<Relation>)` :371
- `fn is_function_like(node_type: &str) -> bool` :376
- `fn find_calls_in_scope(` :383
- `fn extract_type_usage(` :504
- `fn extract_type_refs(` :537
- `fn extract_impl_relations(` :563
- `fn extract_trait_from_impl(impl_text: &str) -> Option<String>` :606

### inspect.rs (514 rader)
- `pub enum InspectMode` :12
- `pub struct InspectResult` :29
- `pub struct Finding` :38
- `pub fn inspect(` :48
- `fn inspect_file(file_path: &str, mode: InspectMode, symbol: Option<&str>) -> Result<InspectResult>` :71
- `fn inspect_project(` :129
- `fn find_callers(` :164
- `fn collect_functions_and_calls(` :205
- `fn calculate_metrics(` :262
- `fn count_nodes(tree: &crate::parser::TreeNode, metrics: &mut HashMap<String, usize>)` :291
- `fn collect_function_sizes(` :300
- `fn find_orphans(` :321
- `fn collect_symbols(` :361
- `fn analyze_relations(` :402
- `fn collect_relations(` :436
- `pub fn format_inspect_text(results: &[InspectResult]) -> String` :477

### label_manager.rs (90 rader)
- `pub struct LabelInfo` :9
- `pub struct LabelStore` :16
- `pub struct LabelManager` :20
- `impl LabelManager` :25
  - `pub fn load(project_root: &Path) -> Result<Self>` :26
  - `pub fn save(&self) -> Result<()>` :38
  - `pub fn add_label(&mut self, file_path: &str, node_content: &str, label: &str) -> Result<()>` :44
  - `pub fn get_labels(&self, file_path: &str, node_content: &str) -> Vec<String>` :61
  - `pub fn remove_label(` :71

### macro_dispatcher.rs (249 rader)
- `pub trait MacroParser: Send + Sync` :14
- `pub struct MacroDispatcher` :26
- `impl MacroDispatcher` :30
  - `pub fn new() -> Self` :31
  - `pub fn register(&mut self, parser: Box<dyn MacroParser>)` :37
  - `pub fn get(&self, name: &str) -> Option<&dyn MacroParser>` :42
  - `pub fn has(&self, name: &str) -> bool` :46
- `static DISPATCHER: std::sync::OnceLock<MacroDispatcher> = std::sync::OnceLock::new()` :52
- `pub fn dispatcher() -> &'static MacroDispatcher` :54
- `struct JsonMacroParser` :64
- `impl MacroParser for JsonMacroParser` :66
  - `fn macro_name(&self) -> &str` :67
  - `fn parse_macro_body(&self, content: &str, base_path: &str) -> Result<Vec<TreeNode>>` :71
- `fn json_value_to_tree(value: &serde_json::Value, base_path: &str) -> TreeNode` :77
- `fn content_str(value: &serde_json::Value) -> String` :126
- `pub fn extract_macro_name(node: &TreeNode) -> Option<&str>` :138
- `pub fn is_token_tree(node: &TreeNode) -> bool` :147
- `pub fn try_expand_macro(` :152

### mod.rs (2270 rader)
- `pub struct GnawTreeWriter` :65
- `pub enum EditOperation` :88
- `pub struct EditReceipt` :112
- `pub(crate) fn skeleton_outline(` :135
- `pub(crate) fn detect_replacement_remnant(` :215
- `pub(crate) fn check_duplicate_attribute_above(` :284
- `impl GnawTreeWriter` :321
  - `pub fn new(file_path: &str) -> Result<Self>` :322
  - `pub fn new_with_boundary(file_path: &str, boundary: Option<&Path>) -> Result<Self>` :332
  - `pub fn new_lenient(file_path: &str) -> Result<(Self, Option<crate::parser::SyntaxError>)>` :364
  - `pub fn new_lenient_with_boundary(` :371
  - `pub fn last_backup_path(&self) -> Option<PathBuf>` :404
  - `pub fn last_transaction_id(&self) -> Option<String>` :410
  - `pub fn last_edit_receipt(&self) -> Option<&EditReceipt>` :417
  - `pub fn last_edit_verdict(&self) -> Option<&serde_json::Value>` :425
  - `fn set_verdict(` :429
  - `pub fn last_edit_impact(&self) -> Option<&serde_json::Value>` :447
  - `pub(crate) fn create_backup(&mut self) -> Result<PathBuf>` :451
  - `pub fn analyze(&self) -> &TreeNode` :485
  - `pub fn show_node(&self, node_path: &str) -> Result<String>` :489
  - `pub fn expect_node(` :503
  - `pub fn edit(&mut self, operation: EditOperation, force: bool) -> Result<()>` :545
  - `pub fn preview_edit(&self, operation: EditOperation) -> Result<String>` :972
  - `fn resolve_path<'a>(&'a self, query: &str) -> Option<&'a TreeNode>` :1011
  - `fn find_node_by_path<'a>(&self, tree: &'a TreeNode, path: &str) -> Option<&'a TreeNode>` :1027
  - `fn find_node_by_name<'a>(` :1042
  - `fn doc_block_start_line(&self, node: &TreeNode) -> usize` :1094
  - `pub fn describe_node(&self, query: &str) -> Option<(String, String, Option<String>)>` :1116
  - `fn json_semantic_edit(&self, node_path: &str, new_content: &str) -> Result<String>` :1137
  - `fn json_new_value(content: &str, allow_string: bool) -> Option<serde_json::Value>` :1240
  - `fn json_content_error(content: &str, target: &str) -> anyhow::Error` :1250
  - `fn json_render(v: &serde_json::Value) -> String` :1262
  - `fn join_preserving_newline<S: AsRef<str>>(lines: &[S], source: &str) -> String` :1277
  - `fn edit_node_at_path(&self, node_path: &str, new_content: &str) -> Result<String>` :1296
  - `fn insert_node_at_path(` :1407
  - `fn delete_node_at_path(&self, node_path: &str) -> Result<String>` :1528
  - `pub fn get_source(&self) -> &str` :1559
- `pub fn find_project_root(start_path: &Path) -> PathBuf` :1566
- `pub fn find_project_root_in(start_path: &Path, boundary: Option<&Path>) -> PathBuf` :1579
- `fn count_changed_bytes(before: &str, after: &str) -> usize` :1617
- `pub(crate) fn verdict_json(` :1629
- `pub(crate) fn signature_impact_for(` :1648

### pack.rs (749 rader)
- `pub enum PackFormat` :15
- `impl PackFormat` :26
  - `pub fn parse(s: &str) -> Self` :27
- `pub struct PackOptions` :39
- `impl Default for PackOptions` :60
  - `fn default() -> Self` :61
- `pub struct PackResult` :78
- `pub struct PackFileInfo` :94
- `fn should_compress(options: &PackOptions, token_count: usize) -> bool` :102
- `fn compress_file(options: &PackOptions, tree_path: &str, content: &str) -> String` :108
- `pub fn pack_project(root: &Path, options: &PackOptions) -> Result<PackResult>` :127
- `fn format_markdown(files: &[(String, String, String)], options: &PackOptions) -> String` :231
- `fn format_json(files: &[(String, String, String)], options: &PackOptions) -> Result<String>` :313
- `fn format_plain(files: &[(String, String, String)], options: &PackOptions) -> String` :371
- `fn format_xml(files: &[(String, String, String)], options: &PackOptions) -> String` :403
- `fn xml_escape(s: &str) -> String` :447

### parse_cache.rs (119 rader)
- `pub struct ParseCache` :10
- `impl ParseCache` :14
  - `pub fn new() -> Self` :15
  - `pub fn get_or_parse(&self, file_path: &str) -> Option<(String, crate::parser::TreeNode)>` :22
  - `pub fn clear(&self)` :45
  - `pub fn len(&self) -> usize` :52
  - `pub fn is_empty(&self) -> bool` :57
- `impl Default for ParseCache` :62
  - `fn default() -> Self` :63
- `static SESSION_CACHE: std::sync::LazyLock<ParseCache> = std::sync::LazyLock::new(ParseCache::new)` :69
- `pub fn session_cache() -> &'static ParseCache` :72

### report.rs (86 rader)
- `pub struct ReportEngine` :6
- `pub struct Snapshot` :8
- `impl Default for ReportEngine` :17
  - `fn default() -> Self` :18
- `impl ReportEngine` :23
  - `pub fn new() -> Self` :24
  - `pub fn generate_markdown_report(&self, project_root: &Path, limit: usize) -> Result<String>` :28

### restoration_engine.rs (446 rader)
- `pub struct RestorationEngine` :9
- `pub struct RestorationResult` :16
- `pub struct BackupFile` :24
- `impl RestorationEngine` :31
  - `pub fn new<P: AsRef<Path>>(project_root: P) -> Result<Self>` :33
  - `pub fn execute_project_restoration(` :50
  - `pub fn restore_file_to_transaction(&self, transaction_id: &str) -> Result<PathBuf>` :104
  - `fn restore_by_hash(&self, transaction: &Transaction) -> Result<PathBuf>` :121
  - `fn restore_by_timestamp(&self, transaction: &Transaction) -> Result<PathBuf>` :149
  - `pub fn restore_files_before_timestamp(` :188
  - `pub fn restore_file_before_timestamp(` :244
  - `pub fn restore_session(&self, session_id: &str) -> Result<RestorationResult>` :266
  - `fn get_session_transactions(&self, session_id: &str) -> Result<Vec<Transaction>>` :294
  - `fn find_next_transaction_for_file(` :303
  - `fn find_backup_by_content_hash(&self, content_hash: &str) -> Result<Option<BackupFile>>` :325
  - `fn list_backup_files(&self) -> Result<Vec<BackupFile>>` :341
  - `fn restore_from_backup(&self, target_path: &Path, backup_path: &Path) -> Result<PathBuf>` :358
  - `pub fn get_restoration_stats(&self) -> Result<RestorationStats>` :364
- `pub struct RestorationStats` :385
- `impl RestorationResult` :393
  - `pub fn print_summary(&self)` :394
  - `pub fn success_rate(&self) -> f64` :412

### rules.rs (1289 rader)
- `pub enum Severity` :17
- `impl Severity` :23
  - `pub fn parse(s: &str) -> Self` :24
- `pub struct Rule` :35
- `fn default_severity() -> Severity` :49
- `pub struct RulesFile` :55
- `pub struct Finding` :61
- `pub struct CompiledRule` :75
- `pub fn load_rules_yaml(yaml: &str) -> Result<Vec<Rule>>` :84
- `pub fn rules_to_yaml(rules: &[Rule]) -> Result<String>` :90
- `pub fn project_rules_path() -> PathBuf` :98
- `pub fn load_project_rules() -> Result<Vec<Rule>>` :105
- `pub fn append_project_rule(rule: &Rule) -> Result<()>` :117
- `pub fn compile_rule(rule: &Rule) -> Result<CompiledRule>` :134
- `fn collect_placeholders(` :292
- `fn scaffold_pattern(language: &str, pattern: &str) -> Option<String>` :313
- `fn extract_pattern_roots(scaffolded: &TreeNode, first_replacement: &str) -> Vec<TreeNode>` :330
- `fn collect_subtree(node: &TreeNode, out: &mut Vec<TreeNode>)` :355
- `fn find_smallest_containing<'a>(node: &'a TreeNode, needle: &str) -> Option<&'a TreeNode>` :363
- `pub fn run_rule(rule: &CompiledRule, tree: &TreeNode, file: &str) -> Vec<Finding>` :377
- `pub fn check_code(code: &str, language: &str, rules: &[Rule]) -> (Vec<Finding>, usize)` :392
- `pub fn language_matches(rule_language: &str, file_language: &str) -> bool` :426
- `pub fn builtin_rules() -> Vec<Rule>` :440
- `pub fn check_code_with_builtin(code: &str, language: &str) -> (Vec<Finding>, usize, bool)` :451
- `pub fn fix_for_finding(finding: &Finding) -> Option<String>` :462
- `pub struct LintOptions<'a>` :471
- `impl Default for LintOptions<'_>` :491
  - `fn default() -> Self` :492
- `pub struct LintResult` :506
- `pub fn find_lintable_files(dir: &std::path::Path) -> Result<Vec<String>>` :524
- `pub fn expand_fix(fix: &str, captures: &HashMap<String, String>) -> Result<String>` :552
- `pub fn validate_fix(language: &str, fix: &str) -> Result<()>` :583
- `pub fn fix_batch(findings: &[Finding], rules: &[Rule]) -> Result<(crate::core::Batch, usize)>` :629
- `pub fn interpolate_message(msg: &str, captures: &HashMap<String, String>) -> String` :673
- `pub fn run_lint(paths: &[String], opts: &LintOptions) -> Result<LintResult>` :701
- `pub fn severity_rank(s: Severity) -> u8` :849
- `pub fn format_findings_for_prompt(findings: &[Finding]) -> String` :859
- `fn match_pattern_recursive(` :878
- `fn is_statement_wrapper(t: &str) -> bool` :922
- `fn match_node(` :929
- `fn merge_bindings(acc: &mut HashMap<String, String>, new: HashMap<String, String>) -> Option<()>` :980
- `fn normalize(mut s: String) -> String` :994
- `fn is_whitespace_node(node: &TreeNode) -> bool` :1013
- `fn is_punct_node(node: &TreeNode) -> bool` :1018

### scaffold.rs (274 rader)
- `struct ScaffoldNode` :4
- `pub struct ScaffoldEngine` :10
- `impl Default for ScaffoldEngine` :12
  - `fn default() -> Self` :13
- `impl ScaffoldEngine` :18
  - `pub fn new() -> Self` :19
  - `pub fn generate(&self, schema: &str) -> Result<String>` :23
  - `fn generate_rust(&self, nodes: &[ScaffoldNode], depth: usize) -> String` :37
  - `fn generate_python(&self, nodes: &[ScaffoldNode], depth: usize) -> String` :116
  - `fn parse_recursive(&self, structure: &str) -> Result<Vec<ScaffoldNode>>` :155

### secrets.rs (401 rader)
- `static SCANNER: LazyLock<Option<secrets_scanner::Scanner>> =` :12
- `pub struct DetectedSecret` :17
- `pub fn scan_for_secrets(source: &str) -> Vec<DetectedSecret>` :29
- `pub fn redact_secrets(source: &str) -> (String, usize)` :91
- `struct SecretPattern` :112
- `impl SecretPattern` :118
  - `fn get_regex(&self) -> Regex` :119
- `macro_rules! secret_pattern` :124
- `static SECRET_PATTERNS: &[SecretPattern] = &[` :134
- `fn is_likely_safe(matched: &str) -> bool` :245
- `fn redact_secret(secret: &str) -> String` :288

### state.rs (164 rader)
- `const STATE_FILE: &str = ".gnawtreewriter_state.json"` :15
- `pub struct ProjectState` :19
- `impl ProjectState` :28
  - `pub fn load(project_root: &Path) -> Self` :30
  - `pub fn save(&self, project_root: &Path) -> Result<()>` :43
  - `pub fn update(project_root: &Path) -> Result<Self>` :51
  - `pub fn git_head(&self) -> &str` :84
  - `pub fn file_hash(&self, path: &str) -> Option<&str>` :89
- `fn get_git_head(project_root: &Path) -> String` :95

### stats.rs (335 rader)
- `pub struct LanguageStats` :12
- `pub struct FileInfo` :22
- `pub struct ProjectStats` :31
- `pub struct CompressionEstimate` :42
- `pub struct ContextWindowInfo` :50
- `pub fn analyze_project(root: &Path) -> Result<ProjectStats>` :58

### tag_manager.rs (297 rader)
- `struct TagsFile` :12
- `struct FileTags` :17
- `pub struct TagManager` :26
- `impl TagManager` :31
  - `pub fn default_tags_path<P: AsRef<Path>>(project_root: P) -> PathBuf` :33
  - `pub fn load<P: AsRef<Path>>(project_root: P) -> Result<Self>` :39
  - `pub fn save(&self) -> Result<()>` :58
  - `pub fn add_tag(` :70
  - `pub fn rename_tag(` :96
  - `pub fn remove_tag(&mut self, file_path: &str, name: &str) -> Result<bool>` :140
  - `pub fn get_path(&self, file_path: &str, name: &str) -> Option<String>` :155
  - `pub fn list_tags(&self, file_path: &str) -> Vec<(String, String)>` :164
  - `pub fn tag_exists(&self, file_path: &str, name: &str) -> bool` :179
  - `pub fn tags_file_path(&self) -> &Path` :188
  - `pub fn all_tags(&self) -> HashMap<String, HashMap<String, String>>` :193

### token_count.rs (150 rader)
- `static BPE: LazyLock<tiktoken_rs::CoreBPE> =` :9
- `pub fn count_tokens(text: &str) -> usize` :16
- `pub fn estimate_code_tokens(text: &str) -> usize` :24
- `pub fn count_line_tokens(line: &str) -> usize` :29
- `pub struct TokenSummary` :35
- `pub struct FileTokenInfo` :42
- `impl TokenSummary` :48
  - `pub fn from_files(entries: &[(&str, &str)]) -> Self` :49

### transaction_log.rs (626 rader)
- `pub struct Transaction` :11
- `pub enum OperationType` :25
- `pub struct TransactionLog` :38
- `impl TransactionLog` :45
  - `pub fn get_current_session_id(&self) -> &str` :46
  - `pub fn new<P: AsRef<Path>>(project_root: P) -> Result<Self>` :51
  - `pub fn load<P: AsRef<Path>>(project_root: P) -> Result<Self>` :81
  - `fn ensure_session_exists(&mut self) -> Result<()>` :112
  - `pub fn log_transaction(` :143
  - `pub fn get_transaction(&self, transaction_id: &str) -> Result<Transaction>` :187
  - `pub fn get_session_history(&self) -> &[Transaction]` :193
  - `pub fn get_full_history(&self) -> Result<Vec<Transaction>>` :198
  - `fn load_full_history_from_file(log_file: &PathBuf) -> Result<Vec<Transaction>>` :203
  - `pub fn get_file_history<P: AsRef<Path>>(&self, file_path: P) -> Result<Vec<Transaction>>` :228
  - `pub fn get_history_since(&self, since: DateTime<Utc>) -> Result<Vec<Transaction>>` :239
  - `pub fn get_history_range(` :249
  - `pub fn get_affected_files_since(&self, since: DateTime<Utc>) -> Result<Vec<PathBuf>>` :263
  - `pub fn get_session_files(&self, session_id: &str) -> Result<Vec<PathBuf>>` :280
  - `pub fn get_last_transaction_before(` :299
  - `pub fn get_project_restoration_plan(` :319
  - `fn count_modifications_since(` :348
  - `fn count_transactions_since(&self, since: DateTime<Utc>) -> Result<usize>` :367
  - `pub fn find_transaction(&self, transaction_id: &str) -> Result<Option<Transaction>>` :381
  - `pub fn get_last_n_transactions(&self, n: usize) -> Result<Vec<Transaction>>` :401
  - `pub fn start_new_session(&mut self, name: Option<String>) -> Result<()>` :408
  - `pub fn export_history(&self, format: ExportFormat) -> Result<String>` :461
  - `fn append_to_log(&self, transaction: &Transaction) -> Result<()>` :473
- `pub enum ExportFormat` :493
- `pub struct ProjectRestorationPlan` :500
- `pub struct FileRestorationPlan` :508
- `impl ProjectRestorationPlan` :515
  - `pub fn has_changes(&self) -> bool` :517
  - `pub fn get_summary(&self) -> String` :522
  - `pub fn get_file_list(&self) -> Vec<&PathBuf>` :532
- `fn generate_session_id() -> String` :538
- `fn generate_transaction_id() -> String` :543
- `pub fn calculate_content_hash(content: &str) -> String` :548

### undo_redo.rs (1107 rader)
- `pub struct UndoRedoManager` :10
- `impl UndoRedoManager` :18
  - `pub fn new<P: AsRef<Path>>(project_root: P) -> Result<Self>` :20
  - `pub fn record_operation(&mut self, transaction_id: String)` :61
  - `pub fn peek_undo(&self, steps: usize) -> Vec<String>` :69
  - `pub fn undo(&mut self, steps: usize) -> Result<Vec<UndoRedoResult>>` :73
  - `pub fn redo(&mut self, steps: usize) -> Result<Vec<UndoRedoResult>>` :89
  - `pub fn get_state(&self) -> UndoRedoState` :105
  - `pub fn get_undo_history(&self, limit: Option<usize>) -> Result<Vec<Transaction>>` :115
  - `pub fn get_redo_history(&self, limit: Option<usize>) -> Result<Vec<Transaction>>` :129
  - `fn undo_single_transaction(&self, transaction_id: &str) -> Result<UndoRedoResult>` :143
  - `fn redo_single_transaction(&self, transaction_id: &str) -> Result<UndoRedoResult>` :168
  - `fn undo_edit(&self, transaction: &Transaction) -> Result<UndoRedoResult>` :193
  - `fn redo_edit(&self, transaction: &Transaction) -> Result<UndoRedoResult>` :219
  - `fn undo_insert(&self, transaction: &Transaction) -> Result<UndoRedoResult>` :247
  - `fn redo_insert(&self, transaction: &Transaction) -> Result<UndoRedoResult>` :292
  - `fn undo_delete(&self, transaction: &Transaction) -> Result<UndoRedoResult>` :336
  - `fn redo_delete(&self, transaction: &Transaction) -> Result<UndoRedoResult>` :378
  - `fn undo_add_property(&self, transaction: &Transaction) -> Result<UndoRedoResult>` :422
  - `fn redo_add_property(&self, transaction: &Transaction) -> Result<UndoRedoResult>` :467
  - `fn undo_add_component(&self, transaction: &Transaction) -> Result<UndoRedoResult>` :511
  - `fn redo_add_component(&self, transaction: &Transaction) -> Result<UndoRedoResult>` :556
  - `fn undo_move(&self, transaction: &Transaction) -> Result<UndoRedoResult>` :600
  - `fn redo_move(&self, transaction: &Transaction) -> Result<UndoRedoResult>` :642
  - `fn undo_restore(&self, transaction: &Transaction) -> Result<UndoRedoResult>` :683
  - `fn redo_restore(&self, transaction: &Transaction) -> Result<UndoRedoResult>` :728
  - `fn find_next_transaction_for_file(` :773
  - `fn find_backup_by_hash(` :794
  - `fn restore_from_backup(&self, target_path: &Path, backup_path: &Path) -> Result<()>` :830
- `pub struct UndoRedoResult` :854
- `pub struct UndoRedoState` :864

### visualizer.rs (200 rader)
- `pub struct TreeVisualizer` :5
- `impl TreeVisualizer` :10
  - `pub fn new(draw_distance: usize, use_colors: bool) -> Self` :11
  - `pub fn generate_sparkline(&self, root: &TreeNode) -> String` :18
  - `pub fn render_with_diff(` :45
  - `fn render_node_internal(` :66
  - `fn calculate_distance(&self, a: &str, b: &str) -> usize` :130

### visualizer_test.rs (82 rader)
- `pub struct TreeVisualizer` :5
- `impl TreeVisualizer` :10
  - `pub fn new(draw_distance: usize, use_colors: bool) -> Self` :11
  - `pub fn generate_sparkline(&self, root: &TreeNode) -> String` :15
  - `pub fn render_with_diff(&self, root: &TreeNode, focus_path: &str, old_node: Option<&TreeNode>) -> String` :30
  - `fn render_node_internal(&self, n: &TreeNode, d: usize, anc: &HashSet<String>, fp: &str, old: Option<&TreeNode>…` :44
  - `fn calculate_distance(&self, a: &str, b: &str) -> usize` :75

## src/llm/

### ai_manager.rs (1401 rader)
- `pub enum AiModel` :19
- `pub enum DeviceType` :33
- `impl From<&str> for DeviceType` :39
  - `fn from(s: &str) -> Self` :40
- `pub fn decide_index_device(` :66
- `fn vram_free_percent() -> Option<f32>` :94
- `pub fn indexing_device() -> DeviceType` :120
- `pub struct ModernBertModel` :154
- `pub enum EmbeddingBackbone` :163
- `pub struct Lfm25Model` :172
- `pub struct Generation` :182
- `pub enum Resolution` :193
- `impl Resolution` :205
  - `pub fn parse(s: &str) -> Self` :206
- `pub struct TimingProfile` :222
- `impl Default for TimingProfile` :235
  - `fn default() -> Self` :236
- `impl TimingProfile` :248
  - `pub fn estimate_seconds(&self, input_tokens: usize, output_tokens: usize) -> f64` :250
- `pub struct TokenBudget` :264
- `impl Default for TokenBudget` :284
  - `fn default() -> Self` :285
- `impl TokenBudget` :300
  - `pub fn record(&mut self, gen: &Generation)` :302
  - `pub fn estimate_seconds(&mut self, profile: &TimingProfile)` :311
- `impl ModernBertModel` :317
  - `pub fn get_embedding(&self, text: &str) -> Result<Tensor>` :318
  - `pub fn get_query_embedding(&self, text: &str) -> Result<Tensor>` :339
  - `fn bge_embed_batched(&self, model: &BertModel, texts: &[&str]) -> Result<Vec<Vec<f32>>>` :360
  - `fn bge_embed(&self, model: &BertModel, text: &str, is_query: bool) -> Result<Tensor>` :443
  - `pub fn get_embeddings(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>` :469
- `pub struct AiManager` :542
- `impl AiManager` :558
  - `pub fn new(project_root: &Path) -> Result<Self>` :559
  - `pub fn timing(&self) -> TimingProfile` :594
  - `pub fn calibrate_timing(&self) -> Result<TimingProfile>` :604
  - `pub fn load_model(` :632
  - `pub fn load_lfm25_model(&self) -> Result<&Lfm25Model>` :711
  - `pub fn estimate_tokens(&self, text: &str) -> usize` :761
  - `pub fn generate_lfm25(` :774
  - `pub async fn generate_semantic_report(&self, file_path: &str) -> Result<SemanticReport>` :848
  - `pub async fn setup(&self, _model: AiModel, _device: DeviceType, _force: bool) -> Result<()>` :911
  - `pub fn get_status(&self) -> Result<AiStatus>` :994
  - `fn get_model_path(&self, model: &AiModel) -> PathBuf` :1014
- `fn sample_token(` :1029
- `pub struct SemanticReport` :1080
- `pub struct QualityFinding` :1087
- `pub struct Source` :1098
- `pub fn sources_from_evidence(evidence: &[(String, String)]) -> Vec<Source>` :1107
- `impl SemanticReport` :1153
  - `pub fn sources(&self) -> Vec<Source>` :1156
- `pub struct AiStatus` :1175
- `fn download_file(url: &str, dest: &std::path::Path) -> Result<()>` :1186
- `fn timing_profile_path(cache_dir: &Path) -> PathBuf` :1199
- `fn load_timing_profile(cache_dir: &Path) -> TimingProfile` :1205
- `fn save_timing_profile(cache_dir: &Path, profile: &TimingProfile)` :1215
- `fn measure_profile(` :1228

### batch.rs (96 rader)
- `pub struct BatchOperation` :6
- `pub enum BatchEdit` :13
- `pub fn apply_batch(operation: BatchOperation) -> Result<BatchResult>` :29
- `pub struct BatchResult` :90

### gnaw_sense.rs (876 rader)
- `pub struct GnawSenseBroker` :12
- `pub(crate) fn truncate_preview(s: &str, max_chars: usize) -> String` :33
- `const MAX_EMBED_NODES: usize = 24` :47
- `const MAX_EMBED_CHARS: usize = 1200` :56
- `const MAX_JIT_CACHE_FILES: usize = 32` :62
- `struct CachedFileIndex` :66
- `pub const SEARCH_LOG_FILE: &str = ".gnawtreewriter_search_log.jsonl"` :74
- `fn normalize_search_query(q: &str) -> String` :76
- `pub fn search_log_prior_failures(project_root: &std::path::Path, query: &str) -> u64` :84
- `pub fn search_log_record_failure(` :107
- `pub struct SatelliteQuality` :146
- `pub struct ExpansionInfo` :157
- `pub enum SenseResponse` :164
- `pub struct FileMatch` :177
- `pub struct NodeMatch` :195
- `pub struct ImpactMatch` :202
- `pub struct EditProposal` :208
- `impl GnawSenseBroker` :216
  - `pub fn new(project_root: &Path) -> Result<Self>` :217
  - `pub fn get_manager(&self) -> &crate::llm::AiManager` :228
  - `pub async fn sense(&self, query: &str, file_context: Option<&str>) -> Result<SenseResponse>` :233
  - `pub async fn sense_with(` :243
  - `pub async fn sense_with(` :458
  - `pub async fn propose_edit(` :471
  - `fn get_parent_path(&self, path: &str) -> String` :534
  - `fn get_next_index(&self, path: &str) -> usize` :544
  - `fn get_current_index(&self, path: &str) -> usize` :562
  - `async fn index_file_cached(` :579
  - `async fn index_file_from_content(` :624
  - `async fn index_file(` :674
  - `fn extract_name_from_preview(&self, preview: &str) -> Option<String>` :725

### harmony.rs (364 rader)
- `pub struct Harmony` :21
- `pub struct HarmonyPillars` :37
- `pub enum HarmonyVerdict` :45
- `pub const DISSONANCE_MINOR: f32 = 0.25` :51
- `pub const DISSONANCE_HIGH: f32 = 0.50` :52
- `pub struct Pillars` :55
- `impl From<&Pillars> for HarmonyPillars` :61
  - `fn from(p: &Pillars) -> Self` :62
- `pub fn dissonance_of(scores: &[f32]) -> f32` :73
- `pub fn harmony(p: &Pillars) -> Harmony` :84
- `fn suggestion_for(v: HarmonyVerdict, p: &Pillars) -> Option<String>` :126

### impact_analyzer.rs (197 rader)
- `pub struct ImpactAnalyzer` :5
- `pub struct ImpactReport` :10
- `pub struct AffectedFile` :20
- `impl ImpactAnalyzer` :25
  - `pub fn new(indexer: RelationalIndexer) -> Self` :26
  - `pub fn new_with_root(project_root: &Path) -> Self` :32
  - `pub fn analyze_impact(&self, symbol_name: &str, defined_in: &str) -> Result<ImpactReport>` :40
  - `fn load_all_graphs(&self) -> Result<Vec<crate::llm::relational_index::FileGraph>>` :86

### llm_integration.rs (266 rader)
- `pub struct LLMEditRequest` :9
- `pub enum EditIntent` :17
- `pub struct LLMAnalysis` :47
- `pub struct NodeContext` :55
- `pub fn process_llm_request(request: LLMEditRequest) -> Result<LLMResponse>` :65
- `pub fn get_node_context(file_path: &str, node_path: &str) -> Result<NodeContext>` :163
- `pub fn suggest_edits(file_path: &str, analysis: LLMAnalysis) -> Result<Vec<LLMResponse>>` :184
- `pub struct LLMResponse` :200
- `impl LLMResponse` :206
  - `fn success(message: String) -> Self` :207
- `fn find_node<'a>(tree: &'a TreeNode, path: &str) -> Option<&'a TreeNode>` :216
- `fn find_parent_path(tree: &TreeNode, node_path: &str) -> Option<String>` :228
- `fn get_sibling_content(tree: &TreeNode, node_path: &str) -> Vec<String>` :240
- `pub fn create_node_map(tree: &TreeNode) -> HashMap<String, TreeNode>` :255
- `fn build_node_map(node: &TreeNode, map: &mut HashMap<String, TreeNode>)` :261

### mod.rs (28 rader)
_(inga toppnivå-items)_

### pipeline.rs (893 rader)
- `pub struct GenerateParams` :13
- `impl Default for GenerateParams` :18
  - `fn default() -> Self` :19
- `pub struct TaskParams` :31
- `impl TaskParams` :40
  - `pub fn from_resolution(r: crate::llm::Resolution) -> Self` :41
- `const EXTRACT_TEMP: f32 = 0.1` :64
- `const SYNTH_TEMP: f32 = 0.4` :66
- `pub fn expand_query_terms(mgr: &AiManager, question: &str) -> Result<Vec<String>>` :73
- `fn parse_json_array(s: &str) -> Option<Vec<String>>` :79
- `fn parse_json_indices(s: &str) -> Option<Vec<usize>>` :93
- `pub fn explain_node(` :101
- `pub fn summarize_dir(` :164
- `fn file_skeleton(path: &Path) -> String` :264
- `fn collect_skeleton(node: &crate::parser::TreeNode, out: &mut Vec<String>, depth: usize)` :275
- `fn summarize_file_chunked(` :311
- `fn summarize_text_chunked(` :348
- `pub fn investigate(` :412
- `fn search_index(` :512
- `fn clean_output(s: &str) -> String` :565
- `pub struct SummarizeResult` :575
- `pub struct InvestigateResult` :584
- `pub struct EditProposal` :595
- `pub fn propose_edit(` :609
- `pub fn propose_edit_all(` :736
- `fn parse_old_new_proposal(s: &str) -> Option<(String, String)>` :807
- `fn find_node_containing_line<'a>(` :836
- `fn parse_edit_proposal(s: &str) -> Option<(usize, String)>` :864

### project_indexer.rs (209 rader)
- `pub struct ProjectIndexer` :10
- `impl ProjectIndexer` :16
  - `pub fn new(project_root: &Path) -> Result<Self>` :17
  - `pub async fn index_all(&self, target_path: &Path) -> Result<usize>` :26
  - `fn extract_module_doc(content: &str) -> String` :114
  - `fn collect_pending(node: &TreeNode, acc: &mut Vec<(String, String, String)>)` :130
  - `fn chunk_text(text: &str, size: usize, overlap: usize) -> Vec<String>` :161

### prompts.rs (174 rader)
- `pub fn explain_prompt(file_path: &str, node_type: &str, node_content: &str) -> String` :7
- `pub fn summarize_file_prompt(file_path: &str, content: &str, max_words: usize) -> String` :17
- `pub fn summarize_dir_prompt(dir: &str, file_summaries: &[(String, String)]) -> String` :28
- `pub fn summarize_project_prompt(dir_summaries: &[(String, String)]) -> String` :42
- `pub fn expand_query_prompt(question: &str) -> String` :56
- `pub fn rank_candidates_prompt(question: &str, candidates: &[(String, String)]) -> String` :65
- `pub fn synthesize_answer_prompt(question: &str, evidence: &[(String, String)]) -> String` :80
- `pub fn edit_ask_prompt(file_path: &str, request: &str, file_preview: &str, issues: &str) -> String` :98
- `pub fn edit_ask_all_prompt(` :123
- `pub fn discover_rules_prompt(code_sample: &str) -> String` :151

### recall_eval.rs (240 rader)
- `pub struct RecallCase` :21
- `pub struct CaseResult` :30
- `pub struct RecallReport` :53
- `pub fn run_recall_eval(eval_path: &Path, k: usize, json_out: bool) -> Result<()>` :71

### relational_index.rs (684 rader)
- `pub enum RelationType` :10
- `pub struct Relation` :17
- `pub struct FileGraph` :26
- `pub struct IndexStats` :41
- `pub struct AmbiguousName` :58
- `fn collect_imports(node: &TreeNode, acc: &mut HashSet<String>)` :70
- `pub struct RelationalIndexer` :93
- `impl RelationalIndexer` :100
  - `pub fn new(project_root: &Path) -> Self` :101
  - `pub fn last_index_stats(&self) -> Option<&IndexStats>` :116
  - `pub fn resolution_stats(&self) -> Result<IndexStats>` :123
  - `pub fn index_directory(&mut self, dir_path: &Path) -> Result<Vec<FileGraph>>` :171
  - `fn collect_definitions(&self, node: &TreeNode, acc: &mut HashMap<String, String>)` :256
  - `fn extract_relations(` :267
  - `fn load_graphs_from_db(&self) -> Result<Vec<FileGraph>>` :344
  - `pub fn save_graph(&self, graph: &FileGraph) -> Result<()>` :473
  - `pub fn load_all_graphs(&self) -> Result<Vec<FileGraph>>` :481

### semantic_index.rs (1247 rader)
- `fn vector_to_blob(vector: &[f32]) -> Vec<u8>` :13
- `fn blob_to_vector(blob: &[u8]) -> Vec<f32>` :17
- `pub struct NodeEmbedding` :26
- `pub struct SemanticIndex` :34
- `pub struct ModelInfo` :39
- `pub struct SemanticIndexManager` :45
- `impl SemanticIndexManager` :49
  - `pub fn new(project_root: &Path) -> Self` :50
  - `pub fn get_storage_dir(&self) -> &Path` :58
  - `fn open_db(&self) -> Result<Connection>` :64
  - `pub fn save_model_info(&self, model_name: &str, dimension: usize) -> Result<()>` :87
  - `pub fn get_model_info(&self) -> Result<Option<ModelInfo>>` :98
  - `pub fn save_index(&self, file_path: &str, entries: Vec<NodeEmbedding>) -> Result<()>` :120
  - `pub fn load_project_index(&self) -> Result<SemanticIndex>` :148
  - `fn migrate_legacy_json(conn: &mut Connection, storage_dir: &Path) -> Result<()>` :174
- `impl SemanticIndex` :242
  - `pub fn search(&self, query_vector: &[f32], limit: usize) -> Vec<(&NodeEmbedding, f32)>` :245
  - `pub fn search_with_threshold(` :250
- `pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32` :273
- `const DECL_PENALTY: f32 = 0.15` :298
- `const IMPL_BONUS: f32 = 0.03` :299
- `const RRF_K: f32 = 60.0` :302
- `const LEX_CHANNEL_WEIGHT: f32 = 0.5` :303
- `const GRAPH_CHANNEL_WEIGHT: f32 = 0.2` :309
- `const GRAPH_SEED_K: usize = 8` :311
- `const GRAPH_HUB_DEGREE: usize = 100` :315
- `enum PreviewKind` :318
- `fn classify_preview(preview: &str) -> PreviewKind` :324
- `pub struct RerankedHit<'a>` :391
- `pub fn parent_node_path(path: &str) -> &str` :412
- `fn lex_norm(word: &str) -> &str` :422
- `const MAX_PER_FILE: usize = 4` :433
- `pub fn fuse_by_max<'a>(` :440
- `pub fn rerank_satellite<'a>(` :476
- `fn graph_path_key(p: &str) -> &str` :487
- `pub fn rerank_satellite_with_graphs<'a>(` :500

## src/mcp/

### mod.rs (4259 rader)
_(inga toppnivå-items)_

## src/parser/

### bash.rs (84 rader)
- `pub struct BashParser` :9
- `impl Default for BashParser` :11
  - `fn default() -> Self` :12
- `impl BashParser` :17
  - `pub fn new() -> Self` :18
  - `fn build_tree(node: &tree_sitter::Node, source: &str, path: String) -> Result<TreeNode>` :22
- `impl ParserEngineLegacy for BashParser` :64
  - `fn parse_legacy(&self, source_code: &str) -> anyhow::Result<TreeNode>` :65
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :81

### c.rs (84 rader)
- `pub struct CParser` :9
- `impl Default for CParser` :11
  - `fn default() -> Self` :12
- `impl CParser` :17
  - `pub fn new() -> Self` :18
  - `fn build_tree(node: &tree_sitter::Node, source: &str, path: String) -> Result<TreeNode>` :22
- `impl ParserEngineLegacy for CParser` :64
  - `fn parse_legacy(&self, source_code: &str) -> anyhow::Result<TreeNode>` :65
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :81

### cmake.rs (33 rader)
- `pub struct CMakeParser` :11
- `impl Default for CMakeParser` :13
  - `fn default() -> Self` :14
- `impl CMakeParser` :19
  - `pub fn new() -> Self` :20
- `impl ParserEngineLegacy for CMakeParser` :25
  - `fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>` :26
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :30

### cpp.rs (84 rader)
- `pub struct CppParser` :9
- `impl Default for CppParser` :11
  - `fn default() -> Self` :12
- `impl CppParser` :17
  - `pub fn new() -> Self` :18
  - `fn build_tree(node: &tree_sitter::Node, source: &str, path: String) -> Result<TreeNode>` :22
- `impl ParserEngineLegacy for CppParser` :64
  - `fn parse_legacy(&self, source_code: &str) -> anyhow::Result<TreeNode>` :65
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :81

### csharp.rs (82 rader)
- `pub struct CSharpParser` :5
- `impl Default for CSharpParser` :7
  - `fn default() -> Self` :8
- `impl CSharpParser` :13
  - `pub fn new() -> Self` :14
- `impl ParserEngineLegacy for CSharpParser` :19
  - `fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>` :20
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :36
- `impl CSharpParser` :41
  - `fn build_tree(node: &tree_sitter::Node, source: &str, path: String) -> Result<TreeNode>` :42

### css.rs (33 rader)
- `pub struct CssParser` :11
- `impl Default for CssParser` :13
  - `fn default() -> Self` :14
- `impl CssParser` :19
  - `pub fn new() -> Self` :20
- `impl ParserEngineLegacy for CssParser` :25
  - `fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>` :26
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :30

### dart.rs (82 rader)
- `pub struct DartParser` :5
- `impl Default for DartParser` :7
  - `fn default() -> Self` :8
- `impl DartParser` :13
  - `pub fn new() -> Self` :14
- `impl ParserEngineLegacy for DartParser` :19
  - `fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>` :20
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :36
- `impl DartParser` :41
  - `fn build_tree(node: &tree_sitter::Node, source: &str, path: String) -> Result<TreeNode>` :42

### error.rs (35 rader)
- `pub struct SyntaxError` :6
- `pub type ParseResult<T> = std::result::Result<T, SyntaxError>` :13
- `impl From<anyhow::Error> for SyntaxError` :15
  - `fn from(err: anyhow::Error) -> Self` :16
- `impl From<tree_sitter::LanguageError> for SyntaxError` :26
  - `fn from(err: tree_sitter::LanguageError) -> Self` :27

### generic.rs (56 rader)
- `pub const GENERIC_NODE_TYPE: &str = "generic"` :4
- `pub struct GenericParser` :6
- `impl Default for GenericParser` :8
  - `fn default() -> Self` :9
- `impl GenericParser` :14
  - `pub fn new() -> Self` :15
- `impl ParserEngine for GenericParser` :20
  - `fn parse(&self, code: &str) -> ParseResult<TreeNode>` :22
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :40

### go.rs (80 rader)
- `pub struct GoParser` :5
- `impl Default for GoParser` :7
  - `fn default() -> Self` :8
- `impl GoParser` :13
  - `pub fn new() -> Self` :14
  - `fn build_tree(node: &tree_sitter::Node, source: &str, path: String) -> Result<TreeNode>` :18
- `impl ParserEngineLegacy for GoParser` :60
  - `fn parse_legacy(&self, source_code: &str) -> anyhow::Result<TreeNode>` :61
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :77

### html.rs (82 rader)
- `pub struct HtmlParser` :5
- `impl Default for HtmlParser` :7
  - `fn default() -> Self` :8
- `impl HtmlParser` :13
  - `pub fn new() -> Self` :14
- `impl ParserEngineLegacy for HtmlParser` :19
  - `fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>` :20
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :36
- `impl HtmlParser` :41
  - `fn build_tree(node: &tree_sitter::Node, source: &str, path: String) -> Result<TreeNode>` :42

### java.rs (84 rader)
- `pub struct JavaParser` :9
- `impl Default for JavaParser` :11
  - `fn default() -> Self` :12
- `impl JavaParser` :17
  - `pub fn new() -> Self` :18
  - `fn build_tree(node: &tree_sitter::Node, source: &str, path: String) -> Result<TreeNode>` :22
- `impl ParserEngineLegacy for JavaParser` :64
  - `fn parse_legacy(&self, source_code: &str) -> anyhow::Result<TreeNode>` :65
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :81

### javascript.rs (82 rader)
- `pub struct JavaScriptParser` :5
- `impl Default for JavaScriptParser` :7
  - `fn default() -> Self` :8
- `impl JavaScriptParser` :13
  - `pub fn new() -> Self` :14
- `impl ParserEngineLegacy for JavaScriptParser` :19
  - `fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>` :20
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :36
- `impl JavaScriptParser` :41
  - `fn build_tree(node: &tree_sitter::Node, source: &str, path: String) -> Result<TreeNode>` :42

### json.rs (98 rader)
- `pub struct JsonParser` :5
- `impl Default for JsonParser` :7
  - `fn default() -> Self` :8
- `impl JsonParser` :13
  - `pub fn new() -> Self` :14
- `impl ParserEngineLegacy for JsonParser` :19
  - `fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>` :20
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :29
- `impl JsonParser` :34
  - `fn build_value_node(` :36

### kotlin.rs (80 rader)
- `pub struct KotlinParser` :5
- `impl Default for KotlinParser` :7
  - `fn default() -> Self` :8
- `impl KotlinParser` :13
  - `pub fn new() -> Self` :14
  - `fn build_tree(node: &tree_sitter::Node, source: &str, path: String) -> Result<TreeNode>` :18
- `impl ParserEngineLegacy for KotlinParser` :60
  - `fn parse_legacy(&self, source_code: &str) -> Result<TreeNode>` :61
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :77

### make.rs (33 rader)
- `pub struct MakeParser` :11
- `impl Default for MakeParser` :13
  - `fn default() -> Self` :14
- `impl MakeParser` :19
  - `pub fn new() -> Self` :20
- `impl ParserEngineLegacy for MakeParser` :25
  - `fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>` :26
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :30

### markdown.rs (33 rader)
- `pub struct MarkdownParser` :11
- `impl Default for MarkdownParser` :13
  - `fn default() -> Self` :14
- `impl MarkdownParser` :19
  - `pub fn new() -> Self` :20
- `impl ParserEngineLegacy for MarkdownParser` :25
  - `fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>` :26
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :30

### mod.rs (438 rader)
- `pub struct TreeNode` :41
- `pub fn char_col_pair(source: &str, start_byte: usize, end_byte: usize) -> (usize, usize)` :59
- `pub fn tree_from_ts_node(node: &tree_sitter::Node, source: &str, path: String) -> TreeNode` :75
- `pub fn parse_with_tree_sitter(` :111
- `impl TreeNode` :124
  - `pub fn find_path(&self, target_path: &str) -> Option<&TreeNode>` :126
  - `pub fn get_name(&self) -> Option<String>` :140
  - `pub fn find_node_at_line(&self, line: usize) -> Option<&TreeNode>` :178
  - `pub fn find_parent_at_line(&self, line: usize) -> &TreeNode` :198
- `pub trait ParserEngine` :249
- `pub fn to_parse_result<T>(res: anyhow::Result<T>) -> ParseResult<T>` :262
- `pub fn validate_file(path: &std::path::Path) -> Result<usize, SyntaxError>` :269
- `pub struct LegacyParserWrapper<P: ParserEngineLegacy>` :281
- `impl<P: ParserEngineLegacy> LegacyParserWrapper<P>` :285
  - `pub fn new(inner: P) -> Self` :286
- `impl<P: ParserEngineLegacy> ParserEngine for LegacyParserWrapper<P>` :291
  - `fn parse(&self, code: &str) -> ParseResult<TreeNode>` :292
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :295
- `pub trait ParserEngineLegacy` :300
- `pub fn get_parser(file_path: &Path) -> anyhow::Result<Box<dyn ParserEngine>>` :304
- `fn parser_for_special_filename(file_path: &Path) -> Option<Box<dyn ParserEngine>>` :387
- `pub fn get_parser_for_language(language: &str) -> anyhow::Result<Box<dyn ParserEngine>>` :406

### php.rs (82 rader)
- `pub struct PhpParser` :5
- `impl Default for PhpParser` :7
  - `fn default() -> Self` :8
- `impl PhpParser` :13
  - `pub fn new() -> Self` :14
- `impl ParserEngineLegacy for PhpParser` :19
  - `fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>` :20
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :36
- `impl PhpParser` :41
  - `fn build_tree(node: &tree_sitter::Node, source: &str, path: String) -> Result<TreeNode>` :42

### powershell.rs (33 rader)
- `pub struct PowerShellParser` :11
- `impl Default for PowerShellParser` :13
  - `fn default() -> Self` :14
- `impl PowerShellParser` :19
  - `pub fn new() -> Self` :20
- `impl ParserEngineLegacy for PowerShellParser` :25
  - `fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>` :26
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :30

### python.rs (117 rader)
- `pub struct PythonParser` :3
- `impl Default for PythonParser` :5
  - `fn default() -> Self` :6
- `impl PythonParser` :11
  - `pub fn new() -> Self` :12
  - `fn find_error<'a>(` :16
  - `fn build_tree(` :34
- `impl ParserEngine for PythonParser` :80
  - `fn parse(&self, code: &str) -> ParseResult<TreeNode>` :81
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :114

### qml.rs (104 rader)
- `pub struct QmlParser` :3
- `impl Default for QmlParser` :5
  - `fn default() -> Self` :6
- `impl QmlParser` :11
  - `pub fn new() -> Self` :12
- `impl ParserEngineLegacy for QmlParser` :17
  - `fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>` :18
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :88
- `impl QmlParser` :93
  - `fn parse_property(&self, line: &str) -> Option<(String, String)>` :94

### qml_tree_sitter.rs (96 rader)
- `pub struct QmlTreeSitterParser` :4
- `impl Default for QmlTreeSitterParser` :6
  - `fn default() -> Self` :7
- `impl QmlTreeSitterParser` :12
  - `pub fn new() -> Self` :13
- `impl ParserEngineLegacy for QmlTreeSitterParser` :18
  - `fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>` :19
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :40
- `impl QmlTreeSitterParser` :45
  - `fn build_tree(node: &tree_sitter::Node, source: &str, path: String) -> Result<TreeNode>` :46

### ruby.rs (33 rader)
- `pub struct RubyParser` :11
- `impl Default for RubyParser` :13
  - `fn default() -> Self` :14
- `impl RubyParser` :19
  - `pub fn new() -> Self` :20
- `impl ParserEngineLegacy for RubyParser` :25
  - `fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>` :26
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :30

### rust.rs (267 rader)
- `pub struct RustParser` :3
- `impl Default for RustParser` :5
  - `fn default() -> Self` :6
- `impl RustParser` :11
  - `pub fn new() -> Self` :12
  - `fn find_error<'a>(` :16
  - `fn build_tree(` :34
- `impl ParserEngine for RustParser` :116
  - `fn parse(&self, code: &str) -> ParseResult<TreeNode>` :117
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :150
  - `fn parse_lenient(&self, code: &str) -> (ParseResult<TreeNode>, Option<SyntaxError>)` :158

### slint.rs (117 rader)
- `pub struct SlintParser` :3
- `impl Default for SlintParser` :5
  - `fn default() -> Self` :6
- `impl SlintParser` :11
  - `pub fn new() -> Self` :12
  - `fn find_error<'a>(` :16
  - `fn build_tree(` :34
- `impl ParserEngine for SlintParser` :80
  - `fn parse(&self, code: &str) -> ParseResult<TreeNode>` :81
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :114

### sql.rs (82 rader)
- `pub struct SqlParser` :5
- `impl Default for SqlParser` :7
  - `fn default() -> Self` :8
- `impl SqlParser` :13
  - `pub fn new() -> Self` :14
- `impl ParserEngineLegacy for SqlParser` :19
  - `fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>` :20
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :36
- `impl SqlParser` :41
  - `fn build_tree(node: &tree_sitter::Node, source: &str, path: String) -> Result<TreeNode>` :42

### svelte.rs (82 rader)
- `pub struct SvelteParser` :5
- `impl Default for SvelteParser` :7
  - `fn default() -> Self` :8
- `impl SvelteParser` :13
  - `pub fn new() -> Self` :14
- `impl ParserEngineLegacy for SvelteParser` :19
  - `fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>` :20
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :36
- `impl SvelteParser` :41
  - `fn build_tree(node: &tree_sitter::Node, source: &str, path: String) -> Result<TreeNode>` :42

### swift.rs (80 rader)
- `pub struct SwiftParser` :5
- `impl Default for SwiftParser` :7
  - `fn default() -> Self` :8
- `impl SwiftParser` :13
  - `pub fn new() -> Self` :14
  - `fn build_tree(node: &tree_sitter::Node, source: &str, path: String) -> Result<TreeNode>` :18
- `impl ParserEngineLegacy for SwiftParser` :60
  - `fn parse_legacy(&self, source_code: &str) -> Result<TreeNode>` :61
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :77

### text.rs (52 rader)
- `pub struct TextParser` :3
- `impl Default for TextParser` :5
  - `fn default() -> Self` :6
- `impl TextParser` :11
  - `pub fn new() -> Self` :12
- `impl ParserEngineLegacy for TextParser` :17
  - `fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>` :18
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :49

### toml.rs (33 rader)
- `pub struct TomlParser` :11
- `impl Default for TomlParser` :13
  - `fn default() -> Self` :14
- `impl TomlParser` :19
  - `pub fn new() -> Self` :20
- `impl ParserEngineLegacy for TomlParser` :25
  - `fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>` :26
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :30

### typescript.rs (82 rader)
- `pub struct TypeScriptParser` :5
- `impl Default for TypeScriptParser` :7
  - `fn default() -> Self` :8
- `impl TypeScriptParser` :13
  - `pub fn new() -> Self` :14
- `impl ParserEngineLegacy for TypeScriptParser` :19
  - `fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>` :20
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :36
- `impl TypeScriptParser` :41
  - `fn build_tree(node: &tree_sitter::Node, source: &str, path: String) -> Result<TreeNode>` :42

### xml.rs (609 rader)
- `pub struct XmlParser` :4
- `impl Default for XmlParser` :6
  - `fn default() -> Self` :7
- `impl XmlParser` :12
  - `pub fn new() -> Self` :13
- `impl ParserEngine for XmlParser` :18
  - `fn parse(&self, code: &str) -> ParseResult<TreeNode>` :19
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :146
- `impl XmlParser` :151
  - `fn element_to_treenode_with_span(` :153
  - `fn find_matching_close_in_slice(slice: &str, rel_open: usize, tag: &str) -> Option<usize>` :464

### yaml.rs (33 rader)
- `pub struct YamlParser` :11
- `impl Default for YamlParser` :13
  - `fn default() -> Self` :14
- `impl YamlParser` :19
  - `pub fn new() -> Self` :20
- `impl ParserEngineLegacy for YamlParser` :25
  - `fn parse_legacy(&self, code: &str) -> anyhow::Result<TreeNode>` :26
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :30

### zig.rs (84 rader)
- `pub struct ZigParser` :9
- `impl Default for ZigParser` :11
  - `fn default() -> Self` :12
- `impl ZigParser` :17
  - `pub fn new() -> Self` :18
  - `fn build_tree(node: &tree_sitter::Node, source: &str, path: String) -> Result<TreeNode>` :22
- `impl ParserEngineLegacy for ZigParser` :64
  - `fn parse_legacy(&self, source_code: &str) -> anyhow::Result<TreeNode>` :65
  - `fn get_supported_extensions(&self) -> Vec<&'static str>` :81

## tag_test/

### test.rs (1 rader)
- `fn main() {}` :1

## test_dir/

### file2.css (2 rader)
_(inga toppnivå-items)_

### fix_list_nodes.rs (16 rader)
- `fn handle_list_nodes(state: Arc<AppState>, file_path: &str, _filter: Option<&str>, _max_depth: Option<usize>, …` :1

### fix_search.rs (16 rader)
- `fn handle_search_nodes(file_path: &str, pattern: &str) -> Value` :1

### fix_skeleton.rs (16 rader)
- `fn handle_get_skeleton(file_path: &str, max_depth: usize) -> Value` :1

### fix_skeleton_final.rs (16 rader)
- `fn handle_get_skeleton(file_path: &str, max_depth: usize) -> Value` :1

### test1.py (1 rader)
- `def test(): pass` :1

### test2.rs (1 rader)
- `fn test() {}` :1

## tests/

### ai_modernbert_tests.rs (184 rader)
- `fn is_modernbert_installed(project_root: &Path) -> bool` :8
- `fn test_ai_status_detection() -> Result<()>` :19
- `fn batch_embeddings_match_singles() -> Result<()>` :55
- `fn bge_batch_embeddings_match_singles() -> Result<()>` :125

### b1_path_validation.rs (106 rader)
- `const SOURCE: &str =` :8
- `fn setup(dir: &tempfile::TempDir) -> (std::path::PathBuf, GnawTreeWriter)` :11
- `fn receipt_carries_new_path_and_content_hash()` :19
- `fn expect_mismatch_rejects_before_writing()` :43
- `fn expect_match_passes_and_edit_lands()` :62
- `fn the_2026_11_03_incident_is_caught_by_expect()` :79
- `fn missing_path_is_a_clear_stale_path_error()` :99

### cli_behaviour.rs (194 rader)
- `const BIN: &str = env!("CARGO_BIN_EXE_gnawtreewriter")` :16
- `fn tmp(name: &str, content: &str) -> std::path::PathBuf` :18
- `fn version_exits_zero_and_writes_stdout()` :29
- `fn a_usage_error_is_exit_two_not_one()` :48
- `fn analyze_of_an_unparseable_file_fails_loudly()` :61
- `fn analyze_of_a_good_file_still_succeeds()` :76
- `fn db_refresh_writes_the_artifact_by_default()` :92
- `fn db_refresh_with_stdout_prints_and_writes_nothing()` :119
- `fn plain_code_index_still_prints_to_stdout()` :141
- `fn expect_name_rejects_a_stale_path_instead_of_editing_the_wrong_node()` :162

### cli_contract.rs (20 rader)
- `fn cli_command_table_is_well_formed()` :18

### code_index_check.rs (117 rader)
- `fn opts() -> CodeIndexOptions` :9
- `fn make_repo(dir: &Path) -> std::path::PathBuf` :16
- `fn fresh_artifact_passes()` :29
- `fn hand_edit_fails_fast_tier()` :38
- `fn stale_artifact_fails_full_tier()` :55
- `fn fix_rewrites_and_then_passes()` :74
- `fn legacy_artifact_without_hash_header_is_flagged()` :99

### code_index_db.rs (189 rader)
- `fn opts() -> CodeIndexOptions` :13
- `fn make_repo(dir: &Path)` :20
- `fn normalize(doc: &str) -> String` :46
- `fn db_render_matches_direct_markdown()` :55
- `fn db_render_matches_direct_json()` :73
- `fn incremental_refresh_reuses_and_reparses()` :88
- `fn generation_key_covers_population_not_presentation()` :129
- `fn removed_files_are_pruned()` :157
- `fn corrupt_db_is_rebuilt_not_fatal()` :169

### code_index_fixtures.rs (231 rader)
- `const LIB_RS: &str = r#"// fixture mirror: F1-F7 markers, see FACTS.md` :14
- `const CALLER_RS: &str = r#"use crate::module_a::run` :52
- `const METHODS_RS: &str = r#"// [F8] impl + method entity / [F9] method_call — H1 flipped: extracted` :61
- `fn build_fixture() -> (tempfile::TempDir, std::path::PathBuf)` :95
- `fn fixture_baseline_post_c()` :114

### doc_comment_handling.rs (123 rader)
- `fn make_file(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf` :14
- `fn temp_dir() -> tempfile::TempDir` :20
- `const SRC: &str = "/// Konverterar ett heltal.\n///\n/// MEDVETEN BUGG: bara repetition.\npub fn till(n: u32) …` :24
- `fn edit_with_own_doc_swallows_the_old_doc_block()` :27
- `fn edit_without_own_doc_keeps_the_old_doc_block()` :56
- `fn delete_leaves_doc_to_the_caller_layer()` :77
- `fn describe_node_reveals_identity_of_the_resolved_target()` :102

### edit_multibyte_regression.rs (113 rader)
- `const JS_FILE: &str = r#"// Tool-guard — mekaniskt stopp för §35-filredigeringsbrott i bash.` :19
- `fn find_export_block_path(w: &gnawtreewriter::GnawTreeWriter) -> Option<String>` :68
- `fn edit_node_on_multibyte_js_file_does_not_panic()` :81

### edit_node_insert_regression.rs (99 rader)
- `const SOURCE: &str = "/// Doc for alpha.\n#[derive(Clone)]\nfn alpha() -> u8 { 1 }\n\n/// Doc for beta.\nfn be…` :21
- `fn edit(path: &str, content: &str, dir: &tempfile::TempDir) -> String` :23
- `fn attribute_path_replaces_attribute_only_and_f2_swallows_doc()` :39
- `fn function_path_replaces_function_only_and_keeps_doc_and_attribute()` :63
- `fn struct_style_multi_item_content_replaces_exactly_the_addressed_node()` :78

### edit_node_midline_block_regression.rs (152 rader)
- `const JS_FILE: &str = r#"// Åäö + emoji i prefixet — kolumnräkningen ska vara tecken-baserad 🌲` :20
- `fn find_path_containing(node: &TreeNode, needle: &str, starts_with: &str) -> Option<String>` :41
- `fn edit_node_on_midline_js_block_preserves_promise_chain()` :61
- `fn edit_node_on_rust_line_with_multibyte_prefix_is_exact()` :117

### gnaw_sense_integration.rs (72 rader)
_(inga toppnivå-items)_

### insert_position.rs (555 rader)
- `fn make_file(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf` :10
- `fn read_file(path: &std::path::Path) -> String` :18
- `fn temp_dir() -> tempfile::TempDir` :24
- `fn rust_file_with_newline() -> (tempfile::TempDir, std::path::PathBuf)` :29
- `fn rust_file_without_newline() -> (tempfile::TempDir, std::path::PathBuf)` :37
- `fn insert_source_file_pos1_with_trailing_newline()` :47
- `fn insert_source_file_pos1_without_trailing_newline()` :80
- `fn insert_source_file_pos1_actually_writes_correctly()` :117
- `fn insert_block_pos0_adds_after_opening_brace()` :150
- `fn insert_block_pos1_adds_before_closing_brace()` :194
- `fn insert_block_pos1_without_trailing_newline()` :243
- `fn insert_python_source_file_pos1()` :289
- `fn insert_python_source_file_pos1_no_trailing_newline()` :317
- `fn insert_typescript_source_file_pos1_no_trailing_newline()` :347
- `fn insert_source_file_pos1_empty_file()` :383
- `fn insert_source_file_pos1_single_function_no_newline()` :405
- `fn insert_out_of_range_position_appends_at_top_level()` :442
- `fn insert_pos3_does_not_nest_inside_first_root_fn()` :485
- `fn insert_in_range_slot_lands_between_siblings()` :518

### legacy_parser_surgical_edit.rs (194 rader)
- `fn deepest_with_content<'a>(node: &'a TreeNode, needle: &str) -> Option<&'a TreeNode>` :24
- `fn edit_file(name: &str, source: &str, needle: &str, replacement: &str) -> String` :38
- `fn markdown_edit_keeps_heading_marker_and_blank_lines()` :62
- `fn markdown_edit_of_heading_text_preserves_hash()` :70
- `fn css_edit_of_a_value_touches_only_that_value()` :78
- `fn yaml_edit_of_a_value_touches_only_that_value()` :86
- `fn toml_edit_of_a_value_touches_only_that_value()` :94
- `fn json_edit_stays_valid_and_goes_through_the_semantic_path()` :102
- `fn added_grammars_parse_through_the_registry()` :132
- `fn collect_kinds(node: &TreeNode) -> Vec<String>` :170
- `fn dotfile_resolves_to_the_bash_parser()` :180

### mcp_integration.rs (2508 rader)
- `async fn integration_mcp_auth() -> Result<(), Box<dyn std::error::Error>>` :16
- `async fn integration_mcp_tools_list() -> Result<(), Box<dyn std::error::Error>>` :85
- `async fn integration_mcp_tools_call_missing_args() -> Result<(), Box<dyn std::error::Error>>` :164
- `async fn integration_mcp_tools_call_batch() -> Result<(), Box<dyn std::error::Error>>` :244
- `async fn integration_mcp_tools_call_unknown_tool() -> Result<(), Box<dyn std::error::Error>>` :384
- `async fn integration_mcp_tools_call_file_not_found() -> Result<(), Box<dyn std::error::Error>>` :453
- `async fn integration_mcp_tools_call_undo() -> Result<(), Box<dyn std::error::Error>>` :522
- `async fn integration_mcp_concurrent_requests() -> Result<(), Box<dyn std::error::Error>>` :610
- `async fn integration_mcp_tools_call_analyze() -> Result<(), Box<dyn std::error::Error>>` :683
- `async fn integration_mcp_tools_adoption_contract() -> Result<(), Box<dyn std::error::Error>>` :771
- `fn integration_sources_semantic_report_payload()` :949
- `fn integration_sources_investigate_payload()` :983
- `async fn integration_mcp_get_skeleton_never_bare_header() -> Result<(), Box<dyn std::error::Error>>` :1005
- `async fn integration_mcp_doctor_reports_health() -> Result<(), Box<dyn std::error::Error>>` :1103
- `fn integration_error_strings_carry_guidance()` :1193
- `fn integration_error_codes_documented()` :1245
- `async fn integration_mcp_index_project_status() -> Result<(), Box<dyn std::error::Error>>` :1301
- `fn integration_mcp_instructions_listed()` :1404
- `fn integration_duplex_metrics_persist()` :1441
- `async fn integration_mcp_partial_parse_read_paths() -> Result<(), Box<dyn std::error::Error>>` :1484
- `async fn integration_mcp_phase10_surfaces() -> Result<(), Box<dyn std::error::Error>>` :1629
- `async fn integration_mcp_write_receipts_idempotent() -> Result<(), Box<dyn std::error::Error>>` :1880
- `async fn integration_mcp_edit_rejection_carries_edit_verdict(` :1986
- `async fn integration_mcp_edit_reports_impact_on_signature_change(` :2090
- `async fn integration_mcp_batch_rejection_carries_edit_verdict(` :2201
- `async fn integration_mcp_search_nodes_alternation() -> Result<(), Box<dyn std::error::Error>>` :2297
- `async fn integration_mcp_edit_node_expect_name_guard() -> Result<(), Box<dyn std::error::Error>>` :2409

### phase3_graph.rs (106 rader)
- `fn opts() -> CodeIndexOptions` :8
- `fn make_store(dir: &std::path::Path)` :15
- `fn graphs_load_from_db_store()` :23
- `fn resolution_stats_work_over_db_graphs()` :53
- `fn impact_analysis_reads_db_graphs()` :67
- `fn index_directory_uses_store_when_present()` :84
- `fn legacy_json_pipeline_still_works_without_db()` :97

### phase4_runners.rs (161 rader)
- `fn opts() -> CodeIndexOptions` :10
- `fn mark_stale_upserts_one_row_per_file_and_refresh_drains()` :18
- `fn mark_stale_never_creates_the_store()` :37
- `fn real_edit_lands_in_the_queue()` :47
- `fn hook_install_is_idempotent_and_uninstall_restores()` :70
- `fn hook_install_preserves_existing_user_hook()` :121

### relational_integration.rs (55 rader)
_(inga toppnivå-items)_

### t6_canonical_ids.rs (206 rader)
- `fn parse(code: &str) -> gnawtreewriter::parser::TreeNode` :16
- `fn impl_methods_are_qualified_in_ids()` :22
- `fn disambiguation_is_deterministic_by_line_order()` :56
- `fn entity_paths_are_canonical()` :97
- `fn relations_carry_honest_targets()` :108
- `fn db_stores_graph_tables_and_resolves_cross_file()` :159

> ⚠ 3 fil(er) med syntaxvarning (partiell AST, items kan saknas):
>   - color_test.rs (Partial parse: syntax error found — results may be incomplete (editors still refuse this file strictly) @ line 1)
>   - src/auth_demo.rs (Partial parse: syntax error found — results may be incomplete (editors still refuse this file strictly) @ line 3)
>   - src/core/visualizer_test.rs (Partial parse: syntax error found — results may be incomplete (editors still refuse this file strictly) @ line 2)
