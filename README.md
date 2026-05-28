<div align="center">

# veol

Fast Markdown reading for the terminal.

A pure-Rust, paginated TUI reader with inline ASCII Mermaid. No browser, no Chrome, no subprocess.

Created by [@guiwohl](https://github.com/guiwohl)

</div>

veol is a single-binary terminal Markdown reader for people who want `less` to render their `.md` files instead of dumping them. It opens a file, paginates it, syntax-highlights the fences, draws the tables, and renders every Mermaid diagram inline as ASCII — all in-process, all synchronous, all instant. The companion to [reedo](https://github.com/guiwohl/reedo): reedo edits, veol reads.

## Why veol

- One binary. No `mmdc`, no Node, no headless Chrome, no image protocol negotiation.
- Mermaid renders as text. 18 diagram types, drawn in pure Rust, instant, never blocks paint.
- Paginated reader workflow — keyboard-first, `j/k`, `]/[`, `/`, `t`. Mouse scroll too.
- Themed: 9 bundled themes ported from reedo + drop-in TOML.
- Always-on file watching (500ms poll) — edit elsewhere, veol re-renders.
- No async runtime. Pure `std::thread` + `mpsc::channel`. ~50 fewer deps.

## Demo

`veol --plain examples/kitchen-sink.md` — actual output, no Photoshop.

```
A simple flowchart:
┌─────────────────┐     ┌───────┐     ┌─────────────────┐     ┌─────────────┐     ┌───────┐
│                 │     │       │     │                 │     │             │     │       │
│ Markdown Source ├─────► Parse ├─────►     Layout      ├─────►  Viewport   ├─────► Draw  │
│                 │     │       │     │                 │     │             │     │       │
└─────────────────┘     └───┬───┘     └─────────────────┘     └──────▲──────┘     └───────┘
                            │                                        │
                            │         ┌─────────────────┐     ┌─────────────┐  ┌───────┐
                            └─────────► Extract Mermaid ├─────► mmdc Worker ├──┤ Cache │
                                      └─────────────────┘     └─────────────┘  └───────┘

A sequence diagram:
┌───────┐     ┌─────┐
│ Alice │     │ Bob │
└───┬───┘     └──┬──┘
    │ Hello      │
    ├───────────►│
    │ Hi back    │
    │◄┈┈┈┈┈┈┈┈┈┈┈┤
    │            │

A pie chart:
Renderer share

Text      █████████████████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░   60.0%
Code      ███████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░   25.0%
Diagrams  █████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░   15.0%

A mindmap:
● Veol
├── · Markdown
├── · Mermaid
└── · Themes
```

## Install

Requires a recent stable Rust toolchain (edition 2021) and a UTF-8 terminal.

```bash
git clone https://github.com/guiwohl/veol.git
cd veol
cargo install --path .
```

Or run from source:

```bash
cargo run --release -- README.md
cat notes.md | cargo run --release -- -
```

## Quick start

```bash
veol README.md                   # open the TUI reader
veol --plain README.md           # dump rendered output to stdout (pipeable)
veol --theme dracula notes.md    # pick a theme
veol --no-mouse docs/spec.md     # disable mouse capture (text-select friendly)
veol --no-watch CHANGELOG.md     # skip the 500ms mtime poll
veol --toc README.md             # open with the TOC modal up
cat - | veol -                   # read from stdin (watch auto-disabled)
```

## CLI flags

| Flag | Purpose |
|---|---|
| `FILE` | path to a `.md` file, or `-` for stdin |
| `--pager` / `--no-pager` | force TUI mode / force stdout mode (default: auto by TTY) |
| `--plain` | render to stdout, never enter the TUI |
| `--theme NAME` | pick a bundled theme or a custom one from `~/.config/veol/themes/` |
| `--theme-list` | list available themes and exit |
| `--width COLS` | cap the rendering width (min 20 cols) |
| `--no-mermaid` | render mermaid fences as plain source code blocks |
| `--no-watch` | disable the 500ms file-mtime poll |
| `--no-mouse` | disable mouse capture (lets the terminal own click + drag selection) |
| `--toc` | open with the Table of Contents modal already up |
| `--line-numbers` | show line numbers in the viewport |
| `--config PATH` | use a non-default config file |
| `--debug-render` | dump internal layout markers for renderer debugging |
| `-V`, `--version` | print version |
| `-h`, `--help` | print help |

## Keybindings

Reader-only. No modes. No leader keys. No vim-isms beyond `h j k l`–style scroll.

| Key | Action |
|---|---|
| `q` | Quit |
| `j`, `↓` | Scroll down one line |
| `k`, `↑` | Scroll up one line |
| `Space` | Page down |
| `b` | Page up |
| `d` | Half-page down |
| `u` | Half-page up |
| `PgDn` | Next paragraph |
| `PgUp` | Previous paragraph |
| `g` | Jump to top |
| `G` | Jump to bottom |
| `]` | Next heading |
| `[` | Previous heading |
| `/` | Start search |
| `n` | Next match |
| `N` | Previous match |
| `t` | Toggle Table of Contents modal |
| `m` | Toggle Mermaid render ↔ source view (global) |
| `f` | Toggle frontmatter card |
| `r` | Reload file from disk |
| `o` | Open link under cursor in `$BROWSER` / `xdg-open` |
| `?` | Help overlay |
| `Ctrl+E` | Toggle modal file browser (`.md`-only) |
| `Ctrl+T` | Toggle theme switcher |
| `Esc` | Close popup / cancel search |

Mouse wheel scrolls the viewport (and tree / TOC inside popups). `Shift+drag` selects text using your terminal's native selection — pass `--no-mouse` if your terminal needs full ownership of pointer events.

## Mermaid support

veol renders Mermaid diagrams inline as Unicode box-drawing text. Dispatcher in [`src/mermaid/ascii/mod.rs`](src/mermaid/ascii/mod.rs):

```rust
pub fn render_mermaid(source: &str, max_width: u16) -> Vec<String>
```

Pipeline per diagram: `parse → AST → layout → Canvas → Vec<String>`. No panics. On any parse failure or unknown diagram kind, the source is preserved as a fenced fallback block prefixed with `// mermaid: <reason>`.

| Mermaid keyword | Status | Notes |
|---|---|---|
| `flowchart` / `graph` | full | TD / TB / LR / BT / RL, subgraphs, A* edge routing |
| `sequenceDiagram` | full | participants, sync / async / dotted arrows, alt / opt / loop / par |
| `classDiagram` | full | members, visibility, relationships |
| `stateDiagram` / `stateDiagram-v2` | full | `[*]` start/end, composite states, transitions |
| `erDiagram` | full | cardinalities, attributes |
| `pie` | full | slices + legend |
| `gantt` | partial | tasks and sections; durations approximated |
| `journey` | full | actors and scores |
| `timeline` | full | events along a single axis |
| `mindmap` | full | hierarchical tree |
| `gitGraph` | full | commits, branches, merges |
| `quadrantChart` | full | 4-quadrant scatter |
| `requirementDiagram` | full | requirements + verifications |
| `sankey-beta` | full | flows between named nodes |
| `xychart-beta` | full | bar / line chart in text |
| `block-beta` | full | grid of labeled blocks |
| `architecture-beta` | full | groups, services, edges |
| `packet-beta` | full | byte-range packet diagram |
| anything else | fallback | rendered as a fenced `mermaid` code block |

`m` toggles all diagrams between rendered and source view for the current document. `--no-mermaid` skips the dispatcher entirely. Full notes: [`docs/mermaid.md`](docs/mermaid.md).

## Theming

Nine bundled themes, ported from reedo. Pick one with `--theme`, switch live with `Ctrl+T`, persist via the switcher's Enter key.

- `Default` — inherits terminal palette
- `reedo-dark`
- `reedo-light`
- `catppuccin`
- `dracula`
- `gruvbox`
- `nord`
- `rose-pine`
- `solarized-dark`

Drop custom TOML themes into `~/.config/veol/themes/*.toml` and they appear in `--theme-list` and the switcher. Markdown-specific color fields cover `heading_1..heading_6`, `link`, `quote_marker`, `quote_text`, `code_bg`, `code_border`, `table_border`, `table_header_fg`, `hr`, `task_done`, `task_pending`, `mermaid_caption`, `search_match`.

## Configuration

Config lives at `~/.config/veol/config.toml` (XDG via the `directories` crate). Minimal example:

```toml
theme = "dracula"
wrap = true
show_statusline = true
frontmatter = true

[cache]
max_size_mb = 128
```

CLI flags always win over config; the theme switcher writes back to this file.

## Architecture

Pure-Rust crate, single binary, no runtime dependencies beyond a UTF-8 terminal.

```
src/
├── main.rs          # entry, raw mode, alternate screen, popup dispatch
├── app.rs           # central App state + Popup enum + Mode dispatch
├── cli.rs           # clap derive — every flag from --pager to --no-mouse
├── config.rs        # ~/.config/veol/config.toml load/save
├── watcher.rs       # 500ms mtime polling via filetime
├── markdown/        # pulldown-cmark events → block model → wrapped lines
├── render/          # syntect code highlight, table shrink, theme, inline spans
├── tui/             # viewport, draw, keymap, search, toc, browser, theme switcher, help
└── mermaid/
    ├── extract.rs   # pulldown events → mermaid sources + cache keys
    ├── display.rs   # per-document render cache
    └── ascii/       # pure-Rust ASCII renderer — 18 diagram types
        ├── mod.rs   # dispatcher + fallback
        ├── canvas.rs / charset.rs / coord.rs / astar.rs / label.rs   # shared primitives
        └── flowchart/ sequence/ class/ state/ er/ pie/ gantt/ journey/
            timeline/ mindmap/ gitgraph/ quadrant/ requirement/ sankey/
            xychart/ block/ architecture/ packet/
```

Threading: `std::thread` + `mpsc::channel` for the file watcher. Mermaid rendering is synchronous and runs on the render path because it's measured in microseconds. No `tokio`, no async ecosystem, no future-soup.

## Why this exists

Every other terminal Markdown reader either dumps the file unrendered, shells out to `mmdc` (which pulls Chrome through `puppeteer`), or assumes your terminal speaks Kitty / Sixel / iTerm2 image protocols. veol's bet is the opposite: Mermaid is text, so render it as text — in-process, deterministic, and fast enough that you don't notice it ran.

The result is a reader that boots in <30ms, paints instantly, has no network surface, no subprocess footprint, and works the same in tmux, mosh, SSH, and CI logs as it does on your desktop. It also reads stdin (`claude -p "..." | veol -`), which makes it a half-decent terminal alternative to opening a browser tab every time an agent emits Markdown at you.

## Documentation

| Topic | Link |
|---|---|
| Mermaid renderer | [docs/mermaid.md](docs/mermaid.md) |
| Full spec | [docs/tasks/guiwohl-veol-spec-2026-05-26.md](docs/tasks/guiwohl-veol-spec-2026-05-26.md) |
| ASCII pivot spec | [docs/tasks/guiwohl-veol-ascii-mermaid-2026-05-27.md](docs/tasks/guiwohl-veol-ascii-mermaid-2026-05-27.md) |
| AI instructions | [.claude/CLAUDE.md](.claude/CLAUDE.md) |

## Acknowledgments

- **[reedo](https://github.com/guiwohl/reedo)** — sibling project. veol borrows reedo's TOML theme schema, modal file browser UX, and the nine bundled themes.
- **[mermaid-ascii](https://github.com/AlexanderGrooff/mermaid-ascii)** — the Go reference renderer that proved Mermaid-as-text was a tractable problem and informed the canvas + A*-edge approach.
- **[pulldown-cmark](https://github.com/raphlinus/pulldown-cmark)**, **[syntect](https://github.com/trishume/syntect)**, **[ratatui](https://github.com/ratatui/ratatui)**, **[crossterm](https://github.com/crossterm-rs/crossterm)** — the load-bearing crates underneath everything.

## License

MIT.
