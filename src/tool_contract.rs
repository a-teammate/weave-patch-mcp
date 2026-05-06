pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_MAP_DEPTH: usize = 3;
pub const DEFAULT_MAP_OUTPUT_LIMIT: usize = 6000;
pub const DEFAULT_READ_LINE_LIMIT: usize = 1000;

pub const PATCH_PARAM_DESCRIPTION: &str = "Patch text containing view/read, map, create, write, update, move, and delete operations. Wrap in === begin / === end. Supports batching multiple operations in one call, glob patterns (e.g., read src/**/*.rs), and creating empty files (use 'create <path>' with no following lines). Native *** Begin Patch blocks are also accepted directly.";
pub const THRESHOLD_PARAM_DESCRIPTION: &str = "Optional fuzzy matching threshold (0.0-1.0). Higher values (e.g., 0.97) require stricter matching. Default: 0.95. If an update fails with a recoverable_error but reports high similarity (e.g., 80-94%), retry with a lower threshold (e.g., 0.80) instead of re-reading the file.";
pub const DRY_RUN_PARAM_DESCRIPTION: &str =
    "When true, preview the batch against staged state without committing filesystem changes.";
pub const RESPONSE_FORMAT_PARAM_DESCRIPTION: &str = "Response format. Use 'text' for the human-readable summary (default) or 'json' for a machine-readable JSON payload in the tool text response.";

pub const PATCH_EXEC_DESCRIPTION: &str = include_str!("patch_exec_description.txt");

/// Returns version and build information for the weave-patch-mcp tool.
///
/// # Example
/// ```
/// use weave_patch_mcp::tool_contract::get_version_info;
///
/// let info = get_version_info();
/// println!("Version: {}", info.version);
/// println!("Name: {}", info.name);
/// ```
pub fn get_version_info() -> VersionInfo {
    VersionInfo {
        version: VERSION.to_string(),
        name: env!("CARGO_PKG_NAME").to_string(),
        description: env!("CARGO_PKG_DESCRIPTION").to_string(),
        repository: env!("CARGO_PKG_REPOSITORY").to_string(),
    }
}

/// Version and build information for the weave-patch-mcp tool.
#[derive(Debug, Clone, PartialEq)]
pub struct VersionInfo {
    /// The crate version (from CARGO_PKG_VERSION)
    pub version: String,
    /// The crate name (from CARGO_PKG_NAME)
    pub name: String,
    /// The crate description (from CARGO_PKG_DESCRIPTION)
    pub description: String,
    /// The repository URL (from CARGO_PKG_REPOSITORY)
    pub repository: String,
}

pub fn readme_defaults_line() -> String {
    format!("Defaults: `depth={DEFAULT_MAP_DEPTH}`, `limit={DEFAULT_MAP_OUTPUT_LIMIT}` chars.")
}

pub fn server_instructions() -> String {
    format!(
        r#"## weave-patch-mcp — File read/edit server

One tool: `patch__exec`. Required param: `patch` (string). Optional params: `threshold` (f64, default 0.85), `dry_run` (bool), `response_format` ("json" for machine-readable output).

### Syntax
All operations go inside `=== begin` / `=== end` delimiters. Multiple operations in one block execute in authored order against a staged workspace (write then read sees staged content). Final filesystem commit is atomic — if any op fails, everything rolls back.

### Operations

**view / read** — Read file contents.
- `view <path>` — full file (truncates at {DEFAULT_READ_LINE_LIMIT} lines)
- `read <path> symbols=Foo,Bar language=rust` — extract specific symbols (supports rust, python, typescript, javascript, go)
- `view <path> start=11 end=60` — 1-based line range
- `view <path> offset=0 limit=100` — offset/limit pagination
- `read src/**/*.rs` — glob patterns are supported for reading multiple files at once
- Multiple reads in one block: list each on its own line.

**map** — Directory tree with file sizes, line counts, function signatures.
- `map <path> depth=2` — default depth={DEFAULT_MAP_DEPTH}, default output limit {DEFAULT_MAP_OUTPUT_LIMIT} chars

**create** — Create a new file. Fails if file already exists.
- To create an empty file, simply use `create <path>` with no `+` lines following it.
```
create src/hello.rs
+pub fn hello() {{ println!("Hello!"); }}
```
Accepts raw text or apply_patch-style `+` lines.

**write** — Create or overwrite a file atomically. Best replacement for traditional whole-file write tools.
```
write src/hello.rs
+pub fn hello() {{ println!("Hello!"); }}
```

**update** — Fuzzy-matched patching. Three-phase matching: exact → whitespace-normalized → fuzzy (85%+ threshold).
Context lines (space-prefixed) anchor the edit. `-` removes, `+` adds.
- `@@ label` is an optional context hint (not a line number — just a label to help matching).
- Multiple hunks per file: just add more `@@ ...` blocks.
- Multiple files: add another `update <path>` line followed by its hunks.
- **Rename during update**: add `move_to src/new.rs` after the `update` line.
- **Native apply_patch blocks**: you can paste `*** Begin Patch` / `*** End Patch` blocks directly — they are translated automatically.
- **Markdown pipe tables**: rows starting with `|` are treated as context. Use `-|` and `+|` for remove/add.
```
update src/lib.rs
@@ impl Server
 pub fn handle(&self, req: Request) -> Response {{
-    self.old_handler(req)
+    self.new_handler(req)
 }}
```

**delete** — Delete a file. Multiple deletes: list each on its own line.

### Combined example (all in one call)
```
=== begin
read src/main.rs
map src/ depth=1
update src/lib.rs
@@ fn main
 fn main() {{
-    old();
+    new();
 }}
create src/greet.rs
+pub fn greet() {{ println!("hi"); }}
delete src/deprecated.rs
=== end
```

### Response statuses (OpStatus enum)
- `ok` — succeeded
- `skipped` — no-op (e.g. delete non-existent file)
- `recoverable_error` — match failed, may work with different hunks. Includes top-3 closest matches with line numbers and similarity %.
- `fatal_error` — filesystem issue (not found, permission denied)
- `validation_warning` — advisory syntax check failed (non-blocking)

If a `recoverable_error` reports a high similarity (e.g., 80-84%), you can retry the exact same update with the `threshold` parameter lowered (e.g., `threshold=0.80`) instead of re-reading the file.

### Robustness
- Handles CRLF/LF line ending normalization automatically.
- Supports raw UTF-8 and Unicode. Do not escape non-ASCII characters (e.g., leave `café` as is, do not convert to `caf\u00e9`).
- Long single lines (e.g., minified JSON) are handled correctly up to the 512KB file limit.

### Limits
- 2MB total output for reads, 512KB per file.

### Advisory validators (non-blocking, run after writes)
rustfmt, gofmt, python -m py_compile, python3 -m json.tool, bash -n, node --check, terraform fmt.

### Security
Symlinks rejected. Relative paths (including ../), absolute paths, and ~ expansion are all allowed.

### Tips for agents
- Prefer `read` with `symbols=` over reading whole files — much more token-efficient.
- Prefer `update` over `write` when changing a subset of lines — survives context drift.
- Use `write` only when you intend to replace the entire file contents.
- Use `create` when you need fail-fast semantics for new files.
- Use `dry_run=true` to preview batch changes before committing.
- For multi-file refactors, batch all operations in one `patch__exec` call for atomicity."#
    )
}

