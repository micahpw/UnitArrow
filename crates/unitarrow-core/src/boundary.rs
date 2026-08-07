//! The registry boundary: reading a table someone else tagged (AMB-059).
//!
//! # The hazard this closes
//!
//! Canonical form (§6.3) is **registry-relative**. `bbl` canonicalizes to
//! `"bbl"` in every registry that defines it, whatever factor it carries — and
//! §5.6 pins the registry per *table*, so two tables can legitimately arrive
//! with different pins. §8.3 then says *"identical canonical units → result
//! keeps the unit"*, so a join across an oil `bbl` and a water `bbl` succeeds
//! and every row is wrong by a third. §8.1 coerces the right operand only *if
//! units differ*; they do not, so nothing is coerced.
//!
//! The one guarantee §6.3 exists to provide — equal canonical strings mean the
//! same quantity — does not survive a differing registry pin, and nothing
//! checked the pin before relying on it.
//!
//! # Why this shape
//!
//! Comparing the **pins** would be sound and useless: a registry version bump
//! that adds units without touching `MW` would reject every join, and version
//! bumps are the common case. So the check compares the **symbols the schema
//! actually uses**, and only when the pins differ.
//!
//! That costs one resolution per distinct unit string in the schema — a
//! handful, never per row and never per element. It is also strictly more
//! precise than a pin comparison: exact where it matters, silent where it does
//! not.
//!
//! # What it also catches
//!
//! A lookalike registry that redefines `MW` (AMB-047's open hole) is caught the
//! moment a table tagged by the real one meets it. That is a side effect rather
//! than the purpose, and it only covers the read path.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::rational::Scale;
use crate::registry::Registry;

/// How one symbol's meaning differs across two registries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disagreement {
    /// Same dimension, different scale. The dangerous one: dimension checks
    /// pass, canonical forms match, and every value is wrong by the ratio.
    Factor,
    /// Different dimension vectors — the two registries do not even agree what
    /// kind of quantity this is.
    Dimension,
    /// One is affine and the other is not, or their offsets differ. A degree
    /// scale whose zero moved is wrong by the offset at every value.
    Offset,
    /// The incoming table used a symbol this deployment cannot resolve.
    UnknownLocally,
    /// The symbol resolves here but not in the registry the table was tagged
    /// against — so the incoming tag could not have meant what it says.
    UnknownForeign,
}

impl Disagreement {
    pub fn as_str(&self) -> &'static str {
        match self {
            Disagreement::Factor => "different factor",
            Disagreement::Dimension => "different dimension",
            Disagreement::Offset => "different offset",
            Disagreement::UnknownLocally => "unknown to this deployment",
            Disagreement::UnknownForeign => "unknown to the source registry",
        }
    }
}

/// One symbol that does not mean the same thing on both sides.
#[derive(Clone, Debug, PartialEq)]
pub struct Conflict {
    pub symbol: String,
    pub kind: Disagreement,
    /// What this deployment's effective registry says, when it resolves.
    pub local: Option<(String, Scale)>,
    /// What the source registry says, when it resolves.
    pub foreign: Option<(String, Scale)>,
}

impl Conflict {
    /// The ratio between the two readings, when both are scales of the same
    /// dimension. This is the number that makes a report actionable — "wrong by
    /// 33%" lands where "factors differ" does not.
    pub fn ratio(&self) -> Option<f64> {
        match (&self.local, &self.foreign) {
            (Some((_, a)), Some((_, b))) if b.to_f64() != 0.0 => Some(a.to_f64() / b.to_f64()),
            _ => None,
        }
    }

    pub fn describe(&self) -> String {
        let mut s = format!("{:?} — {}", self.symbol, self.kind.as_str());
        if let (Some((ld, lf)), Some((fd, ff))) = (&self.local, &self.foreign) {
            s.push_str(&format!(
                "; here {lf} ({ld}), in the source {ff} ({fd})"
            ));
            if let Some(r) = self.ratio() {
                if self.kind == Disagreement::Factor {
                    s.push_str(&format!(" — values differ by {:.1}%", (r - 1.0) * 100.0));
                }
            }
        }
        s
    }
}

