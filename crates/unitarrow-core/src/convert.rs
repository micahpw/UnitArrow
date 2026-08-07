//! Conversion factors, spec §6.4 (kelvin offset convention, as ruled).
//!
//! ```text
//! base = x * factor_from + offset_from
//! y    = (base - offset_to) / factor_to
//! ```
//!
//! `offset` is expressed in the **dimension's base unit**, so offsets are
//! directly comparable across units of one dimension. Every step is exact
//! rational arithmetic; `to_f64` is the only exit to floating point.
//!
//! The guard this exists for: a 20 °C daily swing is 36 °F, not 68 °F. Absolute
//! temperatures convert through the offsets; *differences* must use the delta
//! units, which carry no offset.

use alloc::format;
use alloc::string::{String, ToString};

use crate::canonical::{self, CanonicalUnit};
use crate::error::{Error, ErrorCode, Result};
use crate::rational::{Rational, Scale};
use crate::registry::Registry;

/// A conversion from one unit to another, as exact rationals.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conversion {
    pub from: String,
    pub to: String,
    /// Multiplicative scale applied to the value.
    pub factor: Scale,
    /// Additive term applied *after* scaling, in the target unit. Zero for a
    /// purely multiplicative conversion.
    pub offset: Rational,
}

impl Conversion {
    pub fn is_affine(&self) -> bool {
        !self.offset.is_zero()
    }

    /// Apply exactly, for a rational input.
    ///
    /// `None` when the factor carries a π exponent: πx is irrational for every
    /// non-zero rational x, so there is no exact rational answer to return.
    /// Callers wanting a number in that case use [`Conversion::apply`], which is
    /// where floats are allowed to appear (§7.2).
    pub fn apply_exact(&self, x: Rational) -> Option<Rational> {
        if !self.factor.is_rational() {
            return None;
        }
        x.checked_mul(self.factor.ratio())?.checked_add(self.offset)
    }

    /// Apply in floating point — the final step, and the only place floats
    /// appear (§7.2).
    pub fn apply(&self, x: f64) -> f64 {
        x * self.factor.to_f64() + self.offset.to_f64()
    }

    /// `y = factor·x + offset`, for display.
    pub fn formula(&self) -> String {
        match (self.factor.is_one(), self.offset.is_zero()) {
            (true, true) => "y = x".to_string(),
            (true, false) => format!("y = x + {}", self.offset),
            (false, true) => format!("y = x × {}", self.factor),
            (false, false) => format!("y = x × {} + {}", self.factor, self.offset),
        }
    }
}

/// Compute the conversion between two unit strings.
pub fn conversion(from: &str, to: &str, registry: &Registry) -> Result<Conversion> {
    let a = canonical::canonicalize(from, registry, 1)?;
    let b = canonical::canonicalize(to, registry, 1)?;
    between(&a, &b, registry)
}

/// Compute the conversion between two already-canonicalized units.
pub fn between(a: &CanonicalUnit, b: &CanonicalUnit, registry: &Registry) -> Result<Conversion> {
    if a.dimension != b.dimension {
        return Err(Error::new(
            ErrorCode::DimMismatch,
            format!(
                "{} is {:?} but {} is {:?}; the dimensions must match to convert",
                a.canonical, a.dimension, b.canonical, b.dimension
            ),
        ));
    }

    let overflow = || Error::new(ErrorCode::ExpRange, "factor arithmetic overflowed");

    match (&a.affine_symbol, &b.affine_symbol) {
        // Affine → affine (or affine ↔ a linear unit of the same dimension).
        (Some(_), _) | (_, Some(_)) => {
            let (fa, oa) = affine_parts(a, registry)?;
            let (fb, ob) = affine_parts(b, registry)?;
            // base = x·fa + oa ;  y = (base − ob)/fb
            let factor = fa.checked_div(fb).ok_or_else(overflow)?;
            // Offsets are intercepts in the base unit and stay rational: an
            // affine unit with a π-scaled factor is not a thing this format
            // models, and `fb` is a plain rational for every affine unit.
            let offset = oa
                .checked_sub(ob)
                .ok_or_else(overflow)?
                .checked_div(fb.ratio())
                .ok_or_else(overflow)?;
            Ok(Conversion {
                from: a.canonical.clone(),
                to: b.canonical.clone(),
                factor,
                offset,
            })
        }
        // Purely multiplicative.
        (None, None) => {
            let sa = a.scale(registry).ok_or_else(overflow)?;
            let sb = b.scale(registry).ok_or_else(overflow)?;
            Ok(Conversion {
                from: a.canonical.clone(),
                to: b.canonical.clone(),
                factor: sa.checked_div(sb).ok_or_else(overflow)?,
                offset: Rational::ZERO,
            })
        }
    }
}

