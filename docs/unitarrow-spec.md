# UnitArrow Specification

**Version:** 0.14.0-draft
**Status:** Draft for review
**Editor:** Micah
**Extension name:** `unitarrow.quantity.v1`

A specification for carrying physical units and quantity semantics on
Apache Arrow columns — portable across servers, browsers, and languages,
with one Rust implementation exposed to Python, TypeScript, and WASM.

---

## 1. Goals

1. **Column-level units.** A unit annotates an entire Arrow column via field
   metadata. Conversion is a metadata-level operation (a scalar rescale), never
   a per-element object dispatch.
2. **Survive the wire.** Units and quantity semantics travel inside Arrow
   IPC / Flight / Parquet with no sidecar files. A table is a
   self-contained artifact.
3. **Graceful degradation.** A consumer that does not implement this spec sees
   ordinary storage-typed columns with human-readable metadata. Nothing breaks.
4. **Dimensional soundness over canonical form.** Computation proceeds in
   whatever units the data arrived in. The system checks dimensions and inserts
   coercion factors only where dimensionality forces them. Data is never
   silently canonicalized to SI.
5. **Zero-cost drop-in.** Untagged tables flow through every API bit-identical
   and silent. Tagging is O(columns), touches no data buffers, and every
   feature (checking, quantity kinds, display, sealing) is a separate opt-in.
6. **One implementation.** All normalization, parsing, dimension algebra,
   and factor computation live in a single Rust core
   (`unitarrow-core`). Bindings are thin and never reimplement core logic.

## 2. Non-goals

Half the value of this spec is what it refuses to do:

- **No compute kernels, no plans.** Numeric execution is delegated to
  existing engines (arrow-rs kernels, polars, DuckDB, JS). This spec
  defines checking *rules* (§8) and exposes them as a type system (§8.6);
  plan serialization, attestation, and lowering belong to the Provenance
  Companion.
- **No chart library.** Display support ends at labels, formatted unit strings,
  and conversion factors. Rendering belongs to existing visualization tools.
- **No OWL / description-logic reasoning.** Vocabulary relationships are flat
  data (`parent`, `same_as`, `broader`) resolved by lookup, never by inference.
- **No cross-currency conversion.** Currency is a dimension and currencies are
  units (so `USD/MWh` parses and propagates), but exchange rates are
  time-varying data, not registry constants. Converting USD to EUR is
  explicitly the application's problem.
- **No per-element units.** One unit per column, always.
- **No per-row bases.** Per-unit (p.u.) columns are supported only with a
  column-uniform base (§5.4). Per-row-base data must be shipped as physical
  units or as an application-level companion-column pattern this library does
  not automate.
- **No publishing or trust machinery.** Seals, derivation DAGs,
  reproducibility, and advisories are the Provenance Companion's scope
  (§9).

## 3. Terminology

- **Unit** — a named scale for a dimension (`MW`, `degC`, `GBtu`).
- **Dimension** — an exponent vector over base dimensions (§6.2). Two units
  are *commensurable* iff their dimension vectors are equal.
- **Quantity kind** — a controlled-vocabulary semantic type
  (`cf:air_temperature`, `core:temperature_delta`) carrying behavior rules.
- **Registry** — the versioned data artifact defining units, dimensions,
  display forms, and quantity kinds (§7).
- **Vocabulary** — a namespaced registry fragment contributed by a domain
  (§7.4).
- **Tagged / untagged column** — a column with / without the `unitarrow.quantity.v1`
  extension.
- **Tainted** — a value or column whose unit is unknown because untagged data
  entered its lineage (§5.3, §8.2).
- **Mode** — the checking level a table claims: `strict`, `permissive`, or
  `untracked` (§8.4).

## 4. Architecture

A single cargo workspace producing four artifacts:

