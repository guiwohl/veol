use super::ast::{
    Element, ReqRelation, ReqRelationKind, ReqType, Requirement, RequirementDiagram, RiskLevel,
    VerifyMethod,
};
use crate::mermaid::ascii::error::AsciiRenderError;

pub fn parse(source: &str) -> Result<RequirementDiagram, AsciiRenderError> {
    let lines = preprocess(source);
    if lines.is_empty() {
        return Err(AsciiRenderError::Parse {
            line: 0,
            msg: "empty source".into(),
        });
    }

    let mut idx = 0;
    let header = lines[idx].1.trim();
    if header != "requirementDiagram" {
        return Err(AsciiRenderError::Parse {
            line: lines[idx].0,
            msg: format!("expected 'requirementDiagram' header, found '{header}'"),
        });
    }
    idx += 1;

    let mut diag = RequirementDiagram::default();

    while idx < lines.len() {
        let (lineno, line) = lines[idx].clone();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            idx += 1;
            continue;
        }

        if let Some((kind_kw, name_part)) = try_split_block_header(trimmed) {
            if !trimmed.ends_with('{') {
                return Err(AsciiRenderError::Parse {
                    line: lineno,
                    msg: format!("block header missing '{{' on line: '{trimmed}'"),
                });
            }
            let body_name = name_part.trim_end_matches('{').trim().to_string();
            if body_name.is_empty() {
                return Err(AsciiRenderError::Parse {
                    line: lineno,
                    msg: format!("block '{kind_kw}' missing name"),
                });
            }
            idx += 1;
            let (kv, new_idx) = collect_block_body(&lines, idx, lineno, kind_kw)?;
            idx = new_idx;

            match classify_kind(kind_kw) {
                BlockKind::Requirement(rk) => {
                    let req = build_requirement(&body_name, rk, &kv, lineno)?;
                    diag.requirements.insert(body_name, req);
                }
                BlockKind::Element => {
                    let el = build_element(&body_name, &kv, lineno)?;
                    diag.elements.insert(body_name, el);
                }
            }
            continue;
        }

        if let Some(rel) = try_parse_relation(trimmed) {
            let rel = rel.map_err(|m| AsciiRenderError::Parse {
                line: lineno,
                msg: m,
            })?;
            diag.relations.push(rel);
            idx += 1;
            continue;
        }

        return Err(AsciiRenderError::Parse {
            line: lineno,
            msg: format!("unrecognized line '{trimmed}'"),
        });
    }

    if diag.requirements.is_empty() && diag.elements.is_empty() {
        return Err(AsciiRenderError::Empty);
    }

    Ok(diag)
}

#[derive(Debug, Clone, Copy)]
enum BlockKind {
    Requirement(Option<ReqType>),
    Element,
}

fn classify_kind(kw: &str) -> BlockKind {
    match kw {
        "requirement" => BlockKind::Requirement(Some(ReqType::Requirement)),
        "functionalRequirement" => BlockKind::Requirement(Some(ReqType::FunctionalRequirement)),
        "interfaceRequirement" => BlockKind::Requirement(Some(ReqType::InterfaceRequirement)),
        "performanceRequirement" => BlockKind::Requirement(Some(ReqType::PerformanceRequirement)),
        "physicalRequirement" => BlockKind::Requirement(Some(ReqType::PhysicalRequirement)),
        "designConstraint" => BlockKind::Requirement(Some(ReqType::DesignConstraint)),
        "element" => BlockKind::Element,
        _ => BlockKind::Requirement(None),
    }
}

fn try_split_block_header(s: &str) -> Option<(&str, &str)> {
    let kws = [
        "requirement",
        "functionalRequirement",
        "interfaceRequirement",
        "performanceRequirement",
        "physicalRequirement",
        "designConstraint",
        "element",
    ];
    for kw in kws {
        if let Some(rest) = s.strip_prefix(kw) {
            if rest.starts_with(char::is_whitespace) {
                return Some((kw, rest.trim_start()));
            }
        }
    }
    None
}

