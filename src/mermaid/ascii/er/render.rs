use super::ast::{Attribute, Cardinality, Entity, ErDiagram, Relationship};
use crate::mermaid::ascii::canvas::Canvas;
use crate::mermaid::ascii::charset::get_charset;
use crate::mermaid::ascii::coord::Coord;
use crate::mermaid::ascii::{AsciiRenderError, CharsetKind, StyledRow};
use ratatui::style::Color;
use std::collections::{HashMap, HashSet, VecDeque};
use unicode_width::UnicodeWidthStr;

pub fn render(
    diag: &ErDiagram,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<String>, AsciiRenderError> {
    let canvas = render_to_canvas(diag, max_width, charset)?;
    Ok(canvas.into_lines())
}

pub fn render_styled(
    diag: &ErDiagram,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<StyledRow>, AsciiRenderError> {
    let canvas = render_to_canvas(diag, max_width, charset)?;
    Ok(canvas.into_styled_lines())
}

fn render_to_canvas(
    diag: &ErDiagram,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Canvas, AsciiRenderError> {
    if diag.entities.is_empty() {
        return Err(AsciiRenderError::Empty);
    }

    let placement = place_entities(diag);
    let boxes = compute_boxes(diag, &placement);

    let mut canvas_w = boxes.values().map(|b| b.x2 + 1).max().unwrap_or(1);
    let mut canvas_h = boxes.values().map(|b| b.y2 + 1).max().unwrap_or(1);
    let route_pad_x = 10usize;
    let route_pad_y = 4usize;
    canvas_w += route_pad_x;
    canvas_h += route_pad_y;
    canvas_w = canvas_w.min(max_width as usize).max(1);

    let mut canvas = Canvas::new(canvas_w, canvas_h, charset);

    for name in diag.entities.keys() {
        if let (Some(ent), Some(b)) = (diag.entities.get(name), boxes.get(name)) {
            draw_entity_box(&mut canvas, charset, ent, b);
        }
    }

    let blocked = compute_blocked_cells(&boxes);

    for rel in &diag.relationships {
        if let (Some(lb), Some(rb)) = (boxes.get(&rel.left), boxes.get(&rel.right)) {
            draw_relationship(&mut canvas, charset, rel, lb, rb, &blocked);
        }
    }

    Ok(canvas)
}

#[derive(Debug, Clone, Copy)]
struct Box {
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
}

impl Box {
    fn mid_y(&self) -> usize {
        let sep = self.y1 + 2;
        let mid = (self.y1 + self.y2) / 2;
        if mid == sep && self.y2 > sep + 1 {
            sep + 1
        } else if mid <= self.y1 {
            self.y1 + 1
        } else if mid >= self.y2 {
            self.y2 - 1
        } else {
            mid
        }
    }
}

fn place_entities(diag: &ErDiagram) -> HashMap<String, (usize, usize)> {
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for rel in &diag.relationships {
        adj.entry(rel.left.clone())
            .or_default()
            .push(rel.right.clone());
        adj.entry(rel.right.clone())
            .or_default()
            .push(rel.left.clone());
    }

    let mut placement: HashMap<String, (usize, usize)> = HashMap::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut next_col: usize = 0;
    let mut next_row: usize = 0;
    let cols_per_row = compute_cols(diag.entities.len());

    for start in diag.entities.keys() {
        if visited.contains(start) {
            continue;
        }
        let mut queue: VecDeque<String> = VecDeque::new();
        queue.push_back(start.clone());
        visited.insert(start.clone());
        while let Some(n) = queue.pop_front() {
            if !diag.entities.contains_key(&n) {
                continue;
            }
            placement.insert(n.clone(), (next_col, next_row));
            next_col += 1;
            if next_col >= cols_per_row {
                next_col = 0;
                next_row += 1;
            }
            if let Some(neigh) = adj.get(&n) {
                let mut sorted: Vec<String> = neigh.clone();
                sorted.sort();
                for x in sorted {
                    if !visited.contains(&x) {
                        visited.insert(x.clone());
                        queue.push_back(x);
                    }
                }
            }
        }
    }
    placement
}

fn compute_cols(n: usize) -> usize {
    n.max(1)
}

fn entity_dims(ent: &Entity) -> (usize, usize) {
    let title_w = ent.name.width();
    let mut w = title_w;
    for a in &ent.attributes {
        let line = attr_line(a);
        let lw = line.width();
        if lw > w {
            w = lw;
        }
    }
    let inner_w = w + 4;
    let inner_h = 2 + 1 + ent.attributes.len().max(0);
    (inner_w, inner_h)
}

fn attr_line(a: &Attribute) -> String {
    let mut s = format!("{} {}", a.type_name, a.name);
    if let Some(k) = &a.key {
        s.push(' ');
        s.push_str(k);
    }
    if let Some(c) = &a.comment {
        s.push(' ');
        s.push('"');
        s.push_str(c);
        s.push('"');
    }
    s
}

fn compute_boxes(
    diag: &ErDiagram,
    placement: &HashMap<String, (usize, usize)>,
) -> HashMap<String, Box> {
    let mut col_widths: HashMap<usize, usize> = HashMap::new();
    let mut row_heights: HashMap<usize, usize> = HashMap::new();
    for (name, ent) in &diag.entities {
        let (w, h) = entity_dims(ent);
        if let Some(&(c, r)) = placement.get(name) {
            let cw = col_widths.entry(c).or_insert(0);
            if w + 2 > *cw {
                *cw = w + 2;
            }
            let rh = row_heights.entry(r).or_insert(0);
            if h + 2 > *rh {
                *rh = h + 2;
            }
        }
    }

    let h_gap = 10usize;
    let v_gap = 3usize;

    let max_col = col_widths.keys().copied().max().unwrap_or(0);
    let max_row = row_heights.keys().copied().max().unwrap_or(0);
    let mut col_x: HashMap<usize, usize> = HashMap::new();
    let mut row_y: HashMap<usize, usize> = HashMap::new();
    let mut x_cur = 0usize;
    for c in 0..=max_col {
        col_x.insert(c, x_cur);
        x_cur += col_widths.get(&c).copied().unwrap_or(0) + h_gap;
    }
    let mut y_cur = 0usize;
    for r in 0..=max_row {
        row_y.insert(r, y_cur);
        y_cur += row_heights.get(&r).copied().unwrap_or(0) + v_gap;
    }

    let mut out: HashMap<String, Box> = HashMap::new();
    for (name, ent) in &diag.entities {
        let (c, r) = match placement.get(name) {
            Some(p) => *p,
            None => continue,
        };
        let (w, h) = entity_dims(ent);
        let x1 = col_x.get(&c).copied().unwrap_or(0);
        let y1 = row_y.get(&r).copied().unwrap_or(0);
        let x2 = x1 + w + 1;
        let y2 = y1 + h + 1;
        out.insert(name.clone(), Box { x1, y1, x2, y2 });
    }
    out
}

fn draw_entity_box(canvas: &mut Canvas, charset: CharsetKind, ent: &Entity, b: &Box) {
    canvas.draw_box(b.x1, b.y1, b.x2, b.y2);
    canvas.paint_box(b.x1, b.y1, b.x2, b.y2, Color::Cyan);
    let cs = get_charset(charset);

    let inner_left = b.x1 + 1;
    let inner_right = b.x2 - 1;
    let inner_w = inner_right.saturating_sub(inner_left) + 1;
    let title_w = ent.name.width();
    let pad = inner_w.saturating_sub(title_w) / 2;
    let title_x = inner_left + pad;
    canvas.put_str(title_x, b.y1 + 1, &ent.name);
    canvas.paint_text(title_x, b.y1 + 1, &ent.name, Color::Yellow);

    let sep_y = b.y1 + 2;
    if sep_y < b.y2 {
        for x in (b.x1 + 1)..b.x2 {
            canvas.put_char(x, sep_y, cs.horiz());
        }
        match charset {
            CharsetKind::Unicode => {
                canvas.put_char(b.x1, sep_y, cs.tee_right());
                canvas.put_char(b.x2, sep_y, cs.tee_left());
            }
            CharsetKind::Ascii => {
                canvas.put_char(b.x1, sep_y, '+');
                canvas.put_char(b.x2, sep_y, '+');
            }
        }
        canvas.paint_hline(b.x1, b.x2, sep_y, Color::Cyan);
    }

    for (i, a) in ent.attributes.iter().enumerate() {
        let y = sep_y + 1 + i;
        if y >= b.y2 {
            break;
        }
        let line = attr_line(a);
        canvas.put_str(b.x1 + 2, y, &line);
        if let Some(color) = attr_color(a) {
            canvas.paint_text(b.x1 + 2, y, &line, color);
        }
    }
}

fn attr_color(a: &Attribute) -> Option<Color> {
    match a.key.as_deref() {
        Some("PK") => Some(Color::Magenta),
        Some("FK") => Some(Color::Blue),
        Some("UK") => Some(Color::Green),
        _ => None,
    }
}

fn compute_blocked_cells(boxes: &HashMap<String, Box>) -> HashSet<(i32, i32)> {
    let mut blocked: HashSet<(i32, i32)> = HashSet::new();
    for b in boxes.values() {
        for y in b.y1..=b.y2 {
            for x in b.x1..=b.x2 {
                blocked.insert((x as i32, y as i32));
            }
        }
    }
    blocked
}

fn left_marker(card: Cardinality, charset: CharsetKind) -> [char; 2] {
    match (card, charset) {
        (Cardinality::ExactlyOne, CharsetKind::Unicode) => ['─', '┃'],
        (Cardinality::ExactlyOne, CharsetKind::Ascii) => ['-', '|'],
        (Cardinality::ZeroOrOne, CharsetKind::Unicode) => ['o', '┃'],
        (Cardinality::ZeroOrOne, CharsetKind::Ascii) => ['o', '|'],
        (Cardinality::OneOrMany, CharsetKind::Unicode) => ['>', '┃'],
        (Cardinality::OneOrMany, CharsetKind::Ascii) => ['>', '|'],
        (Cardinality::ZeroOrMany, CharsetKind::Unicode) => ['>', 'o'],
        (Cardinality::ZeroOrMany, CharsetKind::Ascii) => ['>', 'o'],
    }
}

fn right_marker(card: Cardinality, charset: CharsetKind) -> [char; 2] {
    match (card, charset) {
        (Cardinality::ExactlyOne, CharsetKind::Unicode) => ['┃', '─'],
        (Cardinality::ExactlyOne, CharsetKind::Ascii) => ['|', '-'],
        (Cardinality::ZeroOrOne, CharsetKind::Unicode) => ['┃', 'o'],
        (Cardinality::ZeroOrOne, CharsetKind::Ascii) => ['|', 'o'],
        (Cardinality::OneOrMany, CharsetKind::Unicode) => ['┃', '<'],
        (Cardinality::OneOrMany, CharsetKind::Ascii) => ['|', '<'],
        (Cardinality::ZeroOrMany, CharsetKind::Unicode) => ['o', '<'],
        (Cardinality::ZeroOrMany, CharsetKind::Ascii) => ['o', '<'],
    }
}

fn line_char(identifying: bool, charset: CharsetKind) -> char {
    match (identifying, charset) {
        (true, CharsetKind::Unicode) => '─',
        (true, CharsetKind::Ascii) => '-',
        (false, CharsetKind::Unicode) => '┈',
        (false, CharsetKind::Ascii) => '.',
    }
}

fn draw_relationship(
    canvas: &mut Canvas,
    charset: CharsetKind,
    rel: &Relationship,
    lb: &Box,
    rb: &Box,
    blocked: &HashSet<(i32, i32)>,
) {
    let (from_box, to_box, lcard, rcard, flipped) = if lb.x1 <= rb.x1 {
        (lb, rb, rel.left_card, rel.right_card, false)
    } else {
        (rb, lb, rel.right_card, rel.left_card, true)
    };

    let from_y = from_box.mid_y();
    let to_y = to_box.mid_y();
    let from_x = from_box.x2;
    let to_x = to_box.x1;

    let lm = if !flipped {
        left_marker(lcard, charset)
    } else {
        right_marker(lcard, charset)
    };
    let rm = if !flipped {
        right_marker(rcard, charset)
    } else {
        left_marker(rcard, charset)
    };
    let line_c = line_char(rel.identifying, charset);

    let max_x = (canvas.width().saturating_sub(1)) as i32;
    let max_y = (canvas.height().saturating_sub(1)) as i32;
    let start = Coord::new((from_x + 1) as i32, from_y as i32);
    let end = Coord::new((to_x.saturating_sub(1)) as i32, to_y as i32);

    let mut blocked_with_clear = blocked.clone();
    blocked_with_clear.remove(&(start.x, start.y));
    blocked_with_clear.remove(&(end.x, end.y));
    let is_free = |c: Coord| -> bool {
        c.x >= 0
            && c.y >= 0
            && c.x <= max_x
            && c.y <= max_y
            && !blocked_with_clear.contains(&(c.x, c.y))
    };
    let path = crate::mermaid::ascii::astar::find_path(start, end, max_x, max_y, is_free);

    let path = path.unwrap_or_else(|| straight_line(start, end));
    let path = merge_collinear(path);

    draw_path_chars(canvas, &path, line_c, charset);

    let lm_x1 = from_box.x2;
    let lm_x2 = from_box.x2 + 1;
    canvas.put_char(lm_x1, from_y, lm[0]);
    canvas.set_color(lm_x1, from_y, Color::Magenta);
    if lm_x2 < canvas.width() {
        canvas.put_char(lm_x2, from_y, lm[1]);
        canvas.set_color(lm_x2, from_y, Color::Magenta);
    }

    let rm_x2 = to_box.x1;
    let rm_x1 = to_box.x1.saturating_sub(1);
    if rm_x1 < canvas.width() {
        canvas.put_char(rm_x1, to_y, rm[0]);
        canvas.set_color(rm_x1, to_y, Color::Magenta);
    }
    canvas.put_char(rm_x2, to_y, rm[1]);
    canvas.set_color(rm_x2, to_y, Color::Magenta);

    if !rel.label.is_empty() {
        draw_label(canvas, &path, &rel.label);
    }
}

fn straight_line(start: Coord, end: Coord) -> Vec<Coord> {
    let mid_x = (start.x + end.x) / 2;
    vec![
        start,
        Coord::new(mid_x, start.y),
        Coord::new(mid_x, end.y),
        end,
    ]
}

fn merge_collinear(path: Vec<Coord>) -> Vec<Coord> {
    if path.len() <= 2 {
        return path;
    }
    let mut out = vec![path[0]];
    for i in 1..path.len() - 1 {
        let prev = out[out.len() - 1];
        let cur = path[i];
        let next = path[i + 1];
        let d1 = (cur.x - prev.x, cur.y - prev.y);
        let d2 = (next.x - cur.x, next.y - cur.y);
        let unit1 = unit(d1);
        let unit2 = unit(d2);
        if unit1 != unit2 {
            out.push(cur);
        }
    }
    out.push(*path.last().unwrap());
    out
}

fn unit(d: (i32, i32)) -> (i32, i32) {
    (d.0.signum(), d.1.signum())
}

fn draw_path_chars(canvas: &mut Canvas, path: &[Coord], line_c: char, charset: CharsetKind) {
    if path.len() < 2 {
        return;
    }
    let cs = get_charset(charset);
    let vert_c = match charset {
        CharsetKind::Unicode => {
            if line_c == '┈' {
                '┊'
            } else {
                cs.vert()
            }
        }
        CharsetKind::Ascii => {
            if line_c == '.' {
                ':'
            } else {
                cs.vert()
            }
        }
    };

    for w in path.windows(2) {
        let a = w[0];
        let b = w[1];
        if a.y == b.y {
            let (lo, hi) = if a.x <= b.x { (a.x, b.x) } else { (b.x, a.x) };
            for x in lo..=hi {
                if x >= 0 && a.y >= 0 {
                    set_path_char(canvas, x as usize, a.y as usize, line_c);
                }
            }
            if a.y >= 0 && lo >= 0 && hi >= 0 {
                canvas.paint_hline(lo as usize, hi as usize, a.y as usize, Color::Yellow);
            }
        } else if a.x == b.x {
            let (lo, hi) = if a.y <= b.y { (a.y, b.y) } else { (b.y, a.y) };
            for y in lo..=hi {
                if a.x >= 0 && y >= 0 {
                    set_path_char(canvas, a.x as usize, y as usize, vert_c);
                }
            }
            if a.x >= 0 && lo >= 0 && hi >= 0 {
                canvas.paint_vline(a.x as usize, lo as usize, hi as usize, Color::Yellow);
            }
        }
    }

    for i in 1..path.len() - 1 {
        let prev = path[i - 1];
        let cur = path[i];
        let next = path[i + 1];
        let corner = pick_corner(prev, cur, next, charset);
        if cur.x >= 0 && cur.y >= 0 {
            canvas.put_char(cur.x as usize, cur.y as usize, corner);
            canvas.set_color(cur.x as usize, cur.y as usize, Color::Yellow);
        }
    }
}

fn set_path_char(canvas: &mut Canvas, x: usize, y: usize, c: char) {
    let cur = canvas.get(x, y);
    if cur == ' ' {
        canvas.put_char(x, y, c);
        return;
    }
    if crate::mermaid::ascii::canvas::is_junction_char(cur)
        && crate::mermaid::ascii::canvas::is_junction_char(c)
    {
        canvas.put_char(x, y, crate::mermaid::ascii::canvas::merge_junctions(cur, c));
    }
}

fn pick_corner(prev: Coord, cur: Coord, next: Coord, charset: CharsetKind) -> char {
    let cs = get_charset(charset);
    let in_dir = direction(prev, cur);
    let out_dir = direction(cur, next);
    match (in_dir, out_dir) {
        ((1, 0), (0, 1)) | ((0, -1), (-1, 0)) => cs.top_right(),
        ((1, 0), (0, -1)) | ((0, 1), (-1, 0)) => cs.bot_right(),
        ((-1, 0), (0, 1)) | ((0, -1), (1, 0)) => cs.top_left(),
        ((-1, 0), (0, -1)) | ((0, 1), (1, 0)) => cs.bot_left(),
        _ => cs.cross(),
    }
}

fn direction(a: Coord, b: Coord) -> (i32, i32) {
    ((b.x - a.x).signum(), (b.y - a.y).signum())
}

fn draw_label(canvas: &mut Canvas, path: &[Coord], label: &str) {
    let mut best: Option<(Coord, Coord, i32)> = None;
    for w in path.windows(2) {
        let a = w[0];
        let b = w[1];
        if a.y != b.y {
            continue;
        }
        let len = (b.x - a.x).abs();
        if best.map(|(_, _, l)| len > l).unwrap_or(true) {
            best = Some((a, b, len));
        }
    }
    let seg = match best {
        Some((a, b, _)) => (a, b),
        None => return,
    };
    let (lo, hi) = if seg.0.x <= seg.1.x {
        (seg.0.x, seg.1.x)
    } else {
        (seg.1.x, seg.0.x)
    };
    let avail = (hi - lo - 1).max(0) as usize;
    let label_text = if label.width() > avail {
        truncate_to_width(label, avail)
    } else {
        label.to_string()
    };
    let label_w = label_text.width() as i32;
    let mid_x = (lo + hi) / 2;
    let mut start_x = mid_x - label_w / 2;
    if start_x <= lo {
        start_x = lo + 1;
    }
    if start_x + label_w > hi {
        start_x = (hi - label_w).max(lo + 1);
    }
    let y = seg.0.y;
    let label_y = (y - 1).max(0);
    let label_start = start_x.max(0) as usize;
    let mut cx = label_start;
    for ch in label_text.chars() {
        canvas.put_char(cx, label_y as usize, ch);
        cx += unicode_width::UnicodeWidthChar::width(ch)
            .unwrap_or(1)
            .max(1);
    }
    canvas.paint_text(label_start, label_y as usize, &label_text, Color::Green);
}

fn truncate_to_width(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if max == 1 {
        return "…".to_string();
    }
    let mut acc = String::new();
    let mut w = 0usize;
    for ch in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch)
            .unwrap_or(1)
            .max(1);
        if w + cw > max.saturating_sub(1) {
            break;
        }
        acc.push(ch);
        w += cw;
    }
    acc.push('…');
    acc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_src(src: &str) -> String {
        crate::mermaid::ascii::er::render(src, 200, CharsetKind::Unicode)
            .unwrap()
            .join("\n")
    }

    fn render_ascii(src: &str) -> String {
        crate::mermaid::ascii::er::render(src, 200, CharsetKind::Ascii)
            .unwrap()
            .join("\n")
    }

    #[test]
    fn render_two_entities_minimal() {
        insta::assert_snapshot!(render_src("erDiagram\nUSER ||--o{ POST : has"));
    }

    #[test]
    fn render_entity_with_attributes() {
        insta::assert_snapshot!(render_src(
            "erDiagram\nUSER {\n  int id PK\n  string name\n}\nUSER ||--o{ POST : has"
        ));
    }

    #[test]
    fn render_one_to_many() {
        insta::assert_snapshot!(render_src("erDiagram\nA ||--o{ B : many"));
    }

    #[test]
    fn render_zero_to_many() {
        insta::assert_snapshot!(render_src("erDiagram\nA }o--o{ B : x"));
    }

    #[test]
    fn render_one_to_one_identifying() {
        insta::assert_snapshot!(render_src("erDiagram\nA ||--|| B : one"));
    }

    #[test]
    fn render_one_to_one_non_identifying_dotted() {
        insta::assert_snapshot!(render_src("erDiagram\nA ||..|| B : opt"));
    }

    #[test]
    fn render_relationship_with_label() {
        insta::assert_snapshot!(render_src(
            r#"erDiagram
USER ||--o{ POST : "writes many""#
        ));
    }

    #[test]
    fn render_attribute_with_pk() {
        insta::assert_snapshot!(render_src(
            "erDiagram\nUSER {\n  int id PK\n}\nUSER ||--o{ POST : x"
        ));
    }

    #[test]
    fn render_attribute_with_fk_uk() {
        insta::assert_snapshot!(render_src(
            "erDiagram\nUSER {\n  string email UK\n}\nPOST {\n  int user_id FK\n}\nUSER ||--o{ POST : x"
        ));
    }

    #[test]
    fn render_three_entities_star_schema() {
        insta::assert_snapshot!(render_src(
            "erDiagram\nUSER ||--o{ POST : has\nUSER ||--o{ COMMENT : writes\nPOST ||--o{ COMMENT : on"
        ));
    }

    #[test]
    fn render_ascii_charset() {
        insta::assert_snapshot!(render_ascii("erDiagram\nUSER ||--o{ POST : has"));
    }

    // unit tests

    #[test]
    fn render_entity_box_has_name_and_attrs() {
        let out =
            render_src("erDiagram\nUSER {\n  int id PK\n  string name\n}\nUSER ||--o{ POST : has");
        assert!(out.contains("USER"));
        assert!(out.contains("int id PK"));
        assert!(out.contains("string name"));
    }

    #[test]
    fn render_cardinality_left_appears_near_left_box() {
        let out = render_src("erDiagram\nA }o--o{ B : x");
        let joined: String = out.lines().collect();
        assert!(joined.contains('>') && joined.contains('o'));
    }

    #[test]
    fn render_cardinality_right_appears_near_right_box() {
        let out = render_src("erDiagram\nA ||--o{ B : x");
        assert!(out.contains('o'));
        assert!(out.contains('<'));
    }

    #[test]
    fn render_pk_attribute_visible() {
        let out = render_src("erDiagram\nUSER {\n  int id PK\n}\nUSER ||--o{ POST : has");
        assert!(out.contains("PK"));
    }

    #[test]
    fn render_label_centered_on_relationship_line() {
        let out = render_src("erDiagram\nA ||--o{ B : marker");
        assert!(out.contains("marker"));
    }

    #[test]
    fn render_styled_attaches_color_to_borders() {
        let rows = crate::mermaid::ascii::er::render_styled(
            "erDiagram\nUSER ||--o{ POST : has",
            100,
            CharsetKind::Unicode,
        )
        .unwrap();
        assert!(rows
            .iter()
            .flatten()
            .any(|r| r.color == Some(ratatui::style::Color::Cyan)));
    }

    #[test]
    fn render_styled_has_multiple_colors() {
        let rows = crate::mermaid::ascii::er::render_styled(
            "erDiagram\nUSER {\n  int id PK\n  string email UK\n}\nPOST {\n  int user_id FK\n}\nUSER ||--o{ POST : writes",
            100,
            CharsetKind::Unicode,
        )
        .unwrap();
        let colors: std::collections::HashSet<_> = rows.iter().flatten().map(|r| r.color).collect();
        assert!(
            colors.len() >= 2,
            "expected at least 2 distinct colors, got {colors:?}"
        );
    }

    #[test]
    fn render_styled_specific_glyph_colored() {
        let rows = crate::mermaid::ascii::er::render_styled(
            "erDiagram\nUSER {\n  int id PK\n}\nUSER ||--o{ POST : has",
            100,
            CharsetKind::Unicode,
        )
        .unwrap();
        let pk_color = rows
            .iter()
            .flatten()
            .find(|r| r.text.contains("PK"))
            .map(|r| r.color);
        assert_eq!(
            pk_color,
            Some(Some(ratatui::style::Color::Magenta)),
            "PK attribute line should be magenta"
        );
    }
}
