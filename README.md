# code-count

`code-count` is a cross-platform code and document line counting tool.

The project starts with a CLI powered by a reusable Rust core library. A TUI is
planned next, and a full desktop GUI can be added later without rewriting the
scanning engine.

The counting engine is based on `tokei`, while this project owns its public data
model and user experience.

## Planned scope

- Count source files, scripts, Markdown, and plain text documents.
- Report total, code, comment, and blank lines.
- Support both human-readable and JSON output.
- Keep the core scanner reusable for CLI, TUI, and future desktop GUI frontends.

## Architecture

```text
crates/core
  Reusable scanner API that wraps tokei output in project-owned types.

crates/cli
  Command-line interface and terminal output.
```

Future TUI work should build on the same core scanner instead of calling `tokei`
directly.

## Roadmap

1. CLI baseline.
2. Rich `ScanReport` model with language and document categories.
3. TUI dashboard using `ratatui` and `crossterm`.
4. Explorer and report views.
5. Desktop GUI proof of concept.

## Development

```powershell
cargo fmt --check
cargo test
cargo clippy
cargo run -p code-count -- .
cargo run -p code-count -- . --json
```
