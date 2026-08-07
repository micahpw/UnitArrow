//! The §5.2 extension metadata blob — building it and reading it back.
//!
//! # Why this is in the core rather than in each binding
//!
//! §4 is explicit that bindings must not reimplement core logic, and this blob
//! is where that rule earns its keep. Hand-rolling the JSON in Python, then
//! again in TypeScript, would mean the escape prohibition, the required keys,
//! the grammar version, and unknown-key preservation are each enforced three
//! times — which is to say, eventually, zero times. A binding calls
//! [`Metadata::to_json`] and [`Metadata::from_json`] and holds no opinions.
//!
//! # Unknown keys are preserved, and that is load-bearing
//!
//! §5.2 requires a reader to preserve keys it does not understand. That looks
//! like politeness and is not: the companion hashes the schema, so a reader
//! that drops an unrecognised key changes a data digest and breaks a seal it
//! never knew about (AMB-024). [`Metadata::from_json`] keeps them verbatim and
//! [`Metadata::to_json`] writes them back.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::{Error, ErrorCode, Result};
use crate::json_lite::{self, Json};
use crate::{check_unit_token, GRAMMAR_VERSION};

fn bad(msg: impl Into<String>) -> Error {
    Error::new(ErrorCode::BadMetadata, msg)
}

/// The §5.4 base for a per-unit column.
///
/// Column-uniform by construction: §2 makes per-row bases a non-goal, and §5.4
/// says a column whose base varies per row — voltage p.u. across voltage
/// levels, the normal case in a multi-zone model — must be shipped as physical
/// units instead.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Base {
    pub quantity: Option<String>,
    pub value: Option<f64>,
    pub unit: Option<String>,
}

/// §8.5 temporal semantics of each row.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Temporal {
    /// `instant` or `interval`.
    pub kind: Option<String>,
    /// For intervals: `mean`, `sum`, `min`, `max`.
    pub statistic: Option<String>,
    /// ISO-8601 duration.
    pub period: Option<String>,
}

/// The parsed contents of a `unitarrow.quantity.v1` metadata value (§5.2).
#[derive(Clone, Debug, PartialEq)]
pub struct Metadata {
    /// Canonical unit string. `"1"` is explicit dimensionless; absent is a
    /// different claim entirely (§5.3) and cannot be represented here.
    pub unit: String,
    pub grammar: u32,
    pub quantity: Option<String>,
    pub base: Option<Base>,
    pub temporal: Option<Temporal>,
    /// Keys this implementation does not understand, kept verbatim so a
    /// round trip is byte-faithful (§5.2, AMB-024).
    pub unknown: BTreeMap<String, String>,
}

impl Metadata {
    /// The common case: a column carrying a unit and nothing else.
    pub fn new(unit: &str) -> Metadata {
        Metadata {
            unit: unit.to_string(),
            grammar: GRAMMAR_VERSION,
            quantity: None,
            base: None,
            temporal: None,
            unknown: BTreeMap::new(),
        }
    }

    pub fn with_quantity(mut self, curie: &str) -> Metadata {
        self.quantity = Some(curie.to_string());
        self
    }

    /// Serialize for the wire.
    ///
    /// Keys are emitted in a fixed order so the bytes are a function of the
    /// content — the same property §6.3 gives the unit string, one layer up.
    /// A digest over a schema is otherwise sensitive to map iteration order.
    pub fn to_json(&self) -> String {
        let mut s = String::from("{\"unit\":");
        s.push_str(&json_string(&self.unit));
        s.push_str(&format!(",\"grammar\":{}", self.grammar));
        if let Some(q) = &self.quantity {
            s.push_str(",\"quantity\":");
            s.push_str(&json_string(q));
        }
        if let Some(b) = &self.base {
            s.push_str(",\"base\":{");
            let mut first = true;
            if let Some(q) = &b.quantity {
                s.push_str(&format!("\"quantity\":{}", json_string(q)));
                first = false;
            }
            if let Some(v) = b.value {
                if !first {
                    s.push(',');
                }
                s.push_str(&format!("\"value\":{}", fmt_f64(v)));
                first = false;
            }
            if let Some(u) = &b.unit {
                if !first {
                    s.push(',');
                }
                s.push_str(&format!("\"unit\":{}", json_string(u)));
            }
            s.push('}');
        }
        if let Some(t) = &self.temporal {
            s.push_str(",\"temporal\":{");
            let parts: Vec<String> = [("kind", &t.kind), ("statistic", &t.statistic), ("period", &t.period)]
                .iter()
                .filter_map(|(k, v)| v.as_ref().map(|v| format!("{}:{}", json_string(k), json_string(v))))
                .collect();
            s.push_str(&parts.join(","));
            s.push('}');
        }
        for (k, raw) in &self.unknown {
            s.push(',');
            s.push_str(&json_string(k));
            s.push(':');
            s.push_str(raw);
        }
        s.push('}');
        s
    }

