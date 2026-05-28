## ADR-0002: Pure-Rust ASCII Mermaid renderer
Drop external `mmdc` + Chrome dependency. Implement mermaid as in-process ASCII art covering 18 diagram types.

- **Status:** Accepted
- **Date:** 2026-05-27
- **Spec:** [`../tasks/guiwohl-veol-ascii-mermaid-2026-05-27.md`](../tasks/guiwohl-veol-ascii-mermaid-2026-05-27.md)

## Context

Pre-pivot, Veol rendered mermaid blocks by spawning `mmdc` (Mermaid CLI) → Puppeteer → headless Chrome → SVG → PNG → display via Kitty / iTerm2 / sixel image protocols.

Costs:

- **Install friction**: users needed Node.js, `@mermaid-js/mermaid-cli`, and `~150 MB` of Puppeteer's bundled Chromium.
- **Latency**: 800ms–3s per block. Required `DisplayState::{Pending, Ready, Failed}` lifecycle, async worker pool, timeouts.
- **Terminal coupling**: image protocols vary wildly across terminals; many users (including `tmux`) get nothing.
- **Spirit of the spec**: `.claude/CLAUDE.md` says "no browser, no network, no subprocess." Indirectly depending on Chrome violates the principle even if Veol's binary doesn't ship it.

## Decision

**Replace the entire pipeline with a pure-Rust ASCII renderer covering every mermaid diagram type.**

Reference implementation reverse-engineered from `AlexanderGrooff/mermaid-ascii` (Go, ~5300 LOC, flowchart + sequence). Algorithm distilled in §2 of the spec.

Veol's coverage extends to all 18 mermaid keywords:

```
flowchart / graph, sequenceDiagram, classDiagram, stateDiagram, erDiagram,
pie, gantt, journey, timeline, mindmap, gitGraph, quadrantChart,
requirementDiagram, sankey-beta, xychart-beta, block-beta,
architecture-beta, packet-beta
```

Each lives at `src/mermaid/ascii/<kind>/` with the shape `ast.rs` + `parser.rs` + `render.rs` (+ `layout.rs` for positional types). Shared primitives at `src/mermaid/ascii/`: `canvas.rs`, `charset.rs`, `coord.rs`, `astar.rs`, `label.rs`.

Dispatcher: `src/mermaid/ascii/mod.rs::render_mermaid_styled(source, max_width)`. Never panics, never returns `Err` to the caller — fallback emits a `// mermaid: <reason>` fenced source block.

## Implementation notes

- **`Canvas` parallel grids**: `Vec<Vec<char>>` + `Vec<Vec<Option<Color>>>`. See [`../diagrams/color-pipeline.md`](../diagrams/color-pipeline.md).
- **Junction merge**: 108-arm match table ported from Go reference. See [`../diagrams/charset-and-junctions.md`](../diagrams/charset-and-junctions.md).
- **A* edge routing**: `src/mermaid/ascii/astar.rs`. Heuristic: Manhattan + 1 if `dx != 0 && dy != 0` to bias straight runs.
- **Render pattern**: every diagram exposes `render`, `render_styled`, and shares a private `render_to_canvas`. Add new diagrams by mirroring `class/mod.rs` or `sequence/mod.rs`.

## What was deleted

`cache.rs`, `mmdc.rs`, `image.rs`, `kitty.rs`, `sixel.rs`, `iterm2.rs`, the async worker pool, the `DisplayState` lifecycle. `Cargo.toml` shed ~50 transitive deps.

## What survived

- `src/mermaid/extract.rs` — `MermaidJob { source, cache_key }`. `cache_key` is sha256-of-source; unused today, retained per spec §A23 for future cross-document on-disk caching.
- `src/mermaid/display.rs` — `DisplayRegistry`, now a simple `HashMap<usize, Vec<StyledRow>>` per document. No `Pending` state.

## Consequences

**Positive:**

- Zero external binaries. Veol installs with `cargo install --path .` and Just Works.
- Per-block latency drops from seconds to single-digit milliseconds.
- Diagrams render in any terminal (UTF-8 minimum; ASCII fallback exists in `charset.rs`).
- `--no-mermaid` becomes a niche escape hatch — the renderer is now fast enough to always run.

**Accepted trade-offs:**

- Visual fidelity is text, not raster. Shapes are approximate (rhombus → square-with-diagonal-corners, circle → square-with-rounded-corners). The labels and structure are exact.
- Gantt date math is approximated by relative bar widths.
- Dense flowcharts with many crossing edges can produce overlapping routes (A* is deterministic per source but not globally optimal).

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| Bundle `mmdc` + a minimal Chromium | ~150 MB binary; violates the "no browser" rule. |
| Use SVG-to-text libraries | None exist that handle the breadth of mermaid output. |
| Drop mermaid support entirely | Diagrams are a primary use case for technical docs Veol targets. |
| Render only flowchart + sequence | Operators want full coverage; partial support is more confusing than a fallback. |