| Crate / package | Depends on | Role |
|---|---|---|
| `unitarrow-core` | (nothing) | Registry loading, unit-string parsing and normalization, dimension algebra, factor computation, schema-level type-checking (§8). Compiles to a small WASM module. |
| `unitarrow` | `arrow-schema`, `arrow-array` (feature-gated) | Field tagging/reading, schema extraction, provenance envelope, optional convert/check helpers over stock arrow-rs kernels. |
| `unitarrow-py` | pyo3, pyo3-arrow | Python bindings. All table I/O via the Arrow PyCapsule interface; no required dependency on pyarrow, polars, or pandas. abi3 wheels. |
| `unitarrow-wasm` | wasm-bindgen | Browser/Node bindings over `unitarrow-core` only. arrow-rs stays out of the WASM build; arrow-js owns table handling. Plan execution in the browser uses the typed-array or SQL lowerings (§8.6), never arrow-rs-in-WASM. |

Ontology constants (`onto.air_temperature`, …) are **code-generated** from
vocabulary files into plain Rust, plain Python, and plain TypeScript modules —
importable without instantiating any runtime.

**Binding rules (normative):**

1. Bindings MUST NOT reimplement any `unitarrow-core` logic.
2. Core APIs are coarse-grained: submit a whole schema, receive all units,
   labels, dimensions, and factors in one call.
3. A conformance suite (§11) validates every binding against golden files.

---

## 5. Wire format

### 5.1 Extension type

A tagged column is an Arrow extension type over a storage type, serialized per
the Arrow spec as two field-metadata keys:

```
ARROW:extension:name      = "unitarrow.quantity.v1"
ARROW:extension:metadata  = <UTF-8 JSON, §5.2>
```

**Allowed storage types:** `float32`, `float64`, `decimal128/256`, and integer
types (`int8..int64`, `uint8..uint64`).

**Cast rule (normative):** converting a column whose storage is an integer or
decimal type MUST either (a) promote storage to `float64`, or (b) fail with
`E_CAST_LOSSY` if the caller requested storage preservation. Silent lossy
integer conversion is forbidden. Null validity is orthogonal to units and is
preserved untouched by all operations in this spec.

**Nested types (reserved, normative):**

- A unit on a `struct` field annotates the struct as a measured quantity. The
  `struct{value, stderr}` layout is *reserved* for future uncertainty support;
  v1 implementations MUST pass such tags through unmodified and MUST NOT
  attach conversion semantics to struct children.
- A unit on a `list` field's *child* annotates every element of every list.
  A unit on the list field itself is invalid (`E_BAD_PLACEMENT`).

### 5.2 Extension metadata JSON

```json
{
  "unit": "MW",
  "grammar": 1,
  "quantity": "core:power_generation",
  "temporal": { "kind": "interval", "statistic": "mean", "period": "PT1H" },
  "base": { "quantity": "core:apparent_power", "value": 100, "unit": "MVA" }
}
```

| Key | Required | Meaning |
|---|---|---|
| `unit` | yes | Canonical unit string (§6). `"1"` denotes explicit dimensionless. |
| `grammar` | yes | Unit-string grammar version (integer). |
| `quantity` | no | Namespaced quantity-kind identifier (CURIE). |
| `base` | no | Column-uniform base for per-unit quantities (§5.4). |
| `temporal` | no | Temporal semantics of each row (§8.5): `kind` = `instant` \| `interval`; for intervals, `statistic` (`mean`/`sum`/`min`/`max`) and ISO-8601 `period`. |

Unknown keys MUST be preserved on round-trip and ignored otherwise.

### 5.3 Untagged columns and `unknown`

The absence of the extension is the *untracked* state — it is legal, silent by
default, and contagious under arithmetic (§8.2). `unknown` is never written to
the wire; it exists only as the runtime taint state. Note the deliberate
distinction: **dimensionless is an explicit claim** (`"unit": "1"`) and is
quiet; **untagged is the absence of a claim** and is loud inside opted-in
checking contexts. Conflating them is forbidden — a checker MUST NOT suggest
tagging as dimensionless to silence a warning.

