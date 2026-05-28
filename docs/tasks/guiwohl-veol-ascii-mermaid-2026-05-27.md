---
name: veol-ascii-mermaid
description: Replace the external mmdc/Chrome-based Mermaid pipeline with a pure-Rust ASCII renderer covering every Mermaid diagram type.
metadata:
  type: spec
  date: 2026-05-27
  author: guiwohl
---

# Veol — Pure-Rust ASCII Mermaid Renderer

## 1. Why

The current `mmdc` backend transitively depends on Puppeteer → headless Chrome. Installing `mmdc` is not enough; users still need Chrome present. This violates the spirit of L18 / L19 ("no browser, no network") — the dependency was indirect, but the friction is real.

Pivot: implement Mermaid rendering as pure Rust ASCII art, modeled after the Go project `AlexanderGrooff/mermaid-ascii` (flowchart + sequence support). Then extend to cover every diagram type Mermaid supports.

Outcomes:

- Zero external binaries. Pure stdlib + existing crates.
- Instant rendering (no async worker pool needed; remove `DisplayState::Pending`/`Failed`).
- Diagrams display inline as text spans, no image-protocol round trip.
- The whole `cache.rs` / `mmdc.rs` / `image.rs` / `kitty.rs` / `sixel.rs` / `iterm2.rs` mermaid path becomes dead code.

## 2. Reference reverse-engineering

Source: `AlexanderGrooff/mermaid-ascii` (Go, ~5300 LOC). Key algorithm — distilled:

### 2.1 Pipeline

```
mermaid source string
    │
    ▼
splitGraphLines           ── split on \n respecting [...]/quote nesting
    │
    ▼
strip comments (%%) + frontmatter + padding directives
    │
    ▼
detect first directive    ── "graph TD" | "flowchart LR" | "sequenceDiagram" | ...
    │
    ├──► flowchart path:
    │       parse nodes + edges + subgraphs into graphProperties
    │       mkGraph(graphProperties) → graph{nodes,edges}
    │       createMapping()           assign every node a (col,row) on virtual grid (each node = 3×3 cells)
    │       columnWidth/rowHeight     populated from label dimensions + padding
    │       for each edge:
    │           determineStartAndEndDir   pick which side of source/target to attach to
    │           getPath (A*)              manhattan path on free grid cells
    │           mergePath                 collapse colinear steps
    │           determineLabelLine        find widest segment for label
    │       draw():
    │           drawNode(box + label) → drawSubgraphs → drawPath/Corners/ArrowHead/BoxStart/Labels
    │           mergeJunctions on overlapping box-drawing chars
    │       drawingToString()
    │
    └──► sequence path:
            participantRegex + messageRegex
            calculateLayout (participant centers, total width)
            buildLine(top, label row, bottom-with-tee)
            for each message: lifeline + arrow line (or 3-line self-loop)
```

### 2.2 Critical primitives (Go → Rust mapping)

| Go                                | Rust                                                                                                       |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `[][]string` drawing (col-major)  | `Vec<Vec<char>>` row-major (`grid[y][x]`) — strings only needed for ANSI; we colorize via parallel grid    |
| `orderedmap.OrderedMap`           | `indexmap::IndexMap`                                                                                       |
| `container/heap` priority queue   | `std::collections::BinaryHeap` with `Reverse(cost)`                                                        |
| `regexp.MustCompile`              | `regex::Regex` (already a dep candidate, OR hand-written matchers — see §6.4)                              |
| `runewidth.StringWidth`           | `unicode-width::UnicodeWidthStr::width`                                                                    |
| `[x][y]` column-major indexing    | rewrite as `[y][x]` row-major (matches output stringification, idiomatic Rust)                             |
| ANSI color via `gookit/color`     | drop colorization in v1 (theme integration is Phase 5); render plain chars                                 |
| `genericCoord{x,y}`               | `Coord { x: usize, y: usize }`                                                                             |

### 2.3 Junction-merging table (Go `mergeJunctions`)

Verbatim transplant. Two box-drawing chars overlap → lookup pairs to produce `┼ ├ ┤ ┬ ┴` etc. Tested in Go via integration tests; we mirror that.

Charsets:

