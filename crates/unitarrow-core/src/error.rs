//! The §10 error taxonomy.
//!
//! Codes are **wire-stable identifiers**: identical strings across Rust,
//! Python, and TS/WASM. Bindings map these to native error types but MUST
//! preserve `code()` verbatim. Adding a variant is a spec change, not an
//! implementation detail.

use alloc::string::String;

use core::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    code: ErrorCode,
    message: String,
    /// Byte offset into the offending input, where applicable. §10 (0.18.0)
    /// requires errors to carry the offending symbol and its offset when
    /// available, so a syntax error is actionable.
    offset: Option<usize>,
    symbol: Option<String>,
}

impl Error {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Error {
        Error {
            code,
            message: message.into(),
            offset: None,
            symbol: None,
        }
    }

    pub fn at(code: ErrorCode, message: impl Into<String>, offset: usize) -> Error {
        Error {
            code,
            message: message.into(),
            offset: Some(offset),
            symbol: None,
        }
    }

    pub fn with_symbol(mut self, symbol: impl Into<String>) -> Error {
        self.symbol = Some(symbol.into());
        self
    }

    pub fn code(&self) -> ErrorCode {
        self.code
    }

    /// The wire-stable code string — this is what conformance fixtures assert.
    pub fn code_str(&self) -> &'static str {
        self.code.as_str()
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn offset(&self) -> Option<usize> {
        self.offset
    }

    pub fn symbol(&self) -> Option<&str> {
        self.symbol.as_deref()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)?;
        if let Some(sym) = &self.symbol {
            write!(f, " (symbol: {sym:?})")?;
        }
        if let Some(off) = self.offset {
            write!(f, " at byte {off}")?;
        }
        Ok(())
    }
}

impl core::error::Error for Error {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    /// Unit string does not parse under the declared grammar (§6.1).
    UnitSyntax,
    /// Symbol does not resolve against the loaded registry.
    UnknownUnit,
    /// Add/compare/concat across incommensurable dimensions.
    DimMismatch,
    /// Strict-mode concat/join across commensurable but different units.
    UnitMismatch,
    /// Affine unit inside a compound expression (§6.4).
    AffineCompound,
    /// Absolute unit on an `interval: true` quantity kind.
    IntervalMisuse,
    /// `pu` without a base.
    BaseMissing,
    /// p.u. arithmetic across different bases.
    BaseMismatch,
    /// Conversion would lose precision under the requested storage policy (§5.1).
    CastLossy,
    /// Unit on an invalid nesting position (§5.1).
    BadPlacement,
    /// Aggregation across quantity kinds not `summable_with`.
    KindUnsummable,
    /// §8.5 violations.
    TemporalMismatch,
    /// Wire `grammar` newer than this implementation supports.
    GrammarVersion,
    /// Extension metadata absent, not valid UTF-8/JSON, missing a required key,
    /// or containing an escape sequence in a unit-valued field (§5.2).
    BadMetadata,
    /// A loaded registry or vocabulary violates a load-time validation rule.
    RegistryInvalid,
    /// Exponent outside the representable range (§6.2). Proposed, not yet in
    /// §10 — tracked as AMB-015; fixtures using it remain `provisional`.
    ExpRange,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::UnitSyntax => "E_UNIT_SYNTAX",
            ErrorCode::UnknownUnit => "E_UNKNOWN_UNIT",
            ErrorCode::DimMismatch => "E_DIM_MISMATCH",
            ErrorCode::UnitMismatch => "E_UNIT_MISMATCH",
            ErrorCode::AffineCompound => "E_AFFINE_COMPOUND",
            ErrorCode::IntervalMisuse => "E_INTERVAL_MISUSE",
            ErrorCode::BaseMissing => "E_BASE_MISSING",
            ErrorCode::BaseMismatch => "E_BASE_MISMATCH",
            ErrorCode::CastLossy => "E_CAST_LOSSY",
            ErrorCode::BadPlacement => "E_BAD_PLACEMENT",
            ErrorCode::KindUnsummable => "E_KIND_UNSUMMABLE",
            ErrorCode::TemporalMismatch => "E_TEMPORAL_MISMATCH",
            ErrorCode::GrammarVersion => "E_GRAMMAR_VERSION",
            ErrorCode::BadMetadata => "E_BAD_METADATA",
            ErrorCode::RegistryInvalid => "E_REGISTRY_INVALID",
            ErrorCode::ExpRange => "E_EXP_RANGE",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub type Result<T> = core::result::Result<T, Error>;

/// Warning codes. §10 gives only `(W_TAINT_INTRODUCED, W_COERCED, …)` — the
/// ellipsis is doing normative work, tracked as AMB-045. This enum is the
/// working set; it is not yet spec-frozen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WarningCode {
    TaintIntroduced,
    Coerced,
    UnresolvedSymbol,
    CastLossy,
}

impl WarningCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            WarningCode::TaintIntroduced => "W_TAINT_INTRODUCED",
            WarningCode::Coerced => "W_COERCED",
            WarningCode::UnresolvedSymbol => "W_UNRESOLVED_SYMBOL",
            WarningCode::CastLossy => "W_CAST_LOSSY",
        }
    }
}
