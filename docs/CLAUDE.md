## Veol docs
Cross-cutting documentation for the reader. Per-module specifics live in `src/<module>/CLAUDE.md`; this folder covers everything that spans modules.

> **Docs reflect CURRENT state.** Describe the system as it is now — not how it got here. Don't narrate migrations, renames, removals, or "previously X / now Y / legacy / recently added." That belongs in commits and PRs.
>
> **Exception:** *load-bearing why* — a past incident or constraint that still shapes a current decision (e.g., the mmdc → ASCII pivot, the mouse-capture re-enablement). If removing the history would make a current rule harder to judge, keep it. Those live in [`decisions/`](decisions/).

## What lives here

| Folder | Owns | Read when |
|---|---|---|
| [`rendering/`](rendering/) | Paint pipeline — block → layout line → ratatui buffer | Touching `src/markdown/layout.rs`, `src/render/`, or how a Markdown block becomes a TUI cell |
| [`tui/`](tui/) | Viewport math, modal popups, search highlighting | Adding a keybind, popup, or scroll behavior in `src/tui/` |
| [`configuration/`](configuration/) | `config.toml`, every CLI flag, watch mode | Adding or changing a flag / config field, or debugging mtime polling |
| [`diagrams/`](diagrams/) | Mermaid renderer architecture (pivot, charsets, color pipeline) | Touching `src/mermaid/ascii/` or extending the renderer |
| [`decisions/`](decisions/) | ADRs — locked, dated, "why" not "how" | Before re-litigating a design choice |
| [`runbooks/`](runbooks/) | Step-by-step operational procedures | Cutting a release, reviewing snapshots, adding a diagram type |
| [`tasks/`](tasks/) | Locked specs (DO NOT edit) | Source of truth for the spec + mermaid pivot |

Loose files:

| File | Owns |
|---|---|
| [`architecture.md`](architecture.md) | Top-level data flow + threading model |
| [`keybindings.md`](keybindings.md) | Every key in every mode |
| [`theming.md`](theming.md) | Theme schema, bundled palettes, switcher UX |
| [`mermaid.md`](mermaid.md) | User-facing diagram type matrix + limitations |
| [`contributing.md`](contributing.md) | TDD discipline, gates, commit style |

## Map by intent

### Implementation questions

| Question | Read |
|---|---|
| How does Markdown bytes become painted cells? | [`rendering/pipeline.md`](rendering/pipeline.md) |
| How does syntect integrate with the layout? | [`rendering/syntax-highlighting.md`](rendering/syntax-highlighting.md) |
| Why do tables shrink that way? | [`rendering/tables.md`](rendering/tables.md) |
| How are fenced code blocks painted? | [`rendering/code-blocks.md`](rendering/code-blocks.md) |
| What model does the viewport use? | [`tui/viewport.md`](tui/viewport.md) |
| How does the file browser filter / navigate? | [`tui/file-browser.md`](tui/file-browser.md) |
| How does smart-case search work? | [`tui/search.md`](tui/search.md) |
| How is the TOC built? | [`tui/toc.md`](tui/toc.md) |
| What does `?` show? | [`tui/help.md`](tui/help.md) |

### Configuration

| Question | Read |
|---|---|
| All `config.toml` options and defaults | [`configuration/config-file.md`](configuration/config-file.md) |
| All CLI flags | [`configuration/cli-flags.md`](configuration/cli-flags.md) |
| Why is watch mode polling-based? | [`configuration/watch-mode.md`](configuration/watch-mode.md) |

### Mermaid renderer

| Question | Read |
|---|---|
| Why did Veol drop mmdc/Chrome? | [`diagrams/architecture-pivot.md`](diagrams/architecture-pivot.md) |
| Unicode vs ASCII glyphs; junction merging | [`diagrams/charset-and-junctions.md`](diagrams/charset-and-junctions.md) |
| How are diagrams colored? | [`diagrams/color-pipeline.md`](diagrams/color-pipeline.md) |
| User-facing diagram type list | [`mermaid.md`](mermaid.md) |

### Decisions (ADRs)

