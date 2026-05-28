## Help Overlay
`?` opens a centered modal with the full keybind list; any key closes it.

> **Related**
> - [`../keybindings.md`](../keybindings.md) — same list, but full-detail and indexed
> - [`../../src/tui/help.rs`](../../src/tui/help.rs) — `keybinds()` slice is the source of truth

Implementation: `src/tui/help.rs`.

## Single source of truth

`keybinds()` returns a `&'static [(key, description)]` slice. The widget renders it, the markdown doc mirrors it. When you add a key, update both — there's no codegen.

```rust
pub fn keybinds() -> &'static [(&'static str, &'static str)] {
    &[
        ("q", "quit"),
        ("j / k / arrows", "line scroll"),
        // ...
    ]
}
```

`HelpState` itself is `pub struct HelpState;` — zero data. The popup is stateless because there's nothing to track: it opens, paints, closes on next key.

## Layout

Two-column list:

```
╭─ Help ────────────────────────────────╮
│  q              quit                  │
│  j / k / arrows line scroll           │
│  Space / PgDn   page down             │
│  ...                                  │
│              esc / ? close            │
╰───────────────────────────────────────╯
```

- Key column width is computed once per render: `max(key.chars().count()) + 2`.
- Keys are accent-colored + bold.
- Descriptions use the default foreground.
- Footer hint `esc / ? close` is dim and centered.

## Dismiss behavior

`main.rs::handle_key` routes any keypress while `app.popup == Popup::Help` to closing the popup:

```rust
Popup::Help => { app.popup = Popup::None; }
```

This is unique among popups — TOC, Browser, and Theme Switcher require explicit `Esc` or `Enter`. Help is "press anything to dismiss" because the operator just opened it to peek.

## Sizing

The popup is rendered at a centered 50% × 50% of the terminal (`centered_rect` in `main.rs`). If terminal is smaller than `6 × 5`, the widget early-returns without painting — better than a crashed border.

The list truncates at the visible row count without scroll. Today the slice fits in 17 lines so the constraint is academic; if `keybinds()` grows past the popup, add scroll instead of expanding the popup (operators don't want a help that takes the whole screen).
