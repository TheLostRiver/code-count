# TUI Visual Polish v0.2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Polish the existing ratatui interface toward the approved HTML mock style without changing scanner behavior or keyboard workflows.

**Architecture:** Keep the scanner and CLI unchanged. Add small style/rendering helpers in `crates/tui/src/lib.rs`, then update Dashboard, Explorer, and Report render text to share a dark terminal visual language.

**Tech Stack:** Rust, ratatui, crossterm, existing `render_to_text` tests.

---

### Task 1: Dashboard Polish Markers

**Files:**
- Modify: `crates/tui/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Add assertions to dashboard tests for `scan complete`, `Total`, `code`, `comments`, `docs`, and `blank` labels in the polished header/summary.

- [ ] **Step 2: Run targeted tests**

Run: `cargo test -p code-count-tui dashboard`

Expected: FAIL before the rendering changes.

- [ ] **Step 3: Implement Dashboard polish**

Add shared panel/theme helpers, a multi-segment total bar, improved composition rows, and colored language lines.

- [ ] **Step 4: Re-run targeted tests**

Run: `cargo test -p code-count-tui dashboard`

Expected: PASS.

### Task 2: Explorer and Report Consistency

**Files:**
- Modify: `crates/tui/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Add assertions for Explorer `scan complete`, `Largest files`, and Report `Report sections`/`Export target`.

- [ ] **Step 2: Implement shared visual language**

Use the same panel block style, semantic labels, and compact rows for Explorer and Report.

- [ ] **Step 3: Run targeted tests**

Run: `cargo test -p code-count-tui explorer report`

Expected: PASS.

### Task 3: Full Verification

**Files:**
- Modify: `TODO.md` only if a new explicit polish item is added.

- [ ] **Step 1: Run full checks**

Run:

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo run -p code-count -- tui .
```

- [ ] **Step 2: Review diff and commit**

Commit message: `style: polish tui visuals`
