## Viewport
The window into the document — `top_line` + `height` + a few jump primitives.

> **Related**
> - [`../keybindings.md`](../keybindings.md) — keys that drive viewport methods
> - [`toc.md`](toc.md) — `Enter` calls `jump_to_line`
> - [`search.md`](search.md) — `n`/`N` jumps to match line

Implementation: `src/tui/viewport.rs`.

## State

```rust
pub struct ViewportState {
    pub top_line: usize,
    pub height: usize,
    pub total_lines: usize,
    pub cursor_line: Option<usize>,
}
```

Only `top_line` is mutable user state — `height` and `total_lines` are recomputed each frame (`set_height` on the doc-area height, `set_total` after relayout). `cursor_line` is reserved for future row-highlight features; today it's `None`.

`top_line` is **always clamped** to `[0, total_lines - height]` via `clamp_top()`. Every mutator calls it.

## Primitives

| Method | Behavior |
|---|---|
| `scroll_down(n)` / `scroll_up(n)` | Saturating add/sub on `top_line`. |
| `page_down` / `page_up` | Move by `height - 2` (intentional 2-line overlap so the eye doesn't lose context). |
| `half_page_down` / `half_page_up` | Move by `height / 2`. |
| `jump_top` / `jump_bottom` | `top_line = 0` / `top_line = max_top()`. |
| `jump_to_line(idx)` | Absolute jump, then clamp. |
| `next_heading(lines)` / `prev_heading(lines)` | Skip to next/prev `LayoutLine.heading_anchor.is_some()`. |
| `next_paragraph(lines)` / `prev_paragraph(lines)` | Skip past current block, then past blanks. |

## Paragraph nav semantics

`next_paragraph`:

1. Walk forward past non-blank lines (out of the current block).
2. Walk forward past blank lines.
3. Land on the first line of the next block.

`prev_paragraph` mirrors this — find the previous block's first line, not its last. Test pinned in `prev_paragraph_lands_on_first_line_of_previous_block`.

## Percent calc

```rust
pub fn percent(&self) -> u8 {
    if total_lines == 0 || total_lines <= height { return 100; }
    let max = total_lines - height;
    ((top_line * 100 / max.max(1)).min(100)) as u8
}
```

Used by `StatusBarWidget`. Reads `100%` when the entire document fits on screen, or when scrolled to the bottom — the operator sees `100%` and knows there's nothing more. `0%` at top, scaled linearly between.

## Heading navigation contract

`next_heading` starts the search at `top_line + 1` so calling it from a line that *is* a heading advances to the next one rather than no-op-ing. `prev_heading` walks the visible range backwards from `top_line` (exclusive).

Both rely on `LayoutLine.heading_anchor` being set on the *first* line of a heading block — `render_heading` (`src/markdown/layout.rs:415`) does that. Multi-line wrapped headings only have an anchor on row 1.

## Why no separate cursor

Veol is a reader, not an editor. There's no insertion point, no "current line" inside the viewport. Scroll-to-position is the only state. This keeps the model trivial: paint `lines[top_line..top_line+height]` and stop.

`cursor_line` is left in the struct as a hook for a future "highlight current paragraph" hover affordance. It is not read anywhere today.
