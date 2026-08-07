//! Exact rational arithmetic.
//!
//! Spec §7.2: conversion factors and offsets are exact rationals, "computed to
//! floats only at the final step — this is what makes factors auditable and
//! cross-language identical." So every operation here is integer arithmetic,
//! always reduced, and `to_f64` is the only exit to floating point.

use core::fmt;

/// An exact rational, always stored in lowest terms with a positive denominator.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Rational {
    num: i128,
    den: i128,
}

impl Rational {
    pub const ONE: Rational = Rational { num: 1, den: 1 };
    pub const ZERO: Rational = Rational { num: 0, den: 1 };

    /// Construct from a numerator/denominator pair, reducing to lowest terms.
    /// Returns `None` for a zero denominator — registry validation surfaces
    /// this as `E_REGISTRY_INVALID` rather than panicking.
    pub fn new(num: i128, den: i128) -> Option<Rational> {
        if den == 0 {
            return None;
        }
        let sign = if (num < 0) != (den < 0) { -1 } else { 1 };
        let (a, b) = (num.unsigned_abs(), den.unsigned_abs());
        let g = gcd(a, b).max(1);
        Some(Rational {
            num: sign * (a / g) as i128,
            den: (b / g) as i128,
        })
    }

    pub fn integer(n: i128) -> Rational {
        Rational { num: n, den: 1 }
    }

    pub fn numerator(&self) -> i128 {
        self.num
    }

    pub fn denominator(&self) -> i128 {
        self.den
    }

    pub fn is_zero(&self) -> bool {
        self.num == 0
    }

    pub fn is_one(&self) -> bool {
        self.num == 1 && self.den == 1
    }

    /// True when the pair is already in lowest terms — a registry validation
    /// check, since unreduced pairs overflow quickly under exponentiation.
    pub fn is_reduced(num: i128, den: i128) -> bool {
        den != 0 && gcd(num.unsigned_abs(), den.unsigned_abs()).max(1) == 1 && den > 0
    }

    pub fn checked_mul(self, other: Rational) -> Option<Rational> {
        // Cross-reduce before multiplying to keep intermediates small.
        let g1 = gcd(self.num.unsigned_abs(), other.den.unsigned_abs()).max(1) as i128;
        let g2 = gcd(other.num.unsigned_abs(), self.den.unsigned_abs()).max(1) as i128;
        let num = (self.num / g1).checked_mul(other.num / g2)?;
        let den = (self.den / g2).checked_mul(other.den / g1)?;
        Rational::new(num, den)
    }

    pub fn checked_div(self, other: Rational) -> Option<Rational> {
        if other.num == 0 {
            return None;
        }
        self.checked_mul(Rational {
            num: other.den,
            den: other.num,
        })
    }

    pub fn checked_add(self, other: Rational) -> Option<Rational> {
        let num = self
            .num
            .checked_mul(other.den)?
            .checked_add(other.num.checked_mul(self.den)?)?;
        let den = self.den.checked_mul(other.den)?;
        Rational::new(num, den)
    }

    pub fn checked_sub(self, other: Rational) -> Option<Rational> {
        self.checked_add(Rational {
            num: -other.num,
            den: other.den,
        })
    }

    /// Raise to a signed integer power. Used when composing a compound unit's
    /// factor from its terms' exponents.
    pub fn checked_pow(self, exp: i32) -> Option<Rational> {
        let mut acc = Rational::ONE;
        let mut base = if exp < 0 {
            Rational::ONE.checked_div(self)?
        } else {
            self
        };
        let mut n = exp.unsigned_abs();
        while n > 0 {
            if n & 1 == 1 {
                acc = acc.checked_mul(base)?;
            }
            n >>= 1;
            if n > 0 {
                base = base.checked_mul(base)?;
            }
        }
        Some(acc)
    }

    /// The single exit to floating point (spec §7.2).
    pub fn to_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

impl fmt::Debug for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {}]", self.num, self.den)
    }
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduces_on_construction() {
        let r = Rational::new(1_000_000, 1_000).unwrap();
        assert_eq!((r.numerator(), r.denominator()), (1000, 1));
    }

    #[test]
    fn normalises_sign_to_denominator() {
        let r = Rational::new(1, -2).unwrap();
        assert_eq!((r.numerator(), r.denominator()), (-1, 2));
    }

    #[test]
    fn zero_denominator_is_rejected() {
        assert!(Rational::new(1, 0).is_none());
    }

    #[test]
    fn btu_it_is_exact() {
        // Btu (IT) = 1055.05585262 J exactly, reduced.
        let btu = Rational::new(52_752_792_631, 50_000_000).unwrap();
        assert_eq!(btu.to_f64(), 1055.05585262);
    }

    #[test]
    fn mwh_over_btu_stays_exact() {
        // A composite factor that a float pipeline would round.
        let mwh = Rational::integer(3_600_000_000);
        let btu = Rational::new(52_752_792_631, 50_000_000).unwrap();
        let r = mwh.checked_div(btu).unwrap();
        assert_eq!(r.numerator(), 180_000_000_000_000_000);
        assert_eq!(r.denominator(), 52_752_792_631);
    }

    #[test]
    fn pow_handles_negative_exponents() {
        let kilo = Rational::integer(1000);
        assert_eq!(
            kilo.checked_pow(-2).unwrap(),
            Rational::new(1, 1_000_000).unwrap()
        );
        assert_eq!(kilo.checked_pow(0).unwrap(), Rational::ONE);
    }

    #[test]
    fn detects_unreduced_pairs() {
        assert!(Rational::is_reduced(1000, 1));
        assert!(!Rational::is_reduced(1_000_000, 1_000));
    }
}

