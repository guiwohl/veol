## Code Blocks
Fenced code rendering: language tag header, syntect-highlighted body, full-width background, char-wrap on overflow.

> **Related**
> - [`syntax-highlighting.md`](syntax-highlighting.md) — syntect mechanics
> - [`pipeline.md`](pipeline.md) — where code blocks sit in layout
> - [`../theming.md`](../theming.md) — `code_bg`, `code_border` fields

Implementation: `render_code_block` + `emit_code_line` at `src/markdown/layout.rs:420-505`.

## Visual structure

```
<lang> ─────────────────────────────────────────────  ← lang label row (theme.code_border on code_bg)
fn main() {                                            ← syntect-highlighted body
    println!("hello");                                  bg = theme.code_bg
}                                                      fg = syntect RGB
                                                       ← trailing blank
```

The lang label is only emitted if the info string is non-empty after `trim`. ` ```rust ` → header. ` ``` ` → no header, body only.

## Wrapping logic

`emit_code_line` walks each `StyledSpan` char-by-char (not by byte — UTF-8 safe). When `current_width >= width`, it pads the rest of the row to `width` with spaces (background still `code_bg`), pushes the line, and starts a new line with a 2-space soft-wrap continuation indent:

```rust
let prepare = |first: bool, cur, w| {
    if !first { cur.push(span_with_bg("  ", fg, bg)); *w += 2; }
};
```

Wrapped continuation rows are visually distinguishable by the indent. No line is ever truncated.

## Span coalescing

To minimize the number of `StyledSpan` ratatui has to paint, `emit_code_line` merges consecutive chars with identical `(fg, bg, modifier, link)` into one span. Pseudocode:

```rust
if last.fg == s.fg && last.bg == Some(span_bg) && last.modifier == s.modifier && last.link == s.link {
    last.text.push(ch);
}
```

For a 200-line fence with low color churn this drops span count from ~5000 to a few hundred.

## Background-fill discipline

Every cell in a code block carries `bg = Some(theme.code_bg)`. Even the padding at end-of-line and the blank row after the fence. Without this, terminal background bleeds into the fence rectangle and the visual block dissolves. The chrome row (lang label) gets the same `bg` to keep the rectangle continuous.

## Indentation handling

`pulldown-cmark` strips the leading whitespace of indented code blocks but preserves it inside fenced blocks. Veol does not re-indent — whatever the source has, it paints. The 2-space soft-wrap continuation is the only artificial indentation.

## Empty lines inside fences

`code.split('\n')` (used by `highlight_lines`) yields empty strings for blank lines, which `highlight_line` short-circuits to `Vec::new()`. `emit_code_line` then paints a width-of-spaces row with `code_bg`, preserving the empty line visually.

## Mermaid is not a code block

` ```mermaid ` fences are intercepted by `parser::is_mermaid_info` and become `Block::MermaidBlock`, not `Block::CodeBlock`. The ASCII renderer takes over from there. See [`../diagrams/architecture-pivot.md`](../diagrams/architecture-pivot.md) for the dispatch logic.

When `--no-mermaid` is set, the parser still emits `MermaidBlock` but `layout::render_mermaid` falls back to the same path as a regular fence with `lang = "mermaid"`. The user sees the source, syntax-highlighted as plain text (no `mermaid` syntect grammar exists, so `find_syntax_plain_text()` wins).
