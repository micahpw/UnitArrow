//! Typed expression fragments and their lowering to SQL (§8.6).
//!
//! # What this is, and is not
//!
//! §2 delegates numeric execution to existing engines, and this does not take
//! it back. Nothing here touches a value. It answers two questions about a
//! *schema*: given the units of the operands, is this operation legal, and what
//! unit does the result carry — then emits the arithmetic as text for an engine
//! to run. A million-row rescale costs this crate nothing; DuckDB does it in
//! half a millisecond.
//!
//! That inverts the metadata-survival problem. Tags do not need to survive an
//! engine that discards them (AMB-062) if the caller knows the output unit
//! before the query runs, because the result is **retagged from the plan**
//! rather than recovered from the input.
//!
//! # Why the factor is emitted as a rational
//!
//! Measured over 20k values per factor, `x * (num/den)` and `x * num / den`
//! each disagree with exact rational arithmetic on some inputs and agree on
//! others — worst case one ulp, neither uniformly better. Precision does not
//! decide it. Auditability does: `* 52752792631 / 50000000` puts the registry's
//! exact definition in the query a reviewer reads, where `* 1055.05585262`
//! puts a rounding that has to be checked against something else.

use alloc::format;
use alloc::string::{String, ToString};

use crate::canonical::{canonicalize, commensurable, CanonicalUnit};
use crate::convert::conversion;
use crate::error::{Error, ErrorCode, Result};
use crate::registry::Registry;

/// How an engine spells the constants a conversion may need.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dialect {
    /// The engine's π. DuckDB and Postgres spell it `pi()`; a caller building
    /// a polars expression passes a literal instead.
    pub pi: &'static str,
}

impl Dialect {
    pub const SQL: Dialect = Dialect { pi: "pi()" };
    pub const LITERAL_PI: Dialect = Dialect {
        pi: "3.141592653589793115997963468544185161590576171875",
    };
}

impl Default for Dialect {
    fn default() -> Self {
        Dialect::SQL
    }
}

/// An expression and the unit its result carries.
#[derive(Clone, Debug, PartialEq)]
pub struct Fragment {
    /// Text for the engine. Parenthesised, so it composes without precedence
    /// surprises.
    pub sql: String,
    /// The §6.3 canonical unit of the result — what the output column should be
    /// tagged with.
    pub unit: String,
}

/// A bare column reference, carrying whatever unit it is tagged with.
pub fn column(name: &str, unit: &str, registry: &Registry) -> Result<Fragment> {
    let u = canonicalize(unit, registry, 1)?;
    Ok(Fragment {
        sql: name.to_string(),
        unit: u.canonical,
    })
}

/// Convert a fragment to another unit, emitting the scaling arithmetic.
///
/// Affine units add an intercept rather than scaling, so a caller that assumes
/// multiplication is silently wrong for every temperature column — the offset
/// is emitted here rather than left to be remembered.
pub fn convert(f: &Fragment, to: &str, registry: &Registry, d: Dialect) -> Result<Fragment> {
    let c = conversion(&f.unit, to, registry)?;
    let target = canonicalize(to, registry, 1)?;

    let (num, den) = (c.factor.ratio().numerator(), c.factor.ratio().denominator());
    let pi = c.factor.pi_exponent();

    let mut sql = f.sql.clone();
    if num != 1 || den != 1 || pi != 0 {
        sql = format!("({sql}");
        if num != 1 {
            sql.push_str(&format!(" * {num}"));
        }
        for _ in 0..pi.max(0) {
            sql.push_str(&format!(" * {}", d.pi));
        }
        if den != 1 {
            sql.push_str(&format!(" / {den}"));
        }
        for _ in 0..(-pi).max(0) {
            sql.push_str(&format!(" / {}", d.pi));
        }
        sql.push(')');
    }
    if !c.offset.is_zero() {
        let (on, od) = (c.offset.numerator(), c.offset.denominator());
        sql = if od == 1 {
            format!("({sql} + {on})")
        } else {
            format!("({sql} + {on}.0 / {od})")
        };
    }
    Ok(Fragment {
        sql,
        unit: target.canonical,
    })
}

