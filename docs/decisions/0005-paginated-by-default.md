## ADR-0005: Paginated by default; `--plain` for piping
Veol detects TTY context and chooses TUI vs plain output automatically. Operators rarely need a flag.

- **Status:** Accepted
- **Date:** 2026-05-26

## Context

Markdown readers split into two camps:

- **Always-paged** (`less`, `bat`): assumes interactive use; needs `cat` semantics via flag.
- **Always-print** (`glow` without `-p`, `pandoc`): dumps to stdout; needs `-p` for interactive.

Both break the user experience for one of those use cases. Operators want:

```bash
veol notes.md                  # interactive, paged
veol notes.md | grep TODO      # plain stdout, no escape codes
veol notes.md > out.txt        # plain stdout, file write
cat notes.md | veol            # interactive — but no stdin TTY, complicated
```

## Decision

**Heuristic-based mode selection.** Default to TUI when both stdin and stdout are TTYs. Default to plain otherwise. Explicit flags override:

```rust
// src/main.rs::should_use_tui
if cli.plain || cli.no_pager { return false; }
if cli.pager                 { return true; }
stdout_is_tty && stdin_is_tty
```

Matrix:

| Stdin | Stdout | No flags | Mode |
|---|---|---|---|
| TTY | TTY | — | TUI |
| TTY | pipe | — | plain (no terminal to paint into) |
| pipe | TTY | — | plain (no interactive stdin → can't read keys) |
| pipe | pipe | — | plain |
| any | any | `--pager` | TUI (raw mode required; fails on pipe) |
| any | any | `--plain` / `--no-pager` | plain |

`--plain` and `--no-pager` are aliases. Both exist because:
- `--plain` matches `bat`'s muscle memory.
- `--no-pager` matches `git`'s muscle memory.

`--pager` exists for the rare case where the operator *wants* TUI from a non-TTY context (e.g., scripted demo). It will fail to enable raw mode if stdin isn't actually a TTY.

## Plain output shape

`print_plain_to_stdout` (`src/main.rs:92`) writes each `LayoutLine`'s span text to stdout, indent-prefixed but unstyled. Layout is identical to TUI mode — only the paint backend changes. This means tables, code-fence chrome, and ASCII mermaid all render correctly in plain mode.

ANSI escape codes are **not** emitted in plain mode. Pipe-friendly. If you want colors over pipe, set up `less -R` or `bat --plain` upstream.

## Why this matters

The default lets ad-hoc invocations work without thought:

```bash
veol README.md          # operator opens it interactively
veol README.md > a.txt  # operator dumps it to a file
ls *.md | xargs veol    # this falls back to plain (stdin is pipe) — won't error
```

Removing the heuristic means every user learns one flag. The heuristic costs ~10 lines of code and removes that toll for 90% of usage.

## Consequences

**Positive:**

- Zero-config interactive use.
- Pipe-friendly without explicit `--plain`.
- The `--pager` / `--plain` flags are escape hatches, not the primary UX.

**Accepted trade-offs:**

- Operators who script Veol must know `--plain` exists for non-TTY edge cases (e.g. cron jobs whose stdout is somehow a TTY — rare).
- Some shells fake stdin TTY in unexpected ways; `--plain` is the workaround.

## Related

- [`../configuration/cli-flags.md`](../configuration/cli-flags.md) — full flag table
- [`../rendering/pipeline.md`](../rendering/pipeline.md) — same layout for both modes
