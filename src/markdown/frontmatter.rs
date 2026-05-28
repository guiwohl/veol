use crate::markdown::model::FrontmatterKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterBlock {
    pub kind: FrontmatterKind,
    pub pairs: Vec<(String, String)>,
}

pub fn extract(input: &str) -> Option<(FrontmatterBlock, &str)> {
    let (delim, kind) = if input.starts_with("---\n") || input.starts_with("---\r\n") {
        ("---", FrontmatterKind::Yaml)
    } else if input.starts_with("+++\n") || input.starts_with("+++\r\n") {
        ("+++", FrontmatterKind::Toml)
    } else {
        return None;
    };

    let after_open = skip_line(input)?;
    let (inner, rest) = find_close(after_open, delim)?;

    let pairs = match kind {
        FrontmatterKind::Yaml => parse_yaml_pairs(inner),
        FrontmatterKind::Toml => parse_toml_pairs(inner),
    };

    Some((FrontmatterBlock { kind, pairs }, rest))
}

fn skip_line(s: &str) -> Option<&str> {
    let nl = s.find('\n')?;
    Some(&s[nl + 1..])
}

fn find_close<'a>(after_open: &'a str, delim: &str) -> Option<(&'a str, &'a str)> {
    let mut cursor = 0usize;
    let bytes = after_open.as_bytes();
    while cursor < bytes.len() {
        let line_end = memchr_newline(&bytes[cursor..]).map(|i| cursor + i);
        let (line, next_cursor) = match line_end {
            Some(end) => (&after_open[cursor..end], end + 1),
            None => (&after_open[cursor..], bytes.len()),
        };
        let trimmed = line.trim_end_matches('\r');
        if trimmed == delim {
            let inner = &after_open[..cursor];
            let rest = if next_cursor <= after_open.len() {
                &after_open[next_cursor..]
            } else {
                ""
            };
            return Some((inner, rest));
        }
        line_end?;
        cursor = next_cursor;
    }
    None
}

fn memchr_newline(bytes: &[u8]) -> Option<usize> {
    bytes.iter().position(|&b| b == b'\n')
}

fn parse_yaml_pairs(inner: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for raw_line in inner.lines() {
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        if line.trim_start().starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim().to_string();
            let value = v.trim().trim_matches(|c: char| c == '"' || c == '\'');
            if key.is_empty() {
                continue;
            }
            out.push((key, value.to_string()));
        } else {
            out.push((line.trim().to_string(), String::new()));
        }
    }
    out
}

fn parse_toml_pairs(inner: &str) -> Vec<(String, String)> {
    let Ok(value) = inner.parse::<toml::Value>() else {
        return Vec::new();
    };
    let Some(table) = value.as_table() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (key, val) in table {
        match val {
            toml::Value::Table(inner_tbl) => {
                for (sub_key, sub_val) in inner_tbl {
                    out.push((format!("{key}.{sub_key}"), toml_scalar(sub_val)));
                }
            }
            other => out.push((key.clone(), toml_scalar(other))),
        }
    }
    out
}

fn toml_scalar(v: &toml::Value) -> String {
    match v {
        toml::Value::String(s) => s.clone(),
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::Float(f) => f.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Datetime(d) => d.to_string(),
        toml::Value::Array(a) => {
            let items: Vec<String> = a.iter().map(toml_scalar).collect();
            format!("[{}]", items.join(", "))
        }
        toml::Value::Table(_) => "<table>".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_frontmatter_extracted_with_rest_preserved() {
        let input = "---\ntitle: Hello\nauthor: Gui\n---\n# Body\n";
        let (block, rest) = extract(input).expect("frontmatter");
        assert_eq!(block.kind, FrontmatterKind::Yaml);
        assert_eq!(
            block.pairs,
            vec![
                ("title".to_string(), "Hello".to_string()),
                ("author".to_string(), "Gui".to_string()),
            ]
        );
        assert_eq!(rest, "# Body\n");
    }

    #[test]
    fn toml_frontmatter_extracted() {
        let input = "+++\ntitle = \"Hello\"\n[meta]\ntags = [\"a\", \"b\"]\n+++\nbody\n";
        let (block, rest) = extract(input).expect("frontmatter");
        assert_eq!(block.kind, FrontmatterKind::Toml);
        assert!(block
            .pairs
            .iter()
            .any(|(k, v)| k == "title" && v == "Hello"));
        assert!(block
            .pairs
            .iter()
            .any(|(k, v)| k == "meta.tags" && v == "[a, b]"));
        assert_eq!(rest, "body\n");
    }

    #[test]
    fn no_frontmatter_returns_none() {
        assert!(extract("# Just a heading\n").is_none());
    }

    #[test]
    fn unclosed_yaml_frontmatter_returns_none() {
        assert!(extract("---\ntitle: Hello\nbody without close\n").is_none());
    }

    #[test]
    fn empty_yaml_frontmatter_yields_empty_pairs() {
        let input = "---\n---\nrest\n";
        let (block, rest) = extract(input).expect("frontmatter");
        assert_eq!(block.kind, FrontmatterKind::Yaml);
        assert!(block.pairs.is_empty());
        assert_eq!(rest, "rest\n");
    }

    #[test]
    fn yaml_quoted_values_have_quotes_stripped() {
        let input = "---\ntitle: \"Hello\"\n---\n";
        let (block, _) = extract(input).expect("frontmatter");
        assert_eq!(block.pairs, vec![("title".into(), "Hello".into())]);
    }

    #[test]
    fn yaml_line_without_colon_becomes_key_with_empty_value() {
        let input = "---\nbare\nkey: v\n---\n";
        let (block, _) = extract(input).expect("frontmatter");
        assert!(block.pairs.contains(&("bare".into(), String::new())));
        assert!(block.pairs.contains(&("key".into(), "v".into())));
    }
}