/// `(factor, offset)` to the dimension's base unit.
fn affine_parts(u: &CanonicalUnit, registry: &Registry) -> Result<(Scale, Rational)> {
    if let Some(sym) = &u.affine_symbol {
        let unit = registry.resolve(sym).expect("resolved during canonicalization");
        return Ok((unit.factor, unit.offset_or_zero()));
    }
    let scale = u
        .scale(registry)
        .ok_or_else(|| Error::new(ErrorCode::ExpRange, "factor arithmetic overflowed"))?;
    Ok((scale, Rational::ZERO))
}

/// The delta (difference) counterpart of a unit — the unit a `interval: true`
/// quantity kind must use (§6.4). For a non-affine unit this is itself.
pub fn delta_of(symbol: &str, registry: &Registry) -> Option<String> {
    let u = registry.resolve(symbol)?;
    match &u.delta {
        Some(d) => Some(d.clone()),
        None if !u.is_affine() => Some(u.symbol.clone()),
        None => None,
    }
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

    #[test]
    fn celsius_to_fahrenheit_is_exact() {
        let r = reg();
        let c = conversion("degC", "degF", &r).unwrap();
        assert_eq!(c.apply(100.0), 212.0);
        assert_eq!(c.apply(0.0), 32.0);
        assert_eq!(c.apply(-40.0), -40.0);
        assert_eq!(c.apply(20.0), 68.0);
        // Exact rationals throughout: 9/5 and 32.
        assert_eq!(c.factor.ratio(), Rational::new(9, 5).unwrap());
        assert_eq!(c.offset, Rational::integer(32));
    }

    #[test]
    fn the_twenty_degree_swing_guard() {
        let r = reg();
        // A *difference* of 20 °C is 36 °F, not 68 — the classic bug §6.4 names.
        let delta = conversion("delta_degC", "delta_degF", &r).unwrap();
        assert_eq!(delta.apply(20.0), 36.0);
        assert!(!delta.is_affine(), "delta conversions carry no offset");

        let absolute = conversion("degC", "degF", &r).unwrap();
        assert_eq!(absolute.apply(20.0), 68.0);
    }

    #[test]
    fn celsius_kelvin_round_trips() {
        let r = reg();
        let to_k = conversion("degC", "K", &r).unwrap();
        assert_eq!(to_k.apply(100.0), 373.15);
        let back = conversion("K", "degC", &r).unwrap();
        assert_eq!(back.apply(373.15), 100.0);
        assert_eq!(back.apply(to_k.apply(-40.0)), -40.0);
    }

    #[test]
    fn fahrenheit_to_kelvin_uses_the_kelvin_offset_convention() {
        let r = reg();
        let c = conversion("degF", "K", &r).unwrap();

        // The exact-rational path is *exactly* right — 32 °F is 5463/20 K,
        // which is 273.15 with no residue. This is the claim §7.2 makes.
        assert_eq!(c.apply_exact(Rational::integer(32)).unwrap(), Rational::new(5463, 20).unwrap());
        assert_eq!(c.apply_exact(Rational::integer(212)).unwrap(), Rational::new(7463, 20).unwrap());

        // The f64 path carries the usual binary-floating-point residue, because
        // 5/9 is not representable. That is why the rational path exists and
        // why floats appear only at the final step.
        assert!((c.apply(32.0) - 273.15).abs() < 1e-12);
        assert!((c.apply(212.0) - 373.15).abs() < 1e-12);
    }

    #[test]
    fn multiplicative_conversions_are_exact_rationals() {
        let r = reg();
        let c = conversion("MW", "kW", &r).unwrap();
        assert_eq!(c.factor.ratio(), Rational::integer(1000));
        assert!(!c.is_affine());

        // A composite: MWh → Btu, where a float pipeline would round early.
        // The factor stays an exact ratio of two large integers.
        let c = conversion("MW*h", "Btu", &r).unwrap();
        assert_eq!(c.factor.ratio().numerator(), 180_000_000_000_000_000);
        assert_eq!(c.factor.ratio().denominator(), 52_752_792_631);
        assert!((c.apply(1.0) - 3_412_141.633_127_9).abs() < 1e-6);
    }

    #[test]
    fn incommensurable_conversions_are_rejected() {
        let r = reg();
        let e = conversion("MW", "h", &r).unwrap_err();
        assert_eq!(e.code(), ErrorCode::DimMismatch);
    }

    #[test]
    fn delta_lookup_follows_the_registry() {
        let r = reg();
        assert_eq!(delta_of("degC", &r).as_deref(), Some("delta_degC"));
        assert_eq!(delta_of("degF", &r).as_deref(), Some("delta_degF"));
        // A non-affine unit is its own delta.
        assert_eq!(delta_of("K", &r).as_deref(), Some("K"));
        assert_eq!(delta_of("MW", &r).as_deref(), Some("MW"));
    }
}
