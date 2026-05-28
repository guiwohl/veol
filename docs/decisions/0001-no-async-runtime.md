## ADR-0001: No async runtime
Veol uses `std::thread` + `mpsc::channel` exclusively. No tokio, no async-std, no smol.

- **Status:** Accepted
- **Date:** 2026-05-26

## Context

Veol is a TUI Markdown reader. Its concurrency surface is tiny:

- Read a file once at startup. Re-read on watcher signal.
- Render to ratatui on the main thread, ~10 Hz.
- Poll a file mtime every 500ms.

That's it. There's no network I/O, no concurrent request fan-out, no streaming, no socket multiplexing. The watcher loop is the only background work, and even that is a synchronous `std::fs::metadata().modified()` call.

The default Rust TUI starter template often reaches for `tokio` reflexively. It costs:

- **30–50 transitive crate dependencies** (`tokio` core + `mio` + ecosystem plumbing).
- **A runtime executor** in the binary, even when nothing is `async`.
- **A different programming model** (`async fn`, `Pin`, `Send + 'static` lifetime gymnastics) for code that should be a 5-line `std::thread::spawn`.

The original mermaid-via-mmdc path also threatened to need async — subprocess spawn + child-process IO + timeout pooling — but the [ASCII-mermaid pivot](0002-pure-rust-mermaid.md) eliminated subprocesses entirely.

## Decision

**Pure stdlib threading + mpsc channels.** No async runtime.

- File watching: synchronous `Watcher::poll()` called once per frame from the main loop (`src/main.rs::event_loop`).
- Mermaid rendering: synchronous, in-process, completes in microseconds-to-milliseconds (`src/mermaid/ascii/`).
- Config save: synchronous, atomic via `tempfile::persist`.
- Logging: `tracing_appender::non_blocking` uses a background thread internally — that's a dep, not our concurrency model.

Anywhere a thread is needed in the future (e.g., concurrent diagram pre-render across multiple cores), the pattern is `std::thread::spawn` + `std::sync::mpsc::channel`.

## Hard rule

From `.claude/CLAUDE.md` (NEVER list):

> Add `tokio` or any async runtime. Veol uses `std::thread` + `mpsc::channel` where threading is needed. Mermaid rendering is synchronous and instant.

## Consequences

**Positive:**

- Faster compile times. Smaller binary. Fewer deps to audit.
- No "what color is your function" coloring — everything is sync, callable from anywhere.
- Stack traces are readable; no `poll` indirection.
- New contributors can read the entire control flow top-to-bottom.

**Accepted trade-offs:**

- Watcher polling is 500ms instead of event-driven (inotify/FSEvents). For one watched file, polling cost is invisible — see [`../configuration/watch-mode.md`](../configuration/watch-mode.md).
- If we ever need concurrent network I/O (we don't — Veol has no network), this rule revisits.

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| `tokio` | Massive dep tree for a single watcher + render loop. |
| `async-std` | Same problem, smaller ecosystem. |
| `smol` | Lighter, but still imports an executor for code that doesn't need one. |
| `notify` crate (event-driven file watching) | Pulls in `mio` and cross-platform FS-event plumbing for a single file. mtime polling is cheaper and has fewer edge cases. |
