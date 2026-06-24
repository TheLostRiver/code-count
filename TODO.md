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

- [x] Replace the summary-only model with a richer `ScanReport`.
- [x] Keep a top-level `summary` field for totals.
- [x] Add `languages: Vec<LanguageStat>`.
- [x] Add a category breakdown for code, comments, documents, and blanks.
- [x] Classify document-oriented file types such as Markdown and plain text.
- [x] Add tests for a mixed Rust and Markdown sample project.
- [x] Preserve JSON serialization for CLI and future UI consumers.

## Milestone 2: CLI Compatibility

- [x] Keep `code-count .` useful as a compact summary.
- [x] Keep `code-count . --json` valid with the new report model.
- [x] Add `--by-language` output.
- [x] Keep errors readable for missing paths and unsupported inputs.

## Milestone 3: TUI Foundation

- [x] Add a TUI crate or module using `ratatui` and `crossterm`.
- [x] Prefer a unified command shape: `code-count tui .`.
- [x] Define app state that owns the current `ScanReport`.
- [x] Define `AppView` with `Dashboard`, `Explorer`, and `Report`.
- [x] Support `q` to quit.
- [x] Support `r` to rescan.
- [x] Support `Tab` and `1`/`2`/`3` to switch views.

## Milestone 4: Dashboard View v0.1

- [x] Show project path, file count, and total line count.
- [x] Show code, comment, document, and blank line totals.
- [x] Render a horizontal composition bar.
- [x] Show Top Languages.
- [x] Show a compact stats panel.
- [x] Show a footer with available key bindings.
- [x] Handle narrow terminals with a stacked layout.

## Milestone 5: Explorer View

- [x] Show language list on the left.
- [x] Show selected language details on the right.
- [x] Support keyboard selection with up/down.
- [x] Support `/` filtering.
- [x] Add per-file details after language details are stable.

## Milestone 6: Report View

- [x] Choose export format: JSON, Markdown, or CSV.
- [x] Choose whether to include language details.
- [x] Choose whether to include file details.
- [x] Preview the report summary.
- [x] Export with `e`.

## Milestone 6.5: TUI Visual Polish v0.2

- [x] Add a shared dark terminal visual style for TUI panels.
- [x] Add scan status text to the TUI header.
- [x] Polish Dashboard summary, composition, and Top Languages toward the mock.
- [x] Polish Explorer language/details panels with percentages and largest files.
- [x] Polish Report controls with report section language.

## Milestone 7: Quality Gates

- [x] Run `cargo fmt --check`.
- [x] Run `cargo test`.
- [x] Run `cargo clippy`.
- [x] Manually test the CLI on Windows.
- [x] Manually test the TUI in Windows Terminal.
- [x] Add CI for Windows, macOS, and Linux when the CLI stabilizes.

## Later Ideas

- [x] Scan history and diff between scans.
- [ ] Markdown and HTML report templates.
- [x] Project config file: `code-count.toml`.
- [ ] GUI proof of concept after TUI matures.
