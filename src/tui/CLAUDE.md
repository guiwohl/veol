# tui/

TUI widgets: document paint loop, viewport math, search, TOC, file browser, theme switcher, help. For the full key table see [`docs/keybindings.md`](../../docs/keybindings.md).

## Files

| File | Purpose |
|---|---|
| `mod.rs` | Re-exports every public widget + state type. |
| `draw.rs` | `DocumentWidget` (paints `viewport.top_line..top_line+height` from `LayoutLine`s), `StatusBarWidget` (`name  pct%  line cur/total  flash`), `WelcomeWidget` (centered, shown when `lines.is_empty()`). |
| `viewport.rs` | `ViewportState { top_line, height, total_lines, cursor_line }`. Scroll / page / paragraph / heading navigation. `cursor_line` is reserved — Veol's reader has no cursor. |
| `keymap.rs` | `Action` enum + `map_reader_key(KeyEvent) -> Action`. Flat keybinds — reader is modeless. Modes only exist inside browser. |
| `search.rs` | `SearchState` (Off / Editing / Active), `find_matches`, `highlight_line_with_matches`. Smart-case: any uppercase char → case-sensitive. |
| `toc.rs` | `TocState::build(lines, blocks)` walks blocks flat-first, anchors via `LayoutLine.heading_anchor`. `TocWidget` paints the modal. |
| `browser.rs` | `BrowserState`, `TreeAction` (None / NewFile / NewFolder / Rename / Delete), `FsOperation` (Move/Create/Delete/Rename undo stack), `SKIP_DIRS = [".git", "node_modules", "target"]`. `BrowserWidget` paints the popup; folder colors auto-cycle from `FOLDER_PALETTE` (8 entries). |
| `theme_switcher.rs` | `ThemeSwitcherState` with `preview_cache: HashMap<String, [Color; 6]>`. `SwitcherOutcome::{Persisted, Cancelled, StillOpen}`. Dot preview = `[heading_1, code_border, link, quote_marker, table_border, popup_accent]`. |
| `help.rs` | `HelpState` (zero-size — just a marker). `keybinds()` returns a `&'static [(key, desc)]` slice. Any key closes it (handled in `main.rs`). |

## Key design decisions

- **Mouse capture is ON by default (re-enabled 2026-05-27)**, opt out with `--no-mouse`. The rationale: most users want scroll-wheel scrolling, and crossterm's mouse capture works fine alongside reader-mode keybinds. The `--no-mouse` opt-out exists because mouse capture disables native terminal text-selection — power users copying mermaid renders to clipboard need it off. There's a stale `#[allow(dead_code)] // Kept for a future --mouse flag; mouse capture is currently disabled` comment in `main.rs::handle_mouse` from when mouse was wholly disabled — the comment is wrong but the `allow(dead_code)` stays because the call site is conditional. Don't "fix" the comment without rewriting the surrounding logic.
- **Reader is modeless. Browser has two sub-modes.** All reader keys are flat (`j`, `k`, `/`, `n`, `t`, `r`, `m`, `f`, `o`, `?`, `Ctrl+E`, `Ctrl+T`, `Esc`, `q`). Inside the browser, `state.action != TreeAction::None` flips to input mode (typing → buffer, `Enter` confirms, `Esc` cancels back to nav). This is the ONLY mode boundary in the app.
- **Paragraph navigation walks blank lines, not block boundaries** — `viewport::is_blank(line)` returns true iff every span's `text.trim()` is empty. `next_paragraph` skips current block (until blank), then skips blanks (until non-blank). This works because `layout::push_blank` emits a single empty-span line between blocks. If you ever change `push_blank` to emit a colored separator, update `is_blank` accordingly.
- **`DocumentWidget` paints `indent_cols` per line, not per block** — each `LayoutLine` carries its own `indent_cols: u16` set by layout. Prose centering (≥ 120 cols) uses this; code fences and tables stay full-bleed via `indent_cols = 0`. The paint loop in `draw.rs::DocumentWidget::render` does `start_x = area.x + line.indent_cols` and truncates at `area.x + area.width`.
- **Theme switcher caches the 6-dot preview per name** — `preview_cache: HashMap<String, [Color; 6]>` in `ThemeSwitcherState`. Re-rendering on `j`/`k` move is free after the first selection. Cache is per-popup-instance (cleared on close).
- **Browser popup walks the filesystem on open, not lazily** — `BrowserState::build_tree` does a recursive walk respecting `SKIP_DIRS` and filters to `.md` files + dirs that recursively contain `.md`. Cached per popup session. CRUD operations call `BrowserState::rebuild_tree` after mutations.

## Gotchas

- `map_reader_key` returns `Action::None` for any unrecognized key — caller must NOT treat None as "no action needed", it specifically means "no mapping". The reader paint loop does nothing on `None`, but if you add a "default action" handler you'll break the modal popups (which check `Action` per-popup with different mappings).
- The `Esc` key is layered: closes one popup level per press. `main.rs::handle_key` dispatches `Action::Escape` to the active popup's handler, which closes itself. Search has its own Esc semantics (cancel mode, drop matches).
- `BrowserState`'s undo stack is per-session (cleared when the browser popup is reopened from scratch? — no, it persists with the state). Check `app.rs::App::browser` lifetime if you change this.
- `SearchMode::Active` reuses `n`/`N` from the reader keymap — there's no separate input handler for these. If you add a new reader key like `n` for "negate", search will steal it. The mapping order matters.
- `ThemeSwitcherState::SwitcherOutcome::Cancelled` carries the ORIGINAL theme (snapshot taken at popup open); `Persisted` carries the NEW theme. `App::apply_switcher_outcome` swaps in whichever and re-layouts.
- `TocState::build` deduplicates anchors via `consumed: Vec<bool>` — if two headings have the same slug (`# Foo` then `## Foo`), the second one gets a fresh `line_index` but the first wins for the duplicate slug. Same heading text → same anchor → same first match.

## How to extend

- New reader key: add to `Action` enum, add to `map_reader_key`, handle in `app.rs::App::apply_action` (or wherever the dispatch lives). Add to `help.rs::keybinds()` AND `docs/keybindings.md`.
- New popup: add a `Popup::<Name>` variant in `app.rs`, owned state field on `App`, paint arm in `main.rs::draw_frame`, key dispatch arm in `main.rs::handle_key`. Mirror `theme_switcher` as the template — it's the cleanest popup with proper Outcome enum.
- Mouse handler logic: `main.rs::handle_mouse` is the only place mouse events land; the `MouseAction` mapping is in `src/input.rs::map_mouse`. Don't add mouse-only features (the spec emphasizes keyboard-first).
