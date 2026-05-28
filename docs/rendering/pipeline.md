## Rendering Pipeline
The end-to-end paint path: raw Markdown bytes to drawn cells on a ratatui buffer.

> **Related**
> - [`syntax-highlighting.md`](syntax-highlighting.md) — syntect inside the layout step
> - [`tables.md`](tables.md) — auto-shrink table widths
> - [`code-blocks.md`](code-blocks.md) — fence handling specifics
> - [`../architecture.md`](../architecture.md) — module map
> - [`../../src/markdown/CLAUDE.md`](../../src/markdown/CLAUDE.md) — parser + layout conventions

## Stages

```
.md bytes
   │
   ▼  pulldown-cmark events  ── src/markdown/parser.rs::parse
Block AST                    ── src/markdown/model.rs (Block enum)
   │
   ▼  width + theme           ── src/markdown/layout.rs::layout(blocks, width, theme)
Vec<LayoutLine>              ── one TUI row each, fully styled, indent-aware
   │
   ▼  viewport slice           ── src/tui/viewport.rs::visible_range
DocumentWidget::render       ── src/tui/draw.rs paints into ratatui Buffer
```

Every stage is synchronous and pure. No I/O, no threads, no caches except the per-document `DisplayRegistry` for mermaid blocks.

## Entry point

`App::from_cli` (`src/app.rs`) reads the source, calls `markdown::parse`, then `markdown::layout::layout(&blocks, width, &theme)`. The result lives on `App.lines: Vec<LayoutLine>`.

On every frame, the event loop calls `app.relayout(area.width)` — cheap when width is unchanged, full relayout when it shifts. `DocumentWidget` then paints `app.lines[viewport.visible_range()]`.

## LayoutLine — the unit of paint

```rust
pub struct LayoutLine {
    pub spans: Vec<StyledSpan>,
    pub block_index: usize,
    pub heading_anchor: Option<String>,
    pub mermaid_block_index: Option<usize>,
    pub indent_cols: u16,
}
```

| Field | Why it exists |
|---|---|
| `spans` | One styled run per fg/bg/modifier combo. Drawn left-to-right. |
| `block_index` | Source block this line came from. Used by mouse-click to find a block. |
| `heading_anchor` | First line of a heading carries the slug; used by TOC + `]`/`[` nav. |
| `mermaid_block_index` | Hook for the `m` toggle and per-block re-render. |
| `indent_cols` | Margin prefix for prose centering (≥120-col terminals). |

## Prose centering

`layout` computes `prose_width = min(100, raw_width)` and `prose_indent = (raw_width - prose_width) / 2` when terminal width ≥ 120. Below that, full-bleed. Magic numbers at the top of `src/markdown/layout.rs`:

```rust
const MAX_CONTENT_WIDTH: u16 = 100;
const MIN_TERMINAL_FOR_PADDING: u16 = 120;
```

Code fences, tables, and mermaid blocks honor the same `prose_width`, so the centered band stays visually consistent.

## Block → renderer dispatch

`layout`'s match arm in `src/markdown/layout.rs:42-87`:

| Block variant | Renderer |
|---|---|
| `Frontmatter` | `render_frontmatter` (YAML/TOML table card) |
| `Heading` | `render_heading` (bold + heading color + anchor) |
| `Paragraph` | `render_paragraph` (token wrap into width) |
| `CodeBlock` | `render_code_block` (syntect → `emit_code_line`) |
| `MermaidBlock` | `render_mermaid` (`ascii::render_mermaid_styled`) |
| `Quote` | `render_quote` (recursive, `│ ` prefix) |
| `List` | `render_list` (bullets / numerals / task `[x]`) |
| `Table` | `render_table` (see [`tables.md`](tables.md)) |
| `Rule` | `render_rule` (full-width `─`) |
| `Footnote` | `render_footnote` (`[^label]: ` body indent) |

## Viewport slicing

`ViewportState` keeps `top_line`, `height`, `total_lines`. `visible_range()` returns `top_line..min(top_line + height, total_lines)`. `DocumentWidget::render` walks that slice and paints each `LayoutLine` at `area.x + indent_cols, y`.

## Watch + re-layout

`Watcher::poll` (mtime polling, 500ms) signals a content change. `App::reload_from_disk` re-reads, re-parses, re-lays out, resets `DisplayRegistry`, and clamps `viewport.top_line` to the new total. Width changes between frames trigger relayout via `App::relayout`.

## Plain mode

`--plain` (or `--no-pager`, or piped stdout) skips ratatui and writes each `LayoutLine`'s span text to stdout (`main.rs::print_plain_to_stdout`). Styling is dropped — the layout output is identical in shape, only the paint backend changes.