    /// Parse and validate a metadata value.
    ///
    /// Unit-valued fields are checked against their **raw** token, before
    /// decoding: `"MW"` decodes to `MW`, so a reader that only sees the
    /// decoded value cannot enforce §5.2's escape prohibition, and the wire
    /// bytes would stop being a function of the unit (AMB-039).
    pub fn from_json(src: &str) -> Result<Metadata> {
        let doc = json_lite::parse(src).map_err(|e| bad(format!("not valid JSON: {e}")))?;
        let obj = doc
            .as_object()
            .ok_or_else(|| bad("extension metadata must be a JSON object"))?;

        let unit_v = obj.get("unit").ok_or_else(|| bad("`unit` is required (§5.2)"))?;
        let raw = unit_v
            .raw_token()
            .ok_or_else(|| bad("the `unit` value must be a JSON string"))?;
        let unit = check_unit_token(raw)?.to_string();

        let grammar = obj
            .get("grammar")
            .ok_or_else(|| bad("`grammar` is required (§5.2)"))?
            .as_i64()
            .filter(|g| *g > 0)
            .ok_or_else(|| bad("`grammar` must be a positive integer"))?;
        if (grammar as u32) > GRAMMAR_VERSION {
            return Err(Error::new(
                ErrorCode::GrammarVersion,
                format!(
                    "the column declares unit-string grammar {grammar}, but this implementation \
                     supports {GRAMMAR_VERSION}"
                ),
            ));
        }

        let quantity = obj.get("quantity").and_then(Json::as_str).map(str::to_string);

        let base = match obj.get("base") {
            None => None,
            Some(b) => {
                let t = b.as_object().ok_or_else(|| bad("`base` must be an object (§5.4)"))?;
                // `base.unit` is unit-valued, so the same escape rule applies.
                let unit = match t.get("unit") {
                    None => None,
                    Some(v) => {
                        let raw = v
                            .raw_token()
                            .ok_or_else(|| bad("`base.unit` must be a JSON string"))?;
                        Some(check_unit_token(raw)?.to_string())
                    }
                };
                Some(Base {
                    quantity: t.get("quantity").and_then(Json::as_str).map(str::to_string),
                    value: t.get("value").and_then(Json::as_f64),
                    unit,
                })
            }
        };

        let temporal = obj.get("temporal").map(|t| {
            let get = |k: &str| t.get(k).and_then(Json::as_str).map(str::to_string);
            Temporal { kind: get("kind"), statistic: get("statistic"), period: get("period") }
        });

        const KNOWN: [&str; 5] = ["unit", "grammar", "quantity", "base", "temporal"];
        let mut unknown = BTreeMap::new();
        for (k, v) in obj {
            if KNOWN.contains(&k.as_str()) {
                continue;
            }
            unknown.insert(k.clone(), raw_of(v));
        }

        Ok(Metadata { unit, grammar: grammar as u32, quantity, base, temporal, unknown })
    }
}

/// Re-serialize a value for unknown-key preservation.
fn raw_of(v: &Json) -> String {
    match v {
        Json::Str { raw, .. } => raw.clone(),
        Json::Number(n) => fmt_f64(*n),
        Json::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Json::Null => "null".to_string(),
        Json::Array(items) => {
            let inner: Vec<String> = items.iter().map(raw_of).collect();
            format!("[{}]", inner.join(","))
        }
        Json::Object(map) => {
            let inner: Vec<String> =
                map.iter().map(|(k, v)| format!("{}:{}", json_string(k), raw_of(v))).collect();
            format!("{{{}}}", inner.join(","))
        }
    }
}

