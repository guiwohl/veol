use super::ast::UserJourney;
use crate::mermaid::ascii::canvas::Canvas;
use crate::mermaid::ascii::{AsciiRenderError, CharsetKind, StyledRow};
use ratatui::style::Color;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const INDENT: usize = 2;
const GAP: usize = 2;
const BAR_WIDTH: usize = 5;

pub fn render(
    diag: &UserJourney,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<String>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset).map(|c| c.into_lines())
}

pub fn render_styled(
    diag: &UserJourney,
    max_width: u16,
    charset: CharsetKind,
) -> Result<Vec<StyledRow>, AsciiRenderError> {
    render_to_canvas(diag, max_width, charset).map(|c| c.into_styled_lines())
}

fn render_to_canvas(
    diag: &UserJourney,
    _max_width: u16,
    charset: CharsetKind,
) -> Result<Canvas, AsciiRenderError> {
    if diag.sections.is_empty() && diag.title.is_none() {
        return Err(AsciiRenderError::Empty);
    }

    let (filled_ch, empty_ch) = match charset {
        CharsetKind::Unicode => ('█', '░'),
        CharsetKind::Ascii => ('#', '.'),
    };
    let underline_ch = match charset {
        CharsetKind::Unicode => '─',
        CharsetKind::Ascii => '-',
    };
    let actor_sep = match charset {
        CharsetKind::Unicode => '·',
        CharsetKind::Ascii => '*',
    };

    // First pass — compute height + width
    let mut height = 0usize;
    let mut width = 0usize;

    if let Some(title) = &diag.title {
        height += 2;
        width = width.max(UnicodeWidthStr::width(title.as_str()));
    }

    let mut section_widths: Vec<usize> = Vec::with_capacity(diag.sections.len());
    for section in &diag.sections {
        let label_col = section
            .steps
            .iter()
            .map(|s| UnicodeWidthStr::width(s.label.as_str()))
            .max()
            .unwrap_or(0);
        section_widths.push(label_col);

        let name_w = UnicodeWidthStr::width(section.name.as_str()).max(1);
        width = width.max(name_w);

        for step in &section.steps {
            let glyph_w = UnicodeWidthStr::width(score_glyph(step.score, charset));
            let actor_str = build_actor_text(&step.actors, actor_sep);
            let actor_w = UnicodeWidthStr::width(actor_str.as_str());
            let row_w = INDENT + label_col + GAP + glyph_w + 1 + 1 + 1 + BAR_WIDTH + actor_w;
            width = width.max(row_w);
        }
    }

    for (i, section) in diag.sections.iter().enumerate() {
        if i > 0 {
            height += 1;
        }
        height += 2; // name + underline
        height += section.steps.len();
    }

    let width = width.max(1);
    let height = height.max(1);
    let mut canvas = Canvas::new(width, height, charset);

    let mut y = 0usize;

    if let Some(title) = &diag.title {
        canvas.put_str_colored(0, y, title, Color::White);
        y += 2;
    }

    for (i, section) in diag.sections.iter().enumerate() {
        if i > 0 {
            y += 1;
        }
        let name_w = UnicodeWidthStr::width(section.name.as_str()).max(1);
        canvas.put_str_colored(0, y, &section.name, Color::Blue);
        y += 1;
        let underline: String = std::iter::repeat_n(underline_ch, name_w).collect();
        canvas.put_str_colored(0, y, &underline, Color::Blue);
        y += 1;

        let label_col = section_widths[i];

        for step in &section.steps {
            // label
            canvas.put_str(INDENT, y, &step.label);
            // glyph at label_col + GAP
            let glyph = score_glyph(step.score, charset);
            let glyph_color = glyph_color_for(step.score);
            let glyph_x = INDENT + label_col + GAP;
            canvas.put_str_colored(glyph_x, y, glyph, glyph_color);
            let glyph_w = UnicodeWidthStr::width(glyph);
            // " {score} "
            let score_x = glyph_x + glyph_w + 1;
            let score_str = step.score.to_string();
            canvas.put_str(score_x, y, &score_str);
            // bar
            let bar_x = score_x + score_str.chars().count() + 1;
            let s = step.score.clamp(1, 5) as usize;
            for k in 0..s {
                canvas.put_char(bar_x + k, y, filled_ch);
                canvas.set_color(bar_x + k, y, glyph_color);
            }
            for k in s..BAR_WIDTH {
                canvas.put_char(bar_x + k, y, empty_ch);
                canvas.set_color(bar_x + k, y, Color::DarkGray);
            }
            // actors
            if !step.actors.is_empty() {
                let actor_text = build_actor_text(&step.actors, actor_sep);
                let actor_x = bar_x + BAR_WIDTH;
                // ' ' + sep + ' ' default-colored, then names magenta
                canvas.put_str(actor_x, y, &actor_text);
                let names = step.actors.join(", ");
                // names start at actor_x + 3 (" · " is 3 columns)
                canvas.paint_text(actor_x + 3, y, &names, Color::Magenta);
            }
            y += 1;
        }
    }

    Ok(canvas)
}