fn rebuild(terms: &[(String, i32)], registry: &Registry) -> Result<CanonicalUnit> {
    let live: alloc::vec::Vec<String> = terms
        .iter()
        .filter(|(_, e)| *e != 0)
        .map(|(s, e)| {
            if *e == 1 {
                s.clone()
            } else {
                format!("{s}^{e}")
            }
        })
        .collect();
    // Handed back to the one code path that produces canonical form (§6.3);
    // never assembled here.
    let expr = if live.is_empty() {
        "1".to_string()
    } else {
        live.join("*")
    };
    canonicalize(&expr, registry, 1)
}

fn combine(a: &Fragment, b: &Fragment, sign: i32, registry: &Registry) -> Result<CanonicalUnit> {
    let (ua, ub) = (
        canonicalize(&a.unit, registry, 1)?,
        canonicalize(&b.unit, registry, 1)?,
    );
    let mut terms = ua.terms.clone();
    for (sym, exp) in &ub.terms {
        match terms.iter_mut().find(|(s, _)| s == sym) {
            Some(t) => t.1 += sign * exp,
            None => terms.push((sym.clone(), sign * exp)),
        }
    }
    rebuild(&terms, registry)
}

/// `a * b` — dimensions add, and the result unit is the canonicalized product.
pub fn mul(a: &Fragment, b: &Fragment, registry: &Registry) -> Result<Fragment> {
    let unit = combine(a, b, 1, registry)?;
    Ok(Fragment {
        sql: format!("({} * {})", a.sql, b.sql),
        unit: unit.canonical,
    })
}

/// `a / b` — dimensions subtract.
pub fn div(a: &Fragment, b: &Fragment, registry: &Registry) -> Result<Fragment> {
    let unit = combine(a, b, -1, registry)?;
    Ok(Fragment {
        sql: format!("({} / {})", a.sql, b.sql),
        unit: unit.canonical,
    })
}

/// `a + b` (or `-`), per §8.1: the dimensions must match, the right operand is
/// coerced to the left operand's unit, and the result carries the left unit.
///
/// The coercion is emitted, not assumed — adding `MW` to `kW` produces the
/// scaling in the SQL, so the engine adds comparable numbers.
pub fn add(a: &Fragment, b: &Fragment, registry: &Registry, d: Dialect) -> Result<Fragment> {
    sum(a, b, "+", registry, d)
}

pub fn sub(a: &Fragment, b: &Fragment, registry: &Registry, d: Dialect) -> Result<Fragment> {
    sum(a, b, "-", registry, d)
}

