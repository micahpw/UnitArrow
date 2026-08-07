//! Canonical form, spec §6.3 (as ruled 2026-07-29).
//!
//! 1. Resolve every symbol against the registry (aliases → canonical symbol).
//! 2. Merge repeated symbols by summing exponents; drop zero exponents.
//! 3. Serialize as one `*`-joined product: positive-exponent terms first, then
//!    negative-exponent terms, each group sorted ascending by **bytewise** UTF-8
//!    comparison of the symbol. A term renders `symbol` when its exponent is 1
//!    and `symbol^exponent` otherwise, sign inside the exponent.
//!    **Canonical form contains no `/`.**
//! 4. The empty product serializes as `"1"`.
//!
//! Canonicalization normalizes *spelling*, never the producer's choice of unit:
//! `MW*h` and `MWh` are both canonical and are not equal.



use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::dimension::Dimension;
use crate::error::{Error, ErrorCode, Result};
use crate::parse::{self, ParsedUnit};
use crate::rational::Scale;
use crate::registry::Registry;

/// A fully analysed unit expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalUnit {
    /// The §6.3 canonical string — the equality and hashing primitive.
    pub canonical: String,
    pub dimension: Dimension,
    /// Merged `(canonical symbol, exponent)` pairs in canonical order.
    pub terms: Vec<(String, i32)>,
    /// Set when the expression is exactly one affine unit (`degC`), which is the
    /// only shape in which an affine unit may appear.
    pub affine_symbol: Option<String>,
}

impl CanonicalUnit {
    pub fn is_dimensionless(&self) -> bool {
        self.dimension.is_dimensionless()
    }

    /// Multiplicative scale to the dimension's base units. `None` for an affine
    /// unit, where a single factor is not the whole story (§6.4).
    pub fn scale(&self, registry: &Registry) -> Option<Scale> {
        if self.affine_symbol.is_some() {
            return None;
        }
        let mut acc = Scale::ONE;
        for (sym, exp) in &self.terms {
            let u = registry.resolve(sym)?;
            acc = acc.checked_mul(u.factor.checked_pow(*exp)?)?;
        }
        Some(acc)
    }
}

/// Parse, resolve, and canonicalize a unit string.
///
/// `grammar` is the wire-declared grammar version (§5.2); anything newer than 1
/// is `E_GRAMMAR_VERSION` — the string may be perfectly valid under a future
/// grammar, and guessing is how a v2 rational exponent gets silently misread.
pub fn canonicalize(input: &str, registry: &Registry, grammar: u32) -> Result<CanonicalUnit> {
    if grammar > 1 {
        return Err(Error::new(
            ErrorCode::GrammarVersion,
            format!("wire grammar {grammar} is newer than this implementation supports (1)"),
        ));
    }
    let parsed = parse::parse(input).map_err(|e| enrich_syntax_error(e, input, registry))?;
    from_parsed(&parsed, registry)
}

/// Turn "symbols are ASCII" into "`°C` is the display form of `degC`; write
/// `degC`".
///
/// The parser is deliberately registry-independent, so it can only say *that* a
/// character is illegal. Here, where the registry is in hand, we can say what to
/// write instead — driven by the registry's own display forms, so it stays
/// correct for whatever units are actually loaded.
fn enrich_syntax_error(e: Error, input: &str, registry: &Registry) -> Error {
    if e.code() != ErrorCode::UnitSyntax {
        return e;
    }
    let Some((form, unit)) = registry.display_form_in(input) else {
        return e;
    };
    let name = unit
        .display
        .long
        .as_deref()
        .map(|l| format!(" ({l})"))
        .unwrap_or_default();
    // Replace rather than append: once the registry can name the exact symbol,
    // the parser's generic "symbols are ASCII" is noise in front of it.
    let message = format!(
        "{form:?} is a display form, not an input symbol — write `{}`{name}",
        unit.symbol
    );
    let enriched = match e.offset() {
        Some(off) => Error::at(ErrorCode::UnitSyntax, message, off),
        None => Error::new(ErrorCode::UnitSyntax, message),
    };
    enriched.with_symbol(&unit.symbol)
}

