//! A minimal TOML reader covering exactly the registry subset (§7).
//!
//! `unitarrow-core` depends on nothing (§4), so the registry loader cannot pull
//! in a TOML crate. This handles what §7.1–7.3 actually use: `[table.sub]`
//! headers with dotted and quoted keys, integers, strings, arrays of integers
//! and strings, and single-line inline tables.
//!
//! Deliberately *not* supported: multi-line inline tables (invalid TOML anyway —
//! the defect this project logged as AMB-027), arrays of tables, dates, floats.
//! Floats are rejected rather than ignored, because a float in a `factor` is
//! precisely the auditability failure §7.2 exists to prevent.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Integer(i128),
    Bool(bool),
    Str(String),
    Array(Vec<Value>),
    Table(BTreeMap<String, Value>),
}

impl Value {
    pub fn as_integer(&self) -> Option<i128> {
        match self {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }
    pub fn as_table(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Value::Table(t) => Some(t),
            _ => None,
        }
    }
    /// An exact-rational `[num, den]` pair.
    pub fn as_pair(&self) -> Option<(i128, i128)> {
        let a = self.as_array()?;
        if a.len() != 2 {
            return None;
        }
        Some((a[0].as_integer()?, a[1].as_integer()?))
    }
}

/// Parse a TOML document into a nested table. `Err` carries a human-readable
/// reason with a 1-based line number.
pub fn parse(src: &str) -> Result<BTreeMap<String, Value>, String> {
    let mut root: BTreeMap<String, Value> = BTreeMap::new();
    let mut path: Vec<String> = Vec::new();

    for (lineno, raw) in src.lines().enumerate() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        let n = lineno + 1;

        if let Some(header) = line.strip_prefix('[') {
            let header = header
                .strip_suffix(']')
                .ok_or_else(|| format!("line {n}: unterminated table header"))?;
            if header.starts_with('[') {
                return Err(format!("line {n}: arrays of tables are not supported"));
            }
            path = split_key(header).map_err(|e| format!("line {n}: {e}"))?;
            ensure_table(&mut root, &path).map_err(|e| format!("line {n}: {e}"))?;
            continue;
        }

        let (key, rest) = line
            .split_once('=')
            .ok_or_else(|| format!("line {n}: expected `key = value`"))?;
        let keys = split_key(key.trim()).map_err(|e| format!("line {n}: {e}"))?;
        let (value, tail) = parse_value(rest.trim()).map_err(|e| format!("line {n}: {e}"))?;
        if !tail.trim().is_empty() {
            return Err(format!(
                "line {n}: trailing input after value: {:?}",
                tail.trim()
            ));
        }

        let mut full = path.clone();
        full.extend(keys);
        let (leaf, parents) = full.split_last().unwrap();
        let table = ensure_table(&mut root, parents).map_err(|e| format!("line {n}: {e}"))?;
        if table.insert(leaf.clone(), value).is_some() {
            return Err(format!("line {n}: duplicate key {leaf:?}"));
        }
    }
    Ok(root)
}

fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut in_str = false;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => in_str = !in_str,
            b'\\' if in_str => i += 1,
            b'#' if !in_str => return &line[..i],
            _ => {}
        }
        i += 1;
    }
    line
}

fn split_key(s: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                // Quoted key segment, e.g. [quantity."core:power"].
                for q in chars.by_ref() {
                    if q == '"' {
                        break;
                    }
                    cur.push(q);
                }
            }
            '.' => {
                out.push(core::mem::take(&mut cur).trim().to_string());
            }
            _ => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    if out.iter().any(|k| k.is_empty()) {
        return Err("empty key segment".into());
    }
    Ok(out)
}

fn ensure_table<'a>(
    root: &'a mut BTreeMap<String, Value>,
    path: &[String],
) -> Result<&'a mut BTreeMap<String, Value>, String> {
    let mut cur = root;
    for key in path {
        let entry = cur
            .entry(key.clone())
            .or_insert_with(|| Value::Table(BTreeMap::new()));
        cur = match entry {
            Value::Table(t) => t,
            _ => return Err(format!("key {key:?} is not a table")),
        };
    }
    Ok(cur)
}

