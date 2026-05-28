## Charsets and Junctions
Two glyph sets, plus the rule that turns overlapping box-drawing characters into proper junctions.

> **Related**
> - [`architecture-pivot.md`](architecture-pivot.md) — why the ASCII renderer exists
> - [`color-pipeline.md`](color-pipeline.md) — color grid runs in parallel with the char grid
> - [`../mermaid.md`](../mermaid.md) — user-facing charset notes

Implementation:
- `src/mermaid/ascii/charset.rs` — the trait + Unicode and ASCII impls.
- `src/mermaid/ascii/canvas.rs` — junction merge inside `merge_at` / `put_char`.

## The two charsets

```rust
pub enum CharsetKind { Unicode, Ascii }
```

| Slot | Unicode | ASCII |
|---|---|---|
| `horiz` | `─` | `-` |
| `vert` | `│` | `\|` |
| `top_left` / `top_right` | `┌` `┐` | `+` `+` |
| `bot_left` / `bot_right` | `└` `┘` | `+` `+` |
| `tee_right` / `tee_left` | `├` `┤` | `+` `+` |
| `tee_down` / `tee_up` | `┬` `┴` | `+` `+` |
| `cross` | `┼` | `+` |
| `arrow_up/down/left/right` | `▲` `▼` `◄` `►` | `^` `v` `<` `>` |
| `diag_down` / `diag_up` | `╲` `╱` | `\` `/` |
| `round_top_left` / `_top_right` | `╭` `╮` | `+` `+` |
| `round_bot_left` / `_bot_right` | `╰` `╯` | `+` `+` |

Unicode is the default. ASCII is fully wired through every diagram (each `render_styled(source, max_width, charset)` call accepts it) but is **not yet exposed as a CLI flag** — Veol targets modern UTF-8 terminals. Open an issue if you need a `--ascii` flag.

## Trade-offs

| Property | Unicode | ASCII |
|---|---|---|
| Visual fidelity | High — proper rounded corners, arrows, junctions | Coarse — every corner and junction is `+` |
| Terminal support | Any modern UTF-8 terminal | Anything, even POSIX `vt100` |
| Width per cell | Most chars are width-1 (line-drawing block is BMP) | Always width-1 |
| Junction merging | Distinct glyphs let merge produce `┼ ├ ┤ ┬ ┴` | `+` is already the universal join — no merging needed |
| Selection / copy | Renders fine in any X11/Wayland/macOS terminal | Plain ASCII copies even into legacy buffers |

## Junction merging

Two box-drawing chars writing to the same cell would normally clobber. The merge table promotes them to the correct compound glyph instead.

### Origin

Lifted verbatim from `AlexanderGrooff/mermaid-ascii` (Go). The Go project documents and tests the pair → glyph table; Veol mirrors it.

### The rule

Pseudocode for `canvas.put_char(x, y, new_char)`:

```
existing = grid[y][x]
if existing == ' ' { grid[y][x] = new_char; return }
if existing == new_char { return }                       // no-op
merged = merge_table.get((existing, new_char))
        .or(merge_table.get((new_char, existing)))       // symmetric
        .unwrap_or(new_char)                             // new wins on conflict
grid[y][x] = merged
```

### Example pairs

```
─ + │  →  ┼            (horizontal meets vertical)
─ + ┌  →  ┬            (horizontal terminates a top-corner)
│ + └  →  ├            (vertical extends down through a bot-left corner)
┌ + ┐  →  ┬            (two top corners → top tee)
└ + ┘  →  ┴            (two bot corners → bot tee)
```

The full lookup is constructed once per `Canvas` and reused.

### Why it matters

Without merging, a flowchart's edges and nodes would overdraw — every place an edge enters a box, you'd see `─` overwriting `┌` or vice versa. The output would look broken even though the structure is correct. Merging gives the visual fidelity that makes ASCII flowcharts legible.

### ASCII fallback

The merge table is Unicode-specific. For `CharsetKind::Ascii`, all corners and junctions are already `+`, so `put_char('+', ...)` over an existing `+` is a no-op — no merge logic needed.

## Where charset is selected

Every diagram module follows the same signature:

```rust
pub fn render_styled(source: &str, max_width: u16, charset: CharsetKind) -> Result<Vec<StyledRow>, AsciiRenderError>
```

The dispatcher in `src/mermaid/ascii/mod.rs:38` hardcodes `CharsetKind::Unicode`. To enable ASCII globally, change that single line. Per-block selection would require either a new CLI flag or a per-mermaid-block directive (neither is wired).
