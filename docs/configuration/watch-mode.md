## Watch Mode
500ms mtime polling for live reload; opt out with `--no-watch`.

> **Related**
> - [`cli-flags.md`](cli-flags.md) — `--no-watch` flag
> - [`config-file.md`](config-file.md) — `watch = true|false`

Implementation: `src/watcher.rs` (~250 lines, all in stdlib + `filetime`).

## How it works

Each frame in the TUI event loop (`main.rs::event_loop`):

```rust
app.poll_watcher();
```

`poll_watcher` calls `Watcher::poll()`, which:

1. Bails early if `enabled == false`.
2. Bails early if `Instant::now() - last_check < interval` (default 500ms).
3. `std::fs::metadata(path)?.modified()?` — fail-quiet on missing file or unsupported FS.
4. Compares against `last_mtime`:
   - First successful poll: store baseline, return `false` (no change to report).
   - Same mtime as last: return `false`.
   - Different mtime: update baseline, return `true`.

On `true`, the app reloads from disk, re-parses, re-lays out, resets `DisplayRegistry`, and clamps `viewport.top_line`.

## Why 500ms

A balance between responsiveness and CPU. At 500ms:

- Operator perceives "instant" — typical editor save → veol redraw lag is under one frame.
- Polling cost is trivial: one stat call every 500ms is invisible in any profile.
- Faster (e.g., 100ms) makes large directories noisier on `ls -l`-style FS notifications; mtime resolution on some filesystems is 1s anyway, so sub-second polling is fake precision.

Slower (1s+) made operator A/B testing feel laggy.

## Why polling, not inotify

`notify` crate would give event-driven file watching, but:

- Cross-platform inotify abstractions add ~30 deps (`notify` pulls `mio`, `tokio`-adjacent crates, FSEvents bindings, win32 kernel APIs).
- Veol opens one file. The cost of one stat-per-500ms is negligible — polling pays for itself.
- Polling has no edge cases around file-replace, atomic-rename, editor-temp-files. mtime is the only contract; the kernel tracks it for free.
- See [`../decisions/0001-no-async-runtime.md`](../decisions/0001-no-async-runtime.md) — Veol avoids the async ecosystem entirely.

## Opt-out paths

| How | Effect |
|---|---|
| `--no-watch` | Constructs `Watcher::disabled(path)`. `poll()` short-circuits. |
| `watch = false` in `config.toml` | Same as `--no-watch`. |
| Reading stdin (`-`) | `Watcher` is never created — no file path to poll. |
| File at startup doesn't exist | Watcher is never created; `cli.rs` errors out earlier. |

## Auto-rebase on reload

After a successful reload, `Watcher::rebaseline()` clears `last_mtime` and `last_check` so the next poll re-establishes baseline. This avoids double-triggering when the file is touched twice in quick succession (the second touch resets the baseline cleanly).

## Behavior table

| Scenario | `poll()` returns |
|---|---|
| Watcher disabled | `false` always |
| Within interval | `false` |
| First poll after interval | `false` + baseline established |
| File untouched since baseline | `false` |
| File mtime advanced | `true` (and baseline updated) |
| File no longer exists | `false` (and `last_seen()` stays at previous value) |
| File mode unchanged but content edited via in-place write | `true` if the editor bumped mtime, else `false` |

Most editors bump mtime on save (Vim, VSCode, neovim, `cat > foo.md`). `sed -i` does too. If your tool doesn't, the watcher won't fire — that's the tool's bug, not Veol's.
