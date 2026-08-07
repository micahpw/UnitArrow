//! Composing several registries into one **effective** registry (AMB-058, AMB-060).
//!
//! # Why this exists
//!
//! A deployment that needs units from two domains has, without this, no move.
//! `unitarrow-core` resolves against exactly one registry (AMB-036), unit
//! symbols are not namespaced until grammar v2 (AMB-037), and
//! `prefix_collisions` resolves *authored vs prefixed* rather than *authored vs
//! authored*. Concatenating two registry files produces a TOML duplicate-key
//! error that never names the contested symbol.
//!
//! # The ordering constraint
//!
//! Composition happens **before** computation, never as a description of it. If
//! a pipeline computed first and recorded the registries afterwards, an output
//! table could hold two columns both tagged `bbl` meaning different things — and
//! then §6.3 canonical form would no longer be an equality primitive *within a
//! single table*, which is the one place it must hold unconditionally. So this
//! produces one artifact, and everything downstream resolves against that.
//!
//! # What makes it checkable
//!
//! The output carries its own reconciliation, and the pin is the hash of the
//! output bytes. A consumer holding the constituent registries can recompose and
//! compare pins — so the reconciliation is not a claim but a computation, and a
//! seal over the pin covers the reasons transitively. Nobody can change *why*
//! `bbl` means the oil barrel without changing the pin.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::{Error, ErrorCode, Result};
use crate::registry::Registry;
use crate::toml_lite::{self, Value};

/// One constituent registry, as it arrived.
#[derive(Clone, Debug)]
pub struct Source {
    pub name: String,
    pub version: String,
    pub toml: String,
}

impl Source {
    pub fn new(name: &str, version: &str, toml: &str) -> Source {
        Source {
            name: name.into(),
            version: version.into(),
            toml: toml.into(),
        }
    }

    /// The pin §5.6 records for this constituent.
    pub fn pin(&self) -> String {
        crate::sha256::digest_pin(self.toml.as_bytes())
    }
}

/// The composer's ruling on one contested symbol.
///
/// `reason` is required for the same reason it is on `prefix_collisions`
/// (AMB-054): the decision is invisible in a diff of the unit it affects, and a
/// consumer asking *why does `bbl` mean that here?* has nowhere else to look.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolution {
    pub symbol: String,
    pub from: String,
    pub reason: String,
}

impl Resolution {
    pub fn new(symbol: &str, from: &str, reason: &str) -> Resolution {
        Resolution {
            symbol: symbol.into(),
            from: from.into(),
            reason: reason.into(),
        }
    }
}

/// A symbol two sources define differently, before anyone has ruled on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contested {
    pub symbol: String,
    /// Every source defining it, in name order.
    pub sources: Vec<String>,
}

/// The result of composing.
#[derive(Clone, Debug)]
pub struct Composed {
    /// The effective registry's name.
    pub name: String,
    /// The effective registry's version.
    pub version: String,
    /// The effective registry's source. This is what gets embedded or pinned.
    pub toml: String,
    /// `sha256:…` over `toml`'s bytes.
    pub pin: String,
    /// Constituents, in name order: (name, version, pin).
    pub inputs: Vec<(String, String, String)>,
    /// Rulings actually applied — a resolution for a symbol only one source
    /// defines is an error, so every entry here settled a real contest.
    pub applied: Vec<Resolution>,
    /// Symbols several sources define **identically**. Not contests; recorded
    /// because "both registries agree about the metre" is worth seeing.
    pub agreed: Vec<String>,
}

/// How much of the effective registry travels in the table's metadata (§5.6).
///
/// The two modes trade **availability, not integrity** — which is the whole
/// point of pinning by content hash rather than by URL. Both let a consumer
/// prove they are resolving against the registry the publisher actually used;
/// only [`Embedding::Full`] lets them do it when that registry is no longer
/// anywhere to be found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Embedding {
    /// The effective registry travels with the table.
    ///
    /// For a published dataset with a decade of shelf life this is the right
    /// default: a few KB against a 100 MB file is nothing, and it is the only
    /// form that survives the constituent registries going offline — the
    /// dead-link failure, which for archival data is the *likely* one.
    Full,
    /// Only the pin travels; the registry is fetched and verified against it.
    ///
    /// For a pipeline emitting millions of small files that aggregate into TB,
    /// per-file duplication is the dominant cost and this is the right choice.
    /// A bare URL would give no guarantee that what comes back is what was used;
    /// a content hash does — so this mode still cannot silently resolve against
    /// the wrong registry, it can only fail to find the right one.
    Pin,
}