/// An exact scale: a [`Rational`] times an integer power of **π**.
///
/// # Why π needs a symbol rather than digits
///
/// A degree is `(π/180)` radians. π is irrational, so *no* rational is the
/// degree's factor — the best a `[num, den]` pair can do is store a rounding and
/// call it a definition, which is exactly what §7.2 exists to prevent. Storing
/// the exponent instead keeps the definition exact: `deg = (1/180)·π¹ rad`
/// composes, inverts, and raises to a power with no loss, and π reaches an `f64`
/// only at [`Scale::to_f64`] — the same "floats come last" rule the rest of the
/// factor arithmetic already follows.
///
/// Almost every unit has `pi == 0` and behaves exactly as a `Rational` did.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Scale {
    rational: Rational,
    /// Exponent of π. Signed, because inverting a scale negates it.
    pi: i32,
}

impl Scale {
    pub const ONE: Scale = Scale {
        rational: Rational::ONE,
        pi: 0,
    };

    pub const fn rational(r: Rational) -> Scale {
        Scale { rational: r, pi: 0 }
    }

    pub const fn new(rational: Rational, pi: i32) -> Scale {
        Scale { rational, pi }
    }

    pub fn ratio(&self) -> Rational {
        self.rational
    }

    pub fn pi_exponent(&self) -> i32 {
        self.pi
    }

    pub fn is_one(&self) -> bool {
        self.pi == 0 && self.rational.is_one()
    }

    /// True when this scale is a plain rational — the ordinary case, and the
    /// one where a factor may be published as an exact `[num, den]` pair.
    pub fn is_rational(&self) -> bool {
        self.pi == 0
    }

    pub fn checked_mul(self, other: Scale) -> Option<Scale> {
        Some(Scale {
            rational: self.rational.checked_mul(other.rational)?,
            pi: self.pi.checked_add(other.pi)?,
        })
    }

    pub fn checked_div(self, other: Scale) -> Option<Scale> {
        Some(Scale {
            rational: self.rational.checked_div(other.rational)?,
            pi: self.pi.checked_sub(other.pi)?,
        })
    }

    pub fn checked_pow(self, exp: i32) -> Option<Scale> {
        Some(Scale {
            rational: self.rational.checked_pow(exp)?,
            pi: self.pi.checked_mul(exp)?,
        })
    }

    /// The single place π becomes a float.
    ///
    /// `powi` lives in `std`, so the exponent is applied by repeated
    /// multiplication — correct for the small exponents unit algebra produces,
    /// and it keeps the crate `no_std`.
    pub fn to_f64(self) -> f64 {
        let mut v = self.rational.to_f64();
        let mut k = self.pi;
        while k > 0 {
            v *= core::f64::consts::PI;
            k -= 1;
        }
        while k < 0 {
            v /= core::f64::consts::PI;
            k += 1;
        }
        v
    }
}

impl From<Rational> for Scale {
    fn from(r: Rational) -> Scale {
        Scale::rational(r)
    }
}

impl core::fmt::Display for Scale {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.pi {
            0 => write!(f, "{}", self.rational),
            1 => write!(f, "{}·π", self.rational),
            n => write!(f, "{}·π^{n}", self.rational),
        }
    }
}

#[cfg(test)]
mod scale_tests {
    use super::*;

    #[test]
    fn a_degree_is_exact_where_a_rational_cannot_be() {
        // deg = (1/180)·π rad
        let deg = Scale::new(Rational::new(1, 180).unwrap(), 1);
        assert!(!deg.is_rational());
        // 180 degrees is exactly π radians — not 3.141592653582.
        let half_turn = deg
            .checked_mul(Scale::rational(Rational::integer(180)))
            .unwrap();
        assert_eq!(half_turn, Scale::new(Rational::ONE, 1));
        assert_eq!(half_turn.to_f64(), core::f64::consts::PI);
    }

    #[test]
    fn round_tripping_is_exact_not_merely_close() {
        let deg = Scale::new(Rational::new(1, 180).unwrap(), 1);
        // rad -> deg -> rad returns the identity exactly, which a stored
        // decimal approximation of π/180 cannot do.
        let there_and_back = deg.checked_div(deg).unwrap();
        assert!(there_and_back.is_one());
        assert_eq!(there_and_back.to_f64(), 1.0);
    }

    #[test]
    fn pi_exponents_add_multiply_and_negate() {
        let pi = Scale::new(Rational::ONE, 1);
        assert_eq!(pi.checked_mul(pi).unwrap().pi_exponent(), 2);
        assert_eq!(pi.checked_pow(3).unwrap().pi_exponent(), 3);
        assert_eq!(Scale::ONE.checked_div(pi).unwrap().pi_exponent(), -1);
        assert!(
            (Scale::ONE.checked_div(pi).unwrap().to_f64() - 1.0 / core::f64::consts::PI).abs()
                < 1e-15
        );
    }

    #[test]
    fn ordinary_units_are_unaffected() {
        let mw = Scale::rational(Rational::integer(1_000_000));
        assert!(mw.is_rational());
        assert_eq!(mw.to_f64(), 1e6);
        assert_eq!(alloc::format!("{mw}"), "1000000");
        assert_eq!(
            alloc::format!("{}", Scale::new(Rational::new(1, 180).unwrap(), 1)),
            "1/180·π"
        );
    }
}