fn collect_block_body(
    lines: &[(usize, String)],
    mut idx: usize,
    header_lineno: usize,
    kind_kw: &str,
) -> Result<(Vec<(String, String)>, usize), AsciiRenderError> {
    let mut kv: Vec<(String, String)> = Vec::new();
    let mut closed = false;
    while idx < lines.len() {
        let (lineno, line) = &lines[idx];
        let trimmed = line.trim();
        idx += 1;
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "}" {
            closed = true;
            break;
        }
        let (k, v) = split_kv(trimmed).ok_or_else(|| AsciiRenderError::Parse {
            line: *lineno,
            msg: format!("expected 'key: value' inside '{kind_kw}' block, got '{trimmed}'"),
        })?;
        kv.push((k, v));
    }
    if !closed {
        return Err(AsciiRenderError::Parse {
            line: header_lineno,
            msg: format!("unterminated '{kind_kw}' block"),
        });
    }
    Ok((kv, idx))
}

fn split_kv(s: &str) -> Option<(String, String)> {
    let p = s.find(':')?;
    let k = s[..p].trim().to_string();
    let v = strip_quotes(s[p + 1..].trim()).to_string();
    if k.is_empty() {
        return None;
    }
    Some((k, v))
}

fn strip_quotes(s: &str) -> &str {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn build_requirement(
    name: &str,
    kind: Option<ReqType>,
    kv: &[(String, String)],
    lineno: usize,
) -> Result<Requirement, AsciiRenderError> {
    let mut req = Requirement {
        kind,
        name: name.to_string(),
        ..Default::default()
    };
    for (k, v) in kv {
        match k.as_str() {
            "id" => req.id = Some(v.clone()),
            "text" => req.text = Some(v.clone()),
            "risk" => {
                req.risk = Some(parse_risk(v).ok_or_else(|| AsciiRenderError::Parse {
                    line: lineno,
                    msg: format!("unknown risk level '{v}'"),
                })?);
            }
            "verifymethod" | "verifyMethod" => {
                req.verify = Some(parse_verify(v).ok_or_else(|| AsciiRenderError::Parse {
                    line: lineno,
                    msg: format!("unknown verify method '{v}'"),
                })?);
            }
            _ => {
                return Err(AsciiRenderError::Parse {
                    line: lineno,
                    msg: format!("unknown requirement field '{k}'"),
                });
            }
        }
    }
    Ok(req)
}

fn build_element(
    name: &str,
    kv: &[(String, String)],
    lineno: usize,
) -> Result<Element, AsciiRenderError> {
    let mut el = Element {
        name: name.to_string(),
        ..Default::default()
    };
    for (k, v) in kv {
        match k.as_str() {
            "type" => el.element_type = Some(v.clone()),
            "docref" | "docRef" => el.docref = Some(v.clone()),
            _ => {
                return Err(AsciiRenderError::Parse {
                    line: lineno,
                    msg: format!("unknown element field '{k}'"),
                });
            }
        }
    }
    Ok(el)
}

fn parse_risk(s: &str) -> Option<RiskLevel> {
    match s.to_ascii_lowercase().as_str() {
        "low" => Some(RiskLevel::Low),
        "medium" => Some(RiskLevel::Medium),
        "high" => Some(RiskLevel::High),
        _ => None,
    }
}

fn parse_verify(s: &str) -> Option<VerifyMethod> {
    match s.to_ascii_lowercase().as_str() {
        "analysis" => Some(VerifyMethod::Analysis),
        "inspection" => Some(VerifyMethod::Inspection),
        "test" => Some(VerifyMethod::Test),
        "demonstration" => Some(VerifyMethod::Demonstration),
        _ => None,
    }
}

fn try_parse_relation(s: &str) -> Option<Result<ReqRelation, String>> {
    let arrow = s.find("->")?;
    let after_arrow = s[arrow + 2..].trim().to_string();
    let before_arrow = s[..arrow].trim_end();
    let dash = before_arrow.rfind(" - ")?;
    let from = s[..dash].trim().to_string();
    let verb = s[dash + 3..arrow].trim().to_string();
    if from.is_empty() || verb.is_empty() || after_arrow.is_empty() {
        return Some(Err(format!("malformed relation '{s}'")));
    }
    let kind = match verb.to_ascii_lowercase().as_str() {
        "contains" => ReqRelationKind::Contains,
        "copies" => ReqRelationKind::Copies,
        "derives" => ReqRelationKind::Derives,
        "satisfies" => ReqRelationKind::Satisfies,
        "verifies" => ReqRelationKind::Verifies,
        "refines" => ReqRelationKind::Refines,
        "traces" => ReqRelationKind::Traces,
        _ => return Some(Err(format!("unknown relation verb '{verb}'"))),
    };
    Some(Ok(ReqRelation {
        from,
        to: after_arrow,
        kind,
    }))
}

fn preprocess(source: &str) -> Vec<(usize, String)> {
    let mut out: Vec<(usize, String)> = Vec::new();
    let mut in_frontmatter = false;
    let mut frontmatter_seen = false;
    for (i, raw) in source.lines().enumerate() {
        let stripped = strip_comment(raw);
        let trimmed = stripped.trim();
        if !frontmatter_seen && trimmed == "---" {
            in_frontmatter = true;
            frontmatter_seen = true;
            continue;
        }
        if in_frontmatter {
            if trimmed == "---" {
                in_frontmatter = false;
            }
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }
        out.push((i + 1, stripped));
    }
    out
}

fn strip_comment(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut in_str: Option<char> = None;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if let Some(q) = in_str {
            if c == q {
                in_str = None;
            }
        } else if c == '"' {
            in_str = Some('"');
        } else if c == '%' && i + 1 < bytes.len() && bytes[i + 1] as char == '%' {
            return line[..i].to_string();
        }
        i += 1;
    }
    line.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_requirement_minimal() {
        let src = "requirementDiagram\nrequirement r1 {\n  id: 1\n}";
        let d = parse(src).unwrap();
        assert!(d.requirements.contains_key("r1"));
        let r = &d.requirements["r1"];
        assert_eq!(r.id.as_deref(), Some("1"));
        assert_eq!(r.kind, Some(ReqType::Requirement));
    }

    #[test]
    fn parse_no_header_error() {
        let err = parse("requirement r1 {\n  id: 1\n}").unwrap_err();
        assert!(matches!(err, AsciiRenderError::Parse { .. }));
    }

    #[test]
    fn parse_requirement_block_all_fields() {
        let src = "requirementDiagram\nrequirement test_req {\n  id: 1\n  text: the test text.\n  risk: high\n  verifymethod: test\n}";
        let d = parse(src).unwrap();
        let r = &d.requirements["test_req"];
        assert_eq!(r.id.as_deref(), Some("1"));
        assert_eq!(r.text.as_deref(), Some("the test text."));
        assert_eq!(r.risk, Some(RiskLevel::High));
        assert_eq!(r.verify, Some(VerifyMethod::Test));
    }

    #[test]
    fn parse_functional_requirement() {
        let src = "requirementDiagram\nfunctionalRequirement fr {\n  id: 1.1\n}";
        let d = parse(src).unwrap();
        assert_eq!(
            d.requirements["fr"].kind,
            Some(ReqType::FunctionalRequirement)
        );
    }

    #[test]
    fn parse_interface_requirement() {
        let src = "requirementDiagram\ninterfaceRequirement ir {\n  id: 2\n}";
        let d = parse(src).unwrap();
        assert_eq!(
            d.requirements["ir"].kind,
            Some(ReqType::InterfaceRequirement)
        );
    }

    #[test]
    fn parse_performance_requirement() {
        let src = "requirementDiagram\nperformanceRequirement pr {\n  id: 3\n}";
        let d = parse(src).unwrap();
        assert_eq!(
            d.requirements["pr"].kind,
            Some(ReqType::PerformanceRequirement)
        );
    }

    #[test]
    fn parse_physical_requirement() {
        let src = "requirementDiagram\nphysicalRequirement ph {\n  id: 4\n}";
        let d = parse(src).unwrap();
        assert_eq!(
            d.requirements["ph"].kind,
            Some(ReqType::PhysicalRequirement)
        );
    }

    #[test]
    fn parse_design_constraint() {
        let src = "requirementDiagram\ndesignConstraint dc {\n  id: 5\n}";
        let d = parse(src).unwrap();
        assert_eq!(d.requirements["dc"].kind, Some(ReqType::DesignConstraint));
    }

    #[test]
    fn parse_element_with_type() {
        let src = "requirementDiagram\nelement test_entity {\n  type: simulation\n}";
        let d = parse(src).unwrap();
        let e = &d.elements["test_entity"];
        assert_eq!(e.element_type.as_deref(), Some("simulation"));
    }

    #[test]
    fn parse_element_with_docref() {
        let src = "requirementDiagram\nelement test_entity2 {\n  type: word doc\n  docRef: reqs/test_entity\n}";
        let d = parse(src).unwrap();
        let e = &d.elements["test_entity2"];
        assert_eq!(e.element_type.as_deref(), Some("word doc"));
        assert_eq!(e.docref.as_deref(), Some("reqs/test_entity"));
    }

    #[test]
    fn parse_relation_satisfies() {
        let src = "requirementDiagram\nelement a {\n  type: x\n}\nrequirement b {\n  id: 1\n}\na - satisfies -> b";
        let d = parse(src).unwrap();
        assert_eq!(d.relations.len(), 1);
        assert_eq!(d.relations[0].kind, ReqRelationKind::Satisfies);
        assert_eq!(d.relations[0].from, "a");
        assert_eq!(d.relations[0].to, "b");
    }

    #[test]
    fn parse_relation_traces() {
        let src = "requirementDiagram\nrequirement a {\n  id: 1\n}\nrequirement b {\n  id: 2\n}\na - traces -> b";
        let d = parse(src).unwrap();
        assert_eq!(d.relations[0].kind, ReqRelationKind::Traces);
    }

    #[test]
    fn parse_relation_contains() {
        let src = "requirementDiagram\nrequirement a {\n  id: 1\n}\nrequirement b {\n  id: 2\n}\na - contains -> b";
        let d = parse(src).unwrap();
        assert_eq!(d.relations[0].kind, ReqRelationKind::Contains);
    }

    #[test]
    fn parse_relation_copies() {
        let src = "requirementDiagram\nrequirement a {\n  id: 1\n}\nrequirement b {\n  id: 2\n}\na - copies -> b";
        let d = parse(src).unwrap();
        assert_eq!(d.relations[0].kind, ReqRelationKind::Copies);
    }

    #[test]
    fn parse_relation_derives() {
        let src = "requirementDiagram\nrequirement a {\n  id: 1\n}\nrequirement b {\n  id: 2\n}\na - derives -> b";
        let d = parse(src).unwrap();
        assert_eq!(d.relations[0].kind, ReqRelationKind::Derives);
    }

    #[test]
    fn parse_relation_verifies() {
        let src = "requirementDiagram\nrequirement a {\n  id: 1\n}\nrequirement b {\n  id: 2\n}\na - verifies -> b";
        let d = parse(src).unwrap();
        assert_eq!(d.relations[0].kind, ReqRelationKind::Verifies);
    }

    #[test]
    fn parse_relation_refines() {
        let src = "requirementDiagram\nrequirement a {\n  id: 1\n}\nrequirement b {\n  id: 2\n}\na - refines -> b";
        let d = parse(src).unwrap();
        assert_eq!(d.relations[0].kind, ReqRelationKind::Refines);
    }

    #[test]
    fn parse_risk_high_medium_low() {
        let src_h = "requirementDiagram\nrequirement r {\n  id: 1\n  risk: high\n}";
        let src_m = "requirementDiagram\nrequirement r {\n  id: 1\n  risk: medium\n}";
        let src_l = "requirementDiagram\nrequirement r {\n  id: 1\n  risk: low\n}";
        assert_eq!(
            parse(src_h).unwrap().requirements["r"].risk,
            Some(RiskLevel::High)
        );
        assert_eq!(
            parse(src_m).unwrap().requirements["r"].risk,
            Some(RiskLevel::Medium)
        );
        assert_eq!(
            parse(src_l).unwrap().requirements["r"].risk,
            Some(RiskLevel::Low)
        );
    }

    #[test]
    fn parse_verify_methods() {
        let cases = [
            ("analysis", VerifyMethod::Analysis),
            ("inspection", VerifyMethod::Inspection),
            ("test", VerifyMethod::Test),
            ("demonstration", VerifyMethod::Demonstration),
        ];
        for (s, expected) in cases {
            let src =
                format!("requirementDiagram\nrequirement r {{\n  id: 1\n  verifymethod: {s}\n}}");
            let d = parse(&src).unwrap();
            assert_eq!(d.requirements["r"].verify, Some(expected));
        }
    }

    #[test]
    fn parse_comments_stripped() {
        let src = "requirementDiagram\n%% top comment\nrequirement r {\n  id: 1\n}";
        let d = parse(src).unwrap();
        assert!(d.requirements.contains_key("r"));
    }

    #[test]
    fn parse_blank_lines_ignored() {
        let src = "requirementDiagram\n\n\nrequirement r {\n\n  id: 1\n\n}\n\n";
        let d = parse(src).unwrap();
        assert!(d.requirements.contains_key("r"));
    }
}
