# code-count

[简体中文](README.zh-CN.md)

`code-count` is a cross-platform code and document line counting tool.

The project starts with a CLI powered by a reusable Rust core library, then adds
a terminal UI on the same scanner model. A full desktop GUI can be added later
without rewriting the scanning engine.

The counting engine is based on `tokei`, while this project owns its public data
model and user experience.

## Planned scope

- Count source files, scripts, Markdown, and plain text documents.
- Report total, code, comment, and blank lines.
- Support human-readable, JSON, language breakdown, Markdown/HTML reports, TUI
  output, and snapshot diffs.
- Keep the core scanner reusable for CLI, TUI, and future desktop GUI frontends.

## Usage

```powershell
code-count .
code-count . --json
code-count . --by-language
code-count report . --format markdown --output report.md
code-count report . --format html --output report.html --files
code-count tui .
```

## Portable Windows install

Build a portable package:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\package-windows.ps1
```

The package is written to:

```text
dist\code-count-windows-x64\
```

Move that folder wherever you want to keep the tool, then run:

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

The installer adds the portable folder to your user `PATH`, so a new terminal can
run `code-count` from any directory. It does not require administrator rights.

To remove the PATH entry later:

```powershell
powershell -ExecutionPolicy Bypass -File .\uninstall.ps1
```

Use ignore flags for one-off scans:

```powershell
code-count . --ignore-blank
code-count . --ignore-comments
```

Save scan snapshots and compare them:

```powershell
code-count history save . --output before.json
code-count history save . --output after.json
code-count diff before.json after.json
```

Export shareable reports:

```powershell
code-count report . --format markdown --output report.md
code-count report . --format html --output report.html --files
code-count report . --format csv
```

`report` writes to stdout when `--output` is omitted. Supported formats are
`json`, `markdown`, `html`, and `csv`. Add `--files` when you want per-file
details in the exported report.

## Project config

Create `code-count.toml` in the project root to set scan and TUI defaults:

```toml
[scan]
include_blank_lines = true
include_comments = true
ignored_paths = ["target", ".git", "node_modules"]

[tui]
default_view = "dashboard"
report_format = "markdown"
```

Supported TUI views are `dashboard`, `explorer`, and `report`. Supported report
formats are `json`, `markdown`, `html`, and `csv`. `--ignore-blank` and
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
6. Scan history and diff between snapshots.
7. Markdown and HTML report templates.
8. Desktop GUI proof of concept.

## Development

```powershell
cargo fmt --check
cargo test
cargo clippy
cargo run -p code-count -- .
cargo run -p code-count -- . --json
```
