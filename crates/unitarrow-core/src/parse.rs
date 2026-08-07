//! Unit-string lexing and parsing, spec §6.1 (grammar v1, as ruled 2026-07-29).
//!
//! ```text
//! unit-string  = product / inverse / "1"
//! product      = term *( ( "*" / "/" ) term )
//! inverse      = "1" 1*( "/" term )
//! term         = symbol [ "^" integer ]
//! symbol       = ALPHA *( ALPHA / DIGIT / "_" / "@" )
//! integer      = [ "-" ] ( "0" / ( NZDIGIT *DIGIT ) )
//! ```
//!
//! The parser is **mode-independent**: it accepts any well-formed input and
//! reports `E_UNIT_SYNTAX` otherwise. Symbol *resolution* happens later, and
//! mode policy is applied by the reader above both (§6.3, as ruled).
//!
//! Design note on `/`: division is input notation only. Each `/` negates the
//! exponent of the term that follows it, left to right — `a/b/c` is
//! a·b⁻¹·c⁻¹, never a/(b/c). Canonical form (§6.3) renders signs inside
//! exponents and contains no `/` at all.
//!
//! Design note on whitespace: permitted at the edges and around operators,
//! where it cannot change meaning, and **rejected between two adjacent
//! symbols**, where it silently could. Collapsing `m s` to `ms` turns
//! metre-second into millisecond — a different dimension, with no diagnostic —
//! and treating the space as multiplication would give the product a second
//! spelling. Neither is worth the convenience, so that one case is an error
//! that names the fix.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::{Error, ErrorCode, Result};

/// One parsed term: a registry symbol and its signed exponent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Term {
    pub symbol: String,
    pub exponent: i32,
    /// Byte offset of the symbol in the source, for error reporting.
    pub offset: usize,
}

/// The parsed term list, in source order, before alias resolution or merging.
/// §6.4's affine check runs against *this* — before the §6.3 step 2 merge — so
/// `degC/degC` and `degC*h/h` are rejected rather than silently cancelling.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ParsedUnit {
    pub terms: Vec<Term>,
}

impl ParsedUnit {
    pub fn is_empty_product(&self) -> bool {
        self.terms.is_empty()
    }
}

fn is_symbol_start(c: char) -> bool {
    c.is_ascii_alphabetic()
}

fn is_symbol_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '@'
}

/// Parse a unit string into its term list.
/// Advance past ASCII whitespace. Only called where a space cannot change
/// meaning — never between two bare symbols.
fn skip_ws(bytes: &[u8], i: &mut usize) {
    while *i < bytes.len() && (bytes[*i] as char).is_ascii_whitespace() {
        *i += 1;
    }
}

