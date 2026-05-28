## Adding a Mermaid Diagram Type
Step-by-step: from new module to dispatcher hook to snapshots.

> **Related**
> - [`../diagrams/architecture-pivot.md`](../diagrams/architecture-pivot.md) — overall renderer architecture
> - [`../diagrams/charset-and-junctions.md`](../diagrams/charset-and-junctions.md) — shared glyph primitives
> - [`../mermaid.md`](../mermaid.md) — user-facing matrix to update
> - [`../../src/mermaid/ascii/CLAUDE.md`](../../src/mermaid/ascii/CLAUDE.md) — module conventions

Each of the 18 existing diagram types follows the same file shape. New ones follow the same template. ~1 hour of focused work for a simple type (pie, packet); a day or more for positional types (flowchart, sequence).

## 1. Create the module skeleton

```bash
mkdir -p src/mermaid/ascii/<kind>/snapshots
touch src/mermaid/ascii/<kind>/{mod.rs,ast.rs,parser.rs,render.rs}
# Add layout.rs if positional (flowchart-like).
```

Mirror `src/mermaid/ascii/class/` or `src/mermaid/ascii/sequence/` as the canonical template — they're the cleanest. For positional types, mirror `flowchart/`.

## 2. Register the kind

Edit `src/mermaid/ascii/detect.rs`:

```rust
pub enum DiagramKind {
    // ...existing variants...
    YourKind,
    Unknown,
}
```

In `classify(line: &str)`:

```rust
if has_keyword(&lower, "yourdiagram") {
    return DiagramKind::YourKind;
}
```

`has_keyword` lowercases + strips. Add tests for case-insensitive detection.

## 3. Define the AST

`<kind>/ast.rs`:

```rust
#[derive(Debug, Default)]
pub struct YourAst {
    pub nodes: Vec<YourNode>,
    pub edges: Vec<YourEdge>,
}
```

Keep it minimal — only fields the layout / render passes need.

## 4. Write the parser

`<kind>/parser.rs`:

```rust
pub fn parse(source: &str) -> Result<YourAst, AsciiRenderError> { ... }
```

Hand-rolled line scanner. Strip comments (`%%`) and frontmatter. Return `AsciiRenderError::Parse(msg)` on any malformed input — the dispatcher falls back to source-block rendering.

Test every keyword. The fallback is fine but every parser bug should surface in a test.

## 5. (If positional) Write layout

`<kind>/layout.rs`:

```rust
pub fn layout(ast: &YourAst, max_width: u16) -> Result<LaidOut, AsciiRenderError> { ... }
```

Produces a coordinate-mapped structure ready for canvas painting. Use `Coord`, `Direction` from `ascii::coord`. For grid-based diagrams, see `flowchart/layout.rs`. For free-form, see `mindmap/layout.rs`.

## 6. Write the renderer

`<kind>/render.rs`:

```rust
fn render_to_canvas(...) -> Result<Canvas, AsciiRenderError> { ... }

pub fn render(...) -> Result<Vec<String>, AsciiRenderError> {
    Ok(render_to_canvas(...)?.into_lines())
}

pub fn render_styled(...) -> Result<Vec<StyledRow>, AsciiRenderError> {
    Ok(render_to_canvas(...)?.into_styled_lines())
}
```

This `render → render_to_canvas → render_styled` triplet is **mandatory** — `ascii::mod.rs::render_mermaid_styled` calls `render_styled` directly; the plain `render` exists for tests and `--plain` output.

Canvas primitives you'll use:

- `Canvas::new(width, height, charset_kind)`
- `canvas.draw_box(x1, y1, x2, y2)`
- `canvas.draw_hline(x1, x2, y)` / `draw_vline(x, y1, y2)`
- `canvas.put_str(x, y, text)`
- `canvas.set_color(x, y, color)` / `paint_text(x, y, text, color)` / `paint_box(x1, y1, x2, y2, color)`

`Canvas::merge(other_canvas)` is the only way junction glyphs combine correctly — use it when assembling sub-shapes.

## 7. Wire the `mod.rs`

`<kind>/mod.rs`:

```rust
pub mod ast;
pub mod parser;
pub mod render;
pub mod layout;        // if positional

pub use render::{render, render_styled};
```

## 8. Hook the dispatcher

`src/mermaid/ascii/mod.rs::render_mermaid_styled` — add the match arm:

```rust
DiagramKind::YourKind => yourkind::render_styled(source, max_width, charset),
```

And the corresponding `pub mod yourkind;` at the top.

## 9. Color palette

If your diagram uses colors, define a module-level `const PALETTE: &[Color] = &[...];` and cycle through it. Don't hardcode in render fns — extract.

Today, palettes are static. Theme integration is in [`../diagrams/color-pipeline.md`](../diagrams/color-pipeline.md)'s roadmap.

## 10. Snapshot tests

`<kind>/render.rs` test module:

```rust
#[test]
fn render_minimal_example() {
    let source = "yourdiagram\n  ...";
    let rows = render(source, 80, CharsetKind::Unicode).unwrap();
    insta::assert_snapshot!("render_minimal_example", rows.join("\n"));
}
```

Cover:

- Smallest valid input (single node / single bar).
- Each parser keyword you support.
- Width-limited rendering (`max_width = 30` or so) — confirms wrap / shrink behavior.
- ASCII charset variant.

Run `cargo test --all`, then `cargo insta review`, then accept.

## 11. Update user-facing docs

Edit `docs/mermaid.md` — add your `Mermaid keyword | Status | Notes` row to the supported types table.

Optional: add an example to `examples/kitchen-sink.md` so the README demo shows it.

## 12. The three gates, again

```bash
cargo test --all
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

All green. Commit per [`../contributing.md`](../contributing.md) conventions:

```
feat(mermaid): add ASCII renderer for <YourKind>

Covers <list of mermaid keywords supported>. Falls back to source
block on <list of edge cases not covered>.
```

PR per [`release-checklist.md`](release-checklist.md).
