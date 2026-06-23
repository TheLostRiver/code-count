# Project Instructions

Read `TODO.md` before starting any development task in this repository. Update
`TODO.md` when scope, milestones, or verified task status changes.

## Product Direction

`code-count` is a Rust code and document line counting tool.

Build order:

1. CLI first.
2. TUI second.
3. Desktop GUI later.

The TUI should support switchable views:

- Dashboard View: project overview and line composition.
- Explorer View: language and file browsing.
- Report View: interactive report export.

Implement Dashboard View first.

## Architecture Rules

- Keep the scanner logic in `crates/core`.
- Keep CLI argument parsing and terminal output in `crates/cli`.
- Use `tokei` as the counting engine dependency.
- Do not vendor, copy, or fork `tokei` unless the project explicitly decides to.
- Do not expose `tokei` data structures as the public project API.
- Convert `tokei` output into project-owned types such as `ScanReport`,
  `ScanSummary`, and `LanguageStat`.
- Design core data structures for reuse by CLI, TUI, and future GUI code.

## TUI Direction

- Prefer `ratatui` and `crossterm` for terminal UI work.
- Prefer a unified command shape: `code-count tui .`.
- Model views explicitly with an enum such as `AppView`.
- Support at least `q` to quit, `r` to rescan, and `Tab` or `1`/`2`/`3` to
  switch views.
- Keep Dashboard View useful before building Explorer View or Report View.

## Testing And Verification

Use test-first development for new behavior when practical.

Before claiming work is complete, run the relevant verification commands:

```powershell
cargo fmt --check
cargo test
cargo clippy
```

For CLI changes, also run representative commands such as:

```powershell
cargo run -p code-count -- .
cargo run -p code-count -- . --json
```

For TUI changes, manually test in Windows Terminal when possible.

## Repository Hygiene

- Keep generated files out of git, especially `target/` and `.superpowers/`.
- Avoid unrelated refactors.
- Keep files focused by responsibility.
- Update README when user-facing commands or project direction changes.
- Do not push to the remote unless the user asks.
