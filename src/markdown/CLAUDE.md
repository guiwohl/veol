# markdown/

Markdown source → `Block` AST → `Vec<LayoutLine>`. Pulldown-cmark events get walked into a flat block model; layout then wraps every block into width-aware styled lines for the TUI to paint.

## Files

| File | Purpose |
|---|---|
| `parser.rs` | `pulldown-cmark` event stream → `Vec<Block>` via a flat `Walker`. Hand-rolled state machine (no recursion in `consume_block`). Owns the mermaid info-string detection (`mermaid` / `mmd` / `mermaid-js`, case-insensitive, allows `:` / space suffix). |
| `model.rs` | `Block`, `Span`, `SpanStyle`, `ListItem`, `TableRow`, `Alignment`, `FrontmatterKind`. Plus `heading_anchor(text)` — GitHub-style slug (lowercase, alphanumerics + dashes, collapsed). |
| `layout.rs` | `Block` → `Vec<LayoutLine>` width-aware. Owns prose centering, code fence painting, table compute+render, blockquote prefix, list bullets/indent, mermaid emission via `ascii::render_mermaid_styled`. |
| `frontmatter.rs` | YAML (`---`) / TOML (`+++`) detection. YAML is hand-parsed (one `:` per line, quote-stripped). TOML goes through the real `toml` crate then flattens one level (`[meta] tags = …` → `meta.tags`). |

## Key design decisions

- **Prose centering at ≥ 120 cols**: `layout` sets `prose_width = min(100, raw_width)` and `prose_indent = (raw_width - prose_width) / 2`. Every line carries `indent_cols` so `DocumentWidget` paints at `area.x + indent_cols`. Below 120 cols, no margin (full-bleed). Magic numbers live at the top of `layout.rs` (`MAX_CONTENT_WIDTH`, `MIN_TERMINAL_FOR_PADDING`).
- **Mermaid fences become `Block::MermaidBlock { source }` at parse time, not layout time**. `is_mermaid_info` splits on `:` and space so `mermaid:theme=dark` still routes to the renderer. Source is `trim_end_matches('\n')` so snapshot tests are stable.
- **Table widths: max-then-shrink-to-min, distribute slack proportionally**. `compute_table_widths` first sums every cell's longest line; if it fits, ship it. Else falls back to per-column `longest_word` mins; if even those don't fit, ship min widths anyway (cells will wrap). Otherwise, distribute leftover columns by each column's slack share. The last column eats rounding remainder so widths sum exactly.
- **Inline event walking uses depth counters, not stacks**: `InlineCtx` counts nested `<strong>`/`<em>`/`<s>` opens so unbalanced events don't crash. `link` IS a stack (popped on Link/Image end) because the URL has to be the most recent open.
- **`Walker::consume_block` matches `Tag::FootnoteDefinition` so footnotes become their own `Block::Footnote`** rather than sticking inline. Layout indents the body by the `[^label]: ` prefix width.

## Gotchas

- Pulldown's `Tag::Image` is treated as a link (URL pushed onto `ctx.link`). Veol never fetches images — the URL just renders as an underlined span via `Modifier::UNDERLINED`. Don't add image rendering here; the spec forbids it (L11).
- `consume_list_item` is the only place we recurse via `consume_block` for arbitrary nested tags. If you add a new block type that can appear inside a list item, it'll Just Work — but if it can ONLY appear at the top level, gate it here.
- `frontmatter::parse_yaml_pairs` is intentionally dumb (no flow scalars, no multiline, no block scalars). It exists to render the frontmatter card, not to roundtrip YAML. If a YAML value contains `:`, only the first one is the separator.
- `heading_anchor` collapses `_` / `-` / whitespace into a single `-`, drops trailing dashes, and **keeps digits** (`"Section 2.1"` → `section-21`, not `section-2-1`). Tests pin this — don't "fix" it.
- The fallback arm in `Walker::consume_block` (`other =>`) emits an empty `Paragraph`. This is a sentinel — never returns `None` because callers expect a `Block`. If you see empty paragraphs in output, an unexpected `Tag` slipped through and got swallowed; add a real arm.

## How to extend

- New block type: add a variant to `model::Block`, handle it in `parser::Walker::consume_block`, render it in `layout::layout`'s `match block`. Snapshot test in `tests/` or under `src/markdown/snapshots/`.
- Touching layout width math: re-run table snapshot tests, `cargo insta review`. The auto-shrink algorithm has subtle rounding — always assert column sums equal `usable`.