fn parse_value(s: &str) -> Result<(Value, &str), String> {
    let s_trim = s.trim_start();
    let consumed = s.len() - s_trim.len();
    let (v, rest) = match s_trim.chars().next() {
        None => return Err("missing value".into()),
        Some('"') => parse_string(s_trim)?,
        Some('\'') => parse_literal_string(s_trim)?,
        Some('[') => parse_array(s_trim)?,
        Some('{') => parse_inline_table(s_trim)?,
        Some(c) if c == '-' || c == '+' || c.is_ascii_digit() => parse_number(s_trim)?,
        Some('t') | Some('f') => {
            // §7.3 uses `interval = true`.
            if let Some(r) = s_trim.strip_prefix("true") {
                (Value::Bool(true), r)
            } else if let Some(r) = s_trim.strip_prefix("false") {
                (Value::Bool(false), r)
            } else {
                return Err("expected `true` or `false`".into());
            }
        }
        Some(c) => return Err(format!("unexpected value starting with {c:?}")),
    };
    let _ = consumed;
    Ok((v, rest))
}

fn parse_string(s: &str) -> Result<(Value, &str), String> {
    let bytes = s.as_bytes();
    let mut out = String::new();
    let mut i = 1;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => return Ok((Value::Str(out), &s[i + 1..])),
            b'\\' => {
                i += 1;
                let e = *bytes.get(i).ok_or("unterminated escape")?;
                out.push(match e {
                    b'n' => '\n',
                    b't' => '\t',
                    b'r' => '\r',
                    b'"' => '"',
                    b'\\' => '\\',
                    b'u' => {
                        let hex = s.get(i + 1..i + 5).ok_or("truncated \\u escape")?;
                        let cp = u32::from_str_radix(hex, 16).map_err(|_| "bad \\u escape")?;
                        i += 4;
                        char::from_u32(cp).ok_or("invalid code point")?
                    }
                    other => return Err(format!("unsupported escape \\{}", other as char)),
                });
                i += 1;
            }
            _ => {
                let c = s[i..].chars().next().unwrap();
                out.push(c);
                i += c.len_utf8();
            }
        }
    }
    Err("unterminated string".into())
}

fn parse_literal_string(s: &str) -> Result<(Value, &str), String> {
    let rest = &s[1..];
    let end = rest.find('\'').ok_or("unterminated literal string")?;
    Ok((Value::Str(rest[..end].to_string()), &rest[end + 1..]))
}

fn parse_number(s: &str) -> Result<(Value, &str), String> {
    let mut i = 0;
    let bytes = s.as_bytes();
    if bytes[0] == b'-' || bytes[0] == b'+' {
        i = 1;
    }
    let start_digits = i;
    while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'_') {
        i += 1;
    }
    if i == start_digits {
        return Err("expected digits".into());
    }
    // A float in a registry factor is the auditability failure §7.2 forbids.
    if i < bytes.len() && (bytes[i] == b'.' || bytes[i] == b'e' || bytes[i] == b'E') {
        return Err(
            "floats are not permitted in a registry; factors and offsets are exact rationals \
             written as [numerator, denominator] (§7.2)"
                .into(),
        );
    }
    let text: String = s[..i].chars().filter(|c| *c != '_').collect();
    let n: i128 = text
        .parse()
        .map_err(|_| "integer out of range".to_string())?;
    Ok((Value::Integer(n), &s[i..]))
}

fn parse_array(s: &str) -> Result<(Value, &str), String> {
    let mut rest = &s[1..];
    let mut items = Vec::new();
    loop {
        rest = rest.trim_start();
        if let Some(r) = rest.strip_prefix(']') {
            return Ok((Value::Array(items), r));
        }
        if rest.is_empty() {
            // Multi-line arrays are legal TOML but unused by the registry
            // subset; failing loudly beats a half-read factor.
            return Err("unterminated or multi-line array".into());
        }
        let (v, r) = parse_value(rest)?;
        items.push(v);
        rest = r.trim_start();
        if let Some(r) = rest.strip_prefix(',') {
            rest = r;
        }
    }
}

