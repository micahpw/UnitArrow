//! A minimal JSON reader.
//!
//! Needed twice over: the extension metadata blob is UTF-8 JSON (§5.2), and the
//! conformance fixtures are JSON files. `unitarrow-core` depends on nothing
//! (§4), so this is in-crate.
//!
//! Note [`Json::raw_token`]: it preserves the **source text** of a string value,
//! not just its decoded form. §5.2's escape prohibition can only be checked
//! against the raw token, because a JSON parser decodes escapes before the unit
//! string exists — so a reader that only sees decoded values cannot enforce it.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    /// Decoded value plus the raw source token (including quotes).
    Str {
        value: String,
        raw: String,
    },
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

impl Json {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str { value, .. } => Some(value),
            _ => None,
        }
    }

    /// The raw source token of a string value, quotes included — the only form
    /// against which §5.2's escape rule can be checked.
    pub fn raw_token(&self) -> Option<&str> {
        match self {
            Json::Str { raw, .. } => Some(raw),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            // `f64::fract` lives in std; the integer test does not need it.
            Json::Number(n) if *n == (*n as i64) as f64 => Some(*n as i64),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, Json>> {
        match self {
            Json::Object(o) => Some(o),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        self.as_object()?.get(key)
    }
}

pub fn parse(src: &str) -> Result<Json, String> {
    let b = src.as_bytes();
    let mut i = 0;
    let v = parse_value(b, src, &mut i)?;
    skip_ws(b, &mut i);
    if i != b.len() {
        return Err(format!("trailing input at byte {i}"));
    }
    Ok(v)
}

fn skip_ws(b: &[u8], i: &mut usize) {
    while *i < b.len() && matches!(b[*i], b' ' | b'\t' | b'\n' | b'\r') {
        *i += 1;
    }
}

fn parse_value(b: &[u8], src: &str, i: &mut usize) -> Result<Json, String> {
    skip_ws(b, i);
    match b.get(*i) {
        None => Err("unexpected end of input".into()),
        Some(b'{') => parse_object(b, src, i),
        Some(b'[') => parse_array(b, src, i),
        Some(b'"') => parse_string(b, src, i),
        Some(b't') => lit(b, i, "true", Json::Bool(true)),
        Some(b'f') => lit(b, i, "false", Json::Bool(false)),
        Some(b'n') => lit(b, i, "null", Json::Null),
        Some(_) => parse_number(b, src, i),
    }
}

fn lit(b: &[u8], i: &mut usize, word: &str, v: Json) -> Result<Json, String> {
    if b[*i..].starts_with(word.as_bytes()) {
        *i += word.len();
        Ok(v)
    } else {
        Err(format!("invalid literal at byte {i}", i = *i))
    }
}

fn parse_object(b: &[u8], src: &str, i: &mut usize) -> Result<Json, String> {
    *i += 1; // '{'
    let mut map = BTreeMap::new();
    skip_ws(b, i);
    if b.get(*i) == Some(&b'}') {
        *i += 1;
        return Ok(Json::Object(map));
    }
    loop {
        skip_ws(b, i);
        let key = match parse_string(b, src, i)? {
            Json::Str { value, .. } => value,
            _ => unreachable!(),
        };
        skip_ws(b, i);
        if b.get(*i) != Some(&b':') {
            return Err(format!("expected ':' at byte {}", *i));
        }
        *i += 1;
        let v = parse_value(b, src, i)?;
        map.insert(key, v);
        skip_ws(b, i);
        match b.get(*i) {
            Some(b',') => *i += 1,
            Some(b'}') => {
                *i += 1;
                return Ok(Json::Object(map));
            }
            _ => return Err(format!("expected ',' or '}}' at byte {}", *i)),
        }
    }
}

fn parse_array(b: &[u8], src: &str, i: &mut usize) -> Result<Json, String> {
    *i += 1; // '['
    let mut items = Vec::new();
    skip_ws(b, i);
    if b.get(*i) == Some(&b']') {
        *i += 1;
        return Ok(Json::Array(items));
    }
    loop {
        items.push(parse_value(b, src, i)?);
        skip_ws(b, i);
        match b.get(*i) {
            Some(b',') => *i += 1,
            Some(b']') => {
                *i += 1;
                return Ok(Json::Array(items));
            }
            _ => return Err(format!("expected ',' or ']' at byte {}", *i)),
        }
    }
}

fn parse_string(b: &[u8], src: &str, i: &mut usize) -> Result<Json, String> {
    if b.get(*i) != Some(&b'"') {
        return Err(format!("expected string at byte {}", *i));
    }
    let start = *i;
    *i += 1;
    let mut value = String::new();
    loop {
        match b.get(*i) {
            None => return Err("unterminated string".into()),
            Some(b'"') => {
                *i += 1;
                return Ok(Json::Str {
                    value,
                    raw: src[start..*i].to_string(),
                });
            }
            Some(b'\\') => {
                *i += 1;
                let e = *b.get(*i).ok_or("unterminated escape")?;
                *i += 1;
                match e {
                    b'"' => value.push('"'),
                    b'\\' => value.push('\\'),
                    b'/' => value.push('/'),
                    b'b' => value.push('\u{8}'),
                    b'f' => value.push('\u{c}'),
                    b'n' => value.push('\n'),
                    b'r' => value.push('\r'),
                    b't' => value.push('\t'),
                    b'u' => {
                        let hex = src.get(*i..*i + 4).ok_or("truncated \\u escape")?;
                        let cp = u32::from_str_radix(hex, 16).map_err(|_| "bad \\u escape")?;
                        *i += 4;
                        // A lone surrogate is legal JSON and not valid UTF-8 —
                        // preserved as U+FFFD so the caller can still inspect the
                        // raw token and reject it (§5.2).
                        value.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                    }
                    other => return Err(format!("bad escape \\{}", other as char)),
                }
            }
            Some(_) => {
                let c = src[*i..].chars().next().ok_or("invalid UTF-8")?;
                value.push(c);
                *i += c.len_utf8();
            }
        }
    }
}

fn parse_number(b: &[u8], src: &str, i: &mut usize) -> Result<Json, String> {
    let start = *i;
    if b.get(*i) == Some(&b'-') {
        *i += 1;
    }
    while *i < b.len()
        && (b[*i].is_ascii_digit() || matches!(b[*i], b'.' | b'e' | b'E' | b'+' | b'-'))
    {
        *i += 1;
    }
    src[start..*i]
        .parse::<f64>()
        .map(Json::Number)
        .map_err(|_| format!("invalid number at byte {start}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_nested_structures() {
        let v = parse(r#"{"a": [1, 2.5, true, null], "b": {"c": "x"}}"#).unwrap();
        assert_eq!(v.get("a").unwrap().as_array().unwrap().len(), 4);
        assert_eq!(v.get("b").unwrap().get("c").unwrap().as_str(), Some("x"));
    }

    #[test]
    fn preserves_the_raw_token_for_escape_checking() {
        // Both decode to `MW`; only the raw token distinguishes them, which is
        // exactly what §5.2's escape rule needs.
        let v = parse(r#"{"unit": "MW"}"#).unwrap();
        let u = v.get("unit").unwrap();
        assert_eq!(u.as_str(), Some("MW"));
        assert_eq!(u.raw_token(), Some(r#""MW""#));

        let v = parse(r#"{"unit": "MW"}"#).unwrap();
        assert_eq!(v.get("unit").unwrap().raw_token(), Some(r#""MW""#));
    }

    #[test]
    fn decodes_escapes() {
        let v = parse(r#""a\/b\nc\\d""#).unwrap();
        assert_eq!(v.as_str(), Some("a/b\nc\\d"));
    }

    #[test]
    fn rejects_trailing_input() {
        assert!(parse("{} {}").is_err());
    }
}