### 5.4 Per-unit (p.u.) quantities

`"unit": "pu"` is valid only when `base` is present. Conversion to physical
units multiplies by the base value; the result carries the base's unit.
Because the base is column-uniform, columns whose base varies per row (e.g.
voltage p.u. across voltage levels) are out of scope (§2) and MUST be shipped
as physical units.

### 5.5 Display name

A human-facing column name travels as a *plain* field-metadata key, outside
the extension blob, because it is producer knowledge, not registry knowledge:

```
unitarrow:display_name = "Ambient Temperature"
```

Machine identity is the Arrow field name itself. Clients SHOULD build axis
labels as `display_name (unit-display-form)`, falling back to a prettified
field name. Localization is a client concern, keyed on `quantity` or field
name — never on `display_name` string matching.

### 5.6 Schema-level provenance block

Table-level claims live in schema metadata under `unitarrow:provenance`
(JSON): `mode`, `library_version`, `registry` (version + content hash),
and `published_at`. Merging tables merges provenance downward: the
result's mode is the *weakest* of the inputs (`strict` > `permissive` >
`untracked`). Sealing this block — signing it and binding it to the data
— is the Provenance Companion's job (§9); a seal never survives any
transformation.

---

## 6. Unit strings, grammar, and canonical form

### 6.1 Grammar (v1)

```
unit-string  = term *( ("*" / "/") term ) | "1"
term         = symbol [ "^" integer ]
symbol       = registry-resolved identifier (case-sensitive)
```

- No parentheses; division binds left-to-right (`a/b/c` = `a·b⁻¹·c⁻¹`).
- No numeric prefixes or scale factors inside strings (`0.5*MW` is invalid);
  prefixed units (`kW`, `GBtu`) are distinct registry entries.
- Rational exponents are **excluded from grammar v1** (open question §12).

### 6.2 Dimensions

A dimension is a vector of signed 8-bit integer exponents over the **base
dimensions**: the SI seven (`length`, `mass`, `time`, `current`,
`temperature`, `amount`, `luminosity`) plus `count` and `currency`.
Everything else is a **named derived dimension** — a registry alias for a
specific vector. Energy is not a base dimension; it is
`{mass: 1, length: 2, time: -2}`, and power is the same with `time: -3`.
Unit definitions reference dimensions by name for readability, but
commensurability is always decided on the underlying vectors, so two units
defined against different dimension names still compare equal if their
vectors are equal. Multiplication adds vectors, division subtracts,
addition/comparison requires equality.

### 6.3 Canonical form (normative)

Cross-language equality and hashing require one spelling per unit expression:

1. Resolve every symbol against the registry (aliases → canonical symbol).
2. Merge repeated symbols by summing exponents; drop zero exponents.
3. Serialize as a single product: negative exponents rendered with `/`,
   positive-exponent terms sorted lexicographically by symbol, then
   negative-exponent terms sorted lexicographically after all `/` terms;
   exponent `1` is omitted.
4. The empty product serializes as `"1"`.

Writers MUST emit canonical form. Readers MUST accept non-canonical input in
`permissive` mode (normalizing internally, tainting nothing) and MUST reject
unresolvable symbols in `strict` mode (`E_UNKNOWN_UNIT`).

### 6.4 Affine units

Units MAY define an offset (e.g. `degC`, `degF`). Affine units:

- convert as `y = (x + offset_from) * factor - offset_to` between each other;
- are **non-composable**: any compound expression containing an affine unit is
  invalid (`E_AFFINE_COMPOUND`);
- pair with a *delta* counterpart (`delta_degC`) that is purely
  multiplicative and freely composable.

A column whose quantity kind declares `interval: true` (§7.3) MUST use the
delta unit; checkers route conversions accordingly. This is the guard against
the classic bug where a 20 °C daily swing converts to 68 °F instead of 36 °F.

