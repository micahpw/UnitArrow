//! SI prefix expansion.
//!
//! §6.1 is explicit that "prefixed units (`kW`, `GBtu`) are **distinct registry
//! entries**" — resolution stays a flat lookup, and canonical form never has to
//! decide whether `ft` means *foot* or *femto-tonne*. That decision is right,
//! but writing 25 entries per unit by hand is not: each hand-typed factor is a
//! chance to write `[1000000, 1]` where `[1000000000, 1]` was meant, and a wrong
//! prefix factor is silent (AMB-047).
//!
//! So a unit declares `prefixes = "si"` and the **loader** expands it, computing
//! every factor by exact rational arithmetic from one authored value. The
//! registry source stays small; the resolution table stays flat.
//!
//! Two hazards this must catch rather than paper over, both measured on a modest
//! unit set:
//!
//! - **Collisions.** `cd` is candela *and* centi-day; `ft` is foot *and*
//!   femto-tonne; `min` is minute *and* milli-inch. Expansion that silently
//!   overwrote either reading would corrupt every conversion using it, so a
//!   collision is `E_REGISTRY_INVALID` at load.
//! - **Overflow.** Exact rationals are i128. `Btu`'s numerator is 52752792631,
//!   so a quetta prefix (10³⁰) overflows at 10²⁸. Refusing loudly beats
//!   wrapping.

use crate::rational::Rational;

/// `(name, symbol, power of ten)`.
pub type Prefix = (&'static str, &'static str, i32);

/// The full SI set as of the 2022 CGPM additions — 24 prefixes.
///
/// Note `micro` is **`u`**, not `µ`: symbols are ASCII (§6.1), so the Greek mu
/// cannot appear in a unit string. `µW` is a *display* form, never an input.
pub const SI: [Prefix; 24] = [
    ("quetta", "Q", 30),
    ("ronna", "R", 27),
    ("yotta", "Y", 24),
    ("zetta", "Z", 21),
    ("exa", "E", 18),
    ("peta", "P", 15),
    ("tera", "T", 12),
    ("giga", "G", 9),
    ("mega", "M", 6),
    ("kilo", "k", 3),
    ("hecto", "h", 2),
    ("deca", "da", 1),
    ("deci", "d", -1),
    ("centi", "c", -2),
    ("milli", "m", -3),
    ("micro", "u", -6),
    ("nano", "n", -9),
    ("pico", "p", -12),
    ("femto", "f", -15),
    ("atto", "a", -18),
    ("zepto", "z", -21),
    ("yocto", "y", -24),
    ("ronto", "r", -27),
    ("quecto", "q", -30),
];

/// The engineering subset: powers of 10³ from exa down to atto, **symmetric**.
///
/// The 10³ steps are the ones that appear in practice; `hecto`, `deca`, `deci`,
/// and `centi` exist for `hPa`, `cm`, `dL` and belong to registries that want
/// them, via `"si"`.
///
/// The upper bound is `exa` deliberately, not `tera`: this project's own domain
/// routinely needs `TWh` and `PWh`, and `EJ` is standard in global energy
/// statistics. A range that stopped at `tera` would fail on the data the spec
/// was written for.
pub const SI_ENGINEERING: [Prefix; 12] = [
    ("exa", "E", 18),
    ("peta", "P", 15),
    ("tera", "T", 12),
    ("giga", "G", 9),
    ("mega", "M", 6),
    ("kilo", "k", 3),
    ("milli", "m", -3),
    ("micro", "u", -6),
    ("nano", "n", -9),
    ("pico", "p", -12),
    ("femto", "f", -15),
    ("atto", "a", -18),
];

/// Resolve a `prefixes = "…"` value to its prefix set.
pub fn set_by_name(name: &str) -> Option<&'static [Prefix]> {
    match name {
        "si" => Some(&SI),
        "si-engineering" => Some(&SI_ENGINEERING),
        _ => None,
    }
}

/// Every prefix set, for documentation and error messages.
pub const SETS: [(&str, &[Prefix]); 2] = [("si", &SI), ("si-engineering", &SI_ENGINEERING)];

