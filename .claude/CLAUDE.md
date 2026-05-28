# Veol — AI Instructions

## Core Beliefs

### P0 — Non-negotiable

- ***Planning First*** — When designing a plan, entering Plan Mode, or ideating any feature/architecture — use `/brainstorming` skill ***ALWAYS*** before jumping into implementation. The locked spec lives at `docs/tasks/guiwohl-veol-spec-2026-05-26.md`; the ASCII-mermaid pivot lives at `docs/tasks/guiwohl-veol-ascii-mermaid-2026-05-27.md`. Treat both as source of truth.
- ***TDD always*** — When implementing any feature or bugfix, ALWAYS use `/tdd` before writing implementation code. RED → GREEN → REFACTOR. Veol ships with a failing test that proves the new behavior on every PR. No exceptions, no "I'll add the test after".
- ***Systematic Debugging*** — When debugging any bug, test failure, or unexpected behavior, ALWAYS use `/systematic-debugging`.
- ***Veol is a READER, not an editor*** — Never add buffer-editing features. CRUD inside the file browser exists only to manage `.md` files between reads (navigation convenience). The buffer is immutable from the user's perspective.
- ***No browser, no network, no subprocess*** — No embedded browser engine. No outbound HTTP. Remote images are not fetched. Mermaid is rendered in-process by `src/mermaid/ascii/`. Veol does not spawn any external binary at runtime.
- ***Evidence Before Completion*** — Never claim work is "done", "fixed", "passing", or "ready" without having just executed `cargo test --all` and `cargo clippy --all-targets -- -D warnings` and read their output. No "should pass", no trusting prior runs, no trusting agent reports.
- ***No Bloat, No Duplication, No Over-engineering*** — Ship the minimum code that solves the asked task. NOTHING extra. Before writing a new function/file/abstraction, grep for an existing one and reuse it. Forbidden without an explicit user ask: parallel implementations of something that exists, "flexible"/"configurable" knobs not driven by a flag, wrapper layers for a single caller, defensive validation at internal boundaries, error handling for impossible states, fallbacks for cases that cannot happen, premature generalization, backwards-compat shims, dead-code "for later", speculative TODOs, helper utilities used once. If a feature can be done in 30 lines, do NOT write 200. When in doubt: less code, fewer files, fewer abstractions.

### P1 — Standard

- **CLAUDE.md as living knowledge** — Study `src/<module>/CLAUDE.md` files before implementing in that module. They capture hard-won knowledge (quirks, gotchas, non-obvious decisions). If you figure something out the hard way, write it down in the relevant CLAUDE.md so the next agent doesn't burn the same cycles. Skip it if it's obvious from reading the code.
- **Scatter docs by module, not by topic** — Per-module CLAUDE.mds live next to the code they describe (`src/<module>/CLAUDE.md`). Cross-cutting docs live in `docs/` (architecture, keybindings, theming, mermaid). The root `.claude/CLAUDE.md` is the index — never duplicate per-module content here. See the Project Map below.
- **Incremental progress over big bangs** — Small changes that compile and pass tests. Never a single mega-commit that does everything.
- **Surgical changes only** — Touch only what the task requires. Don't "improve" adjacent code, comments, or formatting. Don't refactor what isn't broken. Match existing style even if you'd write it differently. If you spot unrelated dead code, mention it — don't silently fix it.
- **Use subagents liberally for parallel work** — Independent module tasks (per §5.7 dependency graph in the spec) MUST be parallelized via concurrent Agent invocations. Sequential tasks use `blockedBy` in TaskCreate. This is how a single session builds Veol without context bloat.

### P2 — Principles

- **Pragmatic over dogmatic** — Adapt to project reality.
- **Clear intent over clever code** — Boring and obvious wins.
- No comments in code — names should make the code self-explanatory. A comment is only justified when the WHY is non-obvious (hidden constraint, subtle invariant, workaround for a specific bug).
- Single Responsibility Principle, applied pragmatically.
- **Surface confusion, don't hide it** — If a request has multiple valid interpretations, present them. If something is unclear, name what's confusing and ask.

## NEVER

- Add buffer-editing capabilities. Veol reads, never edits content. File browser CRUD on `.md` files is a navigation aid, not an edit feature.
- Spawn any subprocess at runtime. Mermaid is rendered in-process by `src/mermaid/ascii/`. If you ever feel the urge to add a subprocess, you're solving the wrong problem.
- Build shell command strings (`Command::new("sh").arg("-c").arg(format!(...))`). General principle: argv arrays only, never interpolated shell.
- Make outbound network calls of any kind. No image fetches, no version checks, nothing.
- Embed a browser, JS runtime, or HTML/CSS engine. Mermaid is text — see `src/mermaid/ascii/`.
- Skip `cargo test --all` before declaring work done. Skip `cargo clippy --all-targets -- -D warnings` either.
- Commit without running `cargo fmt` first.
- Add `tokio` or any async runtime. Veol uses `std::thread` + `mpsc::channel` where threading is needed. Mermaid rendering is synchronous and instant.

