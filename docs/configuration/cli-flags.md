## CLI Flags
Every flag in `veol --help`, what it does, and where it lives in the codebase.

> **Related**
> - [`config-file.md`](config-file.md) — persistent counterparts
> - [`watch-mode.md`](watch-mode.md) — `--no-watch` deep dive
> - [`../keybindings.md`](../keybindings.md) — interactive controls once running

Source: `src/cli.rs` (clap derive struct).

## Synopsis

```
veol [OPTIONS] [FILE]
```

`FILE` is optional. Use `-` for stdin. When omitted and stdout is a TTY, Veol prints the welcome banner and exits.

## Flag reference

| Flag | Type | Default | Purpose |
|---|---|---|---|
| `<FILE>` | path or `-` | — | The Markdown file to read. `-` reads from stdin. |
| `--pager` | bool | off | Force TUI mode even if stdout is not a TTY. Mutually exclusive with `--no-pager` / `--plain`. |
| `--no-pager` | bool | off | Force plain stdout dump, no TUI. Mutually exclusive with `--pager`. |
| `--plain` | bool | off | Same as `--no-pager` (kept for `glow`/`bat` muscle memory). |
| `--theme <NAME>` | string | `reedo-dark` | Pick a bundled or custom theme by name. Overrides `config.theme`. |
| `--theme-list` | bool | off | Print every available theme name (bundled + custom) and exit. |
| `--width <COLS>` | u16 ≥ 20 | terminal width | Force a content width for layout. Useful for piping into `less -R` or testing snapshots. |
| `--no-mermaid` | bool | off | Skip the ASCII mermaid renderer; emit the source as a fenced code block. |
| `--no-watch` | bool | off | Disable file-mtime polling. See [`watch-mode.md`](watch-mode.md). |
| `--no-mouse` | bool | off | Disable terminal mouse capture; restores native click-drag text selection. |
| `--toc` | bool | off | Auto-open the TOC popup on startup. Equivalent to `show_toc = true`. |
| `--line-numbers` | bool | off | Reserved — no rendering today. |
| `--config <PATH>` | path | XDG default | Load this file instead of `~/.config/veol/config.toml`. |
| `--debug-render` | bool | off | Reserved for layout debugging. |

## Pager heuristic

`should_use_tui` (`main.rs`):

```rust
if cli.plain || cli.no_pager { return false; }
if cli.pager                 { return true;  }
stdout_is_tty && stdin_is_tty
```

| Stdin | Stdout | No flags | Resulting mode |
|---|---|---|---|
| TTY | TTY | — | TUI |
| pipe | TTY | — | plain (stdin not interactive → can't capture keys) |
| TTY | pipe | — | plain (no terminal to paint into) |
| pipe | pipe | — | plain |
| any | any | `--pager` | TUI (raw mode required; will fail on a pipe) |
| any | any | `--plain` | plain |

## Width parsing

`parse_width(s)`:

```rust
const WIDTH_MIN_COLS: u16 = 20;
```

Values below 20 are rejected at parse time. Above 20 there's no upper bound — but layout caps prose at 100 cols (see [`../rendering/pipeline.md`](../rendering/pipeline.md)).

## --no-mouse

Added 2026-05-27 after mouse capture was re-enabled by default. Crossterm's mouse capture intercepts the terminal's native selection, which breaks copy-paste workflows. Pass `--no-mouse` to keep keyboard scroll only and let your terminal handle selection.

See [`../decisions/0003-mouse-capture-on-by-default.md`](../decisions/0003-mouse-capture-on-by-default.md) for the full rationale.

## Conflicting flags

`clap`'s `conflicts_with` rejects illegal combos at parse time:

- `--pager` conflicts with `--no-pager` and `--plain`.
- `--no-pager` conflicts with `--pager`.

You'll get a clap usage error before `main` runs.

## Exit codes

| Code | When |
|---|---|
| `0` | Clean exit (read, view, quit). |
| `1` | File not found, parse error, terminal init failed. |
| `1` | Any `anyhow::Error` from `run()`; the message is `veol: <err:#>` on stderr. |
