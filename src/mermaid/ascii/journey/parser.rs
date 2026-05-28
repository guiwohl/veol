use super::ast::{JourneySection, JourneyStep, UserJourney};
use crate::mermaid::ascii::error::AsciiRenderError;

pub fn parse(source: &str) -> Result<UserJourney, AsciiRenderError> {
    let raw_lines: Vec<&str> = source.lines().collect();

    let mut header_idx: Option<usize> = None;
    for (i, line) in raw_lines.iter().enumerate() {
        let t = strip_comment(line).trim();
        if t.is_empty() {
            continue;
        }
        if t == "journey" || t.starts_with("journey ") || t.starts_with("journey\t") {
            header_idx = Some(i);
            break;
        }
        return Err(AsciiRenderError::Parse {
            line: i + 1,
            msg: format!("expected 'journey' header, got: {t:?}"),
        });
    }
    let header_idx = header_idx.ok_or(AsciiRenderError::Empty)?;

    let mut journey = UserJourney::default();
    let mut current: Option<JourneySection> = None;

    for (offset, raw) in raw_lines.iter().enumerate().skip(header_idx + 1) {
        let stripped = strip_comment(raw);
        let line = stripped.trim();
        if line.is_empty() {
            continue;
        }
        let lineno = offset + 1;

        if let Some(rest) = strip_keyword(line, "title") {
            let title = rest.trim();
            if !title.is_empty() {
                journey.title = Some(title.to_string());
            }
            continue;
        }

        if let Some(rest) = strip_keyword(line, "section") {
            if let Some(sec) = current.take() {
                journey.sections.push(sec);
            }
            current = Some(JourneySection {
                name: rest.trim().to_string(),
                steps: Vec::new(),
            });
            continue;
        }

        let step = parse_step(line, lineno)?;
        match current.as_mut() {
            Some(sec) => sec.steps.push(step),
            None => {
                let mut sec = JourneySection::default();
                sec.steps.push(step);
                current = Some(sec);
            }
        }
    }

    if let Some(sec) = current.take() {
        journey.sections.push(sec);
    }

    Ok(journey)
}

fn parse_step(line: &str, lineno: usize) -> Result<JourneyStep, AsciiRenderError> {
    let mut parts = line.splitn(3, ':');
    let label = parts
        .next()
        .ok_or_else(|| AsciiRenderError::Parse {
            line: lineno,
            msg: format!("expected '<label>: <score>: <actors>', got: {line:?}"),
        })?
        .trim()
        .to_string();
    let score_str = parts.next().ok_or_else(|| AsciiRenderError::Parse {
        line: lineno,
        msg: format!("missing score in step: {line:?}"),
    })?;
    let actors_str = parts.next().unwrap_or("");

    if label.is_empty() {
        return Err(AsciiRenderError::Parse {
            line: lineno,
            msg: "empty step label".into(),
        });
    }

    let score_trim = score_str.trim();
    let score_raw: i64 = score_trim.parse().map_err(|_| AsciiRenderError::Parse {
        line: lineno,
        msg: format!("invalid score: {score_trim:?}"),
    })?;
    let score = score_raw.clamp(1, 5) as u8;

    let actors: Vec<String> = actors_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(JourneyStep {
        label,
        score,
        actors,
    })
}

fn strip_comment(line: &str) -> &str {
    if let Some(idx) = line.find("%%") {
        &line[..idx]
    } else {
        line
    }
}

fn strip_keyword<'a>(s: &'a str, kw: &str) -> Option<&'a str> {
    let s = s.trim_start();
    let rest = s.strip_prefix(kw)?;
    if rest.is_empty() {
        return Some(rest);
    }
    let next = rest.chars().next().unwrap();
    if next.is_whitespace() {
        Some(rest)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_journey_minimal() {
        let src = "journey";
        let j = parse(src).unwrap();
        assert_eq!(j.title, None);
        assert!(j.sections.is_empty());
    }

    #[test]
    fn parse_no_header_error() {
        let src = "graph TD\nA --> B";
        let err = parse(src).unwrap_err();
        match err {
            AsciiRenderError::Parse { .. } => {}
            e => panic!("expected Parse error, got {e:?}"),
        }
    }

    #[test]
    fn parse_with_title() {
        let src = "journey\n    title My Workday";
        let j = parse(src).unwrap();
        assert_eq!(j.title.as_deref(), Some("My Workday"));
    }

    #[test]
    fn parse_section() {
        let src = "journey\n    section Morning";
        let j = parse(src).unwrap();
        assert_eq!(j.sections.len(), 1);
        assert_eq!(j.sections[0].name, "Morning");
    }

    #[test]
    fn parse_step_single_actor() {
        let src = "journey\n    section Morning\n      Wake up: 3: Me";
        let j = parse(src).unwrap();
        let step = &j.sections[0].steps[0];
        assert_eq!(step.label, "Wake up");
        assert_eq!(step.score, 3);
        assert_eq!(step.actors, vec!["Me".to_string()]);
    }

    #[test]
    fn parse_step_multiple_actors() {
        let src = "journey\n    section Morning\n      Make coffee: 5: Me, Spouse";
        let j = parse(src).unwrap();
        let step = &j.sections[0].steps[0];
        assert_eq!(step.actors, vec!["Me".to_string(), "Spouse".to_string()]);
    }

    #[test]
    fn parse_score_high() {
        let src = "journey\n    section S\n      Hi: 5: A";
        let j = parse(src).unwrap();
        assert_eq!(j.sections[0].steps[0].score, 5);
    }

    #[test]
    fn parse_score_low() {
        let src = "journey\n    section S\n      Lo: 1: A";
        let j = parse(src).unwrap();
        assert_eq!(j.sections[0].steps[0].score, 1);
    }

    #[test]
    fn parse_comments_stripped() {
        let src = "%% top\njourney\n    title Foo %% inline\n    section S %% c\n      Step: 3: Me %% trailing";
        let j = parse(src).unwrap();
        assert_eq!(j.title.as_deref(), Some("Foo"));
        assert_eq!(j.sections[0].name, "S");
        assert_eq!(j.sections[0].steps[0].label, "Step");
        assert_eq!(j.sections[0].steps[0].actors, vec!["Me".to_string()]);
    }

    #[test]
    fn parse_blank_lines_ignored() {
        let src = "\n\njourney\n\n    title T\n\n    section S\n\n      A: 3: X\n\n";
        let j = parse(src).unwrap();
        assert_eq!(j.title.as_deref(), Some("T"));
        assert_eq!(j.sections[0].steps.len(), 1);
    }

    #[test]
    fn parse_multiple_sections() {
        let src = "journey\n    section A\n      S1: 3: Me\n    section B\n      S2: 4: Me";
        let j = parse(src).unwrap();
        assert_eq!(j.sections.len(), 2);
        assert_eq!(j.sections[0].name, "A");
        assert_eq!(j.sections[1].name, "B");
        assert_eq!(j.sections[0].steps[0].label, "S1");
        assert_eq!(j.sections[1].steps[0].label, "S2");
    }

    #[test]
    fn parse_step_with_no_actors() {
        let src = "journey\n    section S\n      Alone: 3:";
        let j = parse(src).unwrap();
        let step = &j.sections[0].steps[0];
        assert_eq!(step.label, "Alone");
        assert_eq!(step.score, 3);
        assert!(step.actors.is_empty());
    }
}