pub fn parse(input: &str) -> Result<ParsedUnit> {
    let bytes = input.as_bytes();

    if input.trim().is_empty() {
        // §6.1: explicitly not equivalent to "1", and not equivalent to
        // untagged — an empty field must never become a dimensionless claim.
        return Err(Error::new(
            ErrorCode::UnitSyntax,
            "empty unit string; the empty string is not a valid unit and is not equivalent to \"1\"",
        ));
    }

    let mut i = 0usize;
    skip_ws(bytes, &mut i);
    if input[i..].trim_end() == "1" {
        return Ok(ParsedUnit::default());
    }

    let mut terms: Vec<Term> = Vec::new();
    let mut sign: i32 = 1;
    let mut first_term = true;

    // `1/…` inverse head: consume the `1`, leaving the `/` for the loop.
    if bytes[i] == b'1' {
        let mut probe = i + 1;
        skip_ws(bytes, &mut probe);
        if probe < bytes.len() && bytes[probe] == b'/' {
            i = probe;
            first_term = false;
        } else {
            return Err(Error::at(
                ErrorCode::UnitSyntax,
                "the literal `1` is legal only as the whole string or as the head of an inverse \
                 (`1/s`); `1*MW` and bare digits are invalid",
                i,
            ));
        }
    }

    loop {
        if !first_term {
            skip_ws(bytes, &mut i);
            if i >= bytes.len() {
                break;
            }
            match bytes[i] {
                b'*' => sign = 1,
                b'/' => sign = -1,
                _ if is_symbol_start(input[i..].chars().next().unwrap()) => {
                    // The one case blanket-stripping would corrupt: `m s`
                    // collapses to `ms` (millisecond), and treating the space as
                    // multiplication would give the product a second spelling.
                    let prev = terms.last().map(|t| t.symbol.as_str()).unwrap_or("");
                    let rest = &input[i..];
                    let next: String = rest
                        .chars()
                        .take_while(|c| is_symbol_continue(*c))
                        .collect();
                    return Err(Error::at(
                        ErrorCode::UnitSyntax,
                        format!(
                            "two symbols separated only by whitespace; a space is not \
                             multiplication and joining them would change the meaning — \
                             write `{prev}*{next}` (or `{prev}{next}`, if that is one symbol)"
                        ),
                        i,
                    ));
                }
                _ => {
                    return Err(Error::at(
                        ErrorCode::UnitSyntax,
                        format!(
                            "expected `*` or `/`, found {:?}",
                            input[i..].chars().next().unwrap()
                        ),
                        i,
                    ))
                }
            }
            i += 1;
            skip_ws(bytes, &mut i);
            if i >= bytes.len() {
                return Err(Error::at(
                    ErrorCode::UnitSyntax,
                    "trailing operator with no term following it",
                    i,
                ));
            }
        }
        first_term = false;

        // Symbol.
        let start = i;
        let first = input[i..].chars().next().unwrap();
        if !is_symbol_start(first) {
            let hint = if first == '(' || first == ')' {
                "parentheses are not part of grammar v1; division binds left-to-right"
            } else if first.is_ascii_digit() || first == '.' {
                "numeric prefixes and scale factors are not permitted inside unit strings; \
                 prefixed units such as `kW` are distinct registry entries"
            } else if first == '\u{b5}' || first == '\u{3bc}' {
                "`µ` is a display form, not an input symbol — write `u` (`uW`) or spell it \
                 out (`microwatt`). `µW` remains correct for display and is what a label or \
                 axis should show"
            } else if first == '\u{b0}' {
                "`°` is a display form, not an input symbol — degree units are written `degC` \
                 / `degF`, which display as `°C` / `°F`"
            } else if first == '\u{3a9}' || first == '\u{2126}' {
                "`Ω` is a display form, not an input symbol — write `ohm`, which displays as `Ω`"
            } else if !first.is_ascii() {
                "unit symbols are ASCII; display forms such as `°C` are output-only and are \
                 never parsed"
            } else {
                "expected a symbol starting with an ASCII letter"
            };
            return Err(Error::at(ErrorCode::UnitSyntax, hint, i));
        }
        i += first.len_utf8();
        while i < bytes.len() {
            let c = input[i..].chars().next().unwrap();
            if is_symbol_continue(c) {
                i += c.len_utf8();
            } else {
                break;
            }
        }
        let symbol = input[start..i].to_string();

        // Optional exponent. No whitespace inside a term: `m ^2` reads as two
        // tokens to a human and would need a rule of its own.
        let mut exponent: i32 = 1;
        if i < bytes.len() && bytes[i] == b'^' {
            let caret = i;
            i += 1;
            if i >= bytes.len() {
                return Err(Error::at(
                    ErrorCode::UnitSyntax,
                    "`^` with no integer following it",
                    caret,
                ));
            }
            let mut negative = false;
            if bytes[i] == b'-' {
                negative = true;
                i += 1;
            } else if bytes[i] == b'+' {
                return Err(Error::at(
                    ErrorCode::UnitSyntax,
                    "an explicit `+` is not permitted in an exponent",
                    i,
                ));
            }
            let digits_start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let digits = &input[digits_start..i];
            if digits.is_empty() {
                return Err(Error::at(
                    ErrorCode::UnitSyntax,
                    "`^` with no integer following it",
                    caret,
                ));
            }
            if digits.len() > 1 && digits.starts_with('0') {
                return Err(Error::at(
                    ErrorCode::UnitSyntax,
                    "leading zeros are not permitted in an exponent",
                    digits_start,
                ));
            }
            if negative && digits == "0" {
                return Err(Error::at(
                    ErrorCode::UnitSyntax,
                    "`-0` is not a valid exponent",
                    digits_start,
                ));
            }
            let magnitude: i32 = digits.parse().map_err(|_| {
                Error::at(
                    ErrorCode::ExpRange,
                    "exponent magnitude is out of range",
                    digits_start,
                )
            })?;
            exponent = if negative { -magnitude } else { magnitude };

            if sign == -1 && exponent < 0 {
                return Err(Error::at(
                    ErrorCode::UnitSyntax,
                    "a negative exponent after `/` is a double negative; write the positive form",
                    digits_start,
                ));
            }
        }

        let effective = exponent
            .checked_mul(sign)
            .filter(|e| *e >= i8::MIN as i32 && *e <= i8::MAX as i32)
            .ok_or_else(|| {
                Error::at(
                    ErrorCode::ExpRange,
                    "exponent outside the signed 8-bit range [-128, 127]",
                    start,
                )
                .with_symbol(&symbol)
            })?;

        terms.push(Term {
            symbol,
            exponent: effective,
            offset: start,
        });

        skip_ws(bytes, &mut i);
        if i >= bytes.len() {
            break;
        }
    }

    if terms.is_empty() {
        return Err(Error::new(
            ErrorCode::UnitSyntax,
            "an inverse form requires at least one term after `1/`",
        ));
    }

    Ok(ParsedUnit { terms })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn terms(s: &str) -> Vec<(String, i32)> {
        parse(s)
            .unwrap()
            .terms
            .into_iter()
            .map(|t| (t.symbol, t.exponent))
            .collect()
    }

    fn err(s: &str) -> ErrorCode {
        parse(s).unwrap_err().code()
    }

    #[test]
    fn division_binds_left_to_right() {
        // a/b/c is a·b⁻¹·c⁻¹ — NOT a/(b/c), which would leave c positive.
        assert_eq!(
            terms("MW/h/K"),
            vec![("MW".into(), 1), ("h".into(), -1), ("K".into(), -1)]
        );
    }

    #[test]
    fn only_the_slashed_term_is_negated() {
        assert_eq!(
            terms("W/K*m^2"),
            vec![("W".into(), 1), ("K".into(), -1), ("m".into(), 2)]
        );
    }

    #[test]
    fn inverse_form_is_accepted_as_input() {
        assert_eq!(terms("1/s"), vec![("s".into(), -1)]);
        assert_eq!(terms("1/K/m^2"), vec![("K".into(), -1), ("m".into(), -2)]);
    }

    #[test]
    fn whole_string_one_is_the_empty_product() {
        assert!(parse("1").unwrap().is_empty_product());
    }

    #[test]
    fn empty_string_is_not_one() {
        assert_eq!(err(""), ErrorCode::UnitSyntax);
    }

    #[test]
    fn whitespace_is_accepted_where_it_cannot_change_meaning() {
        // Edges and around operators: the operator makes the intent explicit.
        for spelling in [
            "MW*h", " MW*h", "MW*h ", "  MW*h  ", "MW * h", "MW*  h", "MW / h",
        ] {
            assert!(parse(spelling).is_ok(), "expected {spelling:?} to parse");
        }
        assert_eq!(terms("MW * h"), vec![("MW".into(), 1), ("h".into(), 1)]);
        assert_eq!(terms("MW / h"), vec![("MW".into(), 1), ("h".into(), -1)]);
        assert_eq!(terms(" 1 / s "), vec![("s".into(), -1)]);
        assert_eq!(terms("  1  "), vec![]);
    }

    #[test]
    fn whitespace_between_two_symbols_is_rejected() {
        // The one case blanket-stripping would corrupt: `m s` would collapse to
        // `ms`, which in an SI registry is a millisecond — a different dimension,
        // silently. The error names the fix rather than guessing.
        for bad in ["m s", "MW h", "kW  h"] {
            let e = parse(bad).unwrap_err();
            assert_eq!(e.code(), ErrorCode::UnitSyntax, "for {bad:?}");
            assert!(
                e.message().contains("a space is not"),
                "for {bad:?}: {}",
                e.message()
            );
        }
        let e = parse("m s").unwrap_err();
        assert!(e.message().contains("write `m*s`"), "{}", e.message());
    }

    #[test]
    fn rejects_the_ruled_out_forms() {
        for bad in [
            "1*MW",    // bare 1 outside the inverse head
            "MW**h",   // doubled operator
            "MW^",     // caret with no integer
            "MW*",     // trailing operator
            "(m*s)",   // parentheses
            "0.5*MW",  // numeric scale factor
            "°C",      // non-ASCII symbol
            "MW^+2",   // explicit plus
            "MW^02",   // leading zero
            "m^-0",    // negative zero
            "MW/s^-1", // double negative
            "m^1/2",   // v2 rational exponent
        ] {
            assert_eq!(
                err(bad),
                ErrorCode::UnitSyntax,
                "expected E_UNIT_SYNTAX for {bad:?}"
            );
        }
    }

    #[test]
    fn exponent_zero_and_one_are_legal_input() {
        assert_eq!(terms("m^0"), vec![("m".into(), 0)]);
        assert_eq!(terms("MW^1"), vec![("MW".into(), 1)]);
    }

    #[test]
    fn per_term_exponent_range_is_enforced() {
        assert_eq!(err("m^200"), ErrorCode::ExpRange);
        // Each term in range; the overflow is the merge's problem (§6.3 step 2).
        assert!(parse("m^100*m^100").is_ok());
    }

    #[test]
    fn at_signs_and_underscores_are_symbol_characters() {
        assert_eq!(
            terms("household_yr@CO"),
            vec![("household_yr@CO".into(), 1)]
        );
        assert_eq!(terms("delta_degC"), vec![("delta_degC".into(), 1)]);
    }

    #[test]
    fn non_ascii_display_characters_get_a_targeted_hint() {
        for (input, want) in [
            ("\u{b5}W", "write `u`"),
            ("\u{3bc}W", "remains correct for display"),
            ("\u{b0}C", "written `degC`"),
            ("\u{3a9}m", "write `ohm`"),
        ] {
            let e = parse(input).unwrap_err();
            assert_eq!(e.code(), ErrorCode::UnitSyntax);
            assert!(e.message().contains(want), "for {input:?}: {}", e.message());
        }
    }

    #[test]
    fn verbose_names_lex_as_ordinary_symbols() {
        // Resolution is the registry's job; the grammar just has to accept them.
        assert_eq!(terms("megawatt"), vec![("megawatt".into(), 1)]);
        assert_eq!(terms("microwatt"), vec![("microwatt".into(), 1)]);
        assert_eq!(
            terms("megawatt*hour"),
            vec![("megawatt".into(), 1), ("hour".into(), 1)]
        );
    }

    #[test]
    fn errors_carry_an_offset() {
        let e = parse("MW**h").unwrap_err();
        assert_eq!(e.offset(), Some(3));
        let e = parse("m s").unwrap_err();
        assert_eq!(e.offset(), Some(2), "points at the second symbol");
    }
}