---

## 7. Registry format

The registry is data — versioned, hashable, and shippable independently of any
library release (the tzdata model). Format: TOML.

### 7.1 Header

```toml
[registry]
schema_version = 1
version = "2026.07"
# content hash is computed over the canonicalized file, not stored in it
```

### 7.2 Units

```toml
[dimension.energy]
vector = { mass = 1, length = 2, time = -2 }

[dimension.power]
vector = { mass = 1, length = 2, time = -3 }

[unit.MW]
dimension = "power"
factor = [1000000, 1]          # exact rational, to the dimension's base unit
display = { unicode = "MW", long = "megawatt", plural = "megawatts" }

[unit.degC]
dimension = "temperature"
factor = [1, 1]
offset = [27315, 100]          # exact rational offset to kelvin
delta = "delta_degC"
display = { unicode = "°C", latex = "^{\\circ}C", ascii = "degC",
            long = "degree Celsius" }

[unit.household_yr]
dimension = "energy"
factor = [812394005, 10]       # example — derived from 77 MMBtu (IT)
provenance = { source = "EIA RECS 2020", scope = "US average",
               note = "site energy, electricity + gas" }
variants = ["household_yr@CO", "household_yr@TX"]
```

Conversion factors and offsets are **exact rationals** (`[numerator,
denominator]`), computed to floats only at the final step — this is what makes
factors auditable and cross-language identical.

### 7.3 Quantity kinds

```toml
[quantity."cf:air_temperature"]
parent = "core:temperature"
display = { long = "Air temperature" }

[quantity."core:temperature_delta"]
parent = "core:temperature"
interval = true

[quantity."core:energy_generation"]
parent = "core:energy"
summable_with = ["core:energy_generation"]   # not with consumption

[quantity."core:energy_consumption"]
parent = "core:energy"
opposes = "core:energy_generation"
balance = "core:net_energy"
```

`opposes`/`balance` make balance calculations first-class rather than a
warning to click through: **subtraction** across opposing kinds (or addition
under an explicit sign convention) is legal and yields the declared balance
kind, while an unsigned `sum` mixing them remains `E_KIND_UNSUMMABLE`. The
guard and the balance are the same rule seen from two sides — generation
− consumption is a statement of intent the type system can verify;
generation + consumption is almost always a bug.

Flat fields only: `parent`, `interval`, `summable_with`, `opposes`,
`balance`, `rate_of`, `same_as`, `broader`, `display`, `provenance`. Resolution is lookup; there is no
inference engine.

### 7.4 Vocabulary federation

The registry format is federated from day one:

- Every vocabulary file declares a **namespace prefix**; every term is
  namespace-qualified (`core:power`, `cf:air_temperature`,
  `hydro:streamflow`). A deployment's effective ontology is the union of the
  vocabulary artifacts it loads; prefix collisions are a load-time error.
- **Only two things are PR-gated** in the main repository: this registry
  *schema* and the `core:` namespace (units, dimensions, and genuinely
  cross-domain quantities). Domain vocabularies live in their own
  repositories, on their own release cadence, reviewed by their own experts.
- Each vocabulary is itself a **versioned, hashable, optionally sealed
  artifact** — the Provenance Companion's machinery applies to
  vocabularies exactly as to tables (§9), so a consumer can see that
  `hydro:` terms came from `hydro@2.1` signed by a known organization.
- Convergence is encouraged, not enforced: `same_as` / `broader` record
  equivalences across vocabularies, and the vocabulary-validation CI SHOULD
  fuzzy-match new terms against known vocabularies and suggest existing
  candidates at authoring time. Proven domain terms may graduate into `core:`
  with a `same_as` back-reference.
- The `cf:` namespace is generated from the published CF standard-name table
  rather than hand-minted; domain vocabularies extend rather than compete
  with it.

---

## 8. Semantics

### 8.1 Propagation