impl Composed {
    /// Load the effective registry.
    pub fn registry(&self) -> Result<Registry> {
        Registry::from_toml(&self.toml)
    }

    /// The `unitarrow:provenance` registry block (§5.6) at the given embedding.
    pub fn provenance_json(&self, mode: Embedding) -> String {
        let mut s = String::new();
        s.push_str("{\"registry\":{\"name\":");
        s.push_str(&json_str(&self.name));
        s.push_str(",\"version\":");
        s.push_str(&json_str(&self.version));
        s.push_str(",\"hash\":");
        s.push_str(&json_str(&self.pin));
        if mode == Embedding::Full {
            s.push_str(",\"source\":");
            s.push_str(&json_str(&self.toml));
        }
        s.push_str("},\"composed_from\":[");
        for (i, (n, v, h)) in self.inputs.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str("{\"name\":");
            s.push_str(&json_str(n));
            s.push_str(",\"version\":");
            s.push_str(&json_str(v));
            s.push_str(",\"hash\":");
            s.push_str(&json_str(h));
            s.push('}');
        }
        s.push_str("]}");
        s
    }

    /// Bytes this composition adds to every table, at the given embedding.
    pub fn metadata_bytes(&self, mode: Embedding) -> usize {
        self.provenance_json(mode).len()
    }
}

/// Does `fetched` match the registry a table was published against?
///
/// This is what the [`Embedding::Pin`] mode buys over a bare URL: a substituted
/// or drifted registry is **detected**, not silently used. It is also the check
/// that partially closes AMB-047 on the read path.
pub fn verify_pin(fetched: &str, pin: &str) -> bool {
    crate::sha256::digest_pin(fetched.as_bytes()) == pin
}