fn build_actor_text(actors: &[String], sep: char) -> String {
    if actors.is_empty() {
        return String::new();
    }
    let mut s = String::new();
    s.push(' ');
    for c in sep.to_string().chars() {
        let _ = UnicodeWidthChar::width(c);
        s.push(c);
    }
    s.push(' ');
    s.push_str(&actors.join(", "));
    s
}

fn score_glyph(score: u8, charset: CharsetKind) -> &'static str {
    match charset {
        CharsetKind::Unicode => match score {
            1 | 2 => "☹",
            3 => ":|",
            _ => "☺",
        },
        CharsetKind::Ascii => match score {
            1 | 2 => ":(",
            3 => ":|",
            _ => ":)",
        },
    }
}

fn glyph_color_for(score: u8) -> Color {
    match score {
        1 | 2 => Color::Red,
        3 => Color::DarkGray,
        _ => Color::Yellow,
    }
}

#[cfg(test)]
mod tests {
    use crate::mermaid::ascii::StyledRun;
    use ratatui::style::Color;

    fn render(src: &str) -> String {
        crate::mermaid::ascii::journey::render(
            src,
            100,
            crate::mermaid::ascii::CharsetKind::Unicode,
        )
        .unwrap()
        .join("\n")
    }

    fn render_ascii(src: &str) -> String {
        crate::mermaid::ascii::journey::render(src, 100, crate::mermaid::ascii::CharsetKind::Ascii)
            .unwrap()
            .join("\n")
    }