| Operation | Rule |
|---|---|
| `a * b`, `a / b` | Dimensions add/subtract; result unit is the canonicalized symbolic product of input units. |
| `a + b`, `a - b`, comparisons | Dimensions must match (`E_DIM_MISMATCH` otherwise). If units differ, the **right operand is coerced to the left operand's unit** before the stock kernel runs; result carries the left unit. |
| scalar literals | Dimensionless unless explicitly tagged by the caller. |
| aggregation (sum, mean, min…) | Result carries the input unit. `sum` additionally requires the quantity kind's `summable_with` check when kinds are present. |

### 8.2 Taint

`unknown` (untagged) is contagious: any operation with an `unknown` operand
yields `unknown`. Warnings fire **once per taint introduction per lineage**
and once at export — never per element or per kernel call — and only inside
opted-in contexts (checked arithmetic, finalization-time checks). Loading,
reading, and passing through untagged tables is silent everywhere.

### 8.3 Concat and join

Concatenating or unioning columns with the same name:

- identical canonical units → result keeps the unit;
- commensurable but different units → `strict`: error `E_UNIT_MISMATCH`;
  `permissive`: coerce the *second and subsequent* inputs to the first
  input's unit, with one warning;
- incommensurable → `E_DIM_MISMATCH` in all modes;
- tagged + untagged → result is untagged (tainted), one warning.

Join keys are compared by canonical unit; joining on commensurable-but-
different-unit keys is an error in all modes (silent coercion of key columns
invites wrong joins).

### 8.4 Table modes

`strict` fails on any taint, unknown symbol, or mismatch; `permissive` warns
and proceeds; `untracked` is silence. Mode is a property recorded at
finalization (companion `publish()`) — working tables have no mode. Merging provenance takes the
weakest mode (§5.6). Per-column taint state survives inside any table so a
strict table cannot launder tainted columns.

### 8.5 Temporal semantics

Dimensional analysis alone cannot distinguish an instantaneous power reading
from an hourly average, yet summing the two — or summing either across rows
as if power were energy — is a category error dimensions never catch. The
optional `temporal` block (§5.2) closes this, following the precedent of CF
`cell_methods`:

- `kind: "instant"` — each row is a point-in-time sample.
- `kind: "interval"` — each row summarizes a period (`statistic` + `period`).

**Rules (normative when `temporal` is present on any operand):**

1. Rate quantities (a kind may declare `rate_of = "<kind>"`, e.g. power is
   the rate of energy) MUST NOT be summed across rows into their integral
   kind. The correct operation is `integrate`: value × period, valid only on
   `interval` columns with a known `period`; the result's dimension gains
   `time: +1`, its temporal block becomes `{kind: "interval",
   statistic: "sum", period}`, and its kind follows `rate_of`. The inverse
   `differentiate` divides an interval sum by its period.
2. Element-wise arithmetic and comparisons across operands with differing
   `kind`, `statistic`, or `period` raise `E_TEMPORAL_MISMATCH`
   (`permissive`: warn once, proceed on values as-is).
3. Aggregations across rows of an `instant` column produce statistics
   (mean/max), never integrals; `sum` over an `instant` rate column is
   `E_TEMPORAL_MISMATCH` in all modes — there is no period to integrate over.
4. Columns without `temporal` participate unchecked (absence of a claim, as
   with units); checkers MUST NOT invent a temporal kind.

MW is therefore "always instantaneous" in the useful sense: whether a point
sample or an interval mean, it is a rate, and the only path from MW rows to
MWh is an explicit, period-aware `integrate` — never `sum`.

### 8.6 Checked expression typing (interface)

This specification defines the *type system* for computations — the pure
functions that, given input schemas (units, dimensions, quantity kinds,
temporal blocks, taint), decide whether an operation is legal and what
the output schema is. It deliberately does not define plan
serialization, hashing, lowering, or derivation records: those belong to
the **Provenance Companion Specification**, which embeds this type
system as a pluggable checker (companion §11). Implementations expose
the §8 rules as schema-level functions so any plan or query layer can
consume them.

