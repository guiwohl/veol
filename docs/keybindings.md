# Keybindings

Pulled from `src/tui/keymap.rs::map_reader_key` and the popup handlers in `src/main.rs`. The reader is **modeless** — flat keybinds, no vim modes. Modes only exist *inside* the file browser (navigation vs input).

## Reader (default mode)

| Key | Action |
|---|---|
| `q` | Quit (no confirm — Veol is a reader, nothing to save) |
| `j` / `↓` | Scroll down 1 line |
| `k` / `↑` | Scroll up 1 line |
| `Space` | Page down (overlaps by 2 lines) |
| `b` | Page up (overlaps by 2 lines) |
| `d` | Half-page down |
| `u` | Half-page up |
| `PageDown` | Next paragraph (jump past blank-line block boundary) |
| `PageUp` | Previous paragraph |
| `g` | Jump to top |
| `G` / `Shift+g` | Jump to bottom |
| `]` | Next heading |
| `[` | Previous heading |
| `/` | Start search (smart-case literal) |
| `n` | Next match |
| `N` / `Shift+n` | Previous match |
| `t` | Toggle TOC modal |
| `r` | Reload current file |
| `m` | Toggle Mermaid render ↔ source for all diagrams |
| `f` | Toggle frontmatter card visibility |
| `o` | Open link under cursor externally (`$BROWSER` / `xdg-open`) |
| `?` | Toggle help overlay |
| `Esc` | Close popup → reader (layered: closes one popup level per press) |
| `Ctrl+E` | Toggle file browser popup |
| `Ctrl+T` | Toggle theme switcher popup |

## Search (Editing mode — `/` opened)

| Key | Action |
|---|---|
| typing | Append to query |
| `Backspace` | Delete last char |
| `Ctrl+U` | Clear query |
| `Enter` | Confirm — switch to Active (highlights all matches, cursor on first) |
| `Esc` | Cancel — drop matches, return to reader |

Smart-case: query with any uppercase char → case-sensitive. All-lowercase → case-insensitive.

## Search (Active mode — after Enter)

Reader keys all work; in addition:

| Key | Action |
|---|---|
| `n` / `N` | Next / previous match |
| `Esc` | Cancel — drop matches, return to reader |

## File browser (`Ctrl+E`)

Two sub-modes: navigation (default) and input (active when `state.action != TreeAction::None`).

### Navigation

| Key | Action |
|---|---|
| `j` / `↓` | Move selection down (scrolls when within 8 of edge) |
| `k` / `↑` | Move selection up |
| `Enter` | Open selected `.md` (replaces current document) or toggle folder |
| `n` | Start "new file" prompt (auto-appends `.md`) |
| `f` | Start "new folder" prompt |
| `r` | Start "rename" prompt |
| `d` | Start "delete" confirm |
| `m` | Mark selected for move; press `Enter` on destination folder to drop |
| `Ctrl+Z` | Undo last filesystem op (Move/Create/Delete/Rename) — per-session stack |
| `Ctrl+Y` | Redo last undone op |
| `1`–`9` | Hint jump — enter siblings of currently scoped folder |
| `Backspace` | Pop hint scope (back to parent) |
| `Esc` | Close browser |

### Input (after `n`/`f`/`r`/`d`)

| Key | Action |
|---|---|
| typing | Append to `input_buf` |
| `Backspace` | Delete last char |
| `Enter` | Confirm action |
| `Esc` | Cancel back to navigation |
| `y` / `n` | (Delete only) confirm / cancel without typing |

## TOC modal (`t`)

| Key | Action |
|---|---|
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `Enter` | Jump viewport to selected heading |
| `Esc` | Close TOC |

## Theme switcher (`Ctrl+T`)

| Key | Action |
|---|---|
| `j` / `↓` | Move selection down (color-dot preview updates) |
| `k` / `↑` | Move selection up |
| `Enter` | Apply theme + persist to `~/.config/veol/config.toml` |
| `Esc` | Cancel (revert preview, no persist) |

## Help overlay (`?`)

Any key closes it (handled by `Popup::Help => app.popup = Popup::None` in `main.rs::handle_key`).

## Mouse

Mouse capture is **enabled by default** (re-enabled 2026-05-27). Opt out with `--no-mouse` if you want native terminal text selection. See `src/tui/CLAUDE.md` for why.

| Action | Mapped to |
|---|---|
| Scroll wheel up (over doc) | Scroll up 3 lines |
| Scroll wheel down (over doc) | Scroll down 3 lines |
| Scroll wheel up (over popup) | Popup move up |
| Scroll wheel down (over popup) | Popup move down |

See `src/input.rs::map_mouse` for the full routing table.
