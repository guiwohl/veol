# mermaid/

Mermaid block extract + display cache + dispatcher into `ascii/`. This module is the API surface; `ascii/` is the engine. For the diagram type matrix and renderer architecture see [`docs/mermaid.md`](../../docs/mermaid.md).

## Files

| File | Purpose |
|---|---|
| `mod.rs` | Re-exports `render_mermaid`, `DisplayRegistry`, `extract_jobs`. Nothing else lives here. |
| `display.rs` | `DisplayRegistry` — per-document `HashMap<usize, Vec<StyledRow>>` keyed by block index. `ensure_rendered(idx, src, w)` lazily renders + memoizes. `reset()` on doc reload. |
| `extract.rs` | `extract_jobs(blocks, theme, format, version)` → `Vec<MermaidJob>` with sha256 `cache_key`. Holdover from the mmdc era. |
| `ascii/` | Pure-Rust renderer engine, 18 diagram types. See [src/mermaid/ascii/CLAUDE.md](ascii/CLAUDE.md). |

## Key design decisions

- **`DisplayRegistry` has no `Pending` / `Failed` state anymore**. Pre-pivot, mmdc was async + fallible so the registry tracked rendering lifecycle. With ASCII rendering, `ensure_rendered` is microseconds and never errors — failures fall back inside `ascii::render_mermaid_styled` to a fenced-source row block. The cache exists only to avoid re-allocating `StyledRow`s during scroll.
- **`extract.rs` is dead-ish, intentionally**. `MermaidJob.cache_key` is sha256(source ‖ theme ‖ format ‖ "mmdc" ‖ renderer_version ‖ VEOL_RENDER_VERSION). It's computed and exposed but **nothing consumes it downstream** — the field is retained per the ASCII-pivot spec (`docs/tasks/guiwohl-veol-ascii-mermaid-2026-05-27.md` §A23) for a possible future cross-document on-disk cache. Don't delete it without re-reading the spec.
- **Dispatcher lives in `ascii::mod.rs`, not here**. `mermaid::render_mermaid` is a re-export — the actual detect→parse→layout→canvas path is in `src/mermaid/ascii/mod.rs::render_mermaid_styled`. The `mermaid` namespace exists so callers don't import `mermaid::ascii::…` directly.
- **`OutputFormat` (Png/Svg) is still on the public API** even though we never produce either format anymore. Same rationale as `cache_key`: cheap to keep, breaks API consumers if removed. Tests still exercise it.

## Gotchas

- `DisplayRegistry::ensure_rendered` does NOT key on `max_width`. If you call it twice with different widths, the second call returns the cached first-width rows. Layout currently re-creates the registry on width change via `App::relayout`; if you ever cache across width changes, key the entry on `(idx, width)`.
- `extract::normalize_source` strips trailing whitespace per line AND drops trailing blank lines. Snapshot tests pin this — don't normalize leading whitespace too.
- `MermaidJob.source` is the normalized source. If you wire something that needs byte-perfect original source (re-emit fenced fallback?), pull from `Block::MermaidBlock.source`, not from a job.

## How to extend

- New diagram type → entirely inside `ascii/`. Add a module under `src/mermaid/ascii/`, add a `DiagramKind` variant in `ascii::detect`, wire it into `ascii::mod.rs::render_mermaid_styled`'s match. See [src/mermaid/ascii/CLAUDE.md](ascii/CLAUDE.md) for the per-diagram file shape.
- Cache strategy change → start in `display.rs`. `extract.rs` is a downstream consumer of `MermaidJob.cache_key`; wire a consumer there before touching the key shape (otherwise you'll invalidate everyone's cache for no reason).
