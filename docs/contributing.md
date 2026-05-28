## Contributing
How to get a change into Veol — gates, workflow, and the non-negotiables.

> **Related**
> - [`../.claude/CLAUDE.md`](../.claude/CLAUDE.md) — AI agent rules (P0 / P1 / P2 beliefs)
> - [`runbooks/release-checklist.md`](runbooks/release-checklist.md) — what ships gets in
> - [`runbooks/snapshot-management.md`](runbooks/snapshot-management.md) — `cargo insta` workflow

## Test-driven, always

Every feature, every bugfix lands with the failing test that proves the new behavior, **written first**.

```bash
# RED   write a failing test, watch it fail
cargo test --all -- <new_test_name>

# GREEN  write the minimum code that makes it pass
cargo test --all

# REFACTOR  only if it pays off
```

The `/tdd` skill (in `.claude/`) walks AI agents through this. Humans should follow the same loop manually.

## Planning before coding

For anything beyond a one-line fix, run `/brainstorming` first (or just write the plan out longhand). The pattern:

1. State the problem in one paragraph.
2. List 2-3 strategies; pick one with a one-line rationale.
3. Lay out steps in order with their verification command.
4. Then write code.

The locked specs in [`tasks/`](tasks/) follow this template — read one to see the full shape.

## The three gates

All three MUST pass before declaring work done:

```bash
cargo test --all
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

`cargo fmt --check` failures: just run `cargo fmt`. Clippy failures: fix the lint or justify a `#[allow(...)]` in the same commit. Test failures: don't push.

No `--no-verify` on commits. No skipping snapshot review.

## Snapshot tests

Most layout and render changes will move snapshot output. After a passing local run:

```bash
cargo insta review
```

Walk every diff. Accept what's correct, reject what isn't. Never blanket-`accept` without reading. See [`runbooks/snapshot-management.md`](runbooks/snapshot-management.md).

## Style — short version

From `.claude/CLAUDE.md` (P0 / P1):

- **No bloat.** If a feature can be done in 30 lines, don't write 200. No flexible knobs, no wrapper layers, no defensive validation at internal boundaries, no "for later" abstractions.
- **Surgical changes only.** Don't "improve" adjacent code, comments, or formatting. Don't refactor what isn't broken.
- **No comments in code.** Names should make the code self-explanatory. A comment is justified only when the *why* is non-obvious (hidden constraint, subtle invariant, workaround).
- **Per-module CLAUDE.md as living knowledge.** Read it before touching `src/<module>/`. Add to it when you figure something out the hard way.
- **No emojis** in code, commits, PRs, or docs.

## Commit style

- Conventional commits: `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`.
- One logical change per commit. If a change crosses modules, multiple commits.
- Never reference Claude Code, AI, or the model name in commit messages or PR bodies.
- Always rebase, never merge commits. Feature branches off `main`.

Example:

```
feat: add --no-mouse flag for terminal text selection

Mouse capture intercepts terminal click-drag. Opt-out matches reedo
behavior and unblocks copy-paste of mermaid renders.
```

## What never gets in

From `.claude/CLAUDE.md` (NEVER list):

- Buffer-editing features (Veol is a reader — see [`decisions/0004-no-buffer-editing.md`](decisions/0004-no-buffer-editing.md)).
- Subprocess spawn at runtime.
- Outbound network calls (no version checks, no image fetches).
- Embedded browser / JS runtime / HTML/CSS engine.
- `tokio` or any async runtime (see [`decisions/0001-no-async-runtime.md`](decisions/0001-no-async-runtime.md)).
- Shell command strings (`Command::new("sh").arg("-c"...)` is forbidden — argv arrays only).

## Pull requests

Title: imperative mood, conventional prefix, ≤ 70 chars.

Body: what + why + test plan. No need for "summary" sections — the diff is the summary. Highlight only the surprising parts.

If your PR touches diagram rendering, link to the snapshots that moved. If it touches keybinds, update [`keybindings.md`](keybindings.md) and `src/tui/help.rs`'s `keybinds()` slice in the same PR.

## When in doubt

Read [`tasks/guiwohl-veol-spec-2026-05-26.md`](tasks/guiwohl-veol-spec-2026-05-26.md). It is the locked source of truth. The ASCII mermaid pivot at [`tasks/guiwohl-veol-ascii-mermaid-2026-05-27.md`](tasks/guiwohl-veol-ascii-mermaid-2026-05-27.md) is too.

If neither spec covers your change, you're either (a) writing a feature outside the locked plan and need to think harder about scope, or (b) finding a real gap to flag in an issue first.
