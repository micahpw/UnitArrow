//! Python bindings for UnitArrow.
//!
//! # What crosses the boundary
//!
//! **Strings, and nothing else.** No Arrow types cross the FFI: a table is
//! tagged by attaching metadata to a pyarrow schema on the Python side, and all
//! this layer supplies is a validated canonical unit string and a §5.2 metadata
//! blob. That keeps the binding free of `pyo3-arrow`, keeps the wheel small,
//! and — more importantly — keeps §4's rule intact, because there is no
//! reimplemented logic here to drift. Every function below is a thin call into
//! `unitarrow-core`.
//!
//! Errors carry their §10 code in the message, because the codes are the
//! wire-stable vocabulary (§10) and a Python caller matching on `ValueError`
//! text should still be able to see which condition fired.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use unitarrow_core::{
    canonicalize, conversion as core_conversion, Metadata, Registry as CoreRegistry,
};

fn err(e: unitarrow_core::Error) -> PyErr {
    PyValueError::new_err(format!("{}: {}", e.code_str(), e.message()))
}

/// A loaded unit registry (§7).
#[pyclass(module = "unitarrow", frozen)]
pub struct Registry {
    inner: CoreRegistry,
}

#[pymethods]
impl Registry {
    /// Load from TOML source.
    #[staticmethod]
    fn from_toml(source: &str) -> PyResult<Registry> {
        CoreRegistry::from_toml(source)
            .map(|inner| Registry { inner })
            .map_err(err)
    }

    /// Load from a path.
    #[staticmethod]
    fn from_path(path: &str) -> PyResult<Registry> {
        let src = std::fs::read_to_string(path)
            .map_err(|e| PyValueError::new_err(format!("cannot read {path}: {e}")))?;
        Registry::from_toml(&src)
    }

    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    #[getter]
    fn version(&self) -> &str {
        &self.inner.version
    }

    /// Authored units, as opposed to the larger resolvable set — prefixed forms
    /// are derived on lookup rather than stored (AMB-048).
    #[getter]
    fn authored_count(&self) -> usize {
        self.inner.authored_count()
    }

    #[getter]
    fn unit_count(&self) -> usize {
        self.inner.unit_count()
    }

    /// Normalize a unit string to its §6.3 canonical form.
    ///
    /// This is the equality primitive: two columns denote the same unit exactly
    /// when their canonical strings match. Never compare a raw string from the
    /// wire.
    fn canonicalize(&self, unit: &str) -> PyResult<String> {
        canonicalize(unit, &self.inner, 1)
            .map(|u| u.canonical)
            .map_err(err)
    }

    /// The sparse dimension vector, as a dict over the ten base dimensions.
    fn dimension(&self, unit: &str) -> PyResult<std::collections::BTreeMap<String, i32>> {
        let u = canonicalize(unit, &self.inner, 1).map_err(err)?;
        Ok(u.dimension
            .sparse()
            .into_iter()
            .map(|(k, v)| (k.to_string(), v as i32))
            .collect())
    }

    /// Read a symbol back as words — `MW` is *megawatt*.
    ///
    /// The reverse direction matters as much as the forward one: it is how
    /// someone checks a tag they did not write.
    fn describe(&self, symbol: &str) -> Option<String> {
        self.inner.describe(symbol)
    }

    /// Are two units the same quantity kind of thing (§6.2)?
    fn commensurable(&self, a: &str, b: &str) -> PyResult<bool> {
        let (a, b) = (
            canonicalize(a, &self.inner, 1).map_err(err)?,
            canonicalize(b, &self.inner, 1).map_err(err)?,
        );
        Ok(unitarrow_core::commensurable(&a, &b))
    }