## What is Veol

**Fast Markdown reading for the terminal.**

Veol is a Rust TUI Markdown reader. It opens `.md` files, renders them with syntax highlighting (syntect), code fences, tables (auto-shrink + word-wrap), task lists, footnotes, and inline Mermaid diagrams (rendered by the pure-Rust ASCII renderer in `src/mermaid/ascii/`). It's paginated, searchable (smart-case literal), watchable (always-on file polling), and themeable (9 bundled themes ported from reedo + custom TOML themes). It has a modal file browser (`Ctrl+E`, reedo-style) filtered to `.md` files + dirs containing them. It runs on pure stdlib threading — no tokio, no async ecosystem bloat. No external binaries, no browser, no network.

### Architecture

```
veol/
├── Cargo.toml               # crate manifest — clap, ratatui, crossterm, pulldown-cmark, syntect, indexmap, unicode-width, ...
├── src/
│   ├── main.rs              # entry, event loop, terminal raw mode, alternate screen, popup dispatch
│   ├── lib.rs               # public API surface for tests
│   ├── app.rs               # central App state + Popup enum + Mode dispatch
│   ├── cli.rs               # clap derive — L33 flag set
│   ├── config.rs            # ~/.config/veol/config.toml load/save (serde + toml)
│   ├── error.rs             # anyhow-based AppError + thiserror for mermaid module
│   ├── input.rs             # crossterm event → action mapping
│   ├── watcher.rs           # 500ms mtime polling via filetime
│   ├── markdown/
│   │   ├── mod.rs
│   │   ├── parser.rs        # pulldown-cmark events → block model
│   │   ├── model.rs         # Block enum (Heading, Paragraph, CodeBlock, MermaidBlock, ...)
│   │   ├── layout.rs        # block model → wrapped terminal lines
│   │   └── frontmatter.rs   # YAML/TOML frontmatter detection + table render
│   ├── tui/
│   │   ├── mod.rs
│   │   ├── viewport.rs      # ViewportState (top_line, height, total_lines)
│   │   ├── draw.rs          # main render loop
│   │   ├── keymap.rs        # keybind dispatch
│   │   ├── search.rs        # smart-case literal search
│   │   ├── toc.rs           # heading extraction + modal jump
│   │   ├── browser.rs       # reedo-style file browser popup, filtered to .md
│   │   ├── theme_switcher.rs # Ctrl+T modal with dot previews
│   │   └── help.rs          # ? help overlay
│   ├── render/
│   │   ├── mod.rs
│   │   ├── theme.rs         # 9 bundled themes + markdown color fields + custom loader
│   │   ├── code.rs          # syntect integration for code fences
│   │   ├── table.rs         # auto-shrink + word-wrap
│   │   └── text.rs          # inline span rendering (emphasis, strong, link, etc)
│   └── mermaid/
│       ├── mod.rs           # re-exports — render_mermaid, DisplayRegistry, extract_jobs
│       ├── extract.rs       # pulldown events → mermaid sources + cache keys (kept; sha2 dep)
│       ├── display.rs       # DisplayRegistry — instant in-process render cache
│       └── ascii/           # pure-Rust ASCII renderer
│           ├── mod.rs       # render_mermaid(source, max_width) dispatcher + fallback
│           ├── canvas.rs    # Canvas { Vec<Vec<char>> } + put_char/put_str/draw_box/merge/trim
│           ├── charset.rs   # Unicode + ASCII charsets, junction-merge primitives
│           ├── coord.rs     # Coord, Direction
│           ├── detect.rs    # DiagramKind detection (18 types + Unknown)
│           ├── error.rs     # AsciiRenderError (Empty / Parse / Unsupported)
│           ├── label.rs     # multi-line label width/height, centering
│           ├── astar.rs     # A* on grid coords for flowchart edges
│           ├── flowchart/   # ast + parser + layout + render
│           ├── sequence/    # participants + messages + alt/opt/loop/par blocks
│           ├── class/  state/  er/  pie/  gantt/  journey/  timeline/
│           ├── mindmap/  gitgraph/  quadrant/  requirement/  sankey/
│           └── xychart/  block/  architecture/  packet/
├── tests/
│   ├── cli_smoke.rs         # clap parsing + flag matrix
│   └── ...                  # snapshot + integration tests
├── docs/
│   ├── mermaid.md           # user-facing mermaid renderer doc
│   └── tasks/
│       ├── guiwohl-veol-spec-2026-05-26.md           # ← LOCKED SPEC
│       └── guiwohl-veol-ascii-mermaid-2026-05-27.md  # ← ASCII pivot spec
├── examples/
│   └── kitchen-sink.md      # exercises every block type
└── .claude/
    └── CLAUDE.md            # ← this file
```