fn json_str(s: &str) -> String {
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
                let _ = core::fmt::Write::write_fmt(&mut buf, format_args!("\\u{:04x}", c as u32));
                out.push_str(&buf);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn invalid(msg: impl Into<String>) -> Error {
    Error::new(ErrorCode::RegistryInvalid, msg)
}

/// Merge `sources` into one effective registry.
///
/// Every symbol two sources define **differently** must have a [`Resolution`],
/// or composition fails naming the symbol and the sources — the same
/// resolve-it-where-the-context-is rule as `prefix_collisions`, one level up.
pub fn compose(
    name: &str,
    version: &str,
    sources: &[Source],
    resolutions: &[Resolution],
) -> Result<Composed> {
    if sources.is_empty() {
        return Err(invalid("composing needs at least one source registry"));
    }

    let mut by_name: BTreeMap<&str, &Source> = BTreeMap::new();
    let mut parsed: Vec<(&Source, BTreeMap<String, Value>)> = Vec::new();
    for s in sources {
        if by_name.insert(s.name.as_str(), s).is_some() {
            return Err(invalid(format!(
                "two sources both named {:?}; constituents are identified by name in the \
                 reconciliation, so the names must differ",
                s.name
            )));
        }
        let doc = toml_lite::parse(&s.toml)
            .map_err(|e| invalid(format!("source {:?} is not valid TOML: {e}", s.name)))?;
        parsed.push((s, doc));
    }

    let mut rulings: BTreeMap<&str, &Resolution> = BTreeMap::new();
    for r in resolutions {
        if r.reason.trim().is_empty() {
            return Err(invalid(format!(
                "the ruling on {:?} has an empty reason; a consumer of the published table has \
                 nowhere else to learn why this reading won",
                r.symbol
            )));
        }
        if !by_name.contains_key(r.from.as_str()) {
            return Err(invalid(format!(
                "the ruling on {:?} names source {:?}, which is not being composed",
                r.symbol, r.from
            )));
        }
        if rulings.insert(r.symbol.as_str(), r).is_some() {
            return Err(invalid(format!("two rulings on {:?}", r.symbol)));
        }
    }

    // ---- merge the three definitional tables ----
    let mut out: BTreeMap<String, Value> = BTreeMap::new();
    let mut applied: Vec<Resolution> = Vec::new();
    let mut agreed: Vec<String> = Vec::new();
    let mut settled: BTreeSet<String> = BTreeSet::new();

    for section in ["dimension", "unit", "quantity"] {
        // symbol -> (source name, body), in first-wins-after-ruling order.
        let mut chosen: BTreeMap<String, (&str, Value)> = BTreeMap::new();
        for (src, doc) in &parsed {
            let Some(table) = doc.get(section).and_then(Value::as_table) else {
                continue;
            };
            for (key, body) in table {
                match chosen.get(key) {
                    None => {
                        chosen.insert(key.clone(), (src.name.as_str(), body.clone()));
                    }
                    // Two sources agreeing on a definition is not a contest.
                    // `m` is `m` everywhere, and demanding a ruling for it would
                    // make composition unusable.
                    Some((_, existing)) if existing == body => {
                        if section == "unit" && !agreed.contains(key) {
                            agreed.push(key.clone());
                        }
                    }
                    Some(_) => {
                        let Some(ruling) = rulings.get(key.as_str()) else {
                            let mut who: Vec<String> = Vec::new();
                            for (s, d) in &parsed {
                                if d.get(section)
                                    .and_then(Value::as_table)
                                    .is_some_and(|t| t.contains_key(key))
                                {
                                    who.push(s.name.clone());
                                }
                            }
                            return Err(invalid(format!(
                                "{section} {key:?} is defined differently by {}. Rule on it here \
                                 rather than leaving it to consumers — name the source that wins \
                                 and say why, e.g. {key} = {{ from = {:?}, reason = \"…\" }}. \
                                 Composing without a ruling would publish a table whose units \
                                 cannot be resolved",
                                who.join(" and "),
                                who.first().cloned().unwrap_or_default(),
                            )));
                        };
                        // Checked here, where the section is known: a ruling
                        // naming a source that does not define the symbol would
                        // otherwise silently keep some other source's version.
                        let winner_defines = parsed.iter().any(|(s, d)| {
                            s.name == ruling.from
                                && d.get(section)
                                    .and_then(Value::as_table)
                                    .is_some_and(|t| t.contains_key(key))
                        });
                        if !winner_defines {
                            return Err(invalid(format!(
                                "the ruling on {key:?} names {:?}, which does not define it",
                                ruling.from
                            )));
                        }
                        settled.insert(key.clone());
                        if !applied.iter().any(|a| a.symbol == *key) {
                            applied.push((*ruling).clone());
                        }
                        if ruling.from == src.name {
                            chosen.insert(key.clone(), (src.name.as_str(), body.clone()));
                        }
                    }
                }
            }
        }

        if !chosen.is_empty() {
            let mut table = BTreeMap::new();
            for (key, (_, body)) in chosen {
                table.insert(key, body);
            }
            out.insert(section.to_string(), Value::Table(table));
        }
    }

    for r in resolutions {
        if !settled.contains(&r.symbol) {
            return Err(invalid(format!(
                "the ruling on {:?} settles nothing — no two sources define it differently. A \
                 stale ruling hides the next real contest",
                r.symbol
            )));
        }
    }

    // ---- header: identity, constituents, and the rulings ----
    let mut header: BTreeMap<String, Value> = BTreeMap::new();
    header.insert("schema_version".into(), Value::Integer(1));
    header.insert("name".into(), Value::Str(name.to_string()));
    header.insert("version".into(), Value::Str(version.to_string()));

    // `prefix_collisions` from the constituents carry forward: they are rulings
    // their authors made, and dropping them would fail the load.
    let mut collisions: BTreeMap<String, Value> = BTreeMap::new();
    for (src, doc) in &parsed {
        if let Some(t) = doc
            .get("registry")
            .and_then(Value::as_table)
            .and_then(|h| h.get("prefix_collisions"))
            .and_then(Value::as_table)
        {
            for (sym, reason) in t {
                collisions.entry(sym.clone()).or_insert_with(|| {
                    Value::Str(format!(
                        "{} (carried forward from {})",
                        reason.as_str().unwrap_or(""),
                        src.name
                    ))
                });
            }
        }
    }
    if !collisions.is_empty() {
        header.insert("prefix_collisions".into(), Value::Table(collisions));
    }

    let mut inputs: Vec<(String, String, String)> = Vec::new();
    let mut from_table: BTreeMap<String, Value> = BTreeMap::new();
    for s in by_name.values() {
        let pin = s.pin();
        inputs.push((s.name.clone(), s.version.clone(), pin.clone()));
        let mut e = BTreeMap::new();
        e.insert("version".to_string(), Value::Str(s.version.clone()));
        e.insert("hash".to_string(), Value::Str(pin));
        from_table.insert(s.name.clone(), Value::Table(e));
    }
    header.insert("composed_from".into(), Value::Table(from_table));

    if !applied.is_empty() {
        let mut rec: BTreeMap<String, Value> = BTreeMap::new();
        for r in &applied {
            let mut e = BTreeMap::new();
            e.insert("from".to_string(), Value::Str(r.from.clone()));
            e.insert("reason".to_string(), Value::Str(r.reason.clone()));
            rec.insert(r.symbol.clone(), Value::Table(e));
        }
        header.insert("reconciliation".into(), Value::Table(rec));
    }

    out.insert("registry".into(), Value::Table(header));

    let toml = toml_lite::write(&out);
    let pin = crate::sha256::digest_pin(toml.as_bytes());

    // Composing must not be able to produce an unloadable registry: the whole
    // point is that downstream has one artifact it can resolve against.
    Registry::from_toml(&toml).map_err(|e| {
        // A composition can fail in a way *neither source* could: two sources
        // that are individually valid may put an authored symbol and a
        // prefixable base in the same registry for the first time. The
        // underlying message then advises `prefix_collisions`, which lives in a
        // source header the composer does not author — so name the move that is
        // actually available to them.
        let emergent = e.message().contains("is ambiguous");
        invalid(format!(
            "the composed registry does not load: {}{}",
            e.message(),
            if emergent {
                ". Note this collision exists only in the composition — each source                  is valid alone. As the composer you resolve it by contributing your                  own source declaring [registry.prefix_collisions]; a source that                  defines no units is legal and exists for exactly this"
            } else {
                ""
            }
        ))
    })?;

    applied.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    agreed.sort();
    Ok(Composed {
        name: name.to_string(),
        version: version.to_string(),
        toml,
        pin,
        inputs,
        applied,
        agreed,
    })
}

/// Every symbol the sources define differently — what to rule on, before ruling.
///
/// The proactive form, matching `Registry::would_collide` one level up: a
/// composer should be able to ask "what will I have to decide?" without first
/// triggering a failure.
pub fn contested(sources: &[Source]) -> Result<Vec<Contested>> {
    let mut parsed: Vec<(&Source, BTreeMap<String, Value>)> = Vec::new();
    for s in sources {
        let doc = toml_lite::parse(&s.toml)
            .map_err(|e| invalid(format!("source {:?} is not valid TOML: {e}", s.name)))?;
        parsed.push((s, doc));
    }
    let mut out: BTreeMap<String, Contested> = BTreeMap::new();
    for section in ["dimension", "unit", "quantity"] {
        let mut seen: BTreeMap<String, (&str, Value)> = BTreeMap::new();
        for (src, doc) in &parsed {
            let Some(table) = doc.get(section).and_then(Value::as_table) else {
                continue;
            };
            for (key, body) in table {
                match seen.get(key) {
                    Some((_, existing)) if existing != body => {
                        let e = out.entry(key.clone()).or_insert_with(|| Contested {
                            symbol: key.clone(),
                            sources: Vec::new(),
                        });
                        for (s2, d2) in &parsed {
                            if d2
                                .get(section)
                                .and_then(Value::as_table)
                                .is_some_and(|t| t.contains_key(key))
                                && !e.sources.contains(&s2.name)
                            {
                                e.sources.push(s2.name.clone());
                            }
                        }
                    }
                    Some(_) => {}
                    None => {
                        seen.insert(key.clone(), (src.name.as_str(), body.clone()));
                    }
                }
            }
        }
    }
    Ok(out.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn oil() -> Source {
        Source::new(
            "oil",
            "1.4",
            "[registry]\nschema_version = 1\nname = \"oil\"\nversion = \"1.4\"\n\
             [unit.m]\ndimension = \"length\"\nfactor = [1, 1]\ndisplay = { long = \"metre\" }\n\
             [unit.bbl]\ndimension = \"length\"\nfactor = [9938205933, 62500000000]\n\
             display = { long = \"oil barrel\" }\n",
        )
    }

    fn water() -> Source {
        Source::new(
            "water",
            "2.1",
            "[registry]\nschema_version = 1\nname = \"water\"\nversion = \"2.1\"\n\
             [unit.m]\ndimension = \"length\"\nfactor = [1, 1]\ndisplay = { long = \"metre\" }\n\
             [unit.bbl]\ndimension = \"length\"\nfactor = [29810117799, 250000000000]\n\
             display = { long = \"water barrel\" }\n\
             [unit.acre_ft]\ndimension = \"length\"\nfactor = [616740919, 500000]\n",
        )
    }

    #[test]
    fn a_contest_must_be_ruled_on_and_the_error_names_both_sources() {
        let e = compose("merged", "1", &[oil(), water()], &[]).unwrap_err();
        let m = e.message();
        assert!(m.contains("\"bbl\""), "{m}");
        assert!(
            m.contains("oil") && m.contains("water"),
            "both sources named: {m}"
        );
        assert!(m.contains("say why"), "the reason is asked for: {m}");
    }

    #[test]
    fn agreement_is_not_a_contest() {
        // Both define `m` identically. Demanding a ruling for every shared
        // symbol would make composition unusable in practice.
        let found = contested(&[oil(), water()]).unwrap();
        assert_eq!(found.len(), 1, "only bbl is contested: {found:?}");
        assert_eq!(found[0].symbol, "bbl");
        assert_eq!(
            found[0].sources,
            vec!["oil".to_string(), "water".to_string()]
        );
    }

    #[test]
    fn composing_produces_one_resolvable_registry() {
        let c = compose(
            "merged",
            "1",
            &[oil(), water()],
            &[Resolution::new(
                "bbl",
                "oil",
                "this deployment is upstream oil, not water",
            )],
        )
        .unwrap();

        let r = c.registry().unwrap();
        // The ruling took effect...
        assert_eq!(r.describe("bbl").as_deref(), Some("oil barrel"));
        // ...and the non-contested union survived from both sides.
        assert!(r.resolve("acre_ft").is_some(), "water's unit carried over");
        assert!(r.resolve("m").is_some(), "the agreed unit carried over");
        assert_eq!(c.agreed, vec!["m".to_string()]);
        assert_eq!(c.applied.len(), 1);
        assert_eq!(c.inputs.len(), 2);
    }

    #[test]
    fn the_pin_is_reproducible_and_covers_the_reason() {
        let ruling = |why: &str| {
            compose(
                "merged",
                "1",
                &[oil(), water()],
                &[Resolution::new("bbl", "oil", why)],
            )
            .unwrap()
        };
        let a = ruling("this deployment is upstream oil");
        let b = ruling("this deployment is upstream oil");
        assert_eq!(a.pin, b.pin, "same inputs and ruling -> same pin");

        // The reason lives *inside* the hashed artifact, so a seal over the pin
        // covers it. Editing the justification without anyone noticing is not
        // possible.
        let c = ruling("actually we just guessed");
        assert_ne!(a.pin, c.pin, "changing the reason changes the pin");

        // And the constituents are individually pinned for audit.
        assert!(a.inputs.iter().all(|(_, _, p)| p.starts_with("sha256:")));
    }

    #[test]
    fn the_effective_registry_carries_its_own_provenance() {
        let c = compose(
            "merged",
            "1",
            &[oil(), water()],
            &[Resolution::new("bbl", "oil", "upstream oil")],
        )
        .unwrap();
        // A consumer reading only the embedded artifact can answer all three
        // questions from AMB-060 without fetching anything.
        assert!(
            c.toml.contains("[registry.composed_from.oil]"),
            "{}",
            c.toml
        );
        assert!(
            c.toml.contains("[registry.reconciliation.bbl]"),
            "{}",
            c.toml
        );
        assert!(c.toml.contains("upstream oil"), "{}", c.toml);
    }

    #[test]
    fn stale_and_misdirected_rulings_are_rejected() {
        let stale = compose(
            "merged",
            "1",
            &[oil(), water()],
            &[
                Resolution::new("bbl", "oil", "upstream oil"),
                Resolution::new("m", "oil", "no contest here"),
            ],
        )
        .unwrap_err();
        assert!(
            stale.message().contains("settles nothing"),
            "{}",
            stale.message()
        );

        let unknown = compose(
            "merged",
            "1",
            &[oil(), water()],
            &[Resolution::new("bbl", "gas", "not a source")],
        )
        .unwrap_err();
        assert!(
            unknown.message().contains("not being composed"),
            "{}",
            unknown.message()
        );

        let blank = compose(
            "merged",
            "1",
            &[oil(), water()],
            &[Resolution::new("bbl", "oil", "  ")],
        )
        .unwrap_err();
        assert!(
            blank.message().contains("empty reason"),
            "{}",
            blank.message()
        );
    }

    #[test]
    fn both_embeddings_verify_but_only_one_survives_a_dead_link() {
        let c = compose(
            "merged",
            "1",
            &[oil(), water()],
            &[Resolution::new("bbl", "oil", "upstream oil")],
        )
        .unwrap();

        // Integrity is identical: both modes carry the pin, so a substituted
        // registry is caught either way. This is what a bare URL cannot do.
        for mode in [Embedding::Full, Embedding::Pin] {
            assert!(
                c.provenance_json(mode).contains(&c.pin),
                "{mode:?} carries the pin"
            );
        }
        assert!(verify_pin(&c.toml, &c.pin));
        assert!(
            !verify_pin(&format!("{}\n", c.toml), &c.pin),
            "drift is detected"
        );

        // Availability differs, and that is the whole trade. Only Full carries
        // the bytes, so only Full still resolves when the constituents are gone.
        let full = c.provenance_json(Embedding::Full);
        let pin_only = c.provenance_json(Embedding::Pin);
        assert!(
            full.contains("\\\"oil barrel\\\""),
            "the definitions travel"
        );
        assert!(
            !pin_only.contains("oil barrel"),
            "the pin mode carries none of them"
        );
        assert!(
            c.metadata_bytes(Embedding::Full) > c.metadata_bytes(Embedding::Pin),
            "and that is what it costs"
        );

        // Full mode is self-contained: the embedded source reloads on its own.
        let embedded = crate::json_lite::parse(&full).unwrap();
        let src = embedded
            .get("registry")
            .unwrap()
            .get("source")
            .unwrap()
            .as_str()
            .unwrap();
        assert_eq!(
            Registry::from_toml(src).unwrap().describe("bbl").as_deref(),
            Some("oil barrel")
        );
        assert!(
            verify_pin(src, &c.pin),
            "and it verifies against its own pin"
        );
    }

    #[test]
    fn a_single_source_composes_to_itself_semantically() {
        // The degenerate case must work: composition is not a separate mode.
        let c = compose("merged", "1", &[oil()], &[]).unwrap();
        let r = c.registry().unwrap();
        assert_eq!(r.describe("bbl").as_deref(), Some("oil barrel"));
        assert!(c.applied.is_empty());
        assert_eq!(c.inputs.len(), 1);
    }
}