/// The exact multiplier for a power of ten. `None` on i128 overflow, which the
/// caller reports as `E_REGISTRY_INVALID` naming the offending expansion.
pub fn decimal_factor(power: i32) -> Option<Rational> {
    let magnitude = 10i128.checked_pow(power.unsigned_abs())?;
    if power >= 0 {
        Rational::new(magnitude, 1)
    } else {
        Rational::new(1, magnitude)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[test]
    fn known_sets_resolve() {
        assert_eq!(set_by_name("si").unwrap().len(), 24);
        assert_eq!(set_by_name("si-engineering").unwrap().len(), 12);
        assert!(set_by_name("imperial").is_none());
    }

    #[test]
    fn the_engineering_set_is_symmetric_and_reaches_the_domain() {
        let set = set_by_name("si-engineering").unwrap();
        let hi = set.iter().map(|(_, _, p)| *p).max().unwrap();
        let lo = set.iter().map(|(_, _, p)| *p).min().unwrap();
        assert_eq!((hi, lo), (18, -18), "an asymmetric range is an arbitrary one");
        assert!(set.iter().all(|(_, _, p)| p % 3 == 0), "10^3 steps only");
        // TWh and PWh are routine in energy work; EJ is standard globally.
        for needed in ["T", "P", "E"] {
            assert!(set.iter().any(|(_, s, _)| *s == needed), "missing {needed}");
        }
    }

    #[test]
    fn decimal_factors_are_exact() {
        assert_eq!(decimal_factor(3).unwrap(), Rational::integer(1000));
        assert_eq!(decimal_factor(-3).unwrap(), Rational::new(1, 1000).unwrap());
        assert_eq!(decimal_factor(0).unwrap(), Rational::ONE);
        // 10^30 fits in i128 on its own; combining it with a large unit factor
        // is where overflow actually appears.
        assert!(decimal_factor(30).is_some());
        assert!(decimal_factor(39).is_none());
    }

    #[test]
    fn micro_is_ascii_u_not_greek_mu() {
        let micro = SI.iter().find(|(n, _, _)| *n == "micro").unwrap();
        assert_eq!(micro.1, "u");
        assert!(micro.1.is_ascii());
        for (_, sym, _) in SI {
            assert!(sym.is_ascii(), "prefix symbol {sym:?} must be ASCII (§6.1)");
        }
    }

    #[test]
    fn prefix_symbols_are_unique() {
        let mut seen: Vec<&str> = SI.iter().map(|(_, s, _)| *s).collect();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(seen.len(), before, "duplicate prefix symbol");
    }
}

/// English spelling variants accepted on input.
///
/// The SI brochure uses `metre`, `litre`, and `deca`; US usage is `meter`,
/// `liter`, and `deka`. Both are the same unit and a user should not have to
/// guess which the registry author chose, so both resolve.
///
/// **This is a curated list, not a rule**, and it has to be. A generic
/// "-re/-er" swap looks tempting until `tonne` meets `ton`: those are *different
/// units* — 1000 kg against 907.18474 kg — and generating one as an alias of the
/// other would be a silent 10 % error on every mass in the table. Same shape as
/// the `ft` / femto-tonne collision, and the same answer: enumerate what is
/// safe, and let load-time collision checks catch the rest.
pub const SPELLINGS: [(&str, &str); 3] = [
    ("metre", "meter"),
    ("litre", "liter"),
    ("deca", "deka"),
];

/// The alternate spelling of `name`, if it has one.
///
/// Matches on a suffix so composed names come along: `kilometre` yields
/// `kilometer`, and the plural `metres` yields `meters`.
pub fn spelling_variant(name: &str) -> Option<alloc::string::String> {
    use alloc::string::ToString;
    for (a, b) in SPELLINGS {
        for (from, to) in [(a, b), (b, a)] {
            // Suffix, or suffix + a plural `s`, so `metres` -> `meters`.
            for tail in ["", "s"] {
                let pattern = alloc::format!("{from}{tail}");
                if name.ends_with(&pattern) {
                    let head = &name[..name.len() - pattern.len()];
                    return Some(alloc::format!("{head}{to}{tail}").to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod spelling_tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn both_spellings_map_to_each_other() {
        assert_eq!(spelling_variant("metre"), Some("meter".to_string()));
        assert_eq!(spelling_variant("meter"), Some("metre".to_string()));
        assert_eq!(spelling_variant("litre"), Some("liter".to_string()));
    }

    #[test]
    fn composed_and_plural_names_come_along() {
        assert_eq!(spelling_variant("kilometre"), Some("kilometer".to_string()));
        assert_eq!(spelling_variant("metres"), Some("meters".to_string()));
        assert_eq!(spelling_variant("nanometers"), Some("nanometres".to_string()));
    }

    #[test]
    fn unrelated_names_are_untouched() {
        assert_eq!(spelling_variant("watt"), None);
        assert_eq!(spelling_variant("second"), None);
        // The trap this list exists to avoid: `ton` and `tonne` are different
        // units, so no rule may relate them.
        assert_eq!(spelling_variant("tonne"), None);
        assert_eq!(spelling_variant("ton"), None);
    }
}