fn parse_inline_table(s: &str) -> Result<(Value, &str), String> {
    let mut rest = &s[1..];
    let mut table = BTreeMap::new();
    loop {
        rest = rest.trim_start();
        if let Some(r) = rest.strip_prefix('}') {
            return Ok((Value::Table(table), r));
        }
        if rest.is_empty() {
            return Err(
                "unterminated inline table — TOML forbids newlines inside inline tables \
                 (see the §7.2 example defect)"
                    .into(),
            );
        }
        let (k, r) = rest
            .split_once('=')
            .ok_or("expected `key = value` in inline table")?;
        let keys = split_key(k.trim())?;
        if keys.len() != 1 {
            return Err("dotted keys inside inline tables are not supported".into());
        }
        let (v, r2) = parse_value(r.trim_start())?;
        table.insert(keys[0].clone(), v);
        rest = r2.trim_start();
        if let Some(r3) = rest.strip_prefix(',') {
            rest = r3;
        }
    }
}

/// Serialize a parsed document back to TOML, deterministically.
///
/// Needed because §5.6 pins a registry by *content hash*: a composed registry
/// has no authored file to hash, so its bytes must be generated, and two
/// implementations composing the same inputs must generate the same bytes or the
/// pin means nothing. Determinism comes free from `BTreeMap` ordering plus a
/// fixed shape — scalars before sub-tables, one blank line between tables.
///
/// Round-trips: `parse(&write(&parse(s)?)) == parse(s)?`.
pub fn write(doc: &BTreeMap<String, Value>) -> String {
    let mut out = String::new();
    write_table(&mut out, &[], doc);
    out
}

fn write_table(out: &mut String, path: &[&str], table: &BTreeMap<String, Value>) {
    // Scalars first: TOML binds bare keys to the most recent header, so a
    // sub-table emitted early would swallow every key after it.
    let (subs, scalars): (Vec<_>, Vec<_>) = table
        .iter()
        .partition(|(_, v)| matches!(v, Value::Table(_)));

    if !path.is_empty() && (!scalars.is_empty() || subs.is_empty()) {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push('[');
        for (i, seg) in path.iter().enumerate() {
            if i > 0 {
                out.push('.');
            }
            out.push_str(&quote_key(seg));
        }
        out.push_str("]\n");
    }
    for (k, v) in scalars {
        out.push_str(&quote_key(k));
        out.push_str(" = ");
        write_value(out, v);
        out.push('\n');
    }
    for (k, v) in subs {
        let Value::Table(t) = v else { unreachable!() };
        let mut child: Vec<&str> = path.to_vec();
        child.push(k);
        write_table(out, &child, t);
    }
}

/// Inline form — used inside arrays and for nested tables that appear as values.
fn write_value(out: &mut String, v: &Value) {
    match v {
        Value::Integer(i) => {
            let mut buf = String::new();
            let _ = core::fmt::Write::write_fmt(&mut buf, format_args!("{i}"));
            out.push_str(&buf);
        }
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Str(s) => out.push_str(&quote_string(s)),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write_value(out, item);
            }
            out.push(']');
        }
        Value::Table(t) => {
            out.push_str("{ ");
            for (i, (k, val)) in t.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&quote_key(k));
                out.push_str(" = ");
                write_value(out, val);
            }
            out.push_str(" }");
        }
    }
}

fn quote_key(k: &str) -> String {
    let bare = !k.is_empty()
        && k.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if bare {
        k.to_string()
    } else {
        quote_string(k)
    }
}