- **Unicode** (default): `─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼ ╴ ╵ ╶ ╷ ╱ ╲ ▲ ▼ ◄ ► ◤ ◥ ◣ ◢ ●`
- **ASCII fallback**: `- | + < > ^ v *` and `/ \` for diagonals

## 3. Architecture

```
src/mermaid/
├── mod.rs
├── extract.rs           # KEPT  — pulldown events → mermaid sources
├── display.rs           # SIMPLIFIED — no Pending/Failed states; instant
├── ascii/               # NEW — pure-Rust renderer
│   ├── mod.rs           # public API: render(source, width) -> Result<Vec<StyledLine>>
│   ├── canvas.rs        # Canvas { Vec<Vec<char>> } + put_char/put_str/draw_box/merge/trim
│   ├── charset.rs       # Charset trait + Unicode + ASCII; junction-merge table
│   ├── coord.rs         # Coord, Direction enums
│   ├── detect.rs        # DiagramKind detection (Flowchart, Sequence, Class, …)
│   ├── error.rs         # AsciiRenderError enum
│   ├── label.rs         # multi-line label width/height, centering
│   ├── astar.rs         # A* on grid coords
│   ├── flowchart/
│   │   ├── mod.rs
│   │   ├── ast.rs       # Flowchart, Node, Edge, Subgraph, NodeShape enum
│   │   ├── parser.rs    # line-by-line; regex or hand-coded
│   │   ├── layout.rs    # createMapping + column/row sizing + edge routing
│   │   └── render.rs    # AST + layout → Canvas
│   ├── sequence/
│   │   ├── mod.rs
│   │   ├── ast.rs       # SequenceDiagram, Participant, Message, Block (alt/opt/loop/par)
│   │   ├── parser.rs
│   │   └── render.rs
│   ├── class/
│   ├── state/
│   ├── er/
│   ├── pie/
│   ├── gantt/
│   ├── journey/
│   ├── timeline/
│   ├── mindmap/
│   ├── quadrant/
│   ├── gitgraph/
│   ├── requirement/
│   ├── sankey/
│   ├── xychart/
│   ├── block/
│   ├── architecture/
│   └── packet/
└── (removed: cache.rs, mmdc.rs, backend.rs)
```

Trait at the top:

```rust
pub trait DiagramRenderer {
    fn render(&self, source: &str, max_width: u16, charset: CharsetKind) -> Result<Vec<String>, AsciiRenderError>;
}
```

Single entry point:

```rust
pub fn render_mermaid(source: &str, max_width: u16) -> Vec<String>;
//   detects kind, dispatches; on parse error returns fallback (source-as-code-block).
```

## 4. Diagram type matrix

| Tier  | Type            | Mermaid keyword                                | Status |
| ----- | --------------- | ---------------------------------------------- | ------ |
| **1** | Flowchart       | `graph TD\|LR\|BT\|RL` / `flowchart …`         | core   |
| **1** | Sequence        | `sequenceDiagram`                              | core   |
| **1** | Class           | `classDiagram`                                 | core   |
| **1** | State           | `stateDiagram` / `stateDiagram-v2`             | core   |
| **1** | ER              | `erDiagram`                                    | core   |
| **1** | Pie             | `pie`                                          | core   |
| **2** | Gantt           | `gantt`                                        |        |
| **2** | Journey         | `journey`                                      |        |
| **2** | Timeline        | `timeline`                                     |        |
| **2** | Mindmap         | `mindmap`                                      |        |
| **2** | GitGraph        | `gitGraph`                                     |        |
| **3** | Quadrant        | `quadrantChart`                                |        |
| **3** | Requirement     | `requirementDiagram`                           |        |
| **3** | Sankey          | `sankey-beta`                                  |        |
| **3** | XYChart         | `xychart-beta`                                 |        |
| **3** | Block           | `block-beta`                                   |        |
| **3** | Architecture    | `architecture-beta`                            |        |
| **3** | Packet          | `packet-beta`                                  |        |

For tiers we have not implemented yet: render fallback (source as code block + note `[diagram type 'X' not yet supported in ASCII renderer]`).

## 5. Decisions (locked unless re-brainstormed)

| ID   | Decision                                                                                            |
| ---- | --------------------------------------------------------------------------------------------------- |
| A1   | Pure-Rust renderer; remove all `mmdc` machinery in the same change set.                             |
| A2   | Default charset Unicode; `--ascii-only` flag forces ASCII fallback (for terminals without box glyphs). |
| A3   | Width-aware: caller passes terminal width; renderer never exceeds it. Strategy: shrink padding first, then scale labels (truncate with `…`), then fall back to source. |
| A4   | No color in v1 (ANSI codes complicate junction merging). Phase 5 may add theme color via parallel attribute grid. |
| A5   | Synchronous render (microseconds, not milliseconds). Worker thread pool removed.                    |
| A6   | Parser tolerance: malformed input falls back to source-as-code-block, never panics.                 |
| A7   | Snapshot tests via `insta` for canonical outputs per diagram type.                                  |
| A8   | One module per diagram type under `src/mermaid/ascii/<type>/`.                                       |
| A9   | Subgraphs (clusters) supported in flowchart, class, and state.                                       |
| A10  | Multi-edge labels and chained nodes (`A & B --> C`) supported in flowchart.                         |
| A11  | Bidirectional arrows `<-->`, `==>`, `-.->`, dotted/thick edges, supported in flowchart.             |
| A12  | All node shapes: `[ ]`, `( )`, `(( ))`, `{ }`, `{{ }}`, `[[ ]]`, `[( )]`, `[/ \]`, `[\ /]`, `[/ /]`, `[\ \]`, `> ]`, `(/ \)` (trapezoid alt), `(())`, stadium `([])`. |
| A13  | Sequence: solid `->>`, dotted `-->>`, async `-x`, async dotted `--x`, async dashed `-)`, `--)`. Notes (left/right/over). Activations. Loops/alt/opt/par/critical/break blocks. autonumber. |
| A14  | Class: relations `<\|--`, `*--`, `o--`, `-->`, `..>`, `..\|>`, `<\|..`, generics `<T>`, visibility `+ - # ~`, abstract/static annotations. |
| A15  | State: `[*]` start/stop, transitions with labels, composite states, fork/join `<<fork>>`/`<<join>>`, choice. |
| A16  | ER: cardinality combinations (`\|o`, `\|\|`, `}o`, `}\|`, `o\|`, `o{`, `\|{`, `\|\|`). Attributes table inside entity. |
| A17  | Pie: ASCII bar fallback with percent labels (no actual circle — graceful approximation; comment in output explains it). |
| A18  | Gantt: timeline-grid with `[█████]` bars; section headers; date-axis row at top. |
| A19  | Mindmap: indent-tree with curved branch chars; depth from indentation. |
| A20  | Unsupported: render fallback (source + note); never error out. |
| A21  | Public API: `ascii::render_mermaid(source: &str, max_width: u16) -> Vec<String>`. |
| A22  | Integration: `markdown/layout.rs::emit_mermaid_block` calls `ascii::render_mermaid`, embeds each output line as a `LayoutLine` with the existing `prose_indent`. No image protocol path. |
| A23  | Drop crates: `tempfile` (mermaid-only), `which` (mermaid-only), `base64`/`image`/`icy_sixel` (image-only). Keep `sha2`/`filetime` only if other code uses them. |
| A24  | Bring in: `indexmap` (ordered iteration), `unicode-width`. `regex` only if hand-coded matchers prove painful. |
| A25  | Tests: each module has `#[cfg(test)] mod tests` with ≥10 unit tests per primitive; snapshot tests under `tests/snapshots/mermaid_<type>.rs`. |
| A26  | Naming: `Canvas` not `Drawing`; `Coord{x,y}` not `gridCoord`/`drawingCoord` (single coordinate space — `Canvas` is the final terminal-cell space, layout works in `Cell{col,row}` then converts at render). |

