## ADR-0003: Mouse capture on by default, `--no-mouse` to opt out
Mouse capture is re-enabled by default after a `tmux`-related regression was fixed. Power users who want native text selection pass `--no-mouse`.

- **Status:** Accepted
- **Date:** 2026-05-27

## Context

Veol uses crossterm's mouse capture to forward scroll-wheel events to the viewport (`MouseAction::ScrollUp` / `ScrollDown` → `viewport.scroll_*`). At one point, mouse capture was disabled by default because:

1. Crossterm's mouse capture intercepts the terminal's native click-drag selection. Operators copying mermaid renders into clipboards lost that.
2. A specific `tmux` interaction caused mouse events to bleed into the surrounding pane after Veol exited, requiring `reset` to recover.

The mouse-disabled state shipped with a `#[allow(dead_code)]` `handle_mouse` function in `main.rs` and a comment promising a future `--mouse` flag.

## Decision

**Re-enable mouse capture by default. Add `--no-mouse` to opt out.** The `tmux` mode handling was hardened (`EnterAlternateScreen` + `EnableMouseCapture` ordering, panic-hook restoration of both, explicit `LeaveAlternateScreen` + `DisableMouseCapture` on every exit path).

`src/cli.rs`:

```rust
#[arg(long)]
pub no_mouse: bool,
```

`src/main.rs::run_tui`:

```rust
if mouse {
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
} else {
    execute!(stdout, EnterAlternateScreen)?;
}
```

Panic hook restores raw mode + leaves alternate screen + (if mouse was enabled) disables mouse capture before the original hook runs. Every exit path mirrors the enable/disable pairing.

## Stale comment

The dead-code comment in `handle_mouse` still says:

```rust
#[allow(dead_code)]
// Kept for a future --mouse flag; mouse capture is currently disabled to allow text selection.
fn handle_mouse(...)
```

That comment is **wrong but the `allow(dead_code)` stays** — clippy still flags the function in some build configurations because the call site is conditional. Don't "fix" the comment without rewriting the conditional. See `src/tui/CLAUDE.md` for the same warning.

## When to pass `--no-mouse`

| Scenario | Reason |
|---|---|
| Copying a mermaid render to clipboard | Terminal selection only works with capture off. |
| Multi-pane `tmux` where you want scroll on the wrong pane to behave naively | Capture intercepts the event before tmux sees it. |
| Sharing a terminal recording where mouse events are noise | Cleaner asciinema captures. |
| Anywhere terminal selection > scroll-wheel scroll | Operator preference. |

Persistent users can alias:

```bash
alias veol='veol --no-mouse'
```

There is no `config.toml` field for this today — it's a CLI-only flag.

## Consequences

**Positive:**

- Scroll-wheel works out of the box. Operators don't have to discover a flag.
- Mouse capture is uniformly handled; no in-between states.

**Accepted trade-offs:**

- Operators who copy diagrams must learn `--no-mouse`. Discoverable via `veol --help` and [`../configuration/cli-flags.md`](../configuration/cli-flags.md).

## Related

- [`../configuration/cli-flags.md`](../configuration/cli-flags.md) — `--no-mouse` reference
- [`../tui/file-browser.md`](../tui/file-browser.md) — mouse scroll inside the popup
