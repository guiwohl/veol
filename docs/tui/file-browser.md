## File Browser
`Ctrl+E` modal — reedo-style tree, filtered to `.md`, navigation aid only.

> **Related**
> - [`../keybindings.md`](../keybindings.md) — full key table
> - [`../decisions/0004-no-buffer-editing.md`](../decisions/0004-no-buffer-editing.md) — why CRUD exists but the buffer is immutable
> - [`../../src/tui/CLAUDE.md`](../../src/tui/CLAUDE.md) — widget implementation notes

Implementation: `src/tui/browser.rs` (~600 lines). State on `App.browser: BrowserState`.

## Filter rules

`build_dir` in `browser.rs:118` walks the current root and includes:

- Files where `is_md_file(path)` (extension `.md` or `.markdown`).
- Directories that recursively *contain* at least one `.md` file (`compute_md_dirs`).

Skipped unconditionally:

```rust
pub const SKIP_DIRS: &[&str] = &[".git", "node_modules", "target"];
```

The filter exists because Veol is a reader for documentation. Showing every `.js` and `.png` in a frontend repo is noise — `Ctrl+E` should pop a clean tree of just the prose.

## Navigation

| Key | Action |
|---|---|
| `j` / `k` / arrows | Move selection by 1 |
| `Enter` | Open file (replaces current doc); toggle dir open/closed |
| `Backspace` | Hint scope back one level |
| `1`-`9` | Hint-jump: open the Nth visible entry in scope |
| `Esc` | Close popup |

The hint system reuses reedo's pattern — single-digit hotkeys for the visible entries. `compute_hints` recomputes the index map after every move so digits track the current scope.

## CRUD (navigation aid only)

| Key | Action | Confirm |
|---|---|---|
| `n` | New file (auto-appends `.md` if missing) | `Enter` |
| `f` | New folder | `Enter` |
| `r` | Rename selected entry | `Enter` |
| `d` | Delete selected entry | `y` / `n` |
| `m` | Mark for move; next `m` on a folder moves marked → here | — |
| `Ctrl+Z` | Undo last FS op | — |
| `Ctrl+Y` | Redo last FS op | — |

`normalize_md_name` (used by `confirm_new_file`) enforces `.md` extension. Other extensions return an error and the action cancels.

## Undo stack

`FsOperation` variants — `Move`, `Create`, `Delete`, `Rename` — get pushed to `fs_undo_stack` on every successful mutation. `Delete` snapshots file content so undo can restore it. Stack is per-`BrowserState` instance; survives across popup open/close but resets when `App.browser` is recreated.

This is the only mutable-state path in Veol. **It does not touch the buffer.** Editing a `.md` file's content is out of scope — only its existence/name/location. See [`../decisions/0004-no-buffer-editing.md`](../decisions/0004-no-buffer-editing.md).

## Reedo lineage

The browser was modeled directly on `reedo/src/ui/tree.rs`:

- `TreeState` → `BrowserState`
- `FsOperation` enum copied verbatim (variants + meaning)
- Folder color cycling via 8-entry `FOLDER_PALETTE` (Catppuccin Mocha rainbow)
- Hint-jump (`1`-`9`) is reedo's, lifted unchanged
- Popup widget pattern (border + title + entries + hint footer) mirrors reedo's tree widget

What Veol intentionally *dropped* from reedo:

- Git-status decoration (Veol is not git-aware)
- `.gitignore` walker (`SKIP_DIRS` is the entire filter)
- Tree-sitter symbols (Veol doesn't parse non-Markdown files)
- Multi-pane editor integration (Veol has one buffer)

## Mouse

When mouse capture is on (default; opt out with `--no-mouse`), scroll-wheel events inside the popup scroll the entry list. Click-to-select is not implemented — keyboard-first.
