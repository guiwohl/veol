# Theming

Veol ships 9 themes ported from reedo + a TOML format for custom themes at `~/.config/veol/themes/*.toml`. Switch at runtime with `Ctrl+T` (persists to `~/.config/veol/config.toml`) or pass `--theme NAME` at launch. List with `--theme-list`.

## Bundled themes

From `src/cli.rs::BUNDLED_THEMES` and `src/render/theme.rs::BUNDLED`:

| Name | Style |
|---|---|
| `default` | Terminal defaults — `bg` and `fg` are `Color::Reset` (inherits terminal palette) |
| `reedo-dark` | Tokyo-night-ish dark; default if config missing |
| `reedo-light` | Light variant |
| `catppuccin` | Catppuccin Mocha |
| `dracula` | Dracula |
| `gruvbox` | Gruvbox dark |
| `nord` | Nord palette |
| `rose-pine` | Rosé Pine main |
| `solarized-dark` | Solarized dark |

## Color fields

Every theme is a TOML file with `name = "..."` and a `[colors]` table. Every field defaults via `serde(default = "...")` to the `default` theme's value, so partial themes work — only override what you want.

### Inherited from reedo (syntax-ish, also drive code fence accents)

| Field | Where it shows up |
|---|---|
| `bg` / `fg` | Document background / default text |
| `gutter` | Reserved for line numbers (when `--line-numbers` lands) |
| `cursor_bg` / `cursor_fg` | Reserved (reader has no cursor) |
| `selection` | Reserved (no buffer selection) |
| `statusbar_bg` / `statusbar_fg` | Bottom status line |
| `keyword` / `string` / `comment` / `function` / `type` / `number` / `operator` / `property` | Reserved — syntect overrides via `base16-ocean.dark` for code fences (see [src/render/CLAUDE.md](../src/render/CLAUDE.md)) |

### Markdown-specific (added in Veol)

| Field | Where it shows up |
|---|---|
| `heading_1`..`heading_6` | Per-level heading color (also `Modifier::BOLD`) |
| `link` | Inline link text (also `Modifier::UNDERLINED`) |
| `quote_marker` | The `│` prefix in blockquotes |
| `quote_text` | Body text inside blockquotes |
| `code_bg` | Background fill for code fences and inline `code` spans |
| `code_border` | Language label color above a code fence |
| `table_border` | `─ │ ┌ ┐ └ ┘ ┬ ┴ ├ ┤ ┼` chars in tables |
| `table_header_fg` | Header-row text color |
| `hr` | Horizontal rule (`---` in source) |
| `task_done` / `task_pending` | `[x]` / `[ ]` markers in task lists |
| `mermaid_caption` | Default fg for the entire mermaid block (per-shape colors override) |
| `search_match` | Highlight bg for `/`-search matches (current match: inverted) |

### Popup chrome

| Field | Where it shows up |
|---|---|
| `popup_bg` | File browser / TOC / theme switcher / help panel fill |
| `popup_border` | Modal border chars |
| `popup_selected` | Selected-row bg in any list popup |
| `popup_dim` | De-emphasized helper text (also strikethrough span color) |
| `popup_accent` | Title row, hint key markers |

## Color values

From `src/render/theme.rs::Color::parse`:

- Hex RGB: `"#7aa2f7"` (exactly 6 hex digits, leading `#`)
- ANSI names: `black red green yellow blue magenta cyan white` / `gray` / `grey` (= white)
- Bright ANSI: `bright-red`, `bright_red`, `bright-yellow`, … (16 standard)
- Terminal default: `"default"` or `"reset"` → ratatui `Color::Reset`

Anything else → parse error with a clear message.

## Minimal custom theme

`~/.config/veol/themes/midnight.toml`:

```toml
name = "midnight"

[colors]
bg = "#0a0a14"
fg = "#d0d4e0"
heading_1 = "#7aa2f7"
heading_2 = "#bb9af7"
link = "#9ece6a"
code_bg = "#15151f"
mermaid_caption = "#7dcfff"
```

Load with `veol --theme midnight FILE.md`. Switch live: `Ctrl+T`, select, `Enter`.

Custom themes shadow bundled ones with the same name (the custom dir is checked first by `Theme::load`).

## Theme switcher UX

`Ctrl+T` opens a centered modal listing every theme (bundled + custom, sorted). For each row, six color dots preview the theme via `Theme::preview_dots()`:

```rust
[heading_1, code_border, link, quote_marker, table_border, popup_accent]
```

Picked because those six are usually the most visually distinct fields. The preview is cached per name (in `ThemeSwitcherState::preview_cache`) so re-rendering on selection move is free.

`Enter` calls `Config::save_with_theme(name)` — atomic write via `tempfile::NamedTempFile::persist` — then immediately applies the new theme to the running session (re-layouts the document).

`Esc` cancels — the cached preview ColorMap is discarded but no config write happens.

## Default theme behavior

`name = "default"` sets `bg` and `fg` to `Color::Reset`. The paint loop in `tui::draw::DocumentWidget` then skips `buf.set_style(area, Style::default().bg(bg))`. Result: Veol inherits the terminal palette (background, scrollback, ligatures, italics — whatever your terminal does for plain text). Use this for tools like `alacritty` or `kitty` where you already love the colorscheme.
