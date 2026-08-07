//! Dimension vectors over the ten base dimensions (spec §6.2).
//!
//! The SI seven plus `count`, `currency`, and `angle`, exponents signed 8-bit.
//! Energy is *not* a base dimension — it is `{mass:1, length:2, time:-2}`.
//! Commensurability is always decided on the vector, never on the name a unit
//! references, so two units defined against differently-named derived
//! dimensions still compare equal when their vectors match.
//!
//! # Why `angle` is here, when SI says the radian is dimensionless
//!
//! `count` and `currency` were already not SI base quantities — this project
//! decided at M0 that engineering utility beats SI purity in the dimension
//! vector, and `count` is the exact precedent: a count is a pure number in SI
//! too, and was separated so that "5 widgets" cannot be added to a ratio.
//!
//! Angle earned the same treatment on the same grounds (AMB-066). With the
//! radian dimensionless, `rad/s` and `Hz` share the vector `{time: -1}` and are
//! therefore commensurable — so a conversion between angular frequency and
//! frequency is offered, silently, at a factor of 1, when the truth is
//! `ω = 2πf`. In power-systems work that confusion is routine and the error is
//! 6.28×. Separating angle makes the two incommensurable and the mistake
//! impossible to express.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use core::fmt;

/// The ten base dimensions, in the canonical order used by every display and
/// serialization in this crate.
pub const BASE_DIMENSIONS: [&str; 10] = [
    "length",
    "mass",
    "time",
    "current",
    "temperature",
    "amount",
    "luminosity",
    "count",
    "currency",
    "angle",
];

/// A signed-8-bit exponent vector over [`BASE_DIMENSIONS`].
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Dimension {
    exps: [i8; 10],
}

impl Dimension {
    pub const DIMENSIONLESS: Dimension = Dimension { exps: [0; 10] };

    pub fn from_exponents(exps: [i8; 10]) -> Dimension {
        Dimension { exps }
    }

    /// Look up a base dimension's index by name. `None` for a name that is not
    /// one of the nine — registry validation reports that as
    /// `E_REGISTRY_INVALID` rather than silently ignoring the key.
    pub fn index_of(name: &str) -> Option<usize> {
        BASE_DIMENSIONS.iter().position(|d| *d == name)
    }

    pub fn get(&self, index: usize) -> i8 {
        self.exps[index]
    }

    pub fn set(&mut self, index: usize, value: i8) {
        self.exps[index] = value;
    }

    pub fn is_dimensionless(&self) -> bool {
        self.exps.iter().all(|e| *e == 0)
    }

    /// Multiplication adds vectors (§6.2). Returns `None` on i8 overflow, which
    /// the caller surfaces as `E_EXP_RANGE`.
    pub fn checked_mul(&self, other: &Dimension) -> Option<Dimension> {
        let mut out = [0i8; 10];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = self.exps[i].checked_add(other.exps[i])?;
        }
        Some(Dimension { exps: out })
    }

    /// Raise to an integer power — a term's exponent applied to its unit's
    /// dimension. Returns `None` on i8 overflow.
    pub fn checked_pow(&self, exp: i32) -> Option<Dimension> {
        let mut out = [0i8; 10];
        for (i, slot) in out.iter_mut().enumerate() {
            let scaled = (self.exps[i] as i32).checked_mul(exp)?;
            if scaled < i8::MIN as i32 || scaled > i8::MAX as i32 {
                return None;
            }
            *slot = scaled as i8;
        }
        Some(Dimension { exps: out })
    }

    /// Sparse `(name, exponent)` pairs in canonical order, zeros omitted — the
    /// shape the conformance fixtures assert against.
    pub fn sparse(&self) -> Vec<(&'static str, i8)> {
        BASE_DIMENSIONS
            .iter()
            .enumerate()
            .filter(|(i, _)| self.exps[*i] != 0)
            .map(|(i, name)| (*name, self.exps[i]))
            .collect()
    }
}

impl fmt::Display for Dimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts = self.sparse();
        if parts.is_empty() {
            return write!(f, "dimensionless");
        }
        let rendered: Vec<String> = parts
            .iter()
            .map(|(n, e)| if *e == 1 { (*n).to_string() } else { format!("{n}^{e}") })
            .collect();
        write!(f, "{}", rendered.join("·"))
    }
}

impl fmt::Debug for Dimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts = self.sparse();
        if parts.is_empty() {
            return write!(f, "{{}}");
        }
        let rendered: Vec<String> = parts.iter().map(|(n, e)| format!("{n}: {e}")).collect();
        write!(f, "{{{}}}", rendered.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dim(pairs: &[(&str, i8)]) -> Dimension {
        let mut d = Dimension::DIMENSIONLESS;
        for (n, e) in pairs {
            d.set(Dimension::index_of(n).unwrap(), *e);
        }
        d
    }

    #[test]
    fn energy_and_power_differ_by_one_time_exponent() {
        let energy = dim(&[("mass", 1), ("length", 2), ("time", -2)]);
        let power = dim(&[("mass", 1), ("length", 2), ("time", -3)]);
        assert_ne!(energy, power);
        let time = dim(&[("time", 1)]);
        assert_eq!(power.checked_mul(&time).unwrap(), energy);
    }

    #[test]
    fn division_cancels_to_dimensionless() {
        let power = dim(&[("mass", 1), ("length", 2), ("time", -3)]);
        let inverse = power.checked_pow(-1).unwrap();
        assert!(power.checked_mul(&inverse).unwrap().is_dimensionless());
    }

    #[test]
    fn overflow_is_reported_not_wrapped() {
        let d = dim(&[("length", 100)]);
        assert!(d.checked_mul(&d).is_none(), "100 + 100 must not wrap to -56");
        assert!(d.checked_pow(2).is_none());
    }

    #[test]
    fn unknown_base_dimension_is_rejected() {
        assert!(Dimension::index_of("energy").is_none());
        assert!(Dimension::index_of("length").is_some());
    }
}
