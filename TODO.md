# code-count TODO

Read this file before starting any development task. Keep it updated as the
project changes, and mark items only after they are verified.

## Current Direction

`code-count` starts as a Rust CLI backed by `tokei`, then grows a terminal UI
before any full desktop GUI work.

The UI family is intentionally split into switchable views:

- Dashboard View: project health overview.
- Explorer View: language and file browsing.
- Report View: interactive report export.

The first TUI milestone is Dashboard View.

## Milestone 0: Repository Baseline

- [x] Commit the current workspace initialization.
- [x] Remove the unused root `src/` directory.
- [x] Keep generated files ignored: `target/` and `.superpowers/`.
- [x] Keep README aligned with the product direction: CLI first, TUI next, GUI later.

## Milestone 1: Core Report Model

- [ ] Replace the summary-only model with a richer `ScanReport`.
- [ ] Keep a top-level `summary` field for totals.
- [ ] Add `languages: Vec<LanguageStat>`.
- [ ] Add a category breakdown for code, comments, documents, and blanks.
- [ ] Classify document-oriented file types such as Markdown and plain text.
- [ ] Add tests for a mixed Rust and Markdown sample project.
- [ ] Preserve JSON serialization for CLI and future UI consumers.

## Milestone 2: CLI Compatibility

- [ ] Keep `code-count .` useful as a compact summary.
- [ ] Keep `code-count . --json` valid with the new report model.
- [ ] Add `--by-language` output.
- [ ] Keep errors readable for missing paths and unsupported inputs.

## Milestone 3: TUI Foundation

- [ ] Add a TUI crate or module using `ratatui` and `crossterm`.
- [ ] Prefer a unified command shape: `code-count tui .`.
- [ ] Define app state that owns the current `ScanReport`.
- [ ] Define `AppView` with `Dashboard`, `Explorer`, and `Report`.
- [ ] Support `q` to quit.
- [ ] Support `r` to rescan.
- [ ] Support `Tab` and `1`/`2`/`3` to switch views.

## Milestone 4: Dashboard View v0.1

- [ ] Show project path, file count, and total line count.
- [ ] Show code, comment, document, and blank line totals.
- [ ] Render a horizontal composition bar.
- [ ] Show Top Languages.
- [ ] Show a compact stats panel.
- [ ] Show a footer with available key bindings.
- [ ] Handle narrow terminals with a stacked layout.

## Milestone 5: Explorer View

- [ ] Show language list on the left.
- [ ] Show selected language details on the right.
- [ ] Support keyboard selection with up/down.
- [ ] Support `/` filtering.
- [ ] Add per-file details after language details are stable.

## Milestone 6: Report View

- [ ] Choose export format: JSON, Markdown, or CSV.
- [ ] Choose whether to include language details.
- [ ] Choose whether to include file details.
- [ ] Preview the report summary.
- [ ] Export with `e`.

## Milestone 7: Quality Gates

- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo test`.
- [ ] Run `cargo clippy`.
- [ ] Manually test the CLI on Windows.
- [ ] Manually test the TUI in Windows Terminal.
- [ ] Add CI for Windows, macOS, and Linux when the CLI stabilizes.

## Later Ideas

- [ ] Scan history and diff between scans.
- [ ] Markdown and HTML report templates.
- [ ] Project config file: `code-count.toml`.
- [ ] GUI proof of concept after TUI matures.