fn sum(a: &Fragment, b: &Fragment, op: &str, registry: &Registry, d: Dialect) -> Result<Fragment> {
    let (ua, ub) = (
        canonicalize(&a.unit, registry, 1)?,
        canonicalize(&b.unit, registry, 1)?,
    );
    if !commensurable(&ua, &ub) {
        return Err(Error::new(
            ErrorCode::DimMismatch,
            format!(
                "cannot add {} to {}: {:?} and {:?} are different dimensions",
                a.unit, b.unit, ua.dimension, ub.dimension
            ),
        ));
    }
    let rhs = if ua.canonical == ub.canonical {
        b.clone()
    } else {
        convert(b, &a.unit, registry, d)?
    };
    Ok(Fragment {
        sql: format!("({} {op} {})", a.sql, rhs.sql),
        unit: ua.canonical,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg() -> Registry {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../registries/power-systems.toml"
        ))
        .unwrap();
        Registry::from_toml(&src).unwrap()
    }

    fn col(n: &str, u: &str, r: &Registry) -> Fragment {
        column(n, u, r).unwrap()
    }

    #[test]
    fn a_bare_column_carries_its_unit_and_no_arithmetic() {
        let r = reg();
        let f = col("p_mw", "MW", &r);
        assert_eq!(f.sql, "p_mw");
        assert_eq!(f.unit, "MW");
    }

    #[test]
    fn the_exact_rational_reaches_the_query() {
        let r = reg();
        // Auditability: a reviewer sees the registry's definition, not a
        // rounding they would have to check against something else.
        let f = convert(&col("q", "Btu", &r), "J", &r, Dialect::SQL).unwrap();
        assert_eq!(f.sql, "(q * 52752792631 / 50000000)");
        assert_eq!(f.unit, "J");
    }

    #[test]
    fn a_whole_number_factor_emits_no_denominator() {
        let r = reg();
        let f = convert(&col("p_mw", "MW", &r), "kW", &r, Dialect::SQL).unwrap();
        assert_eq!(f.sql, "(p_mw * 1000)");
    }

    #[test]
    fn pi_stays_symbolic_all_the_way_into_the_engine() {
        let r = reg();
        // deg -> rad has no rational form. Emitting the engine's own pi keeps
        // "floats come last" true past this process boundary.
        let f = convert(&col("theta", "deg", &r), "rad", &r, Dialect::SQL).unwrap();
        assert_eq!(f.sql, "(theta * pi() / 180)");
    }

    #[test]
    fn an_affine_conversion_emits_its_intercept() {
        let r = reg();
        // The case an emitter that assumes scaling gets silently wrong.
        let f = convert(&col("t_c", "degC", &r), "K", &r, Dialect::SQL).unwrap();
        assert!(f.sql.contains("5463"), "{}", f.sql);
        assert!(
            f.sql.contains('+'),
            "an offset must be added, not multiplied: {}",
            f.sql
        );
    }

    #[test]
    fn products_and_quotients_carry_the_computed_unit() {
        let r = reg();
        let (p, h) = (col("p_mw", "MW", &r), col("hours", "h", &r));
        assert_eq!(mul(&p, &h, &r).unwrap().unit, "MW*h");
        assert_eq!(div(&p, &h, &r).unwrap().unit, "MW*h^-1");
        assert_eq!(mul(&p, &h, &r).unwrap().sql, "(p_mw * hours)");
    }

    #[test]
    fn a_quotient_of_like_units_is_dimensionless() {
        let r = reg();
        // Power factor: MW / MVA. Both are power, so the result is a ratio.
        let f = div(&col("p", "MW", &r), &col("s", "MVA", &r), &r).unwrap();
        // §6.3 orders positives before negatives, each group sorted bytewise.
        assert_eq!(f.unit, "MW*MVA^-1");
        assert!(canonicalize(&f.unit, &r, 1).unwrap().is_dimensionless());
    }

    #[test]
    fn addition_emits_the_coercion_rather_than_assuming_it() {
        let r = reg();
        let f = add(&col("a", "MW", &r), &col("b", "kW", &r), &r, Dialect::SQL).unwrap();
        // The right operand is scaled to the left unit (§8.1), in the SQL, so
        // the engine adds comparable numbers.
        // A unit numerator emits no `* 1`.
        assert_eq!(f.sql, "(a + (b / 1000))");
        assert_eq!(f.unit, "MW");
    }

    #[test]
    fn adding_matching_units_emits_no_scaling() {
        let r = reg();
        let f = add(&col("a", "MW", &r), &col("b", "MW", &r), &r, Dialect::SQL).unwrap();
        assert_eq!(f.sql, "(a + b)");
    }

    #[test]
    fn adding_across_dimensions_is_refused_before_any_sql_exists() {
        let r = reg();
        let e = add(&col("p", "MW", &r), &col("v", "kV", &r), &r, Dialect::SQL).unwrap_err();
        assert_eq!(e.code(), ErrorCode::DimMismatch);
    }

    #[test]
    fn nesting_composes_without_precedence_surprises() {
        let r = reg();
        let (p, h) = (col("p_mw", "MW", &r), col("hours", "h", &r));
        let energy = mul(&p, &h, &r).unwrap();
        let kwh = convert(&energy, "kWh", &r, Dialect::SQL).unwrap();
        assert_eq!(kwh.sql, "((p_mw * hours) * 1000)");
        assert_eq!(kwh.unit, "kWh");
    }

    #[test]
    fn a_dialect_can_supply_its_own_pi() {
        let r = reg();
        let f = convert(&col("t", "deg", &r), "rad", &r, Dialect::LITERAL_PI).unwrap();
        assert!(f.sql.contains("3.14159"), "{}", f.sql);
        assert!(!f.sql.contains("pi()"), "{}", f.sql);
    }
}