fn fmt_f64(v: f64) -> String {
    if v == (v as i64) as f64 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let mut buf = String::new();
                let _ =
                    core::fmt::Write::write_fmt(&mut buf, format_args!("\\u{:04x}", c as u32));
                out.push_str(&buf);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn the_common_case_is_one_call() {
        let m = Metadata::new("MW");
        assert_eq!(m.to_json(), r#"{"unit":"MW","grammar":1}"#);
        assert_eq!(Metadata::from_json(&m.to_json()).unwrap(), m);
    }

    #[test]
    fn round_trips_every_documented_key() {
        let src = r#"{"unit":"MW","grammar":1,"quantity":"core:power_generation","base":{"quantity":"core:apparent_power","value":100,"unit":"MVA"},"temporal":{"kind":"interval","statistic":"mean","period":"PT1H"}}"#;
        let m = Metadata::from_json(src).unwrap();
        assert_eq!(m.unit, "MW");
        assert_eq!(m.quantity.as_deref(), Some("core:power_generation"));
        assert_eq!(m.base.as_ref().unwrap().unit.as_deref(), Some("MVA"));
        assert_eq!(m.temporal.as_ref().unwrap().period.as_deref(), Some("PT1H"));
        // Byte-identical on the way back out: the emitted order is fixed, so a
        // digest over the schema is stable.
        assert_eq!(m.to_json(), src);
    }

    #[test]
    fn unknown_keys_survive_because_a_seal_depends_on_them() {
        // AMB-024: the companion hashes the schema, so dropping a key nobody
        // understood changes a digest and breaks a seal.
        let src = r#"{"unit":"MW","grammar":1,"vendorX:calib":"2026-03-01"}"#;
        let m = Metadata::from_json(src).unwrap();
        assert_eq!(m.unknown.get("vendorX:calib").map(String::as_str), Some(r#""2026-03-01""#));
        assert_eq!(m.to_json(), src);
    }

    #[test]
    fn escapes_are_rejected_in_every_unit_valued_field() {
        // Both decode to a unit a reader would accept, which is why the raw
        // token is what gets checked (§5.2, AMB-039).
        for src in [
            r#"{"unit":"\u004dW","grammar":1}"#,
            r#"{"unit":"MW\/h","grammar":1}"#,
            r#"{"unit":"MW","grammar":1,"base":{"unit":"\u004dVA"}}"#,
        ] {
            let e = Metadata::from_json(src).unwrap_err();
            assert_eq!(e.code(), ErrorCode::BadMetadata, "for {src}");
        }
    }

    #[test]
    fn the_required_keys_are_required() {
        for (src, want) in [
            (r#"{"grammar":1}"#, "`unit` is required"),
            (r#"{"unit":"MW"}"#, "`grammar` is required"),
            (r#"[]"#, "must be a JSON object"),
            (r#"{"unit":7,"grammar":1}"#, "must be a JSON string"),
        ] {
            let e = Metadata::from_json(src).unwrap_err();
            assert_eq!(e.code(), ErrorCode::BadMetadata, "for {src}");
            assert!(e.message().contains(want), "for {src}: {}", e.message());
        }
    }

    #[test]
    fn a_newer_grammar_is_reported_as_such_not_as_bad_metadata() {
        // §10 distinguishes these: the blob is well-formed, the reader is old.
        let e = Metadata::from_json(r#"{"unit":"MW","grammar":99}"#).unwrap_err();
        assert_eq!(e.code(), ErrorCode::GrammarVersion);
    }

    #[test]
    fn explicit_dimensionless_is_not_the_same_as_absent() {
        // §5.3: `"1"` is a claim; no tag at all is the absence of one, and this
        // type cannot express the latter — which is the point.
        let m = Metadata::from_json(r#"{"unit":"1","grammar":1}"#).unwrap();
        assert_eq!(m.unit, "1");
    }

    #[test]
    fn nested_unknown_values_survive_intact() {
        let src = r#"{"unit":"MW","grammar":1,"x":{"a":[1,2],"b":true}}"#;
        assert_eq!(Metadata::from_json(src).unwrap().to_json(), src);
        let _ = vec![0u8];
    }
}
