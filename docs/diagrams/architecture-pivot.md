## Architecture Pivot — mmdc → ASCII
The dependency on headless Chrome was the largest hidden cost in Veol. On 2026-05-27 we replaced the whole external pipeline with a pure-Rust ASCII renderer.

> **Related**
> - [`../decisions/0002-pure-rust-mermaid.md`](../decisions/0002-pure-rust-mermaid.md) — the ADR
> - [`../mermaid.md`](../mermaid.md) — user-facing reference, 18 diagram types
> - [`../tasks/guiwohl-veol-ascii-mermaid-2026-05-27.md`](../tasks/guiwohl-veol-ascii-mermaid-2026-05-27.md) — locked spec for the pivot
> - [`charset-and-junctions.md`](charset-and-junctions.md), [`color-pipeline.md`](color-pipeline.md)

## Before

```
mermaid source → spawn mmdc (Node)
                     │
                     └── puppeteer → headless Chrome → SVG → PNG → image protocol → terminal
```

Per-block latency: 800ms–3s. Required:

- Node.js installed.
- `@mermaid-js/mermaid-cli` (`mmdc`) installed.
- Puppeteer's bundled Chromium downloaded (~150 MB).
- A terminal speaking Kitty / iTerm2 / sixel image protocols.
- `cache.rs`, `mmdc.rs`, `image.rs`, `kitty.rs`, `sixel.rs`, `iterm2.rs` modules to glue it all together.

Async worker pool, `DisplayState::{Pending, Ready, Failed}`, timeouts. ~1500 LoC of orchestration.

## After

```
mermaid source → ascii::render_mermaid_styled(source, max_width) → Vec<StyledRow> → painted inline
```

Per-block latency: low milliseconds (single-digit ms for typical diagrams). Required:

- Nothing. Pure-Rust, in-process.

Dispatcher at `src/mermaid/ascii/mod.rs:36`:

```rust
pub fn render_mermaid_styled(source: &str, max_width: u16) -> Vec<StyledRow> {
    let kind = detect::detect_kind(source);
    match kind {
        DiagramKind::Flowchart(_)  => flowchart::render_styled(source, max_width, charset),
        DiagramKind::Sequence       => sequence::render_styled(source, max_width, charset),
        DiagramKind::Class          => class::render_styled(source, max_width, charset),
        // ... 18 variants ...
        DiagramKind::Unknown        => Err(AsciiRenderError::Unsupported(format!("{kind:?}"))),
    }
}
```

## What survived

Two things from the old pipeline still exist:

| File | Reason kept |
|---|---|
| `src/mermaid/extract.rs` | The `MermaidJob { source, cache_key }` shape. `cache_key` is a SHA-256 of the source — unused today but wired through so future cross-document caching is a drop-in. |
| `src/mermaid/display.rs` | `DisplayRegistry` cache, but now keyed by per-document `block_index` and storing `Vec<StyledRow>` directly. No `Pending` state. |

## What was deleted

| File | Why gone |
|---|---|
| `cache.rs` | Persistent SVG/PNG cache — no images now. |
| `mmdc.rs` | Subprocess driver — no subprocess now. |
| `image.rs` | Image protocol detector — no images. |
| `kitty.rs`, `sixel.rs`, `iterm2.rs` | Per-protocol encoders. |
| Async worker pool | Synchronous now. |

`Cargo.toml` shed ~50 transitive deps in the cleanup.

## Fallback contract

The dispatcher never panics, never returns `Result` to the caller. On parse failure or `DiagramKind::Unknown`, it returns:

```
// mermaid: <reason>
```mermaid
<original source verbatim>
```
```

Operators never lose content — even an unsupported diagram displays as readable source they can copy into mermaid.live.

## Date and rationale

Pivot date: **2026-05-27**. Spec: [`../tasks/guiwohl-veol-ascii-mermaid-2026-05-27.md`](../tasks/guiwohl-veol-ascii-mermaid-2026-05-27.md). Reference implementation reverse-engineered from `AlexanderGrooff/mermaid-ascii` (Go, ~5300 LOC, flowchart + sequence). Veol then extended the algorithm to the full 18-type Mermaid surface.

The Go project covered flowchart + sequence + the junction-merge primitive. Class, state, ER, pie, gantt, journey, timeline, mindmap, gitGraph, quadrant, requirement, sankey, xychart, block, architecture, packet were all designed and implemented for Veol from scratch following the same module shape.