| ADR | Topic |
|---|---|
| [`decisions/0001-no-async-runtime.md`](decisions/0001-no-async-runtime.md) | `std::thread` + `mpsc`, no tokio |
| [`decisions/0002-pure-rust-mermaid.md`](decisions/0002-pure-rust-mermaid.md) | Drop mmdc; 18-type ASCII renderer |
| [`decisions/0003-mouse-capture-on-by-default.md`](decisions/0003-mouse-capture-on-by-default.md) | Mouse capture re-enabled; `--no-mouse` to opt out |
| [`decisions/0004-no-buffer-editing.md`](decisions/0004-no-buffer-editing.md) | Veol is a reader; the buffer is immutable |
| [`decisions/0005-paginated-by-default.md`](decisions/0005-paginated-by-default.md) | TUI when stdin+stdout are TTY; `--plain` for piping |

### Runbooks

| Question | Read |
|---|---|
| How do I cut a release? | [`runbooks/release-checklist.md`](runbooks/release-checklist.md) |
| What do I do with snapshot diffs? | [`runbooks/snapshot-management.md`](runbooks/snapshot-management.md) |
| How do I add a mermaid diagram type? | [`runbooks/adding-a-diagram-type.md`](runbooks/adding-a-diagram-type.md) |

## How to add a doc

1. Pick the right folder using the table above. If nothing fits, propose a new folder before sneaking it in.
2. Top of the file: `## Title` then a one-line summary on the second line.
3. Add a `> **Related**` block linking to adjacent docs (and update those docs' Related blocks to point back).
4. Stay under 200 lines; most docs ≤ 120. Code-grounded — quote real function names and file paths (with line numbers when stable).
5. Markdown tables for anything tabular. Fenced ASCII for diagrams (no embedded mermaid — meta-confusing).
6. No emojis.
7. Add a row to the [Map by intent](#map-by-intent) table in this file.

## Update protocol

When you change code, update docs. Use this table.

| If you change... | Update... |
|---|---|
| `src/cli.rs` — flag set | [`configuration/cli-flags.md`](configuration/cli-flags.md) |
| `src/config.rs` — fields or defaults | [`configuration/config-file.md`](configuration/config-file.md) |
| `src/watcher.rs` | [`configuration/watch-mode.md`](configuration/watch-mode.md) |
| `src/markdown/layout.rs` — table math | [`rendering/tables.md`](rendering/tables.md) |
| `src/markdown/layout.rs` — code fence painting | [`rendering/code-blocks.md`](rendering/code-blocks.md) |
| `src/render/code.rs` | [`rendering/syntax-highlighting.md`](rendering/syntax-highlighting.md) |
| `src/tui/viewport.rs` | [`tui/viewport.md`](tui/viewport.md) |
| `src/tui/browser.rs` | [`tui/file-browser.md`](tui/file-browser.md) |
| `src/tui/search.rs` | [`tui/search.md`](tui/search.md) |
| `src/tui/toc.rs` | [`tui/toc.md`](tui/toc.md) |
| `src/tui/help.rs::keybinds()` | [`tui/help.md`](tui/help.md), [`keybindings.md`](keybindings.md) |
| `src/mermaid/ascii/mod.rs` — dispatcher | [`diagrams/architecture-pivot.md`](diagrams/architecture-pivot.md), [`mermaid.md`](mermaid.md) |
| `src/mermaid/ascii/charset.rs` | [`diagrams/charset-and-junctions.md`](diagrams/charset-and-junctions.md) |
| `src/mermaid/ascii/canvas.rs` — color grid | [`diagrams/color-pipeline.md`](diagrams/color-pipeline.md) |
| Adding a new mermaid diagram type | [`runbooks/adding-a-diagram-type.md`](runbooks/adding-a-diagram-type.md), [`mermaid.md`](mermaid.md) |

## Cross-references

- [`../.claude/CLAUDE.md`](../.claude/CLAUDE.md) — root AI instructions; P0/P1/P2 beliefs, NEVER list, project map.
- [`../README.md`](../README.md) — user-facing landing page.
- [`tasks/guiwohl-veol-spec-2026-05-26.md`](tasks/guiwohl-veol-spec-2026-05-26.md) — locked spec.
- [`tasks/guiwohl-veol-ascii-mermaid-2026-05-27.md`](tasks/guiwohl-veol-ascii-mermaid-2026-05-27.md) — mermaid pivot spec.
- Per-module knowledge: `src/markdown/CLAUDE.md`, `src/mermaid/CLAUDE.md`, `src/mermaid/ascii/CLAUDE.md`, `src/render/CLAUDE.md`, `src/tui/CLAUDE.md`.
