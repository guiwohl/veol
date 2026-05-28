## Color Pipeline
How the ASCII renderer carries color from per-diagram render code into ratatui styled spans.

> **Related**
> - [`architecture-pivot.md`](architecture-pivot.md) — context for why this exists
> - [`charset-and-junctions.md`](charset-and-junctions.md) — sibling primitive
> - [`../theming.md`](../theming.md) — `mermaid_caption` and friends

Implementation: `src/mermaid/ascii/canvas.rs`.

## The two-grid model

`Canvas` keeps two parallel matrices, both `width × height`:

```rust
pub struct Canvas {
    rows:   Vec<Vec<char>>,
    colors: Vec<Vec<Option<Color>>>,
    charset_kind: CharsetKind,
}
```

- `rows[y][x]` is the glyph.
- `colors[y][x]` is `Some(Color)` if a render path painted it, else `None`.

The split is intentional: glyph writes (`put_char`, `draw_box`, `draw_hline`) stay color-free, and coloring is an explicit second step (`set_color`, `paint_text`, `put_str_colored`, `paint_box`).

## Why parallel grids

The naive alternative — one `Vec<Vec<(char, Color)>>` — couples drawing and coloring. Every box draw would need a color argument. The parallel-grid design lets:

- Layout code draw shapes uncolored and a render pass color them at the end.
- Junction merging operate on chars without ever consulting colors.
- Painters apply color to a region by coordinate rectangle (`paint_box(x1, y1, x2, y2, c)`).

## StyledRow / StyledRun

The output unit:

```rust
pub struct StyledRun {
    pub text: String,
    pub color: Option<Color>,
}
pub type StyledRow = Vec<StyledRun>;
```

`Canvas::into_styled_lines` walks each row left-to-right, grouping consecutive cells that share the same `Option<Color>` into one `StyledRun`. Trailing colorless whitespace is trimmed; trailing colored whitespace is preserved (useful for background-fill effects, never required today).

The `render_mermaid_styled` dispatcher returns `Vec<StyledRow>` straight from this.

## Integration with the markdown layout

`src/markdown/layout.rs::render_mermaid`:

```rust
let rows = ascii::render_mermaid_styled(source, ctx.width as u16);
for row in rows {
    let spans: Vec<StyledSpan> = if row.is_empty() {
        vec![empty_styled_span(default_fg)]
    } else {
        row.into_iter().map(|run| StyledSpan {
            text: run.text,
            fg:   run.color.unwrap_or(default_fg),
            bg:   None,
            modifier: Modifier::empty(),
            link:  None,
        }).collect()
    };
    ctx.push_line(spans);
}
```

- `default_fg = theme.colors.mermaid_caption.to_ratatui()` — uncolored cells inherit the document's mermaid caption color, so a single theme variable governs the overall hue of a diagram.
- Colored cells emit their explicit `Color` directly into the ratatui span.

## Color sourcing per diagram

Each diagram's `render.rs` chooses what to color and how:

| Diagram | Coloring strategy |
|---|---|
| `flowchart` | Edges use one color, node boxes another, labels default. |
| `sequence` | Participant headers tinted; messages remain default. |
| `pie` | Each slice has its own color cycled from a fixed palette. |
| `gantt` | Bars tinted by section; labels default. |
| `class` / `state` / `er` | Box outlines tinted; labels default. |

These palettes are hardcoded per-diagram today. A theme-driven palette (per `Theme.colors.mermaid_*` fields) is the obvious next iteration but is **not yet wired** — see the roadmap below.

## Roadmap — theme integration

Today, the dispatcher knows nothing about the active `Theme`. Per-diagram palettes are static module constants. To integrate themes:

1. Thread `&Theme` through `render_mermaid_styled` (or expose a `MermaidPalette` struct built from `Theme.colors.mermaid_*` fields).
2. Add fields to `ThemeColors`: `mermaid_edge`, `mermaid_node_outline`, `mermaid_pie_palette: [Color; 8]`, etc.
3. Replace per-diagram constants with palette lookups.

The current `mermaid_caption` field is the only theme-controlled mermaid color. Operators who want themed diagrams should file an issue with the desired field set.

## What is NOT colored

- Junctions and corner glyphs are not specifically colored — they inherit whatever `set_color` was called on them. Most diagrams leave them colorless.
- ASCII charset (`+`, `-`, `|`) follows the same rules. ASCII doesn't reduce color granularity.

## Trimming rule

`build_styled_row` in `canvas.rs` trims trailing whitespace **from the final colorless run only**. The rationale: a colored trailing space is intentional (e.g., to extend a background band), but a colorless trailing space is layout padding that ratatui doesn't need to paint.
