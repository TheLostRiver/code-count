# code-count

`code-count` is a cross-platform code and document line counting tool.

The project starts with a CLI powered by a reusable Rust core library, then adds
a terminal UI on the same scanner model. A full desktop GUI can be added later
without rewriting the scanning engine.

The counting engine is based on `tokei`, while this project owns its public data
model and user experience.

## Planned scope

- Count source files, scripts, Markdown, and plain text documents.
- Report total, code, comment, and blank lines.
- Support human-readable, JSON, language breakdown, and TUI output.
- Keep the core scanner reusable for CLI, TUI, and future desktop GUI frontends.

## Usage

```powershell
code-count .
code-count . --json
code-count . --by-language
code-count tui .
```

Use ignore flags for one-off scans:

```powershell
code-count . --ignore-blank
code-count . --ignore-comments
```

## Project config

Create `code-count.toml` in the project root to set scan and TUI defaults:

```toml
[scan]
include_blank_lines = true
include_comments = true
ignored_paths = ["target", ".git", "node_modules"]

[tui]
default_view = "dashboard"
report_format = "json"
```

Supported TUI views are `dashboard`, `explorer`, and `report`. Supported report
formats are `json`, `markdown`, and `csv`. `--ignore-blank` and
`--ignore-comments` override config by disabling those counts for the current
run. `code-count.toml` is ignored automatically when scanning the configured
project.

## Architecture

```text
crates/core
  Reusable scanner API that wraps tokei output in project-owned types.

crates/cli
  Command-line interface and terminal output.

crates/tui
  Terminal UI built on the shared scanner report model.
```

Frontends should build on the same core scanner instead of calling `tokei`
directly.

## Roadmap

1. CLI baseline.
2. Rich `ScanReport` model with language and document categories.
3. TUI dashboard using `ratatui` and `crossterm`.
4. Explorer and report views.
5. Project config with `code-count.toml`.
6. Desktop GUI proof of concept.

## Development

```powershell
cargo fmt --check
cargo test
cargo clippy
cargo run -p code-count -- .
cargo run -p code-count -- . --json
```