## 6. Task graph

```
A. Research mermaid-ascii Go            ✓ DONE
B. Write this spec                       ← here
C. Canvas + Charset                      depends B
D. Diagram kind detector                 depends B
E. Flowchart parser                      depends B
F. Sequence parser                       depends B
G. Flowchart layout (A*)                 depends E + C
H. Flowchart renderer                    depends G + C
I. Sequence renderer                     depends F + C
J. Class parser+renderer                 depends H
K. State parser+renderer                 depends H
L. ER parser+renderer                    depends C
M. Pie renderer                          depends C
N. Wire into Veol pipeline               depends H + I
O. Polish + fallbacks                    depends N
P. Tier-2 diagrams (gantt, journey, mindmap, timeline, gitgraph) depends O
Q. Tier-3 diagrams (remaining)           depends P
R. Final gates                           depends Q
```

Wave plan (parallelism):

- **Wave 1** (parallel): C, D, E, F
- **Wave 2** (parallel): G, I (G unblocks H; I unblocks N)
- **Wave 3**: H
- **Wave 4** (parallel): J, K, L, M, N (after H+I land — N depends on both)
- **Wave 5**: O
- **Wave 6** (parallel): all of P
- **Wave 7** (parallel): all of Q
- **Wave 8**: R

## 7. TDD strategy

Each module is written test-first:

