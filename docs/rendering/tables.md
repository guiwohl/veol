## Tables
Auto-shrink columns to fit the terminal; wrap cell content when even minimum widths overflow.

> **Related**
> - [`pipeline.md`](pipeline.md) — where tables sit in layout
> - [`../../src/markdown/CLAUDE.md`](../../src/markdown/CLAUDE.md) — overall layout conventions

Implementation lives at `src/markdown/layout.rs:759-914` (`compute_table_widths` + `render_table`). The empty `src/render/table.rs` is a placeholder — the live code is in the layout module.

## Width algorithm

Goal: pick per-column character widths so the rendered table fits `ctx.width`, given border chars `│` between every column plus a 1-space pad each side.

For `n` columns:

```
usable = ctx.width - (n + 1)   borders
                  - (n * 2)    inner padding
```

If `usable < n`, every column gets width 1 — a degenerate render. Otherwise:

1. **Scan headers + rows** to find per-column `max_widths[i]` (longest cell line in cols) and `min_widths[i] = max(longest_word, 1)`.
2. **If `sum(max_widths) <= usable`**: ship `max_widths`. Nothing to shrink.
3. **Else if `sum(min_widths) >= usable`**: ship `min_widths` anyway. Cells will wrap — at least no word gets chopped.
4. **Else**: distribute `extra = usable - sum(min_widths)` proportionally to each column's `slack[i] = max[i] - min[i]`. The last column eats any rounding remainder so widths sum to exactly `usable`.

```
widths[i] = min_widths[i] + (extra * slack[i] / total_slack)   for i < n-1
widths[n-1] = min_widths[n-1] + (extra - sum_distributed)
```

## Word-wrap inside a cell

`wrap_plain_text(text, width)` greedily packs space-separated words into lines no wider than `width`. A word longer than `width` is split at character boundaries — chars, not bytes, so multi-byte UTF-8 stays intact.

Cell content is plain-text-extracted via `cell_plain_text`, which concatenates `Span.text`. Inline emphasis/code/links lose their styling inside table cells today; this is by design — table layout is already tight on width.

## Render

`render_table` (`layout.rs:833`):

1. Compute `widths` once.
2. Top border `┌──┬──┐` printed via `border(left, sep, right, ctx)`.
3. Each row: wrap each cell to its column width; compute `height = max(cell_lines.len())`; emit `height` rows, each with `│ <padded-cell> │ … │`.
4. `├──┼──┤` divider between data rows (matches reedo/glow visual density).
5. Header row uses `Modifier::BOLD` and `theme.colors.table_header_fg`.

## Alignment

Each `TableRow` carries `alignments: Vec<Alignment>` straight from the `| :--- |`, `| :---: |`, `| ---: |` markers. `align_cell(raw, width, align)`:

- `Left`: pad right with spaces.
- `Right`: pad left with spaces.
- `Center`: split the leftover space; odd remainder lands on the right.

## Snapshot coverage

`tests/snapshots/` + `src/markdown/snapshots/` pin three flavors:

| Test | Pins |
|---|---|
| `layout_table_with_alignments` | Center / right alignment rendering. |
| `layout_table_wide_shrinks_via_wrap` | Slack distribution + cell wrapping. |
| `render_table_has_divider_between_data_rows` | The `├──┼──┤` interior dividers. |

Run `cargo insta review` after touching width math. The algorithm has subtle rounding — always assert column sums equal `usable`.