/// The verdict on reading a foreign table.
#[derive(Clone, Debug, PartialEq)]
pub enum Boundary {
    /// The pins match. Nothing was checked because nothing could differ — this
    /// is the common case and it costs one string comparison.
    SamePin,
    /// The pins differ, but every symbol the schema uses means the same thing
    /// on both sides. A version bump that did not touch the units in play lands
    /// here, which is what makes the rule usable.
    Compatible { checked: usize },
    /// The pins differ and these symbols disagree.
    Incompatible(Vec<Conflict>),
    /// The pins differ and the source registry could not be obtained, so
    /// nothing can be certified. This is the cost of §5.6's `pin` embedding
    /// mode, made explicit rather than silently treated as agreement.
    Unverifiable { foreign_pin: String },
}

impl Boundary {
    /// May values cross this boundary without qualification?
    pub fn is_safe(&self) -> bool {
        matches!(self, Boundary::SamePin | Boundary::Compatible { .. })
    }

    /// The §10 code a **strict** reader raises. `None` when safe.
    ///
    /// `E_UNIT_MISMATCH` is the right code for a disagreement: §10 already
    /// defines it as *commensurable but different units*, which is precisely
    /// what two registries disagreeing about `bbl` produces — the dimension
    /// matches and the scale does not.
    pub fn strict_code(&self) -> Option<crate::error::ErrorCode> {
        use crate::error::ErrorCode::*;
        match self {
            Boundary::SamePin | Boundary::Compatible { .. } => None,
            Boundary::Unverifiable { .. } => Some(RegistryInvalid),
            Boundary::Incompatible(cs) => Some(
                if cs.iter().any(|c| c.kind == Disagreement::Dimension) {
                    DimMismatch
                } else if cs.iter().any(|c| c.kind == Disagreement::UnknownLocally) {
                    UnknownUnit
                } else {
                    UnitMismatch
                },
            ),
        }
    }

    pub fn message(&self) -> String {
        match self {
            Boundary::SamePin => "same registry pin".to_string(),
            Boundary::Compatible { checked } => {
                format!("{checked} symbol(s) agree across differing registry pins")
            }
            Boundary::Unverifiable { foreign_pin } => format!(
                "the table was tagged against registry {foreign_pin}, which is not available \
                 here, so its units cannot be checked against this deployment's registry. \
                 Obtain that registry, or republish the table with the registry embedded (§5.6)"
            ),
            Boundary::Incompatible(cs) => {
                let mut s = String::from(
                    "the table was tagged against a different registry, and these symbols do \
                     not mean the same thing here: ",
                );
                for (i, c) in cs.iter().enumerate() {
                    if i > 0 {
                        s.push_str("; ");
                    }
                    s.push_str(&c.describe());
                }
                s
            }
        }
    }
}

fn facts(r: &Registry, symbol: &str) -> Option<(String, Scale, Option<crate::rational::Rational>)> {
    r.resolve(symbol)
        .map(|u| (u.dimension_name.clone(), u.factor, u.offset))
}

/// Check whether a table tagged against `foreign` can be read against `local`.
///
/// `symbols` are the distinct unit symbols the incoming schema uses — not every
/// symbol either registry defines. Pass what the table actually carries.
///
/// Pins are compared first: equal pins short-circuit, so the ordinary
/// single-registry path pays one string comparison and nothing else.
pub fn check_boundary(
    local: &Registry,
    local_pin: &str,
    foreign: &Registry,
    foreign_pin: &str,
    symbols: &[&str],
) -> Boundary {
    if local_pin == foreign_pin {
        return Boundary::SamePin;
    }
    let mut conflicts = Vec::new();
    for symbol in symbols {
        match (facts(local, symbol), facts(foreign, symbol)) {
            (Some((ld, lf, lo)), Some((fd, ff, fo))) => {
                let kind = if local.resolve(symbol).map(|u| u.dimension)
                    != foreign.resolve(symbol).map(|u| u.dimension)
                {
                    Some(Disagreement::Dimension)
                } else if lo != fo {
                    Some(Disagreement::Offset)
                } else if lf != ff {
                    Some(Disagreement::Factor)
                } else {
                    None
                };
                if let Some(kind) = kind {
                    conflicts.push(Conflict {
                        symbol: (*symbol).to_string(),
                        kind,
                        local: Some((ld, lf)),
                        foreign: Some((fd, ff)),
                    });
                }
            }
            (None, Some((fd, ff, _))) => conflicts.push(Conflict {
                symbol: (*symbol).to_string(),
                kind: Disagreement::UnknownLocally,
                local: None,
                foreign: Some((fd, ff)),
            }),
            (Some((ld, lf, _)), None) => conflicts.push(Conflict {
                symbol: (*symbol).to_string(),
                kind: Disagreement::UnknownForeign,
                local: Some((ld, lf)),
                foreign: None,
            }),
            (None, None) => conflicts.push(Conflict {
                symbol: (*symbol).to_string(),
                kind: Disagreement::UnknownLocally,
                local: None,
                foreign: None,
            }),
        }
    }
    if conflicts.is_empty() {
        Boundary::Compatible { checked: symbols.len() }
    } else {
        Boundary::Incompatible(conflicts)
    }
}

