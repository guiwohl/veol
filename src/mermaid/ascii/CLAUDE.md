# mermaid/ascii/

Pure-Rust ASCII / Unicode diagram renderer. Dispatcher + shared primitives at the top level; one subdirectory per diagram type (18 of them). For the user-facing diagram type matrix and limitations, see [`docs/mermaid.md`](../../../docs/mermaid.md). For the renderer pivot rationale, see [`docs/tasks/guiwohl-veol-ascii-mermaid-2026-05-27.md`](../../../docs/tasks/guiwohl-veol-ascii-mermaid-2026-05-27.md).

## Files (top level)

| File | Purpose |
|---|---|
| `mod.rs` | `render_mermaid_styled(src, w)` dispatcher → routes to per-type `render_styled`. Holds `fallback_source_styled` (synthesized `// mermaid: <reason>\n\`\`\`mermaid\n…\n\`\`\``). Never panics, never returns Err. |
| `canvas.rs` | `Canvas { Vec<Vec<char>>, Vec<Vec<Option<Color>>>, CharsetKind }`. `put_char`, `put_str`, `draw_box`, `draw_hline/vline`, `merge` (junction-aware), `into_styled_lines`. Color grid is parallel to char grid — one cell per glyph. |
| `charset.rs` | `Charset` trait + `UnicodeSet` (`─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼ ╱ ╲ ▲ ▼ ◄ ► ╭ ╮ ╰ ╯`) + `AsciiSet` (`- \| + < > ^ v / \\`). Plus the giant `merge_junctions(a,b)` table (108 entries, ported verbatim from the Go reference). |
| `coord.rs` | `Coord { x, y }` + `Direction` (8-way + Middle) with `from_to` and `opposite`. |
| `astar.rs` | `find_path(from, to, max_x, max_y, is_free)` — 4-connected A* on grid coords. Heuristic = Manhattan + 1 if dx≠0 AND dy≠0 (penalizes corner-required paths so straight runs win ties). |
| `detect.rs` | `detect_kind(src)` → `DiagramKind`. Looks at the first non-comment / non-blank line; case-insensitive. |
| `error.rs` | `AsciiRenderError { Parse, Unsupported, Layout, Empty }`. `thiserror`. |
| `label.rs` | `Label::new(text)` splits on `<br>`; `width()` uses `UnicodeWidthStr`. Used by every diagram for multi-line node labels. |

Per-diagram dirs: `architecture/`, `block/`, `class/`, `er/`, `flowchart/`, `gantt/`, `gitgraph/`, `journey/`, `mindmap/`, `packet/`, `pie/`, `quadrant/`, `requirement/`, `sankey/`, `sequence/`, `state/`, `timeline/`, `xychart/`. Each holds `ast.rs`, `parser.rs`, `render.rs`, optionally `layout.rs`, plus a `snapshots/` dir.

## Key design decisions

- **`render → render_to_canvas → render_styled` refactor pattern** — EVERY diagram type follows this. Per-type `render.rs` exposes:

  ```rust
  pub fn render(...) -> Result<Vec<String>, AsciiRenderError>      // plain
  pub fn render_styled(...) -> Result<Vec<StyledRow>, AsciiRenderError>  // colored
  fn render_to_canvas(...) -> Result<Canvas, AsciiRenderError>     // shared core
  ```

  Both public entry points call the private `render_to_canvas`, then dispatch to either `canvas.into_lines()` (drops color) or `canvas.into_styled_lines()` (groups consecutive cells by color into `StyledRun`s, trims trailing colorless whitespace). When adding a new diagram, mirror this exact shape — `ascii::mod.rs::render_mermaid_styled` calls `render_styled` directly; the plain `render` exists for tests and `--plain` output.

- **Parallel color grid in `Canvas`** — `colors: Vec<Vec<Option<Color>>>` runs lockstep with `rows: Vec<Vec<char>>`. `put_char` only writes a glyph; coloring needs an explicit `set_color` / `paint_text` / `put_str_colored`. This split exists so layout-time draws stay color-free and `render_styled` can paint a finished canvas in one pass.

- **Junction merging is char-pair-table-based, not algorithmic** — `merge_junctions(a, b)` in `charset.rs` is a 108-arm `match`. It's ported from the Go ASCII-mermaid reference; every entry came from that table. Don't try to "simplify" with a bitmask scheme until you've checked every snapshot still matches. Triggered in `Canvas::merge` when both source and target cells are junction chars AND the canvas is `Unicode` (ASCII charset skips it — `+` already absorbs everything).

- **A* corner penalty in `astar.rs`** — `h = dx + dy + (if dx != 0 && dy != 0 { 1 } else { 0 })`. The +1 nudge biases toward L-shapes that finish their long leg first; without it, deterministic tie-breaking would zigzag. Tested implicitly via flowchart snapshots — change at your peril.

- **`StyledRow` trims trailing colorless whitespace, keeps trailing colored whitespace** — `build_styled_row` in `canvas.rs` strips spaces off the last colorless run only. If you ever need a trailing colored gap (e.g. background-fill), the current behavior keeps it. If you need a colorless trailing pad, you can't — by design.

- **`fallback_source_styled` is the only error escape hatch from the dispatcher** — `render_mermaid_styled` wraps the typed result and emits the synthetic fenced block on any `Err(_)`. The first line is `// mermaid: <reason>` in `Color::DarkGray`; rest is plain. Tests assert this format.

## Gotchas

- `Canvas::merge` only merges non-space chars from `other` into `self`. If your sub-canvas has intentional spaces meant to overwrite (rare), use `put_char(' ')` after `merge`.
- `CharsetKind::Ascii` is wired internally (per-diagram param) but not exposed as a CLI flag — terminal capability auto-detect is intentionally not done. `docs/mermaid.md §Charsets` documents this.
- `Coord::Direction::Middle` is the only "no movement" variant. `from_to(p, p)` returns it. If your routing code does `dir.opposite()` and then steps, guard against Middle.
- Per-diagram snapshots live next to their renderer (`<type>/snapshots/`). Run `cargo insta review` after any layout change — never delete a snapshot without inspecting the diff. The flowchart snapshot tree is the canary; if it changes, expect every diagram's edges to need a look.
- `mod.rs::flatten_row` (used by `render_mermaid` plain) `trim_end()`s each row. Don't rely on trailing whitespace surviving to stdout.

## How to extend

- New diagram type: create `src/mermaid/ascii/<kind>/` with `mod.rs` (`render` + `render_styled` wrappers), `ast.rs`, `parser.rs`, `render.rs` (with the `render_to_canvas` private core), and a `snapshots/` dir. Add a `DiagramKind::<Kind>` to `detect.rs`, detect it in `detect_kind`, and add the dispatch arm in `mod.rs::render_mermaid_styled`. Mirror `class/mod.rs` or `sequence/mod.rs` as the canonical templates — they're the simplest. For positional layout (flowchart-like), mirror `flowchart/` which adds a `layout.rs`.
- New shape glyph: extend `Charset` trait + `UnicodeSet` + `AsciiSet`. If it's a junction-style char, add its merge rules to `merge_junctions` and `is_junction_char`.
- Routing improvements: `astar.rs` is the only place edges get smart. If A* is too slow on dense flowcharts, profile before swapping algorithms — the heuristic is tuned for this grid.