    /// The exact conversion between two units.
    ///
    /// Returned as integers plus a π exponent rather than a float, because the
    /// factor is exact and the caller decides when to lose that (§7.2). `apply`
    /// is provided for when they are ready to.
    fn conversion(&self, from: &str, to: &str) -> PyResult<Conversion> {
        core_conversion(from, to, &self.inner)
            .map(|c| Conversion {
                from: c.from.clone(),
                to: c.to.clone(),
                numerator: c.factor.ratio().numerator(),
                denominator: c.factor.ratio().denominator(),
                pi: c.factor.pi_exponent(),
                offset_numerator: c.offset.numerator(),
                offset_denominator: c.offset.denominator(),
            })
            .map_err(err)
    }

    /// The §5.2 metadata value for a tagged column.
    ///
    /// Built here rather than in Python so the escape prohibition, the required
    /// keys, and the grammar version are enforced once (§4). The unit is
    /// canonicalized on the way in, so a column can never be tagged with a
    /// spelling this registry does not accept.
    #[pyo3(signature = (unit, quantity=None))]
    fn metadata(&self, unit: &str, quantity: Option<&str>) -> PyResult<String> {
        let canonical = canonicalize(unit, &self.inner, 1).map_err(err)?;
        let mut m = Metadata::new(&canonical.canonical);
        if let Some(q) = quantity {
            m = m.with_quantity(q);
        }
        Ok(m.to_json())
    }

    /// A bare column reference carrying a unit — the leaf of an expression.
    fn col(&self, name: &str, unit: &str) -> PyResult<Fragment> {
        unitarrow_core::expr::column(name, unit, &self.inner)
            .map(|f| Fragment {
                sql: f.sql,
                unit: f.unit,
            })
            .map_err(err)
    }

    /// Spellings this registry refuses on purpose, and why.
    fn refusals(&self) -> Vec<(String, String, Vec<String>)> {
        self.inner
            .ambiguities()
            .into_iter()
            .map(|(s, a)| (s.to_string(), a.reason.clone(), a.use_instead.clone()))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "<Registry {:?} v{} — {} authored, {} resolvable>",
            self.inner.name,
            self.inner.version,
            self.inner.authored_count(),
            self.inner.unit_count()
        )
    }
}

/// An exact conversion (§7.2): a rational, optionally times a power of π.
#[pyclass(module = "unitarrow", frozen, get_all)]
pub struct Conversion {
    from: String,
    to: String,
    numerator: i128,
    denominator: i128,
    /// Exponent of π. Non-zero only for angle conversions, where no rational is
    /// the answer — `deg` to `rad` is exactly `(1/180)·π` (AMB-066).
    pi: i32,
    offset_numerator: i128,
    offset_denominator: i128,
}

#[pymethods]
impl Conversion {
    /// Apply to a value. The single point at which the exact factor becomes a
    /// float, matching §7.2's "floats come last".
    fn apply(&self, x: f64) -> f64 {
        let mut f = self.numerator as f64 / self.denominator as f64;
        let mut k = self.pi;
        while k > 0 {
            f *= core::f64::consts::PI;
            k -= 1;
        }
        while k < 0 {
            f /= core::f64::consts::PI;
            k += 1;
        }
        x * f + self.offset_numerator as f64 / self.offset_denominator as f64
    }

    #[getter]
    fn is_exact_rational(&self) -> bool {
        self.pi == 0
    }

    fn __repr__(&self) -> String {
        let pi = match self.pi {
            0 => String::new(),
            1 => "·π".to_string(),
            n => format!("·π^{n}"),
        };
        format!(
            "<Conversion {} -> {}: y = x × {}/{}{}{}>",
            self.from,
            self.to,
            self.numerator,
            self.denominator,
            pi,
            if self.offset_numerator == 0 {
                String::new()
            } else {
                format!(" + {}/{}", self.offset_numerator, self.offset_denominator)
            }
        )
    }
}

