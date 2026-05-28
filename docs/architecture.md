# Architecture

High-level system overview of Veol — a TUI Markdown reader. Single binary, pure Rust, no async runtime, no subprocess, no network.

See [`README.md`](../README.md) for user-facing intro, [`docs/mermaid.md`](mermaid.md) for the Mermaid renderer matrix, [`docs/keybindings.md`](keybindings.md) for the key table, [`docs/theming.md`](theming.md) for color fields.

## Top-level flow

```
                  ┌────────────────────────────────────────────────────┐
                  │                main.rs / run_tui                   │
                  │  raw mode + alternate screen + mouse capture       │
                  │  (mouse on by default, opt out with --no-mouse)    │
                  └───────────────────────┬────────────────────────────┘
                                          │
                  read file or stdin → app::App::from_cli
                                          │
                                          ▼
┌─────────────┐   parse        ┌─────────────────┐   layout        ┌──────────────────┐
│  .md bytes  ├───────────────►│ Vec<Block>      ├────────────────►│ Vec<LayoutLine>  │
└─────────────┘  pulldown      │ (markdown::model)│  width-aware    │  (StyledSpan…)   │
                  +cmark events└─────────────────┘  +theme          └──────┬───────────┘
                                          ▲                                │
                                          │                                ▼
                                  every mermaid fence            ┌──────────────────┐
                                  becomes Block::MermaidBlock    │  DocumentWidget  │
                                                                 │  paints viewport │
                                                                 │  (tui::draw)     │
                                                                 └──────────────────┘
```

## Modules

| Module | Responsibility | CLAUDE.md |
|---|---|---|
| `src/markdown/` | pulldown-cmark → `Block` AST → wrapped `LayoutLine`s | [src/markdown/CLAUDE.md](../src/markdown/CLAUDE.md) |
| `src/mermaid/` | Mermaid block extract + display cache + ASCII dispatcher | [src/mermaid/CLAUDE.md](../src/mermaid/CLAUDE.md) |
| `src/mermaid/ascii/` | Pure-Rust diagram renderers (18 types) | [src/mermaid/ascii/CLAUDE.md](../src/mermaid/ascii/CLAUDE.md) |
| `src/render/` | 9 bundled themes, syntect code highlighting, color fields | [src/render/CLAUDE.md](../src/render/CLAUDE.md) |
| `src/tui/` | DocumentWidget paint loop, viewport, search, TOC, browser, theme switcher, help | [src/tui/CLAUDE.md](../src/tui/CLAUDE.md) |
| `src/app.rs` | `App` state + `Popup` enum dispatch + action handler | — |
| `src/main.rs` | entry, event loop, terminal setup, popup rendering | — |
| `src/watcher.rs` | 500ms mtime polling via `filetime`, channel-based reload signal | — |
| `src/config.rs` | `~/.config/veol/config.toml`, atomic save via `tempfile` | — |
| `src/cli.rs` | `clap` derive — flag set (see [keybindings.md](keybindings.md)) | — |

## Data flow for Mermaid

```
markdown::parser
   │  recognizes ```mermaid / ```mmd / ```mermaid-js fences
   ▼
Block::MermaidBlock { source }
   │
   ▼  markdown::layout::render_mermaid (called per block at layout time)
ascii::render_mermaid_styled(source, max_width)
   │  detect::detect_kind → DiagramKind
   │  dispatch to flowchart / sequence / class / … (18 modules)
   │  on parse error or Unknown → fenced-source fallback
   ▼
Vec<StyledRow>   (StyledRow = Vec<StyledRun{ text, color: Option<Color> }>)
   │
   ▼  one StyledRow per LayoutLine
LayoutLine { spans, mermaid_block_index: Some(idx), indent_cols }
```

`extract.rs` produces `Vec<MermaidJob>` with `cache_key` (sha256 of source + theme + format + version). The key is computed but **no longer consumed downstream** since rendering is now instant in-process — the field is kept per the ASCII-pivot spec (`docs/tasks/guiwohl-veol-ascii-mermaid-2026-05-27.md` §A23) in case cross-document caching is wired later.

`display::DisplayRegistry` is a per-document `HashMap<usize, Vec<StyledRow>>` keyed by block index. `ensure_rendered(idx, src, w)` lazily renders and memoizes. `reset()` is called on reload. With the ASCII pivot, render is microseconds; the cache is more about not re-allocating rows during scroll than dodging mmdc latency.

## Threading model

`std::thread` + `std::sync::mpsc` only. No tokio, no async, no executor.

- Main loop runs synchronously in `main.rs::event_loop`. Polls crossterm events with a 100ms timeout.
- `watcher.rs` owns a worker thread that polls `filetime::FileTime` every 500ms. On mtime change it sends a `WatcherEvent::Changed` over an `mpsc::Sender`. `App::poll_watcher` drains the receiver at the top of every event loop iteration.
- Mermaid rendering is **synchronous** on the main thread — finishes in microseconds because it's pure ASCII drawing. There is no worker pool, no `Pending` state, no `DisplayState::Failed` (those existed when `mmdc` was the backend; both gone since the ASCII pivot).

## Paint loop (per frame)

1. `app.poll_watcher()` — drain reload events, re-parse if needed.
2. `app.relayout(width)` — re-run `layout::layout(&blocks, width, &theme)` if cached width changed.
3. `terminal.draw(|frame| draw_frame(frame, app))`:
   - `doc_and_status_areas` splits screen → doc, optional search bar, status.
   - `render_lines_with_search` clones lines and applies highlight spans where matches sit.
   - `DocumentWidget` paints the visible window (top_line .. top_line + height).
   - `StatusBarWidget` paints `filename  N%  line cur/total  flash`.
   - If `app.popup != Popup::None`, paints centered modal over the doc.
4. `event::poll(100ms)` → key / mouse / resize handlers.

## Pagination model

`ViewportState { top_line, height, total_lines, cursor_line }`. There is no cursor in the reader (cursor_line is unused — kept for future). Scroll == shift `top_line`. `next_paragraph` / `prev_paragraph` walk over blank lines (lines whose all spans are empty/whitespace) to find block boundaries. `next_heading` / `prev_heading` use the `heading_anchor: Option<String>` flag on `LayoutLine`.

## Why no async

The locked spec L23 (and re-affirmed in the ASCII-pivot spec A5) bans tokio. Justification: the only blocking work used to be `mmdc` spawn; with ASCII rendering, everything is sub-millisecond. Watcher is a single 500ms poll thread. Cost of tokio would be ~50 deps + a binary larger than the entire renderer.

## Why no subprocess

L11 and the ASCII-pivot spec §1: the original `mmdc` backend transitively dragged in headless Chrome via Puppeteer. The pivot collapses Mermaid into pure-Rust ASCII. Result: zero external binaries, zero startup probe, no `which` lookup, no PATH dependency.

## What's NOT here (rejected scope)

- No editor. CRUD inside the file browser exists only to manage `.md` files between reads (L10).
- No image protocol (Kitty / iTerm2 / Sixel) — removed with mmdc (`docs/tasks/guiwohl-veol-ascii-mermaid-2026-05-27.md` §A1).
- No git status in browser (L13).
- No multi-buffer / tabs (L12).
- No math / KaTeX rendering (deferred, §5.6).
