## Table of Contents
`t` opens a modal listing every heading; `Enter` jumps the viewport to it.

> **Related**
> - [`viewport.md`](viewport.md) — `jump_to_line` is what TOC selection drives
> - [`../keybindings.md`](../keybindings.md) — `t`, `j`/`k`, `Enter`, `Esc`
> - [`../theming.md`](../theming.md) — `heading_1..6` colors used in the list

Implementation: `src/tui/toc.rs`.

## Building entries

`TocState::build(lines, blocks)`:

1. Index `lines` for any `LayoutLine.heading_anchor.is_some()` → `(anchor, line_index)` pairs.
2. Flatten blocks recursively (`Quote`, `List`, `Footnote` children included) and for each `Block::Heading`, find the next unconsumed matching anchor.
3. Emit `TocEntry { level, text, anchor, line_index }`.

The two-pass walk is intentional: blocks own the heading text + level, but only `LayoutLine`s know the post-layout row position. Walking them in parallel handles duplicate slugs correctly via the `consumed: Vec<bool>` sentinel — second `## Foo` gets its own row even though it shares the slug.

## Widget

`TocWidget` paints a centered popup (70% w × 70% h) with:

```
╭─ Table of Contents ──────────────╮
│ # Heading One                    │
│   ## Sub heading                 │
│     ### Even deeper              │
│ # Another section                │
│                                  │
│       ↑↓ navigate ⏎ jump esc close│
╰──────────────────────────────────╯
```

- Each entry shows `<indent><prefix><text>` where indent = `(level-1)*2` spaces and prefix = `#` repeated `level` times in the popup's `dim` color.
- Heading text uses the same `heading_<level>` color as the document, so the visual mapping is exact.
- Selected row inverts via `popup_selected` background.
- Empty state: centered `(no headings)` in dim color.

## Keys

| Key | Effect |
|---|---|
| `j` / `Down` | `move_down(visible_height)` |
| `k` / `Up` | `move_up()` |
| `Enter` | `viewport.jump_to_line(selected.line_index)`, close popup |
| `Esc` | Close popup |

## Scroll math

`compute_scroll_offset(current_offset, selected, total, visible)` keeps the selected entry inside the visible window:

- If `selected < offset`, slide up.
- If `selected >= offset + visible`, slide down.
- Else: leave offset alone.

`offset` is then capped at `total - visible` so the final entry is never above the popup bottom.

## When TOC is empty

If the document has no headings, the modal still opens — it just shows `(no headings)` centered. `is_empty()` is exposed for callers that want to skip opening the popup entirely (none do today; the keybinding is unconditional).

## Heading anchors

Anchors are GitHub-style slugs (see `markdown::model::heading_anchor`):

- Lowercased.
- Alphanumerics + dashes, everything else collapsed.
- **Digits preserved as-is**: `"Section 2.1"` → `section-21` (the dot vanishes, the digits stay together). Pinned by `markdown/CLAUDE.md`'s heading-anchor section — don't "fix" it.
