use super::ast::{Timeline, TimelineEvent, TimelineSection};
use crate::mermaid::ascii::error::AsciiRenderError;

pub fn parse(source: &str) -> Result<Timeline, AsciiRenderError> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err(AsciiRenderError::Empty);
    }

    let raw_lines: Vec<&str> = trimmed.lines().collect();

    let mut header_idx: Option<usize> = None;
    for (i, line) in raw_lines.iter().enumerate() {
        let t = strip_comment(line).trim();
        if t.is_empty() {
            continue;
        }
        if t == "timeline" || t.starts_with("timeline ") || t.starts_with("timeline\t") {
            header_idx = Some(i);
            break;
        } else {
            return Err(AsciiRenderError::Parse {
                line: i + 1,
                msg: format!("expected 'timeline' header, got: {t:?}"),
            });
        }
    }
    let header_idx = header_idx.ok_or(AsciiRenderError::Empty)?;

    let mut timeline = Timeline::default();
    let mut current_section: Option<TimelineSection> = None;

    for (offset, raw) in raw_lines.iter().enumerate().skip(header_idx + 1) {
        let stripped = strip_comment(raw);
        let line = stripped.trim();
        if line.is_empty() {
            continue;
        }
        let lineno = offset + 1;

        if let Some(rest) = strip_keyword(line, "title") {
            timeline.title = Some(rest.trim().to_string());
            continue;
        }

        if let Some(rest) = strip_keyword(line, "section") {
            if let Some(sec) = current_section.take() {
                timeline.sections.push(sec);
            }
            current_section = Some(TimelineSection {
                name: rest.trim().to_string(),
                entries: Vec::new(),
            });
            continue;
        }

        if let Some(rest) = line.strip_prefix(':') {
            let event = rest.trim().to_string();
            let section = current_section.get_or_insert_with(TimelineSection::default);
            let last = section
                .entries
                .last_mut()
                .ok_or_else(|| AsciiRenderError::Parse {
                    line: lineno,
                    msg: "continuation event with no prior period".into(),
                })?;
            last.events.push(event);
            continue;
        }

        if let Some(colon_idx) = line.find(':') {
            let (period_part, event_part) = line.split_at(colon_idx);
            let period = period_part.trim().to_string();
            let event = event_part[1..].trim().to_string();
            if period.is_empty() {
                return Err(AsciiRenderError::Parse {
                    line: lineno,
                    msg: "empty period".into(),
                });
            }
            let section = current_section.get_or_insert_with(TimelineSection::default);
            section.entries.push(TimelineEvent {
                period,
                events: vec![event],
            });
            continue;
        }

        return Err(AsciiRenderError::Parse {
            line: lineno,
            msg: format!("unrecognized line: {line:?}"),
        });
    }

    if let Some(sec) = current_section.take() {
        timeline.sections.push(sec);
    }

    Ok(timeline)
}

fn strip_comment(line: &str) -> &str {
    if let Some(idx) = line.find("%%") {
        &line[..idx]
    } else {
        line
    }
}

fn strip_keyword<'a>(s: &'a str, kw: &str) -> Option<&'a str> {
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
    fn parse_timeline_minimal() {
        let src = "timeline";
        let t = parse(src).unwrap();
        assert_eq!(t.title, None);
        assert!(t.sections.is_empty());
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
        let src = "timeline\n    title History of Tech";
        let t = parse(src).unwrap();
        assert_eq!(t.title.as_deref(), Some("History of Tech"));
    }

    #[test]
    fn parse_section() {
        let src = "timeline\n    section 2000s";
        let t = parse(src).unwrap();
        assert_eq!(t.sections.len(), 1);
        assert_eq!(t.sections[0].name, "2000s");
        assert!(t.sections[0].entries.is_empty());
    }

    #[test]
    fn parse_period_with_one_event() {
        let src = "timeline\n    section 2000s\n      2001 : Wikipedia launched";
        let t = parse(src).unwrap();
        assert_eq!(t.sections.len(), 1);
        assert_eq!(t.sections[0].entries.len(), 1);
        assert_eq!(t.sections[0].entries[0].period, "2001");
        assert_eq!(
            t.sections[0].entries[0].events,
            vec!["Wikipedia launched".to_string()]
        );
    }

    #[test]
    fn parse_period_with_multiple_events_continuation() {
        let src = "timeline\n    section 2000s\n      2004 : Facebook\n           : Gmail";
        let t = parse(src).unwrap();
        assert_eq!(t.sections[0].entries.len(), 1);
        assert_eq!(t.sections[0].entries[0].period, "2004");
        assert_eq!(t.sections[0].entries[0].events, vec!["Facebook", "Gmail"]);
    }

    #[test]
    fn parse_multiple_periods() {
        let src = "timeline\n    section 2000s\n      2001 : Wikipedia\n      2004 : Facebook";
        let t = parse(src).unwrap();
        assert_eq!(t.sections[0].entries.len(), 2);
        assert_eq!(t.sections[0].entries[0].period, "2001");
        assert_eq!(t.sections[0].entries[1].period, "2004");
    }

    #[test]
    fn parse_multiple_sections() {
        let src = "timeline\n    section 2000s\n      2001 : Wikipedia\n    section 2010s\n      2010 : Instagram";
        let t = parse(src).unwrap();
        assert_eq!(t.sections.len(), 2);
        assert_eq!(t.sections[0].name, "2000s");
        assert_eq!(t.sections[1].name, "2010s");
        assert_eq!(t.sections[1].entries[0].period, "2010");
    }

    #[test]
    fn parse_no_section_period_at_root() {
        let src = "timeline\n    2001 : Wikipedia";
        let t = parse(src).unwrap();
        assert_eq!(t.sections.len(), 1);
        assert_eq!(t.sections[0].name, "");
        assert_eq!(t.sections[0].entries[0].period, "2001");
    }

    #[test]
    fn parse_comments_stripped() {
        let src =
            "%% top\ntimeline\n    %% mid\n    title T\n    section S\n      2001 : E1 %% inline";
        let t = parse(src).unwrap();
        assert_eq!(t.title.as_deref(), Some("T"));
        assert_eq!(t.sections[0].name, "S");
        assert_eq!(t.sections[0].entries[0].events, vec!["E1"]);
    }

    #[test]
    fn parse_blank_lines_ignored() {
        let src = "\n\ntimeline\n\n    title T\n\n    section S\n\n      2001 : E1\n\n";
        let t = parse(src).unwrap();
        assert_eq!(t.title.as_deref(), Some("T"));
        assert_eq!(t.sections[0].entries.len(), 1);
    }
}
