# Markdown and HTML Report Templates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add reusable Markdown and HTML report rendering, expose it through `code-count report`, and make TUI export reuse the same renderer.

**Architecture:** `crates/core` owns a new `report` module that renders project-owned `ScanReport` data into JSON, Markdown, HTML, or CSV. `crates/cli` parses user-facing command flags and writes stdout or a file. `crates/tui` keeps terminal state and delegates report rendering and file extensions to core.

**Tech Stack:** Rust 2024, `tokei`, `serde_json`, `clap`, `ratatui`, `crossterm`, `assert_cmd`.

---

### Task 1: Core Report Renderer

**Files:**
- Create: `crates/core/src/report.rs`
- Modify: `crates/core/src/lib.rs`
- Modify: `crates/core/Cargo.toml`

- [ ] **Step 1: Write failing core renderer tests**

Add tests that call the desired `render_report` API with a synthetic `ScanReport`.

Expected checks:
- Markdown contains a summary list, a language table, and optional file rows.
- HTML contains a full document, escaped paths, summary metrics, language rows, and optional file rows.
- CSV behavior is preserved in the shared renderer.

- [ ] **Step 2: Run focused core test and verify red**

Run:

```powershell
$env:CARGO_TARGET_DIR='target\verify'; cargo test -p code-count-core report
```

Expected: compile failure or test failure because the renderer API does not exist yet.

- [ ] **Step 3: Implement renderer module**

Create:
- `ReportFormat` enum with `Json`, `Markdown`, `Html`, `Csv`.
- `ReportRenderOptions` with `include_language_details` and `include_file_details`.
- `render_report(&ScanReport, ReportFormat, &ReportRenderOptions) -> Result<String, ReportRenderError>`.
- `report_extension(ReportFormat) -> &'static str`.
- Markdown, HTML, and CSV escaping helpers.

- [ ] **Step 4: Run focused core tests and verify green**

Run:

```powershell
$env:CARGO_TARGET_DIR='target\verify'; cargo test -p code-count-core report
```

Expected: all report renderer tests pass.

### Task 2: CLI Report Command

**Files:**
- Modify: `crates/cli/src/main.rs`
- Modify: `crates/cli/tests/cli.rs`

- [ ] **Step 1: Write failing CLI integration tests**

Add tests for:
- `code-count report <path> --format markdown --output <file>` writes a Markdown file.
- `code-count report <path> --format html --output <file> --files` writes a self-contained HTML file with file details.
- `code-count report <path> --format csv` prints CSV to stdout.

- [ ] **Step 2: Run focused CLI test and verify red**

Run:

```powershell
$env:CARGO_TARGET_DIR='target\verify'; cargo test -p code-count --test cli report
```

Expected: failure because the `report` subcommand is not implemented.

- [ ] **Step 3: Implement CLI command**

Add:
- `Command::Report { path, format, output, files }`.
- format parsing for `json`, `markdown`/`md`, `html`, and `csv`.
- report rendering through `code_count_core::render_report`.
- file writing with parent directory creation when `--output` is used.
- stdout printing when `--output` is omitted.

- [ ] **Step 4: Run focused CLI tests and verify green**

Run:

```powershell
$env:CARGO_TARGET_DIR='target\verify'; cargo test -p code-count --test cli report
```

Expected: report command tests pass.

### Task 3: TUI Reuse and HTML Format

**Files:**
- Modify: `crates/tui/src/lib.rs`
- Modify: `crates/tui/Cargo.toml` only if no longer needed dependencies can be removed.
- Modify: `crates/cli/src/main.rs` for config parsing if needed.

- [ ] **Step 1: Update failing TUI tests**

Adjust existing report export tests to expect the shared renderer output and add HTML export coverage.

- [ ] **Step 2: Run focused TUI tests and verify red**

Run:

```powershell
$env:CARGO_TARGET_DIR='target\verify'; cargo test -p code-count-tui report
```

Expected: failure while TUI still uses its local renderer and does not cycle through HTML.

- [ ] **Step 3: Replace local TUI renderer**

Use core `ReportFormat`, `ReportRenderOptions`, `render_report`, and `report_extension`. Keep TUI state and key handling local.

- [ ] **Step 4: Run focused TUI tests and verify green**

Run:

```powershell
$env:CARGO_TARGET_DIR='target\verify'; cargo test -p code-count-tui report
```

Expected: TUI report tests pass.

### Task 4: Docs, TODO, and Full Verification

**Files:**
- Modify: `README.md`
- Modify: `README.zh-CN.md`
- Modify: `TODO.md`
- Modify: `.planning/2026-06-24-markdown-and-html-report-templates/task_plan.md`
- Modify: `.planning/2026-06-24-markdown-and-html-report-templates/findings.md`

- [ ] **Step 1: Update docs**

Document:
- `code-count report . --format markdown --output report.md`
- `code-count report . --format html --output report.html --files`
- Supported report formats.

- [ ] **Step 2: Run full verification**

Run:

```powershell
$env:CARGO_TARGET_DIR='target\verify'; cargo fmt --check
$env:CARGO_TARGET_DIR='target\verify'; cargo test
$env:CARGO_TARGET_DIR='target\verify'; cargo clippy --all-targets -- -D warnings
$env:CARGO_TARGET_DIR='target\verify'; cargo run -p code-count -- report . --format markdown
$env:CARGO_TARGET_DIR='target\verify'; cargo run -p code-count -- report . --format html --files --output target\verify\code-count-report.html
```

Expected: all commands exit successfully.

- [ ] **Step 3: Update status files**

Mark the TODO item complete only after verification passes. Record verification results in PWF findings.

- [ ] **Step 4: Review git diff**

Run:

```powershell
git status --short --branch --ahead-behind
git diff --stat
```

Expected: only planned files changed, plus local branch remains ahead because of existing unpublished commits.
