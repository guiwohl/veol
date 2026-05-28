## Snapshot Management
`insta` workflow: review, accept, regenerate.

> **Related**
> - [`../contributing.md`](../contributing.md) — TDD gates
> - [`release-checklist.md`](release-checklist.md) — snapshot review is part of releasing
> - [`adding-a-diagram-type.md`](adding-a-diagram-type.md) — new diagram types ship with their own snapshot dir

Veol uses [`insta`](https://insta.rs) for snapshot testing layout, render, and TUI widget output. Snapshots live next to the code:

- `src/markdown/snapshots/` — markdown layout tests
- `src/mermaid/ascii/<kind>/snapshots/` — per-diagram render tests
- `src/tui/snapshots/` — TUI widget paint tests (TOC, help, etc.)
- `tests/snapshots/` — integration tests

## After `cargo test --all`

Insta writes `*.snap.new` (or `*.pending-snap`) alongside the existing `*.snap`. The test fails until those are reviewed.

```bash
cargo test --all                        # generates pending snapshots
cargo insta pending-snapshots           # lists them
cargo insta review                      # interactive triage
```

The reviewer shows each diff with `a` (accept), `r` (reject), `s` (skip — leave pending), `q` (quit).

## When to accept

- Layout change you made on purpose: accept.
- Width math reshuffles columns: read the diff, confirm the column sums still match `usable`, accept.
- Theme color change: accept.
- New diagram-type variant: accept.

## When to reject

- Mermaid junction-merge regression (a `┼` became `─` or vice versa): reject, debug, fix.
- Table cells truncated where they used to wrap: reject, the wrap algorithm regressed.
- Lost trailing whitespace inside a colored row (a `StyledRun` got trimmed when it shouldn't): reject.
- Heading anchor changed (`section-21` → `section-2-1`): reject, you broke the GitHub-style slug rule.

## Force-regenerate

If you're certain the entire snapshot set should be reset (e.g., a theme rename), nuke and rebuild:

```bash
find src/ tests/ -name '*.snap' -delete
cargo test --all                        # generates fresh .snap.new everywhere
cargo insta review                      # walk them
```

This is **dangerous**. Don't do it without a clean tree and a careful diff review afterward. Real regressions look identical to "intended new snapshots" if you blanket-accept.

## CI behavior

Insta's `INSTA_FORCE_PASS=1` is **not** set in CI. Pending snapshots fail the suite. PRs that move snapshots must include the new `.snap` files in the same commit as the code change.

## File hygiene

- One `.snap` per test name. Insta auto-names from the function path + the test name.
- `.snap.new` files in the tree are a working state — never commit them.
- If two tests produce identical snapshots, they each get their own file. That's fine; insta dedupes nothing.

## Patterns we use

```rust
insta::assert_snapshot!("descriptive_name", rendered_buffer_text);
```

For layout tests, `rendered` is typically `lines.iter().map(|l| line_to_string(l)).collect::<Vec<_>>().join("\n")` so the snapshot is human-readable text.

For widget tests, render into a `Buffer` and stringify cell-by-cell via a `render_buffer_text` helper in the test module.

## Common failure modes

| Symptom | Likely cause |
|---|---|
| Snapshot moves on EVERY test run | Non-determinism — random colors, HashMap iteration order, instant-based time. Pin it. |
| Snapshot moves on first run after `cargo clean` | Build-script that injects a version or timestamp. Pin it via a const. |
| Snapshot moves only on macOS / only on Linux | Path separator, line endings, or unicode-width disagreement. Normalize before assert. |

## When in doubt

Don't accept. Open the diff, read it, decide. Insta's `--force-update-snapshots` and the `INSTA_FORCE_PASS=1` env var are escape hatches that exist for one-off migrations — never for routine PRs.