/// The verdict when the source registry cannot be obtained.
///
/// Separated from [`check_boundary`] because "I could not check" is a different
/// claim from "I checked and it was fine", and collapsing them is how a `pin`
/// embedding silently becomes an assumption of agreement.
pub fn unverifiable(local_pin: &str, foreign_pin: &str) -> Boundary {
    if local_pin == foreign_pin {
        Boundary::SamePin
    } else {
        Boundary::Unverifiable { foreign_pin: foreign_pin.to_string() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HDR: &str = "[registry]\nschema_version = 1\nname = \"r\"\nversion = \"1\"\n";

    fn reg(body: &str) -> Registry {
        Registry::from_toml(&format!("{HDR}{body}")).expect("loads")
    }

    fn oil() -> Registry {
        reg("[unit.m]\ndimension = \"length\"\nfactor = [1, 1]\n\
             [unit.bbl]\ndimension = \"length\"\nfactor = [9938205933, 62500000000]\n")
    }
    fn water() -> Registry {
        reg("[unit.m]\ndimension = \"length\"\nfactor = [1, 1]\n\
             [unit.bbl]\ndimension = \"length\"\nfactor = [29810117799, 250000000000]\n")
    }

    #[test]
    fn the_silent_case_is_now_loud() {
        let b = check_boundary(&oil(), "sha256:aaa", &water(), "sha256:bbb", &["bbl"]);
        assert!(!b.is_safe());
        let Boundary::Incompatible(cs) = &b else { panic!("{b:?}") };
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].kind, Disagreement::Factor);
        // The report must carry the magnitude — "factors differ" is not
        // actionable, "wrong by 33%" is.
        let m = b.message();
        assert!(m.contains("33.4%"), "{m}");
        assert_eq!(b.strict_code(), Some(crate::error::ErrorCode::UnitMismatch));
    }

    #[test]
    fn a_version_bump_that_touches_nothing_in_use_is_free() {
        // The case a pin comparison would wrongly reject: different registry,
        // same meaning for every symbol the schema actually carries.
        let v1 = oil();
        let v2 = Registry::from_toml(&format!(
            "{HDR}[unit.m]\ndimension = \"length\"\nfactor = [1, 1]\n\
             [unit.bbl]\ndimension = \"length\"\nfactor = [9938205933, 62500000000]\n\
             [unit.acre_ft]\ndimension = \"length\"\nfactor = [616740919, 500000]\n"
        ))
        .unwrap();
        let b = check_boundary(&v1, "sha256:v1", &v2, "sha256:v2", &["bbl", "m"]);
        assert_eq!(b, Boundary::Compatible { checked: 2 });
        assert!(b.is_safe());
        assert_eq!(b.strict_code(), None);
    }

    #[test]
    fn matching_pins_cost_one_comparison_and_check_nothing() {
        // Deliberately pass registries that DO disagree: an equal pin means the
        // same bytes, so a disagreement is impossible and checking is waste.
        let b = check_boundary(&oil(), "sha256:same", &water(), "sha256:same", &["bbl"]);
        assert_eq!(b, Boundary::SamePin);
        assert!(b.is_safe());
    }

    #[test]
    fn a_moved_zero_is_caught_even_when_the_scale_matches() {
        // Affine units fail differently: identical factor, different offset.
        // A degree scale whose zero moved is wrong at every value.
        let a = reg("[unit.degC]\ndimension = \"temperature\"\nfactor = [1, 1]\noffset = [5463, 20]\n");
        let b = reg("[unit.degC]\ndimension = \"temperature\"\nfactor = [1, 1]\noffset = [5463, 21]\n");
        let v = check_boundary(&a, "sha256:a", &b, "sha256:b", &["degC"]);
        let Boundary::Incompatible(cs) = &v else { panic!("{v:?}") };
        assert_eq!(cs[0].kind, Disagreement::Offset);
    }

    #[test]
    fn a_dimension_disagreement_reports_as_one() {
        let a = reg("[unit.pu]\ndimension = \"mass\"\nfactor = [1, 1]\n");
        let b = reg("[unit.pu]\ndimension = \"length\"\nfactor = [1, 1]\n");
        let v = check_boundary(&a, "sha256:a", &b, "sha256:b", &["pu"]);
        assert_eq!(v.strict_code(), Some(crate::error::ErrorCode::DimMismatch));
    }

    #[test]
    fn an_unavailable_source_registry_is_not_agreement() {
        // The cost of §5.6's `pin` embedding, made explicit. Treating "could
        // not check" as "checked and fine" is exactly the silence AMB-059 is
        // about.
        let b = unverifiable("sha256:local", "sha256:foreign");
        assert!(!b.is_safe());
        assert!(b.message().contains("not available here"), "{}", b.message());
        assert!(b.message().contains("embedded"), "names the fix: {}", b.message());
        // ...but an equal pin needs no registry at all.
        assert_eq!(unverifiable("sha256:x", "sha256:x"), Boundary::SamePin);
    }

    #[test]
    fn only_the_symbols_in_use_are_checked() {
        // The property that makes this cheap: `acre_ft` disagrees, but the
        // schema does not use it, so the boundary is clean.
        let a = reg("[unit.m]\ndimension = \"length\"\nfactor = [1, 1]\n\
                     [unit.acre_ft]\ndimension = \"length\"\nfactor = [616740919, 500000]\n");
        let b = reg("[unit.m]\ndimension = \"length\"\nfactor = [1, 1]\n\
                     [unit.acre_ft]\ndimension = \"length\"\nfactor = [1, 1]\n");
        assert!(check_boundary(&a, "p1", &b, "p2", &["m"]).is_safe());
        assert!(!check_boundary(&a, "p1", &b, "p2", &["m", "acre_ft"]).is_safe());
    }

    #[test]
    fn a_symbol_this_deployment_cannot_resolve_is_reported_as_such() {
        let local = reg("[unit.m]\ndimension = \"length\"\nfactor = [1, 1]\n");
        let foreign = reg("[unit.m]\ndimension = \"length\"\nfactor = [1, 1]\n\
                           [unit.acre_ft]\ndimension = \"length\"\nfactor = [616740919, 500000]\n");
        let v = check_boundary(&local, "p1", &foreign, "p2", &["m", "acre_ft"]);
        let Boundary::Incompatible(cs) = &v else { panic!("{v:?}") };
        assert_eq!(cs[0].kind, Disagreement::UnknownLocally);
        assert_eq!(v.strict_code(), Some(crate::error::ErrorCode::UnknownUnit));
    }

    #[test]
    fn prefixed_symbols_are_checked_through_the_derived_branch() {
        // `km` is not authored anywhere; it must still be compared correctly,
        // because AMB-048 derives prefixed units at resolution.
        let a = reg("[unit.m]\ndimension = \"length\"\nfactor = [1, 1]\n\
                     prefixes = \"si-engineering\"\ndisplay = { long = \"metre\" }\n");
        let b = reg("[unit.m]\ndimension = \"length\"\nfactor = [100, 1]\n\
                     prefixes = \"si-engineering\"\ndisplay = { long = \"metre\" }\n");
        let v = check_boundary(&a, "p1", &b, "p2", &["km"]);
        let Boundary::Incompatible(cs) = &v else { panic!("{v:?}") };
        assert_eq!(cs[0].symbol, "km");
        assert_eq!(cs[0].kind, Disagreement::Factor);
    }
}