### Project Map

Per-module CLAUDE.mds + cross-cutting docs. Read the relevant one before touching code in that area — they document the non-obvious stuff (quirks, gotchas, design choices) that you'd otherwise rediscover the hard way.

| Path | What you'll find |
|---|---|
| [src/markdown/CLAUDE.md](../src/markdown/CLAUDE.md) | pulldown-cmark event walker, block model, prose centering, table auto-shrink, frontmatter YAML/TOML quirks, mermaid info-string detection |
| [src/mermaid/CLAUDE.md](../src/mermaid/CLAUDE.md) | DisplayRegistry (instant cache, no Pending state), why `extract.rs::cache_key` is computed-but-unused, dispatcher seam |
| [src/mermaid/ascii/CLAUDE.md](../src/mermaid/ascii/CLAUDE.md) | `render` / `render_to_canvas` / `render_styled` pattern (every diagram follows it), junction-merge table, parallel color grid, A* corner penalty, how to add a diagram type |
| [src/render/CLAUDE.md](../src/render/CLAUDE.md) | 9 bundled themes, syntect `default-fancy` + `base16-ocean.dark` bridge, `Color::parse` rules, why `table.rs`/`text.rs` are empty |
| [src/tui/CLAUDE.md](../src/tui/CLAUDE.md) | DocumentWidget paint loop, paragraph nav via blank-line detection, mouse capture re-enablement + `--no-mouse` rationale, browser/theme switcher/help modal patterns |
| [docs/architecture.md](../docs/architecture.md) | Top-level data flow, threading model (std::thread + mpsc only), paint loop, why no async / no subprocess |
| [docs/keybindings.md](../docs/keybindings.md) | Every key in every mode (reader, search, browser, TOC, theme switcher, help) + mouse table |
| [docs/theming.md](../docs/theming.md) | Bundled theme list, color field reference, custom TOML format, switcher UX |
| [docs/mermaid.md](../docs/mermaid.md) | Diagram type matrix (18 types), charset details, limitations, bug-report flow |

### Stack

| Crate | Purpose |
|---|---|
| `clap` (derive) | CLI parsing |
| `ratatui` | TUI widgets |
| `crossterm` | terminal I/O, raw mode, mouse |
| `pulldown-cmark` | Markdown parser — events with footnotes/tables/tasklist/strikethrough |
| `syntect` | code fence syntax highlighting |
| `serde` + `toml` | config + theme parsing |
| `directories` | XDG / cross-platform paths |
| `anyhow` + `thiserror` | error model (app-level + module boundaries) |
| `tracing` + `tracing-appender` | file-based debug logging |
| `indexmap` | order-preserving maps for mermaid AST + layout |
| `unicode-width` | glyph width for canvas + labels |
| `filetime` | mtime polling in watcher.rs |
| `tempfile` | atomic config saves + tests |
| `sha2` | mermaid extract.rs cache key |
| `insta` (dev) | snapshot tests |

### Key Design Decisions