fn quote_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_bytes_that_parse_back_to_the_same_document() {
        let src = "[registry]\nschema_version = 1\nname = \"core\"\n\
                   [registry.prefix_collisions]\nft = \"foot wins here\"\n\
                   [dimension.power]\nvector = { mass = 1, length = 2, time = -3 }\n\
                   [unit.MW]\ndimension = \"power\"\nfactor = [1000000, 1]\n\
                   aliases = [\"megawatt\"]\n";
        let doc = parse(src).unwrap();
        let written = write(&doc);
        assert_eq!(parse(&written).unwrap(), doc, "round-trip:\n{written}");
        // Byte-identical on a second pass — the property the content hash needs.
        assert_eq!(write(&parse(&written).unwrap()), written);
    }

    #[test]
    fn orders_scalars_before_sub_tables() {
        // `[registry] name = ...` followed by `[registry.prefix_collisions]`:
        // emitting the sub-table first would capture `name` into it.
        let doc = parse(
            "[registry.prefix_collisions]\nft = \"x\"\n[registry]\nname = \"c\"\nversion = \"1\"\n",
        )
        .unwrap();
        let written = write(&doc);
        let reparsed = parse(&written).unwrap();
        let header = reparsed.get("registry").unwrap().as_table().unwrap();
        assert_eq!(header.get("name").unwrap().as_str(), Some("c"));
        assert!(header
            .get("prefix_collisions")
            .unwrap()
            .as_table()
            .is_some());
    }

    #[test]
    fn reads_the_registry_shape() {
        let src = r#"
[registry]
schema_version = 1
name = "core"

[dimension.power]
vector = { mass = 1, length = 2, time = -3 }

[unit.MW]
dimension = "power"
factor = [1000000, 1]
aliases = ["megawatt"]
display = { unicode = "MW", long = "megawatt" }
"#;
        let doc = parse(src).unwrap();
        let unit = doc["unit"].as_table().unwrap()["MW"].as_table().unwrap();
        assert_eq!(unit["factor"].as_pair(), Some((1_000_000, 1)));
        assert_eq!(unit["dimension"].as_str(), Some("power"));
        let vec = doc["dimension"].as_table().unwrap()["power"]
            .as_table()
            .unwrap()["vector"]
            .as_table()
            .unwrap();
        assert_eq!(vec["time"].as_integer(), Some(-3));
    }

    #[test]
    fn handles_quoted_keys_for_curies() {
        let doc = parse("[quantity.\"core:power\"]\nrate_of = \"core:energy\"\n").unwrap();
        let q = doc["quantity"].as_table().unwrap();
        assert!(q.contains_key("core:power"));
    }

    #[test]
    fn rejects_floats_in_factors() {
        let e = parse("[unit.x]\nfactor = 1.5\n").unwrap_err();
        assert!(e.contains("exact rationals"), "{e}");
    }

    #[test]
    fn rejects_multiline_inline_tables() {
        // Exactly the defect in the published §7.2 example.
        let e = parse("[unit.degC]\ndisplay = { unicode = \"C\",\n long = \"x\" }\n").unwrap_err();
        assert!(e.contains("inline table"), "{e}");
    }

    #[test]
    fn strips_comments_outside_strings() {
        let doc = parse("[a]\nb = 1 # trailing\nc = \"has # inside\"\n").unwrap();
        let a = doc["a"].as_table().unwrap();
        assert_eq!(a["b"].as_integer(), Some(1));
        assert_eq!(a["c"].as_str(), Some("has # inside"));
    }

    #[test]
    fn reads_booleans_for_interval_kinds() {
        let doc = parse("[quantity.d]\ninterval = true\n[quantity.a]\ninterval = false\n").unwrap();
        let q = doc["quantity"].as_table().unwrap();
        assert_eq!(q["d"].as_table().unwrap()["interval"].as_bool(), Some(true));
        assert_eq!(
            q["a"].as_table().unwrap()["interval"].as_bool(),
            Some(false)
        );
    }

    #[test]
    fn rejects_duplicate_keys() {
        assert!(parse("[a]\nb = 1\nb = 2\n")
            .unwrap_err()
            .contains("duplicate"));
    }
}