fn from_parsed(parsed: &ParsedUnit, registry: &Registry) -> Result<CanonicalUnit> {
    // Step 1 — resolve aliases to canonical symbols.
    let mut resolved: Vec<(String, i32)> = Vec::with_capacity(parsed.terms.len());
    for t in &parsed.terms {
        let unit = registry.resolve(&t.symbol).ok_or_else(|| {
            // A spelling the registry deliberately refuses is not a typo, and a
            // "did you mean" reads as though it were. Say why it was refused.
            if let Some(a) = registry.ambiguity(&t.symbol) {
                return Error::at(ErrorCode::UnknownUnit, a.describe(&t.symbol), t.offset)
                    .with_symbol(&t.symbol);
            }
            let hints = registry.suggest(&t.symbol);
            let tail = if hints.is_empty() {
                String::new()
            } else {
                format!(" — did you mean {}?", 
                    hints.iter().map(|h| format!("`{h}`")).collect::<Vec<_>>().join(", "))
            };
            Error::at(
                ErrorCode::UnknownUnit,
                format!(
                    "symbol {:?} does not resolve against registry {:?}{tail}",
                    t.symbol, registry.name
                ),
                t.offset,
            )
            .with_symbol(&t.symbol)
        })?;
        resolved.push((unit.symbol.clone(), t.exponent));
    }

    // §6.4 affine check — against the *parsed* term list, before the step 2
    // merge. Checking after merging would let `degC/degC` cancel to
    // dimensionless and `degC*h/h` reduce to a bare `degC`, both of which
    // reach a temperature through a product.
    let mut affine_symbol = None;
    let non_composable: Vec<&str> = resolved
        .iter()
        .filter(|(s, _)| registry.resolve(s).is_some_and(|u| u.is_non_composable()))
        .map(|(s, _)| s.as_str())
        .collect();
    if !non_composable.is_empty() {
        let single = resolved.len() == 1 && resolved[0].1 == 1;
        if !single {
            return Err(Error::new(
                ErrorCode::AffineCompound,
                format!(
                    "{:?} is non-composable: it may appear only as a single term with exponent 1",
                    non_composable[0]
                ),
            )
            .with_symbol(non_composable[0]));
        }
        let u = registry.resolve(&resolved[0].0).unwrap();
        if u.is_affine() {
            affine_symbol = Some(u.symbol.clone());
        }
    }

    // Step 2 — merge repeated symbols, drop zero exponents.
    let mut merged: BTreeMap<String, i32> = BTreeMap::new();
    for (sym, exp) in resolved {
        let slot = merged.entry(sym).or_insert(0);
        *slot = slot.checked_add(exp).ok_or_else(|| {
            Error::new(ErrorCode::ExpRange, "exponent overflow while merging repeated symbols")
        })?;
    }
    merged.retain(|_, e| *e != 0);
    for (sym, exp) in &merged {
        if *exp < i8::MIN as i32 || *exp > i8::MAX as i32 {
            return Err(Error::new(
                ErrorCode::ExpRange,
                format!("merged exponent {exp} for {sym:?} is outside the signed 8-bit range"),
            )
            .with_symbol(sym));
        }
    }

    // Dimension vector.
    let mut dimension = Dimension::DIMENSIONLESS;
    for (sym, exp) in &merged {
        let u = registry.resolve(sym).expect("resolved above");
        let contribution = u.dimension.checked_pow(*exp).ok_or_else(|| {
            Error::new(ErrorCode::ExpRange, format!("dimension overflow raising {sym:?} to {exp}"))
        })?;
        dimension = dimension.checked_mul(&contribution).ok_or_else(|| {
            Error::new(ErrorCode::ExpRange, "dimension exponent overflow while combining terms")
        })?;
    }

    // Steps 3 and 4 — serialize. Bytewise ordering within each group.
    let mut positives: Vec<(&String, &i32)> = merged.iter().filter(|(_, e)| **e > 0).collect();
    let mut negatives: Vec<(&String, &i32)> = merged.iter().filter(|(_, e)| **e < 0).collect();
    positives.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    negatives.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));

    let render = |(sym, exp): &(&String, &i32)| -> String {
        if **exp == 1 {
            (*sym).clone()
        } else {
            format!("{sym}^{exp}")
        }
    };
    let parts: Vec<String> = positives.iter().chain(negatives.iter()).map(render).collect();
    let canonical = if parts.is_empty() { "1".to_string() } else { parts.join("*") };

    let terms: Vec<(String, i32)> = positives
        .iter()
        .chain(negatives.iter())
        .map(|(s, e)| ((*s).clone(), **e))
        .collect();

    Ok(CanonicalUnit { canonical, dimension, terms, affine_symbol })
}

