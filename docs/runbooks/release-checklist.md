## Release Checklist
Step-by-step gate before tagging a Veol release.

> **Related**
> - [`../contributing.md`](../contributing.md) — testing + commit conventions
> - [`snapshot-management.md`](snapshot-management.md) — insta workflow

## Pre-flight

```bash
git status                              # working tree must be clean
git pull --rebase origin main           # current with main
git log --oneline -10                   # eyeball recent commits
```

If anything is uncommitted, decide whether to commit it or stash. Don't release with a dirty tree.

## 1. The three gates

All three must pass clean:

```bash
cargo test --all
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Any failure: fix, commit, restart from step 1.

## 2. Snapshot review

```bash
cargo test --all                        # generates *.pending-snap files if any moved
cargo insta review
```

Walk every diff. Accept correct changes, reject regressions. See [`snapshot-management.md`](snapshot-management.md) for the full triage flow.

If snapshots accepted: commit them with the relevant code change, NOT separately:

```bash
git add src/**/snapshots tests/snapshots
git commit -m "test: update snapshots for <change>"
```

## 3. Release build

```bash
cargo build --release
./target/release/veol --version         # sanity check
./target/release/veol README.md         # smoke test interactively
./target/release/veol --plain examples/kitchen-sink.md | head -50
./target/release/veol --no-mermaid examples/kitchen-sink.md --plain | head -20
```

Things to look for in the smoke test:

- Welcome banner when invoked with no args.
- `q` quits cleanly (terminal not left in raw mode).
- `Ctrl+E` opens browser, `Esc` closes.
- `t` opens TOC, `Enter` jumps.
- `/foo` searches; `n`/`N` cycle.
- `--no-mouse` smoke test: scroll with arrow keys still works.

If terminal is left in raw mode, abort and fix the panic-hook / exit path before tagging.

## 4. Doc audit

```bash
grep -rn "TODO\|FIXME\|XXX" docs/      # outstanding doc TODOs?
```

Update `README.md` if the public surface changed (new flag, removed flag, version bump).

Update `docs/keybindings.md` if `src/tui/help.rs::keybinds()` changed.

Update `Cargo.toml` `version = "x.y.z"` per semver:
- Patch: bug fix, no public surface change.
- Minor: new flag, new theme, new diagram type (additive only).
- Major: removed flag, behavior change, schema change in `config.toml`.

## 5. Tag

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: release v<x.y.z>"
git tag -a v<x.y.z> -m "release v<x.y.z>"
git push origin main
git push origin v<x.y.z>
```

## 6. Verify

```bash
cargo install --path .                  # confirm install works
which veol                               # PATH points to the new binary
veol --version                           # matches the tag
```

## Rollback

If a release breaks something users hit:

```bash
git tag -d v<x.y.z>                     # delete local tag
git push origin :refs/tags/v<x.y.z>     # delete remote tag (DESTRUCTIVE)
git revert <release-commit>             # revert the version bump
git tag v<x.y.z-1>-recovery HEAD
git push origin main --tags
```

Then iterate on a fix branch and re-release.

## Checklist (copy-paste into PR / release notes)

```
- [ ] `cargo test --all` — all green
- [ ] `cargo clippy --all-targets -- -D warnings` — clean
- [ ] `cargo fmt --check` — formatted
- [ ] `cargo insta review` — snapshots triaged
- [ ] `cargo build --release` — builds clean
- [ ] Smoke test: interactive open / quit / Ctrl+E / TOC / search / --no-mouse / --plain
- [ ] README.md updated if public surface changed
- [ ] docs/keybindings.md matches src/tui/help.rs
- [ ] Cargo.toml version bumped per semver
- [ ] Tagged and pushed
```