---

## 9. Relationship to the Provenance Companion

Sealed publication, derivation DAGs, reproducibility closure, and
advisories are specified separately in the Provenance Companion
Specification, which applies to any Arrow artifact and requires no unit
metadata. The two specifications meet at exactly three optional
interface points (companion §11):

1. the `unitarrow:provenance` block (§5.6) travels as a claim inside the
   companion's seal envelope;
2. the companion's plan IR embeds this specification's §8 type system as
   its checker;
3. this specification's registry and vocabulary artifacts are sealable
   and advisable under the companion's machinery (§7.4).

Nothing else in either document references the other.

---

## 10. Error taxonomy

Identical codes across Rust, Python, TS/WASM; bindings map to native error
types but preserve the code.

| Code | Raised when |
|---|---|
| `E_UNKNOWN_UNIT` | Symbol does not resolve against the loaded registry (strict) |
| `E_DIM_MISMATCH` | Add/compare/concat across incommensurable dimensions |
| `E_UNIT_MISMATCH` | Strict-mode concat/join across commensurable but different units |
| `E_AFFINE_COMPOUND` | Affine unit inside a compound expression |
| `E_INTERVAL_MISUSE` | Absolute unit on an `interval: true` quantity kind (or vice versa) |
| `E_BASE_MISSING` / `E_BASE_MISMATCH` | `pu` without base; p.u. arithmetic across different bases |
| `E_CAST_LOSSY` | Conversion requires lossy storage change and promotion was refused |
| `E_BAD_PLACEMENT` | Unit on an invalid nesting position (§5.1) |
| `E_KIND_UNSUMMABLE` | Aggregation across quantity kinds not `summable_with` |
| `E_TEMPORAL_MISMATCH` | §8.5 violations: summing a rate over rows, mixing instant/interval, differing periods |
| `E_GRAMMAR_VERSION` | Wire `grammar` newer than implementation supports |

Warnings mirror the codes (`W_TAINT_INTRODUCED`, `W_COERCED`, …).

## 11. Conformance suite

A spec artifact, not an afterthought. Golden files cover:

1. **Parsing/normalization** — unit string → canonical form + dimension
   vector (valid and invalid cases with expected error codes).
2. **Factors** — (from, to) → exact rational + f64, including affine and
   delta pairs and p.u. bases.
3. **Propagation** — expression trees → result unit / error code, including
   taint and mode behavior.
4. **Wire round-trips** — IPC fixtures that MUST re-serialize byte-stable
   metadata, including unknown-key preservation and degradation behavior.
5. **Type functions** — schema-level checking fixtures (§8.6) shared with
   the Provenance Companion's plan conformance.

Every binding runs the full suite. A new binding is conformant when it passes
without modification to the golden files.

## 12. Open questions (deferred, tracked)

Companion-scope questions (plans, seals, advisories, reproduction) are
tracked in the Provenance Companion §10.

- **Rational exponents** in grammar v2 (signal-processing units).
- **Uncertainty** — activate the reserved `struct{value, stderr}` layout;
  correlated-error semantics are explicitly out of scope even then.
- **CIM identity binding** — field-level object identities
  (`cim:` CURIEs) for cross-tool linkage; likely a sibling spec.
- **Vocabulary index governance** — the one-line-per-vocab discovery list
  (Homebrew-tap model) and the fuzzy-match linter for near-duplicate terms.
- **Localization data** — whether CLDR-derived regional unit preferences ship
  in the registry or remain a client concern.

## 13. Spec versioning

This document is versioned semver-style, independently of any implementation.
Wire-format changes bump `unitarrow.quantity.vN`; grammar changes bump the `grammar`
integer; registry schema changes bump `schema_version`. All three are
designed to coexist in one codebase so old files never strand.