/// Two units are equal iff their canonical strings are identical (§6.3). This is
/// deliberately *not* dimensional equality: `MW*h` and `MWh` are commensurable
/// and numerically identical, yet not equal.
pub fn units_equal(a: &CanonicalUnit, b: &CanonicalUnit) -> bool {
    a.canonical == b.canonical
}

/// Commensurability — equality of dimension vectors (§6.2).
pub fn commensurable(a: &CanonicalUnit, b: &CanonicalUnit) -> bool {
    a.dimension == b.dimension
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg() -> Registry {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../conformance/registry/conformance-core.toml"
        ))
        .unwrap();
        Registry::from_toml(&src).unwrap()
    }

    fn canon(s: &str) -> String {
        canonicalize(s, &reg(), 1).unwrap().canonical
    }

    #[test]
    fn contains_no_slashes() {
        assert_eq!(canon("W/K/m^2"), "W*K^-1*m^-2");
        assert_eq!(canon("MW/h"), "MW*h^-1");
        assert_eq!(canon("1/s"), "s^-1");
        assert_eq!(canon("USD/MWh"), "USD*MWh^-1");
    }

    #[test]
    fn input_sugar_converges_on_one_spelling() {
        // Slash form, exponent form, and reordered input all agree.
        for spelling in ["1/K/m^2", "K^-1*m^-2", "m^-2/K", "m^-2*K^-1"] {
            assert_eq!(canon(spelling), "K^-1*m^-2", "for {spelling}");
        }
    }

    #[test]
    fn is_idempotent() {
        let r = reg();
        for s in ["h*MW", "W/K/m^2", "1/s", "m*m", "MW/MW"] {
            let once = canonicalize(s, &r, 1).unwrap().canonical;
            let twice = canonicalize(&once, &r, 1).unwrap().canonical;
            assert_eq!(once, twice, "canon(canon({s})) != canon({s})");
        }
    }

    #[test]
    fn positives_precede_negatives_each_group_bytewise() {
        assert_eq!(canon("s^-1*MW"), "MW*s^-1");
        // Uppercase sorts before lowercase under bytewise comparison.
        assert_eq!(canon("m*kW*h*MW*W*Btu"), "Btu*MW*W*h*kW*m");
    }

    #[test]
    fn merges_and_drops_zero_exponents() {
        assert_eq!(canon("m*m"), "m^2");
        assert_eq!(canon("MW*MW*MW"), "MW^3");
        assert_eq!(canon("m^2*m^-2"), "1");
        assert_eq!(canon("s/s"), "1");
        assert_eq!(canon("m^3/m^2"), "m");
        assert_eq!(canon("m^0"), "1");
    }

    #[test]
    fn division_associativity_matches_the_grammar() {
        // a/b/c is a·b⁻¹·c⁻¹; the negative group sorts K before h.
        assert_eq!(canon("MW/h/K"), "MW*K^-1*h^-1");
        // One operator different, and m^2 stays positive.
        assert_eq!(canon("W/K*m^2"), "W*m^2*K^-1");
    }

    #[test]
    fn spelling_is_normalised_but_unit_choice_is_not() {
        assert_eq!(canon("MWh"), "MWh");
        assert_eq!(canon("MW*h"), "MW*h");
        let r = reg();
        let a = canonicalize("MW*h", &r, 1).unwrap();
        let b = canonicalize("MWh", &r, 1).unwrap();
        assert!(commensurable(&a, &b), "same dimension");
        assert!(!units_equal(&a, &b), "but not equal — §1 goal 4");
    }

    #[test]
    fn a_dimensionless_expression_need_not_canonicalise_to_one() {
        let r = reg();
        let u = canonicalize("MW*h/MWh", &r, 1).unwrap();
        assert!(u.is_dimensionless());
        assert_eq!(u.canonical, "MW*h*MWh^-1");
    }

    #[test]
    fn affine_units_are_non_composable() {
        let r = reg();
        assert!(canonicalize("degC", &r, 1).is_ok());
        assert!(canonicalize("degC^1", &r, 1).is_ok());
        for bad in ["degC*h", "degC^2", "degC/degC", "degC*h/h", "degF*delta_degC", "pu*MW"] {
            let e = canonicalize(bad, &r, 1).unwrap_err();
            assert_eq!(e.code(), ErrorCode::AffineCompound, "for {bad}");
        }
        // Delta counterparts compose freely — the reason the split exists.
        assert_eq!(canon("delta_degC/h"), "delta_degC*h^-1");
    }

    #[test]
    fn display_forms_are_pointed_at_their_symbol() {
        let r = reg();
        let e = canonicalize("°C", &r, 1).unwrap_err();
        assert_eq!(e.code(), ErrorCode::UnitSyntax);
        let m = e.message();
        assert!(m.contains("write `degC`"), "{m}");
        assert!(m.contains("degree Celsius"), "{m}");

        let e = canonicalize("°F", &r, 1).unwrap_err();
        assert!(e.message().contains("write `degF`"), "{}", e.message());

        // Inside a compound too, not just alone.
        let e = canonicalize("°C*h", &r, 1).unwrap_err();
        assert!(e.message().contains("write `degC`"), "{}", e.message());
    }

    #[test]
    fn micro_sign_gets_a_hint_even_without_a_display_form() {
        let r = reg();
        let e = canonicalize("µW", &r, 1).unwrap_err();
        assert!(e.message().contains("write `u`"), "{}", e.message());
        assert!(e.message().contains("remains correct for display"), "{}", e.message());
    }

    #[test]
    fn verbose_names_resolve_through_canonicalize() {
        let r = reg();
        assert_eq!(canonicalize("megawatt", &r, 1).unwrap().canonical, "MW");
        assert_eq!(canonicalize("megawatt*hour", &r, 1).unwrap().canonical, "MW*h");
        assert_eq!(canonicalize("kilowatt/hour", &r, 1).unwrap().canonical, "kW*h^-1");
    }

    #[test]
    fn unknown_symbols_and_newer_grammars_are_distinguished() {
        let r = reg();
        assert_eq!(canonicalize("Zorkmid", &r, 1).unwrap_err().code(), ErrorCode::UnknownUnit);
        assert_eq!(canonicalize("unknown", &r, 1).unwrap_err().code(), ErrorCode::UnknownUnit);
        assert_eq!(canonicalize("MW", &r, 2).unwrap_err().code(), ErrorCode::GrammarVersion);
    }

    #[test]
    fn merge_overflow_is_caught_after_merging() {
        let r = reg();
        assert_eq!(canonicalize("m^100*m^100", &r, 1).unwrap_err().code(), ErrorCode::ExpRange);
    }

    #[test]
    fn scale_composes_exactly() {
        let r = reg();
        let mwh = canonicalize("MW*h", &r, 1).unwrap();
        // 1e6 W * 3600 s = 3.6e9 J
        assert_eq!(mwh.scale(&r).unwrap(), crate::Rational::integer(3_600_000_000).into());
        assert!(canonicalize("degC", &r, 1).unwrap().scale(&r).is_none());
    }
}
