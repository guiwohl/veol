# Mermaid in Veol

Veol renders Mermaid diagrams inline as ASCII / Unicode text — no browser, no external `mmdc`, no image protocols, no async. Pure Rust, in-process, instant.

## Supported diagram types

| Mermaid keyword       | Status   | Notes                                                              |
|-----------------------|----------|--------------------------------------------------------------------|
| `flowchart` / `graph` | full     | TD / TB / LR / BT / RL, subgraphs, A* edge routing                 |
| `sequenceDiagram`     | full     | participants, sync / async / dotted arrows, alt / opt / loop / par |
| `classDiagram`        | full     | members, visibility, relationships                                 |
| `stateDiagram` / `stateDiagram-v2` | full | `[*]` start/end, composite states, transitions                |
| `erDiagram`           | full     | cardinalities, attributes                                          |
| `pie`                 | full     | slices + legend                                                    |
| `gantt`               | partial  | tasks and sections; durations approximated                         |
| `journey`             | full     | actors and scores                                                  |
| `timeline`            | full     | events along a single axis                                         |
| `mindmap`             | full     | hierarchical tree                                                  |
| `gitGraph`            | full     | commits, branches, merges                                          |
| `quadrantChart`       | full     | 4-quadrant scatter                                                 |
| `requirementDiagram`  | full     | requirements + verifications                                       |
| `sankey-beta`         | full     | flows between named nodes                                          |
| `xychart-beta`        | full     | bar / line chart in text                                           |
| `block-beta`          | full     | grid of labeled blocks                                             |
| `architecture-beta`   | full     | groups, services, edges                                            |
| `packet-beta`         | full     | byte-range packet diagram                                          |
| anything else         | fallback | rendered as a fenced `mermaid` code block                          |

"Full" means every Mermaid keyword on the syntax cheat sheet for that diagram type is parsed and renders without panicking. "Partial" means common cases work but exotic syntax may degrade to the fallback. "Fallback" means the source prints verbatim inside a fenced block prefixed with `// mermaid: <reason>`.

## How it works

```
source string
   │
   ▼
detect_kind         identifies diagram type from first non-comment line
   │
   ▼
parse               line-by-line into a typed AST
   │
   ▼
layout              assign grid coordinates + sizes
   │
   ▼
canvas              draw onto a Vec<Vec<char>> grid; merge box-drawing junctions
   │
   ▼
Vec<String>         pre-wrapped rows ready to paint into the TUI
```

Every diagram type lives under `src/mermaid/ascii/<type>/` with the same shape (`ast.rs`, `parser.rs`, `render.rs`, plus `layout.rs` where positional layout is involved). Shared utilities — `canvas.rs`, `charset.rs`, `coord.rs`, `astar.rs`, `label.rs` — sit at the top level of `src/mermaid/ascii/`.

The dispatcher entry point:

```rust
pub fn render_mermaid(source: &str, max_width: u16) -> Vec<String>
```

It never panics, never returns an error to the caller. On any parse failure, unsupported kind, or empty input, it produces the fenced-source fallback so output is preserved.

## Disabling mermaid

Pass `--no-mermaid` to render every mermaid fence as a plain code block — useful when piping output or when the diagram source matters more than the rendered shape.

```bash
veol --no-mermaid README.md
```

Inside the TUI, the `m` key toggles all diagrams between rendered and source view for the current document.

## Charsets

Two charsets are implemented:

- **Unicode** (default) — `─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼ ╱ ╲ ▲ ▼ ◄ ► ╭ ╮ ╰ ╯`
- **ASCII fallback** — `- | + < > ^ v / \`

The ASCII set exists in `src/mermaid/ascii/charset.rs` (`CharsetKind::Ascii`) and is selected by every renderer module via parameter. It is not yet exposed as a CLI flag — terminal capability auto-detection is intentionally not wired (Veol targets modern UTF-8 terminals). Open an issue if you need a flag for it.

## Limitations (honest)

- Shape approximations: a Mermaid rhombus (`{decision}`) renders as a square with diagonal corner glyphs; a circle (`((node))`) renders as a square with rounded corners. The labels and connectivity are exact — the silhouette is approximate.
- No colors. The renderer outputs plain characters; the TUI may apply theme color to the whole block via `mermaid_caption`, but per-shape coloring is out of scope.
- Layout heuristics: dense flowcharts with many crossing edges can produce overlapping routes. A* routes are deterministic per source but not globally optimal.
- Gantt date math is approximate — bar widths are proportional to the longest task, not anchored to real calendar offsets.
- The `--width` flag is honored as `max_width`, but very narrow widths (< ~30 cols) will trigger the fallback for complex diagrams.

## Reporting bugs

If a diagram renders incorrectly, falls back unexpectedly, or panics:

1. Minimal repro: drop the failing mermaid source into a single `.md` and run `veol --plain that.md > out.txt`.
2. Compare against the official Mermaid preview at https://mermaid.live to confirm the source is valid.
3. Open an issue including the source, the actual output, and what you expected.

Source of truth for the renderer pivot lives at `docs/tasks/guiwohl-veol-ascii-mermaid-2026-05-27.md`.