- **Always-on watch (500ms poll)**, opt-out with `--no-watch`. Auto-disabled for stdin.
- **Pager heuristic**: TUI if stdout is TTY + stdin is interactive; else `--plain` stdout.
- **File browser**: `Ctrl+E` toggles. Filter: `.md` only + dirs containing `.md` (recursive walk, cached per-session). Full reedo CRUD (n/f/r/d/m + Ctrl+Z/Y undo). Auto-appends `.md` extension; rejects other extensions. Opens replace current doc in-place.
- **Theme**: 9 reedo-ported themes + markdown color fields (heading_1..6, link, quote_*, code_bg, table_*, hr, task_*, mermaid_caption, search_match). `Ctrl+T` opens switcher.
- **Mermaid (ASCII renderer)**: 18 diagram types dispatched by `src/mermaid/ascii/mod.rs::render_mermaid(source, max_width)`. Pure Rust, in-process, instant. No subprocess, no async, no image protocols. Output is `Vec<String>` of pre-wrapped rows. Failure or unknown diagram kind falls back to a fenced source-code block. `m` toggles all diagrams rendered↔source. `--no-mermaid` skips the dispatcher entirely.
- **Charsets**: Unicode box-drawing default (`─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼ ╱ ╲ ▲ ▼ ◄ ►`). ASCII fallback (`- | + < > ^ v / \`) wired internally via `CharsetKind::Ascii` — not yet exposed as a CLI flag.
- **Error model**: anyhow app-wide, thiserror in `mermaid/` (AsciiRenderError).
- **Async**: `std::thread` + `mpsc::channel` for watcher only. NO tokio. Mermaid is synchronous.

For the full decision list, read `docs/tasks/guiwohl-veol-spec-2026-05-26.md` §2.

## Mermaid ASCII Renderer

The dispatcher in `src/mermaid/ascii/mod.rs`:

```rust
pub fn render_mermaid(source: &str, max_width: u16) -> Vec<String>
```

Pipeline per diagram: `parse → AST → layout → Canvas → Vec<String>`. Each diagram type lives in its own subdirectory (`flowchart/`, `sequence/`, `class/`, etc.) with the same shape: `ast.rs` / `parser.rs` / `render.rs` (and `layout.rs` for the flow/grid ones). Shared primitives sit at the `ascii/` top level (`canvas.rs`, `charset.rs`, `coord.rs`, `astar.rs`, `label.rs`).

Fallback contract: when a parser returns `AsciiRenderError` or the kind is `Unknown`, `render_mermaid` produces a synthetic fenced block:

```
// mermaid: <reason>
```mermaid
<original source verbatim>
```
```

This means user-facing output is never lost — even an unsupported diagram displays as readable source.

`DisplayRegistry` in `display.rs` is a per-document `HashMap<usize, Vec<String>>` cache keyed by block index. `ensure_rendered` lazily renders + memoizes per block. It's reset on document reload.

`extract.rs` is retained intact (sha2 cache_key field included) per the ASCII-pivot spec — the field is wired through `MermaidJob` for future cross-document caching, but currently not consumed.

## CLI Quick Start

```bash
cargo run -- README.md           # run veol on a file
cargo run -- -                   # stdin
cargo test --all                 # all tests
cargo clippy --all-targets -- -D warnings
cargo fmt
cargo build --release            # final binary at target/release/veol
```

Useful flag combos during development:

```bash
RUST_LOG=veol=debug cargo run -- README.md   # debug logs to ~/.cache/veol/veol.log
cargo run -- --no-mermaid README.md          # skip the ASCII renderer; render source as code fence
cargo run -- --plain README.md               # dump to stdout, no TUI
```

## Git Usage

- All feature branches from `main`. ALWAYS rebase, never merge commits.
- Conventional commits with good descriptions. Separate commits when logical.
- NEVER reference Claude Code in commit messages or PRs.
- All implementation work happens on a feature branch.
- Worktrees by default for parallel agent work; live under `.claude/worktrees/`. Cleanup on merge.

## Testing

```bash
cargo test --all                              # everything
cargo test --all -- --nocapture              # see println output
cargo insta review                            # review snapshot diffs
cargo insta accept                            # accept new snapshots
```

Test gates:
- `cargo test --all` — must pass
- `cargo clippy --all-targets -- -D warnings` — must be clean
- `cargo fmt --check` — must be formatted
- Snapshot tests via `insta` for every block-type render
- Unit tests for parser/extract/detect/canvas/charset modules + each ASCII diagram type
- CLI smoke tests for every flag

## How to use this project (for AI sessions)

1. Read the locked spec first: `docs/tasks/guiwohl-veol-spec-2026-05-26.md` and the ASCII pivot `docs/tasks/guiwohl-veol-ascii-mermaid-2026-05-27.md`. Treat both as source of truth.
2. Read `Cargo.toml` and the relevant `src/<module>/CLAUDE.md` for the area you're touching.
3. For any new feature → `/tdd`. Write the failing test first. Make it pass with the minimum code. Refactor only if it pays off.
4. For any bug → `/systematic-debugging`. Reproduce. Bisect. Fix root cause, not symptom.
5. Multi-step work → externalize into TaskCreate with the dependency graph from §5.7 of the spec. Steps at the same depth run as concurrent subagents.
6. Before declaring done: `cargo test --all` + `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check`. All green. Same message you reply in.

## Reedo as reference (not dependency)

Veol borrows reedo's modal file-browser UX and TOML theme schema as **architectural patterns**, not as code. Reedo lives at `/home/wohl/W/projects/reedo/`; useful references:
- `reedo/src/ui/tree.rs` — `TreeState`, `FsOperation`, hint navigation, color cycling, modal popup widget.
- `reedo/src/ui/theme_switcher.rs` — color-dot preview modal.
- `reedo/src/config/theme.rs` — TOML theme loader, bundled themes, color field map.
- `reedo/docs/theming.md`, `reedo/docs/file-explorer.md` — UX spec.

When implementing the Veol equivalent, **read the pattern, rewrite it Veol-clean** — don't copy-paste. Veol has no editor, no git status, no `.gitignore` walker, no tree-sitter dependencies.
