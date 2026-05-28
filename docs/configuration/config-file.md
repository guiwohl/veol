## Config File
`~/.config/veol/config.toml` — every setting, every default, loaded on startup.

> **Related**
> - [`cli-flags.md`](cli-flags.md) — flags override config values
> - [`../theming.md`](../theming.md) — `theme` field picks the color scheme
> - [`../keybindings.md`](../keybindings.md) — `[keymap]` table customizes hotkeys

Source: `src/config.rs`. XDG-resolved via `directories::ProjectDirs` — on Linux `$XDG_CONFIG_HOME/veol/config.toml`, on macOS `~/Library/Application Support/veol/config.toml`, on Windows `%APPDATA%\veol\config.toml`.

Missing file: defaults are used silently.
Malformed file: warning logged to `~/.cache/veol/veol.log`, defaults used.

## Full reference

```toml
# ~/.config/veol/config.toml

theme = "reedo-dark"          # any bundled name or custom from ~/.config/veol/themes/<name>.toml
image_protocol = "auto"       # reserved — no image rendering today, see L11 in .claude/CLAUDE.md
mermaid = true                # render mermaid blocks; set false (or pass --no-mermaid) to show source
mermaid_timeout_ms = 1500     # vestigial from mmdc era; the pure-Rust renderer is synchronous
show_statusline = true        # bottom status bar (filename, line, %)
show_toc = false              # auto-open the TOC popup on startup
wrap = true                   # word-wrap paragraphs (only false is mostly untested)
frontmatter = true            # render YAML / TOML frontmatter as a header card
watch = true                  # poll file mtime every 500ms; --no-watch overrides

[cache]
dir = "~/.cache/veol"         # currently only used for the log file
max_size_mb = 128             # vestigial — no cache eviction today

[keymap]
quit = "q"
search = "/"
toggle_toc = "t"
toggle_mermaid = "m"
toggle_browser = "Ctrl+E"
toggle_theme = "Ctrl+T"
```

## Defaults — at a glance

| Field | Default | Why |
|---|---|---|
| `theme` | `"reedo-dark"` | Matches reedo, Veol's editor companion. |
| `image_protocol` | `"auto"` | Placeholder — no image rendering today. Pinned for future use. |
| `mermaid` | `true` | The whole point of Veol's diagram support. |
| `mermaid_timeout_ms` | `1500` | Vestigial — pure-Rust ASCII renderer is synchronous and instant. Kept to avoid breaking older config files. |
| `show_statusline` | `true` | Operators want to know filename + position. |
| `show_toc` | `false` | Modal-on-demand; auto-opening clutters narrow terminals. |
| `wrap` | `true` | Long paragraphs become unreadable without it. |
| `frontmatter` | `true` | YAML/TOML frontmatter is metadata users wrote on purpose. |
| `watch` | `true` | Edit elsewhere, see it here. 500ms poll. |
| `cache.max_size_mb` | `128` | Vestigial — see `dir`. |

## Loading + saving

`Config::load()` reads the file; on any error returns `Config::default()`. `Config::save()` uses `tempfile::NamedTempFile` + `persist` for atomic writes — no half-written file if the process crashes mid-save.

The only auto-save today is `Config::update_theme(name)`, triggered by the theme switcher popup committing a selection.

## Partial files

Every field uses `#[serde(default = "...")]` so the file can omit anything:

```toml
# minimal valid config
theme = "gruvbox"
```

Missing fields fall back to defaults. Tests pin this — see `partial_config_uses_defaults_for_missing_fields`.

## Custom themes

Themes are *not* in `config.toml`. They live in `~/.config/veol/themes/<name>.toml`. `config.toml`'s `theme` field just names one. See [`../theming.md`](../theming.md).
