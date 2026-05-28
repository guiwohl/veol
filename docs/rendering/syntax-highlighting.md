## Syntax Highlighting
syntect powers every fenced code block; one shared instance, baked-in syntaxes, baked-in theme.

> **Related**
> - [`code-blocks.md`](code-blocks.md) — how fences are wrapped into LayoutLines
> - [`pipeline.md`](pipeline.md) — where highlighting sits in the paint path
> - [`../theming.md`](../theming.md) — `code_bg` and `code_border` color fields

## Crate setup

`Cargo.toml`:

```toml
syntect = { version = "5", default-features = false, features = ["default-fancy"] }
```

`default-fancy` pulls in `fancy-regex` instead of the heavy `onig` (Oniguruma C bindings). That keeps the build pure-Rust — no system library dependency, no `cc` toolchain at link time.

It also bundles the default syntax + theme dump files, so `SyntaxSet::load_defaults_newlines()` and `ThemeSet::load_defaults()` work offline with zero config.

## The highlighter

`src/render/code.rs` exposes a single static:

```rust
pub static HIGHLIGHTER: LazyLock<CodeHighlighter> = LazyLock::new(CodeHighlighter::new);
```

`CodeHighlighter` owns a `SyntaxSet` and a `ThemeSet`. Construction loads ~250 syntaxes and a handful of bundled themes. Cost is paid once on the first `highlight_lines` call.

Theme selection is hardcoded:

```rust
const SYNTECT_THEME: &str = "base16-ocean.dark";
```

Veol's UI theme (heading colors, link, quote, …) is independent of the syntect color palette. Mixing them was tried and produced muddy contrast on light themes — the syntect theme is locked dark because the `code_bg` token in every Veol theme is dark.

## Language detection

`CodeHighlighter::resolve_syntax(lang: Option<&str>)`:

1. `syntax_set.find_syntax_by_token(lang)` — exact token (`"rust"`, `"py"`, `"go"`).
2. `syntax_set.find_syntax_by_extension(lang)` — when the fence tag is a file extension (`"rs"`, `"tsx"`).
3. `syntax_set.find_syntax_by_name(lang)` — full name (`"Rust"`, `"TypeScript"`).
4. Fallback: `find_syntax_plain_text()` — unstyled.

Unknown languages render as plain text without panic. Empty / missing fence tag also falls through to plain.

## Per-line vs. per-block highlighting

Two entry points:

| Method | Use | Why |
|---|---|---|
| `highlight_line` | One-shot, no syntax state | Used internally for the lang label row. |
| `highlight_lines` | Multi-line fence body | Maintains `HighlightLines` state across lines so block comments, heredocs, and multi-line strings render correctly. |

`render_code_block` (`src/markdown/layout.rs:420`) always uses `highlight_lines` so a `/* … */` opened on line 1 keeps coloring through line 3.

## Span shape

Each syntect range becomes a `StyledSpan`:

```rust
StyledSpan {
    text,
    fg: Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b),
    bg: Some(code_bg_from_veol_theme),
    modifier: font_style_to_modifier(style.font_style),
    link: None,
}
```

`fg` is RGB straight from syntect's theme. `bg` is **always** overridden to the Veol theme's `code_bg`, which means a single fence has uniform background regardless of which token colors syntect emits.

`font_style_to_modifier` maps `BOLD`/`ITALIC`/`UNDERLINE` from syntect into ratatui's `Modifier`.

## Trailing-newline quirk

syntect's `highlight_line` requires a `\n`-terminated input. The wrapper appends one if missing, then pops the trailing `\n` from the last styled span and drops the span entirely if it becomes empty:

```rust
if last.text.ends_with('\n') {
    last.text.pop();
    if last.text.is_empty() { spans.pop(); }
}
```

Without this, every fence line ended in a phantom newline that ratatui would happily paint as an extra cell.

## When highlighting is skipped

Empty `line` returns `Vec::new()` immediately — `emit_code_line` paints a width-of-spaces row with the `code_bg` so the fence chrome stays contiguous. The lang label row (`" rust "` etc.) is never sent through syntect.
