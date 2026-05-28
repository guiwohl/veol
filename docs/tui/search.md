## Search
Smart-case literal substring search; `n`/`N` cycles matches; highlights at the column level.

> **Related**
> - [`viewport.md`](viewport.md) — `jump_to_line` lands on a match's row
> - [`../keybindings.md`](../keybindings.md) — `/`, `n`, `N`, `Esc`

Implementation: `src/tui/search.rs`.

## State machine

```rust
pub enum SearchMode { Off, Editing, Active }
```

| From | Event | To | Effect |
|---|---|---|---|
| `Off` | `/` | `Editing` | Clear query, clear matches, render `SearchBarWidget` |
| `Editing` | typing | `Editing` | Append char to query (Ctrl+U clears) |
| `Editing` | `Enter` | `Active` | Run `find_matches(lines, query)`, jump to first |
| `Editing` | `Esc` | `Off` | Drop query + matches |
| `Active` | `n` / `N` | `Active` | Wrap-around next / prev match |
| `Active` | `/` | `Editing` | Start a new query |
| `Active` | `Esc` | `Off` | Clear highlights |

## Smart-case

```rust
pub fn is_smart_case(&self) -> bool {
    self.query.chars().any(|c| c.is_uppercase())
}
```

- All-lowercase query → case-insensitive search (`"hello"` matches `Hello`, `HELLO`, `hello`).
- Any uppercase char → case-sensitive search (`"Hello"` matches only `Hello`).

Mirrors `ripgrep`'s `--smart-case` behavior. No regex — literal substring only.

## Match search

`find_matches(lines, query)` walks `Vec<LayoutLine>`. For each line:

1. Concatenate every `span.text` into a flat `text: String`.
2. Decompose to `Vec<char>` (UTF-8 safe — no byte-offset panics).
3. Slide-window compare against the needle chars. **Non-overlapping**: on match, advance by `needle_len`, not 1.

```rust
SearchMatch { line_index, start_col, length }
```

`start_col` and `length` are **char offsets**, not bytes. The highlight pipeline splits styled spans at char boundaries to apply the match background.

## Highlight rendering

`highlight_line_with_matches(line, matches, current, theme) -> LayoutLine` produces a new `LayoutLine` where every matched range gets:

- `bg = Some(theme.colors.search_match)` — soft band of color
- `modifier |= Modifier::BOLD` if it's the *current* match (the one `n`/`N` lands on)

The split walks each `StyledSpan`, computes the cuts for any overlapping match range, and emits sub-spans with the highlight applied. Non-match cells keep their original fg/modifier.

Per-frame cost: only the visible window is rehighlighted (`main.rs::render_lines_with_search` calls it inside the layout-to-paint pipeline). Hidden lines aren't touched.

## Status bar

`SearchBarWidget` paints the bottom row when `mode != Off`:

```
  /<query>█                                          5/127 matches
```

- Editing: trailing `█` cursor block, no count yet.
- Active with matches: `cur/total matches` right-aligned in dim color.
- Active with zero matches: `  no matches for <query>` in the warning color.

## What is *not* implemented

| Feature | Why |
|---|---|
| Regex | Literal substring is enough for prose; regex bloats the input UX. |
| Multi-line match | Search operates per `LayoutLine`. Hard wraps split words mid-search by design. |
| Search in headings only / code only | YAGNI. Filter by reading. |
| Replace | Veol is a reader. See [`../decisions/0004-no-buffer-editing.md`](../decisions/0004-no-buffer-editing.md). |
