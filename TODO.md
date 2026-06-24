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

## Milestone 6.6: TUI Visual Polish v0.3

- [x] Add view-specific header titles for Dashboard, Explorer, and Report.
- [x] Make Dashboard emphasize the Total line composition before secondary panels.
- [x] Improve Explorer selected language and detail visual hierarchy.
- [x] Polish Report Builder controls and preview copy.
- [x] Render key bindings as compact key-hint chips.

## Milestone 6.7: TUI Visual Polish v0.4

- [x] Tighten TUI footer key hints toward the mock density.
- [x] Add Space toggle support in Report View.
- [x] Add Dashboard Top Languages columns for language, lines, share, and type.
- [x] Add Explorer selected-language share information.
- [x] Clarify Report Export options.

## Milestone 6.8: TUI Visual Polish v0.5

- [x] Replace ASCII bar glyphs with block-style TUI bars.
- [x] Add line counts to Dashboard composition rows.
- [x] Add Explorer detail breakdown and project share labels.
- [x] Add Report Export ready state.

## Milestone 6.9: TUI Visual Polish v0.6

- [x] Format large TUI counts with thousands separators.
- [x] Add Explorer detail metric/value rows for denser scanning.
- [x] Add Explorer file table headers for path, total, and code lines.
- [x] Keep Report preview focused on target and summary counts first.
- [x] Split Report export metadata into compact label/value rows.
- [x] Verify with `cargo fmt --check`, `cargo test`, and `cargo clippy`.

## Milestone 6.10: TUI Visual Polish v0.7

- [x] Make Report View default to Markdown with a `.md` export target.
- [x] Add selected-language file navigation to the Explorer language panel.
- [x] Simplify Explorer detail metrics toward the mock layout.
- [x] Tighten Report preview summary labels toward the mock.
- [x] Verify with `cargo fmt --check`, `cargo test`, and `cargo clippy`.

## Milestone 6.11: TUI Visual Polish v0.8

- [x] Lengthen the Dashboard total composition bar toward the mock.
- [x] Make Explorer file navigation use a single selected-file marker.
- [x] Reduce Report Export metadata to the mock-focused essentials.
- [x] Verify with `cargo fmt --check`, `cargo test`, and `cargo clippy`.

## Milestone 6.12: TUI Visual Polish v0.9

- [x] Keep Report preview summary-only even on taller terminals.
- [x] Reduce Report Export to Format and Target rows before export.
- [x] Tighten Report footer hints toward the mock.
- [x] Verify with `cargo fmt --check`, `cargo test`, and `cargo clippy`.

## Milestone 6.13: TUI Visual Polish v0.10

- [x] Right-align the Dashboard total count in the summary panel.
- [x] Spread Dashboard summary category counts into left and right columns.
- [x] Preserve compact Dashboard summary rendering on narrow terminals.
- [x] Verify with `cargo fmt --check`, `cargo test`, and `cargo clippy`.

## Milestone 6.14: TUI Visual Polish v0.11

- [x] Simplify Dashboard Top Languages into a mock-style list.
- [x] Simplify Explorer largest files into path and total rows.
- [x] Verify with `cargo fmt --check`, `cargo test`, and `cargo clippy`.

## Milestone 6.15: TUI Visual Polish v0.12

- [x] Add compact view tabs to the TUI header.
- [x] Preserve active scan-scope context in the polished header.
- [x] Verify with `cargo fmt --check`, `cargo test`, and `cargo clippy`.

## Milestone 7: Quality Gates

- [x] Run `cargo fmt --check`.
- [x] Run `cargo test`.
- [x] Run `cargo clippy`.
- [x] Manually test the CLI on Windows.
- [x] Manually test the TUI in Windows Terminal.
- [x] Add CI for Windows, macOS, and Linux when the CLI stabilizes.

## Milestone 8: Portable Windows Release

- [x] Build a single `code-count.exe` release artifact.
- [x] Add a Windows portable package script.
- [x] Add user PATH install and uninstall scripts.
- [x] Document portable install in English and Chinese README files.

## Milestone 9: Scan Scope Controls

- [x] Add a repeated `--ignore <path>` CLI option.
- [x] Merge CLI ignore paths with `code-count.toml` `ignored_paths`.
- [x] Apply merged ignore paths to CLI, report, history, and TUI scans.
- [x] Deduplicate ignore paths while preserving user-facing order.
- [x] Document one-off and config-based ignore workflows.
- [x] Verify with `cargo fmt --check`, `cargo test`, and `cargo clippy`.

## Milestone 10: TUI Scope Assistant

- [x] Show active ignored paths in the TUI.
- [x] Add a scan-scope hint area for active ignore rules.
- [x] Add session-scoped interactive ignore-and-rescan support in Explorer.
- [x] Add Explorer status feedback and undo for session-scoped ignores.
- [x] Verify with `cargo fmt --check`, `cargo test`, and `cargo clippy`.

## Later Ideas

- [x] Scan history and diff between scans.
- [x] Markdown and HTML report templates.
- [x] Project config file: `code-count.toml`.
- [ ] GUI proof of concept after TUI matures.
