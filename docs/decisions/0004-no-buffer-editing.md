## ADR-0004: Veol is a reader, not an editor
The document buffer is immutable from the user's perspective. File browser CRUD is a navigation aid, not an editing feature.

- **Status:** Accepted
- **Date:** 2026-05-26

## Context

Reedo is the editing companion. Veol is the reader. They share themes, file-browser UX, and Rust stack, but the responsibility split is hard:

| Veol | Reedo |
|---|---|
| Reads `.md` files | Edits any text file |
| Paginated, modal | Multi-pane, modal editor |
| File browser sees only `.md` + dirs with `.md` | Full filesystem with git status |
| No buffer mutation | Full edit |
| No cursor inside text | Per-pane cursor |

The temptation: "what if Veol could just fix a typo inline?" The cost: every reader feature now has to coexist with an editor. Search becomes search-and-replace. The viewport becomes a cursor. Themes become syntax-highlighting palettes. File browser becomes a tab manager.

## Decision

**Veol never mutates buffer content.** The user reads. To edit, the user opens reedo (or their editor) and Veol's watch mode re-renders on save.

The file browser is allowed CRUD on `.md` files specifically because *managing where files live* is part of the reading workflow:

- Move a draft from `notes/` to `docs/published/`.
- Rename `untitled.md` to `2026-05-27-pivot.md`.
- Delete a stale draft.
- Create a new file to capture a thought, then immediately open it (still empty, still a reader).

The browser intentionally does NOT:

- Open files in an inline editor.
- Show modified state.
- Have a "save" key (no buffer to save).
- Allow renaming to non-`.md` extensions.

## Hard rule

From `.claude/CLAUDE.md` (P0):

> **Veol is a READER, not an editor.** Never add buffer-editing features. CRUD inside the file browser exists only to manage `.md` files between reads (navigation convenience). The buffer is immutable from the user's perspective.

## What stays out of scope

- Inline edit (Vim-like `i`, `o`, `a`, `cw`).
- Search-and-replace.
- Annotation / comment / highlight persistence.
- Bookmark anchors saved across sessions.
- Anything that writes to the source `.md` file.

If a feature request implies *modifying the rendered file's content*, it gets the "no" before we even discuss it.

## Consequences

**Positive:**

- Search, viewport, theme, and TOC stay tiny and focused.
- No need for an undo/redo stack on text (browser has one for FS ops).
- No conflict with watch mode — Veol never re-reads its own writes.
- Pairs cleanly with any external editor: edit in Reedo / Vim / VSCode, watch mode in Veol redraws.

**Accepted trade-offs:**

- Operators who want to fix one typo open a separate editor. Pivoting in/out is the cost of focus.

## Related

- [`../tui/file-browser.md`](../tui/file-browser.md) — what the browser does + does not do
- [`../tui/search.md`](../tui/search.md) — read-only search by design
- [`../configuration/watch-mode.md`](../configuration/watch-mode.md) — the "external editor" half of the loop