1. **Canvas**: write `canvas_put_char_in_bounds`, `canvas_put_str_with_width`, `draw_box_unicode`, `draw_box_ascii`, `merge_junction_horiz_vert`, `merge_drops_space_overlays`, `trim_trailing_whitespace`. Then implement until all pass.
2. **Detect**: one test per supported keyword + edge cases (frontmatter, comments, leading blank lines, `%%{init: ...}` directive).
3. **Flowchart parser**: one test per node shape, edge style, chained nodes, subgraph, classDef, comments, multiline.
4. **Flowchart layout**: tests for root placement, child placement, sibling placement, collision avoidance, column-width propagation, path-finding straight/L-shaped/around-obstacle.
5. **Flowchart renderer**: ≥10 insta snapshots covering linear chain, branch, diamond decision, subgraph cluster, all shapes, bidirectional.
6. **Sequence renderer**: insta snapshots for 2-participant exchange, self-message, autonumber, dotted, loop/alt blocks.
7. **Each Tier-1+ type**: ≥1 parse-success test, ≥1 parse-failure-falls-back test, ≥3 render snapshots.

Snapshots are committed; `cargo insta review` between iterations.

## 8. Integration (replacing mmdc)

Changes after Wave 3 + I land:

1. `src/mermaid/mod.rs`: re-export only `ascii::render_mermaid`, `extract::extract_jobs`, and remove old re-exports.
2. `src/mermaid/display.rs`: simplify — replace `DisplayState::{Pending, Rendered, Failed}` with a single function `render_inline(source) -> Vec<String>`. `DisplayRegistry` becomes a memoization cache `HashMap<sha256, Vec<String>>`.
3. `src/markdown/layout.rs`: where it emits a mermaid placeholder line, instead call `ascii::render_mermaid(source, prose_width)` and emit one `LayoutLine` per output row, with `indent_cols = prose_indent`.
4. `src/tui/draw.rs`: no special mermaid path. The lines are already there as styled spans.
5. `src/app.rs`: remove `mark_all_mermaid_failed_if_no_mmdc`. Remove worker-thread join. `from_cli` no longer needs `mermaid_enabled`/`mmdc_path`.
6. `src/cli.rs`: remove `--no-mermaid`, `--clear-cache`, `--image-protocol`, `--mermaid-timeout` flags (or repurpose `--no-mermaid` to "render as code block instead").
7. `src/main.rs`: remove image-protocol initialization and mermaid worker spawn.
8. Delete: `src/mermaid/cache.rs`, `src/mermaid/mmdc.rs`, `src/mermaid/backend.rs`, `src/terminal/` (entire dir).
9. `Cargo.toml`: remove `tempfile`, `which`, `base64`, `image`, `icy_sixel`, `sha2`, `filetime` unless used elsewhere.
10. Update `.claude/CLAUDE.md` P0 list: `mmdc` no longer applies, "no browser, no network" still applies (and is now actually true).

## 9. Removal plan

After integration:

- Delete: `src/mermaid/{cache,mmdc,backend}.rs`, `src/terminal/{detect,image,kitty,sixel,iterm2,blocks}.rs`.
- Delete tests targeting deleted modules.
- Update `examples/kitchen-sink.md` to include every diagram type as smoke-tests for the new renderer.

## 10. Test inventory target

| Module                  | Min tests |
| ----------------------- | --------- |
| canvas.rs               | 25        |
| charset.rs              | 10        |
| detect.rs               | 30        |
| label.rs                | 15        |
| astar.rs                | 10        |
| flowchart/parser.rs     | 50        |
| flowchart/layout.rs     | 25        |
| flowchart/render.rs     | 30 snapshots |
| sequence/parser.rs      | 20        |
| sequence/render.rs      | 15 snapshots |
| class                   | 20 + 10 snapshots |
| state                   | 20 + 10 snapshots |
| er                      | 15 + 8 snapshots  |
| pie                     | 10 + 5 snapshots  |
| (each tier-2)           | 10 + 5 snapshots  |
| (each tier-3)           | 8 + 3 snapshots   |
| Integration             | 15 end-to-end markdown → rendered     |

Target: ≥500 new tests covering the ASCII renderer alone. Existing 359 tests must still pass post-integration.

## 11. Definition of done

- All Tier-1 + Tier-2 diagram types render correctly for every example in `examples/kitchen-sink.md`.
- All Tier-3 either render correctly or fall back gracefully (source-as-code-block + note).
- `cargo test --all` green.
- `cargo clippy --all-targets -- -D warnings` clean.
- `cargo fmt --check` clean.
- `mmdc` references removed from code, Cargo.toml, docs, README.
- Veol opens a markdown with mixed diagrams in <100ms (no spinner, no async).
- README and `.claude/CLAUDE.md` updated.