/// A typed expression: SQL text plus the unit its result carries.
///
/// Composed with the ordinary Python operators, so a plan reads like the
/// arithmetic it describes. Nothing here touches data — the engine runs the
/// SQL, and the `unit` is what the output column gets tagged with.
#[pyclass(module = "unitarrow", frozen, get_all, skip_from_py_object)]
#[derive(Clone)]
pub struct Fragment {
    sql: String,
    unit: String,
}

fn core_frag(f: &Fragment) -> unitarrow_core::Fragment {
    unitarrow_core::Fragment {
        sql: f.sql.clone(),
        unit: f.unit.clone(),
    }
}

fn wrap(f: unitarrow_core::Fragment) -> Fragment {
    Fragment {
        sql: f.sql,
        unit: f.unit,
    }
}

#[pymethods]
impl Fragment {
    /// Convert to another unit, emitting the scaling arithmetic.
    fn to(&self, unit: &str, registry: &Registry) -> PyResult<Fragment> {
        unitarrow_core::expr::convert(
            &core_frag(self),
            unit,
            &registry.inner,
            unitarrow_core::Dialect::SQL,
        )
        .map(wrap)
        .map_err(err)
    }

    fn mul(&self, other: &Fragment, registry: &Registry) -> PyResult<Fragment> {
        unitarrow_core::expr::mul(&core_frag(self), &core_frag(other), &registry.inner)
            .map(wrap)
            .map_err(err)
    }

    fn div(&self, other: &Fragment, registry: &Registry) -> PyResult<Fragment> {
        unitarrow_core::expr::div(&core_frag(self), &core_frag(other), &registry.inner)
            .map(wrap)
            .map_err(err)
    }

    fn add(&self, other: &Fragment, registry: &Registry) -> PyResult<Fragment> {
        unitarrow_core::expr::add(
            &core_frag(self),
            &core_frag(other),
            &registry.inner,
            unitarrow_core::Dialect::SQL,
        )
        .map(wrap)
        .map_err(err)
    }

    fn sub(&self, other: &Fragment, registry: &Registry) -> PyResult<Fragment> {
        unitarrow_core::expr::sub(
            &core_frag(self),
            &core_frag(other),
            &registry.inner,
            unitarrow_core::Dialect::SQL,
        )
        .map(wrap)
        .map_err(err)
    }

    fn __repr__(&self) -> String {
        format!("<Fragment {} :: {}>", self.sql, self.unit)
    }
}

/// Parse a §5.2 metadata value read from a column.
#[pyfunction]
fn parse_metadata(json: &str) -> PyResult<std::collections::BTreeMap<String, String>> {
    let m = Metadata::from_json(json).map_err(err)?;
    let mut out = std::collections::BTreeMap::new();
    out.insert("unit".to_string(), m.unit);
    out.insert("grammar".to_string(), m.grammar.to_string());
    if let Some(q) = m.quantity {
        out.insert("quantity".to_string(), q);
    }
    for (k, v) in m.unknown {
        out.insert(k, v);
    }
    Ok(out)
}

/// The Arrow field-metadata key a tagged column carries (§5.1).
#[pyfunction]
fn extension_name() -> &'static str {
    unitarrow_core::EXTENSION_NAME
}

#[pyfunction]
fn spec_version() -> &'static str {
    unitarrow_core::SPEC_VERSION
}

#[pymodule]
fn _unitarrow(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Registry>()?;
    m.add_class::<Conversion>()?;
    m.add_class::<Fragment>()?;
    m.add_function(wrap_pyfunction!(parse_metadata, m)?)?;
    m.add_function(wrap_pyfunction!(extension_name, m)?)?;
    m.add_function(wrap_pyfunction!(spec_version, m)?)?;
    m.add("EXTENSION_NAME", unitarrow_core::EXTENSION_NAME)?;
    m.add("SPEC_VERSION", unitarrow_core::SPEC_VERSION)?;
    m.add("GRAMMAR_VERSION", unitarrow_core::GRAMMAR_VERSION)?;
    Ok(())
}
