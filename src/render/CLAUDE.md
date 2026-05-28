# render/

Theme system + code-fence syntax highlighting. Despite the name, **most actual rendering happens in `markdown::layout`** — this module owns the `Theme` struct, the syntect bridge, and the 9 bundled `*.toml`. For color field reference + custom theme format see [`docs/theming.md`](../../docs/theming.md).

## Files

| File | Purpose |
|---|---|
| `mod.rs` | Re-exports `CodeHighlighter`, `HIGHLIGHTER`. That's it. |
| `theme.rs` | `Theme`, `ColorMap`, `Color` (Rgb / Ansi / Reset). `Theme::load(name)` — custom dir first, then bundled. `parse`, `list_all`, `list_custom`, `bundled_names`. `Color::parse` accepts `#rrggbb`, ANSI names, `default`/`reset`, `bright-*` variants. |
| `code.rs` | `CodeHighlighter` wrapping syntect's `SyntaxSet` + `ThemeSet`. Single global via `HIGHLIGHTER: LazyLock`. Always uses `base16-ocean.dark` syntect theme, overlays Veol's `code_bg` from the active theme. |
| `table.rs` | **EMPTY** (1 byte). Table rendering lives in `markdown::layout::render_table` + `compute_table_widths`. |
| `text.rs` | **EMPTY** (1 byte). Inline span rendering lives in `markdown::layout::spans_to_tokens` + `wrap_tokens_into_lines`. |
| `themes/*.toml` | 9 bundled theme files, `include_str!`'d into `BUNDLED` constant. |

## Key design decisions

- **`syntect` uses `default-fancy` feature** (`Cargo.toml`) — pulls the full bundled syntax + theme set with onig regex but stripped of the slow Sublime preferences. ~80 languages. If you ever swap to `default-syntaxes-only`, the `base16-ocean.dark` theme stops loading and `code.rs::syn_theme` panics on index.
- **Syntect theme is hardcoded to `base16-ocean.dark`** — `SYNTECT_THEME: &str = "base16-ocean.dark"` in `code.rs`. Per-Veol-theme syntect themes would mean shipping 9× the theme data; instead the `code_bg` from the Veol theme paints the background and syntect provides the FG only. Side effect: code fences look "the same" across all 9 themes except for the background tint.
- **`Color::parse` is permissive on case but strict on hex format** — `"#7AA2F7"` works, `"#7af"` does not (exactly 6 digits required). All ANSI names accept `bright-red`/`bright_red` (both kebab and snake). `gray` and `grey` both alias to `Color::Gray`. Anything else returns a parse error with a helpful message — never falls back silently.
- **`Color::Reset` short-circuits the bg paint in `tui::draw::DocumentWidget`** — if `bg == Color::Reset`, the widget skips `buf.set_style(area, ...).bg(bg)`. The `default` theme uses Reset for `bg` AND `fg` so it inherits the terminal palette wholesale (ligatures, scrollback, the lot).
- **Every theme field has a `serde(default = "...")` fallback to the `default` theme value** (see top of `theme.rs`). Custom TOML themes only need to list the fields they override — partial themes are first-class.

## Gotchas

- `CodeHighlighter::highlight_line` and `highlight_lines` both append `\n` before calling syntect (which expects line-terminated input) and strip it off after. If a span ends with `\n` and you didn't add it, syntect highlighting will eat the last char of your code. Don't change the `if line.ends_with('\n')` check without re-running `code.rs` tests.
- `HIGHLIGHTER` is a `LazyLock<CodeHighlighter>` — the first code-fence in the first document triggers a ~50ms syntect load. Acceptable for a reader (one-shot cost) but if you ever benchmark startup, this is why first-paint can be slow.
- Custom theme dir is `~/.config/veol/themes/` (XDG, via `directories` crate). `Theme::load` checks here FIRST — a custom theme named `dracula` shadows the bundled one. Useful for users tweaking a bundled theme; intentional.
- The empty `table.rs` and `text.rs` files exist as namespace placeholders from the original module split. If you ever move the table/text helpers out of `markdown::layout`, this is where they go — but don't move them speculatively (P2: no premature generalization).

## How to extend

- New bundled theme: drop `themes/foo.toml`, add `("foo", include_str!("themes/foo.toml"))` to `BUNDLED`, add `"foo"` to `Theme::bundled_names()` AND `cli::BUNDLED_THEMES` (both arrays need to stay in sync — there's no single source of truth; one is for runtime lookup, the other for `--theme-list` ordering).
- New color field: add it to `ColorMap`, add `#[serde(default = "default_<field>")]`, write the `default_<field>` fn, then add the field to every bundled `*.toml` so the defaults are explicit. Document in `docs/theming.md`.
