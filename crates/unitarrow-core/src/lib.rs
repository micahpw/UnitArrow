//! `unitarrow-core` — registry loading, unit-string parsing and canonical form,
//! dimension algebra, and exact-rational factor computation.
//!
//! Implements UnitArrow Specification **0.21.0-draft**, §§6–7, at the scope
//! roadmap M1 defines. Deliberately dependency-free (spec §4): bindings are thin
//! and never reimplement this logic.
//!
//! # What is frozen here
//!
//! - **Canonical form** (§6.3) is the cross-language equality and hashing
//!   primitive. Exactly one code path produces it — [`canonical::canonicalize`].
//!   It is slash-free: signs live inside exponents, positives precede negatives,
//!   each group sorted bytewise.
//! - **Exact rationals** (§7.2). Factors and offsets are integer pairs; floats
//!   appear only at [`Rational::to_f64`].
//! - **Error codes** (§10) are wire-stable strings; see [`ErrorCode`].
//!
//! # Example
//!
//! ```
//! use unitarrow_core::{canonicalize, conversion, Registry};
//!
//! let registry = Registry::from_toml(r#"
//! [registry]
//! schema_version = 1
//! name = "example"
//! version = "0.1.0"
//!
//! [dimension.power]
//! vector = { mass = 1, length = 2, time = -3 }
//!
//! [unit.W]
//! dimension = "power"
//! factor = [1, 1]
//!
//! [unit.MW]
//! dimension = "power"
//! factor = [1000000, 1]
//!
//! [unit.h]
//! dimension = "time"
//! factor = [3600, 1]
//! "#).unwrap();
//!
//! // Input notation is flexible; canonical form is not.
//! let u = canonicalize("h*MW", &registry, 1).unwrap();
//! assert_eq!(u.canonical, "MW*h");
//!
//! // Slashes are input sugar — canonical form puts the sign in the exponent.
//! assert_eq!(canonicalize("MW/h", &registry, 1).unwrap().canonical, "MW*h^-1");
//!
//! // Factors stay exact until the final step.
//! let c = conversion("MW", "W", &registry).unwrap();
//! assert_eq!(c.factor.ratio().numerator(), 1_000_000);
//! assert_eq!(c.apply(2.5), 2_500_000.0);
//! ```

#![no_std]
#![forbid(unsafe_code)]

// Spec §4 compiles this crate to WASM under a "tens of KB" budget, and nothing
// here needs an operating system: the registry arrives as a &str, and the only
// collections used are in `alloc`. Staying `no_std` keeps the browser build from
// dragging in std's I/O and formatting machinery.
extern crate alloc;

#[cfg(test)]
extern crate std;

pub mod boundary;
pub mod canonical;
pub mod compose;
pub mod convert;
pub mod dimension;
pub mod error;
pub mod expr;
pub mod json_lite;
pub mod metadata;
pub mod parse;
pub mod prefix;
pub mod rational;
pub mod registry;
pub mod sha256;
pub mod toml_lite;

pub use boundary::{check_boundary, unverifiable, Boundary, Conflict, Disagreement};
pub use canonical::{canonicalize, commensurable, units_equal, CanonicalUnit};
pub use compose::{compose, contested, Composed, Resolution, Source};
pub use convert::{conversion, delta_of, Conversion};
pub use dimension::{Dimension, BASE_DIMENSIONS};
pub use error::{Error, ErrorCode, Result, WarningCode};
pub use expr::{column, Dialect, Fragment};
pub use metadata::{Base, Metadata, Temporal};
pub use rational::{Rational, Scale};
pub use registry::{Ambiguity, CollisionRisk, Provenance, QuantityKind, Registry, Unit};
pub use sha256::digest_pin;

/// The spec version this crate implements.
pub const SPEC_VERSION: &str = "0.21.0-draft";

/// The unit-string grammar version this crate implements (§5.2, §6.1).
pub const GRAMMAR_VERSION: u32 = 1;

/// The Arrow extension name a tagged column carries (§5.1).
pub const EXTENSION_NAME: &str = "unitarrow.quantity.v1";

/// Validate the `unit` value's raw JSON token before decoding (§5.2).
///
/// Canonical form fixes one spelling per unit expression; JSON escapes would
/// give that spelling many byte encodings, so the guarantee has to hold at the
/// encoding layer too. Every character legal in a unit string is representable
/// literally, so this constrains encoding and never expression.
///
/// `raw` is the token **including** its surrounding quotes, exactly as it
/// appears on the wire.
pub fn check_unit_token(raw: &str) -> Result<&str> {
    let inner = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .filter(|_| raw.len() >= 2)
        .ok_or_else(|| Error::new(ErrorCode::BadMetadata, "the `unit` value must be a JSON string"))?;
    if let Some(pos) = inner.find('\\') {
        return Err(Error::at(
            ErrorCode::BadMetadata,
            "escape sequences are not permitted in the `unit` value; every character legal in a \
             unit string is representable literally (§5.2)",
            pos + 1,
        ));
    }
    Ok(inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_literal_token() {
        assert_eq!(check_unit_token("\"MW\"").unwrap(), "MW");
        assert_eq!(check_unit_token("\"\"").unwrap(), "");
    }

    #[test]
    fn rejects_escapes_in_the_unit_token() {
        // Each token below decodes to something a reader would accept,
        // which is precisely why the raw form has to be rejected.
        for bad in [r#""\u004dW""#, r#""MW\/h""#, r#""M\\W""#, r#""\u0001""#] {
            let e = check_unit_token(bad).unwrap_err();
            assert_eq!(e.code(), ErrorCode::BadMetadata, "for {bad}");
        }
    }

    #[test]
    fn a_non_string_unit_value_is_bad_metadata() {
        assert_eq!(check_unit_token("7").unwrap_err().code(), ErrorCode::BadMetadata);
    }
}