    #[test]
    fn render_journey_minimal() {
        let src = "journey\n    section S\n      A: 3: Me";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_with_title() {
        let src = "journey\n    title My Workday\n    section Morning\n      Wake up: 3: Me";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_section_with_steps() {
        let src = "journey\n    section Morning\n      Wake up: 3: Me\n      Make coffee: 5: Me, Spouse\n      Read news: 2: Me";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_two_sections() {
        let src = "journey\n    title My Workday\n    section Morning\n      Wake up: 3: Me\n      Make coffee: 5: Me, Spouse\n    section Work\n      Commute: 1: Me\n      Code: 5: Me, Boss";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_step_high_score_smile() {
        let src = "journey\n    section S\n      Great: 5: Me";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_step_low_score_frown() {
        let src = "journey\n    section S\n      Awful: 1: Me";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_step_multi_actors() {
        let src = "journey\n    section S\n      Pair: 4: Alice, Bob, Carol";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_long_label() {
        let src = "journey\n    section S\n      A really long label describing the step in detail: 3: Me\n      Short: 5: Me";
        insta::assert_snapshot!(render(src));
    }

    #[test]
    fn render_ascii_charset() {
        let src = "journey\n    title ASCII Mode\n    section Morning\n      Wake up: 3: Me\n      Coffee: 5: Me, Spouse";
        insta::assert_snapshot!(render_ascii(src));
    }

    #[test]
    fn render_actors_appear_as_columns() {
        let src = "journey\n    section S\n      Step: 3: Alice, Bob";
        let out = render(src);
        assert!(out.contains("Alice"), "expected Alice in:\n{out}");
        assert!(out.contains("Bob"), "expected Bob in:\n{out}");
    }

    #[test]
    fn render_score_emoji_in_unicode() {
        let happy = render("journey\n    section S\n      Hi: 5: Me");
        let sad = render("journey\n    section S\n      Lo: 1: Me");
        assert!(
            happy.contains('☺'),
            "expected smile glyph in happy out:\n{happy}"
        );
        assert!(sad.contains('☹'), "expected frown glyph in sad out:\n{sad}");
    }

    #[test]
    fn render_score_text_in_ascii() {
        let happy = render_ascii("journey\n    section S\n      Hi: 5: Me");
        let sad = render_ascii("journey\n    section S\n      Lo: 1: Me");
        assert!(happy.contains(":)"), "expected :) in ascii happy:\n{happy}");
        assert!(sad.contains(":("), "expected :( in ascii sad:\n{sad}");
    }

    #[test]
    fn render_section_label_visible() {
        let src = "journey\n    section Morning\n      A: 3: Me";
        let out = render(src);
        assert!(
            out.contains("Morning"),
            "section name should be visible:\n{out}"
        );
    }

    // ---- Color verification tests ----

    fn distinct_colors(rows: &[Vec<StyledRun>]) -> std::collections::HashSet<Color> {
        let mut set = std::collections::HashSet::new();
        for row in rows {
            for run in row {
                if let Some(c) = run.color {
                    set.insert(c);
                }
            }
        }
        set
    }

    fn find_color_at(row: &[StyledRun], target: char) -> Option<Color> {
        for run in row {
            if run.text.contains(target) {
                return run.color;
            }
        }
        None
    }

    #[test]
    fn render_styled_attaches_colors() {
        let src =
            "journey\n    title T\n    section Morning\n      Wake: 5: Me\n      Sleep: 1: Me";
        let diag = crate::mermaid::ascii::journey::parser::parse(src).unwrap();
        let rows =
            super::render_styled(&diag, 100, crate::mermaid::ascii::CharsetKind::Unicode).unwrap();
        let colors = distinct_colors(&rows);
        assert!(
            colors.len() >= 2,
            "expected at least 2 distinct colors, got {colors:?}"
        );
    }

    #[test]
    fn render_styled_specific_element_colored() {
        let src = "journey\n    section S\n      Good: 5: Me\n      Bad: 1: Me\n      Mid: 3: Me";
        let diag = crate::mermaid::ascii::journey::parser::parse(src).unwrap();
        let rows =
            super::render_styled(&diag, 100, crate::mermaid::ascii::CharsetKind::Unicode).unwrap();
        // row 0 = "S" section name → Blue
        let section_color = rows[0].iter().find_map(|r| r.color);
        assert_eq!(section_color, Some(Color::Blue));
        // row 1 = underline (also Blue)
        let underline_color = rows[1].iter().find_map(|r| r.color);
        assert_eq!(underline_color, Some(Color::Blue));
        // row 2 = Good (score 5) → Yellow glyph + bar
        let good_filled = find_color_at(&rows[2], '█');
        assert_eq!(good_filled, Some(Color::Yellow));
        // row 3 = Bad (score 1) → Red
        let bad_filled = find_color_at(&rows[3], '█');
        assert_eq!(bad_filled, Some(Color::Red));
        // row 4 = Mid (score 3) → DarkGray
        let mid_filled = find_color_at(&rows[4], '█');
        assert_eq!(mid_filled, Some(Color::DarkGray));
    }

    #[test]
    fn render_styled_no_alignment_drift() {
        let src = "journey\n    title My Workday\n    section Morning\n      Wake up: 3: Me\n      Make coffee: 5: Me, Spouse\n    section Work\n      Commute: 1: Me\n      Code: 5: Me, Boss";
        let diag = crate::mermaid::ascii::journey::parser::parse(src).unwrap();
        let plain = crate::mermaid::ascii::journey::render(
            src,
            100,
            crate::mermaid::ascii::CharsetKind::Unicode,
        )
        .unwrap();
        let styled =
            super::render_styled(&diag, 100, crate::mermaid::ascii::CharsetKind::Unicode).unwrap();
        assert_eq!(plain.len(), styled.len());
        for (i, (line, row)) in plain.iter().zip(styled.iter()).enumerate() {
            let flat: String = row.iter().map(|r| r.text.as_str()).collect();
            let flat = flat.trim_end().to_string();
            assert_eq!(&flat, line, "row {i} mismatch");
        }
    }
}
