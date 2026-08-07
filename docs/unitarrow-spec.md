# UnitArrow Specification

**Version:** 0.21.0-draft
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
- **Registry** — the versioned data artifact defining *measurement itself*:
  dimensions and units — vectors, factors, offsets, deltas, aliases, display
  forms (§7). Exactly one is *in force* per deployment, and the `core` registry
  is PR-gated. It answers: *what is a `MW`, and what is it worth in `W`?*
- **Effective registry** — the single registry a deployment actually resolves
  against. Usually an authored registry as-is; where a deployment needs units
  from several, the artifact produced by composing them under explicit rulings
  (§7.5). Downstream, nothing distinguishes the two cases: resolution is always
  one lookup in one artifact.
- **Ruling** — a composer's decision about a symbol two source registries define
  differently, naming the winning source and stating why (§7.5). The
  registry-level analogue is `prefix_collisions` (§7.1).
- **Vocabulary** — a versioned data artifact defining *domain meaning*:
  namespace-qualified quantity kinds and their behavior rules (§7.4). Many
  are loaded per deployment, each on its own release cadence. It answers:
  *what does this column mean, and what may it be combined with?* A
  vocabulary MUST NOT define units or dimensions (§7.4).
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
decimal type is governed by a caller-selected storage policy:

- `promote` (the default) — the result storage is `float64`;
- `preserve` — storage is kept; the conversion succeeds only when it is exact
  and overflow-free (an integer rescale by a whole factor, a decimal rescale
  by a power of ten), and fails with `E_CAST_LOSSY` otherwise;
- `preserve` with a declared tolerance — storage is kept; the conversion
  succeeds when every value's rounding error is within the tolerance,
  emitting `W_CAST_LOSSY`, and fails with `E_CAST_LOSSY` otherwise.
  Tolerance semantics are an open question (§12).

Silent lossy conversion is forbidden under every policy. Null validity is
orthogonal to units and is preserved untouched by all operations in this spec.

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

The `unit` value — and every other unit-valued field, such as `base.unit` —
MUST be written without JSON escape sequences: a `\` anywhere in the raw
token is `E_BAD_METADATA`, detected at the metadata layer before the unit
grammar applies. Every character permitted in a unit string (§6.1) is
representable literally in JSON, so this constrains encoding, never
expression. Canonical form (§6.3) fixes one spelling per unit expression;
this rule fixes one byte sequence per spelling, which is what data digests
and cross-language hashing actually require.

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

### 5.5 Display name and description

Human-facing documentation travels as *plain* field-metadata keys, outside
the extension blob, because it is producer knowledge, not registry
knowledge:

```
unitarrow:display_name = "Ambient Temperature"
unitarrow:description  = "2m air temperature at the site met station; QC'd per ..."
```

A schema-level `unitarrow:description` documents the table as a whole.
Together these make a tagged table self-documenting — the data dictionary
rides inside the artifact instead of in a sidecar document. Descriptions
are free prose for humans and MUST NOT be used for validation; they are
paragraphs, not documents — longer material belongs behind a link.
Transformations carry a column's description only when the column passes
through unchanged; derived columns (converted, integrated, renamed) get no
description unless one is explicitly authored — honest emptiness over
stale prose. For sealed tables the Provenance Companion makes staleness
*detectable*: any data change breaks the seal (and its optional
column-level digests identify which columns' descriptions no longer
describe their data), and its republish flow requires a disposition for
descriptions inherited onto changed columns.

Schema-level `unitarrow:references` holds external documentation the
file itself should not carry — a methodology PDF, a companion dataset —
as a list of `{url, description, digest?}` entries. The optional digest
makes the referenced document tamper-evident (a swapped PDF no longer
matches); it cannot cure link rot, so durable identifiers (DOIs,
archived URLs) are RECOMMENDED. References are for material that
genuinely cannot live inline, not an escape hatch from the size norm.

Machine identity is the Arrow field name itself. Clients SHOULD build axis
labels as `display_name (unit-display-form)`, falling back to a prettified
field name. Localization is a client concern, keyed on `quantity` or field
name — never on `display_name` string matching.

### 5.6 Schema-level provenance block

Table-level claims live in schema metadata under `unitarrow:provenance`
(JSON):

| Key | Meaning |
|---|---|
| `mode` | the checking level claimed (§8.4) |
| `library_version` | the implementation that made the claim |
| `registry` | `{name, version, hash}` of the **effective registry** (§7.5), optionally plus `source` |
| `composed_from` | present iff the effective registry was composed: a list of `{name, version, hash}`, one per constituent |
| `vocabularies` | a list of `{name, version, hash}` covering every vocabulary loaded when the claim was made |
| `published_at` | RFC 3339 timestamp |

`vocabularies` is a list because a checked-at-a-stated-registry claim is not
reproducible without every artifact it ran against. `composed_from` exists for
exactly the same reason and MUST be present whenever the effective registry was
composed: a consumer holding the constituents can recompose, hash the result,
and compare it to `registry.hash` — so a reconciliation is a *computation the
consumer can repeat*, not a claim they must accept.

`registry` names the effective registry, singular, even when `composed_from`
lists several. Resolution is therefore always one lookup in one artifact, and
canonical form (§6.3) remains an equality primitive within a table.

#### Embedding modes

`registry.source` carries the effective registry's bytes. Publishers choose
whether to include it:

| Mode | `registry.source` | Resolves with no network | Survives the constituents disappearing |
|---|---|---|---|
| **full** | present | yes | **yes** |
| **pin** | absent | no | no |

The two modes trade **availability, not integrity.** Both carry
`registry.hash`, so an implementation MUST verify a fetched registry against it
and MUST refuse to resolve against a mismatch. Neither mode can silently
resolve against the wrong registry; only **full** can still resolve when the
right one is no longer retrievable. A bare URL provides neither guarantee and
MUST NOT be used in place of the hash.

Choose by deployment shape. A published dataset with a long shelf life SHOULD
use **full**: the cost is kilobytes against a file that is typically many
megabytes, and link rot is the expected failure for archived data rather than an
exotic one. A pipeline emitting very many small files MAY use **pin**, where
per-file duplication dominates. Implementations MUST NOT assume the duplication
compresses away: identical blocks in sibling files are invisible to per-entry
compression (a ZIP member, a Parquet metadata block) regardless of window size,
and deduplicate only in a solid stream whose compression window exceeds the
distance between copies.

#### Merging

Merging tables merges provenance downward: the result's mode is the *weakest*
of the inputs (`strict` > `permissive` > `untracked`), and `composed_from` and
`vocabularies` merge as unions keyed on `(name, version, hash)`.

#### Reading across a registry boundary

Canonical form (§6.3) is **registry-relative**: `bbl` canonicalizes to `"bbl"`
in every registry defining it, whatever factor it carries. Because `registry` is
pinned per table, two tables can carry equal canonical forms that denote
different quantities. An implementation MUST NOT rely on canonical-form equality
across differing pins without the check below.

When a table's `registry.hash` differs from the reader's effective registry:

1. Take the **distinct unit symbols the incoming schema uses** — not every
   symbol either registry defines.
2. Resolve each in both registries, following aliases and derived prefixed forms
   (§7.2).
3. They agree only if **dimension, factor, and offset** all match. Offset is
   part of the test: an affine scale whose zero moved has an unchanged factor
   and is wrong at every value.

Symbols compared per pin comparison — never per row or per element.

| Outcome | `strict` | `permissive` |
|---|---|---|
| pins equal | proceed, nothing checked | proceed |
| every symbol agrees | proceed | proceed |
| a symbol's dimension differs | `E_DIM_MISMATCH` | taint, one warning |
| a symbol's factor or offset differs | `E_UNIT_MISMATCH` | taint, one warning |
| a symbol does not resolve locally | `E_UNKNOWN_UNIT` | taint, one warning |
| the source registry is unobtainable | `E_REGISTRY_INVALID` | taint, one warning |

Symbols rather than pins, deliberately: comparing pins would reject every
registry version bump, and a bump that does not touch the units in play is the
common case. The symbol check is exact where it matters and silent where it is
not.

The last row is the cost of §5.6's **pin** embedding mode. An implementation
MUST NOT treat an unobtainable source registry as agreement — *could not check*
and *checked and agreed* are different claims, and conflating them reintroduces
precisely the silence this rule removes.

A diagnostic SHOULD report the ratio between the two readings, not merely that
they differ: the actionable form is *values differ by 33%*.

Sealing this block — signing it and binding it to the data — is the Provenance
Companion's job (§9); a seal never survives any transformation.

---

## 6. Unit strings, grammar, and canonical form

### 6.1 Grammar (v1)

```
unit-string  = product / inverse / "1"
product      = term *( ( "*" / "/" ) term )
inverse      = "1" 1*( "/" term )
term         = symbol [ "^" integer ]
symbol       = ALPHA *( ALPHA / DIGIT / "_" / "@" )
integer      = [ "-" ] ( "0" / ( NZDIGIT *DIGIT ) )
NZDIGIT      = %x31-39
```

- Symbols are ASCII and case-sensitive (`mW` ≠ `MW`). Display forms (`°C`)
  are output-only and are never parsed; a character outside the class is
  `E_UNIT_SYNTAX`.
- No whitespace anywhere, including leading and trailing — rejected, never
  trimmed. The empty string is invalid and is **not** equivalent to `"1"`.
- No parentheses; division binds left-to-right (`a/b/c` = `a·b⁻¹·c⁻¹`). `/`
  is input notation only: canonical form renders signs inside exponents
  (§6.3).
- A negative exponent after `/` (`MW/s^-1`) is invalid — a double negative
  that almost always spells a typo.
- The literal `1` is legal only as the whole string or as the head of an
  `inverse` (`1/s`, `1/K/m^2`); `1*MW` is invalid.
- Exponents are written without `+`, without leading zeros, and without
  `-0`; `^0` and `^1` are legal input and normalize away (§6.3).
- No numeric prefixes or scale factors inside strings (`0.5*MW` is invalid);
  prefixed units (`kW`, `GBtu`) are distinct registry entries.
- Rational exponents and namespaced symbols are **excluded from grammar v1**
  (open questions §12; both change term syntax and would migrate together).

### 6.2 Dimensions

A dimension is a vector of signed 8-bit integer exponents over the **ten base
dimensions**: the SI seven (`length`, `mass`, `time`, `current`,
`temperature`, `amount`, `luminosity`) plus `count`, `currency`, and `angle`.
Everything else is a **named derived dimension** — a registry alias for a
specific vector. Energy is not a base dimension; it is
`{mass: 1, length: 2, time: -2}`, and power is the same with `time: -3`.
Unit definitions reference dimensions by name for readability, but
commensurability is always decided on the underlying vectors, so two units
defined against different dimension names still compare equal if their
vectors are equal. Multiplication adds vectors, division subtracts,
addition/comparison requires equality.

#### Why `angle` is a base dimension

SI makes the radian dimensionless (`rad = m/m`). This specification does not,
for the same reason `count` and `currency` are already base dimensions here and
are not SI base quantities: the dimension vector serves engineering checking,
not SI conformance. A count is a pure number in SI too, and was separated so
that a tally cannot be added to a ratio.

With a dimensionless radian, `rad/s` and `Hz` both reduce to `{time: -1}`. They
are then commensurable, and an implementation will offer a conversion between
angular frequency and frequency at a factor of 1 — when the relationship is
`ω = 2πf`. The error is 6.28×, and no dimensional check can see it. Separating
`angle` makes the two incommensurable, so the mistake becomes
`E_DIM_MISMATCH` rather than a silent rescale.

Angles remain interconvertible with each other: `deg` and `rad` share
`{angle: 1}`. The separation is from *dimensionless*, not from one another. An
angle divided by an angle is dimensionless, exactly as any base dimension
cancels.

Implementations MUST NOT treat `angle` as an optional extension: a registry
that defines the radian as dimensionless produces different commensurability
answers, which is a different type system.

### 6.3 Canonical form (normative)

Cross-language equality and hashing require one spelling per unit expression:

1. Resolve every symbol against the registry (aliases → canonical symbol,
   §7.2).
2. Merge repeated symbols by summing exponents; drop zero exponents.
3. Serialize as a single `*`-joined product: positive-exponent terms first,
   then negative-exponent terms, each group sorted ascending by bytewise
   comparison of the UTF-8 encoding of the symbol. A term renders as
   `symbol` when its exponent is 1 and `symbol^exponent` otherwise, with the
   sign inside the exponent — canonical form contains no `/`. Examples:
   `W*K^-1*m^-2`, `MW*s^-1`, `USD*MWh^-1`; a pure inverse is `s^-1`.
4. The empty product serializes as `"1"`.

Canonicalization normalizes *spelling*, never the producer's choice of unit:
`MW*h` and `MWh` are both canonical and are not equal, though they are
commensurable and numerically identical (§1 goal 4). Two units are equal iff
their canonical strings are identical. Human-readable rendering (`W/m²·K`)
is the display layer's concern (§5.5, §7.2 display forms), never canonical
form's.

Writers MUST emit canonical form. Readers MUST accept non-canonical input in
`permissive` mode (normalizing internally, tainting nothing) and MUST reject
unresolvable symbols in `strict` mode (`E_UNKNOWN_UNIT`).

### 6.4 Affine units

Units MAY define an offset (e.g. `degC`, `degF`). Affine units:

- convert between each other through the dimension's base unit:

  ```
  base = x * factor_from + offset_from
  y    = (base - offset_to) / factor_to
  ```

  `offset` is an exact rational expressed **in the dimension's base unit**
  (kelvin, for temperature), so offsets are directly comparable across units
  of one dimension. A unit without an `offset` has offset 0 and is not
  affine. Check: 100 °C → 100·1 + 273.15 = 373.15 K →
  (373.15 − 45967/180) / (5/9) = 212 °F;
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
name = "core"          # required, [a-z0-9-]+ — the artifact's identity for pinning
version = "2026.07"
# content hash is computed over the canonicalized file, not stored in it
```

Every pin of a registry or vocabulary — the provenance block (§5.6), a
companion seal — carries `name`, `version`, and content hash together; a
version without a name identifies nothing.

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
offset = [27315, 100]          # exact rational offset, in kelvin (§6.4)
delta = "delta_degC"
aliases = ["degreeC"]          # input-only spellings; never emitted
display = { unicode = "°C", latex = "^{\\circ}C", ascii = "degC", long = "degree Celsius" }

[unit.degF]
dimension = "temperature"
factor = [5, 9]
offset = [45967, 180]          # 255.372… — in kelvin, NOT 459.67 (§6.4)
delta = "delta_degF"
display = { unicode = "°F", ascii = "degF", long = "degree Fahrenheit" }

[unit.rad]
dimension = "angle"
factor = [1, 1]
display = { unicode = "rad", long = "radian", plural = "radians" }

[unit.deg]
dimension = "angle"
factor = [1, 180]
pi = 1                         # exact: deg = (1/180)·π rad, never a decimal
display = { unicode = "°", ascii = "deg", long = "degree", plural = "degrees" }

[unit.household_yr]
dimension = "energy"
factor = [812394005, 10]       # example — derived from 77 MMBtu (IT)
provenance = { source = "EIA RECS 2020", scope = "US average", note = "site energy, electricity + gas" }
variants = ["household_yr@CO", "household_yr@TX"]
```

Conversion factors and offsets are **exact rationals** (`[numerator,
denominator]`), computed to floats only at the final step — this is what makes
factors auditable and cross-language identical.

#### The `pi` exponent

`pi` is an optional integer exponent of **π** applied to `factor`, defaulting to
`0`. It exists because a degree is `(π/180)` radians and π is irrational: no
`[numerator, denominator]` pair is the degree's factor, so without this a
registry would store a rounding and present it as a definition — the precise
failure exact rationals exist to prevent.

With it the definition is exact. `deg = (1/180)·π¹ rad` composes, inverts, and
raises to a power losslessly; 180 degrees is π radians exactly, and a
degree→radian→degree round trip is the identity rather than merely close. π
reaches a float only where every other factor does, at application.

Implementations MUST support `pi`, and MUST reject an exponent outside
`-8..=8` — a unit needing more is a modelling error, not a unit. A conversion
whose composed π exponent is non-zero has **no exact rational form**; an API
returning exact rationals MUST report that rather than round.

Only `factor` takes `pi`. `offset` does not: affine units are temperature
scales, whose intercepts are rational.

`aliases` lists additional input-only spellings that resolve to the entry's
canonical symbol (§6.3 step 1). Aliases are symbols and share their ASCII
lexical class (§6.1). Across the union of loaded artifacts, an alias MUST NOT
equal any canonical symbol or any other alias, and a canonical symbol MUST
NOT be defined twice; each violation is a load-time error
(`E_REGISTRY_INVALID`). Aliases are never emitted.

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

- Every vocabulary file declares a **namespace prefix**; every quantity kind
  is namespace-qualified (`core:power`, `cf:air_temperature`,
  `hydro:streamflow`). A deployment's effective ontology is the union of the
  vocabulary artifacts it loads; prefix collisions are a load-time error.
- **Vocabularies define quantity kinds only.** Units and dimensions are
  defined solely by the registry — one per deployment, PR-gated — because a
  unit definition carries a conversion factor every consumer silently
  trusts. A vocabulary declaring `[unit.…]` or `[dimension.…]` fails to load
  (`E_REGISTRY_INVALID`). Unit symbols are global and unnamespaced in
  grammar v1; a symbol or alias defined twice across loaded artifacts is a
  load-time error, symmetric with prefix collisions. Namespaced unit symbols
  are deferred to grammar v2 (§12).
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

### 7.5 Composing registries

§7.4 restricts unit definitions to a single registry. A deployment needing
units from two domains therefore composes them into one **effective registry**,
which is what everything downstream resolves against.

**Composition happens before computation.** It is not a description of a
computation already performed. If a pipeline computed first and recorded the
registries afterwards, an output table could carry two columns with the same
canonical form denoting different quantities — and §6.3 would cease to be an
equality primitive *within a single table*, which is the one place it must hold
unconditionally.

A composition takes a set of **sources** — registries with distinct names — and
a set of **rulings**. Loading it:

1. A symbol several sources define **identically** is not a contest. (`m` is
   `m` everywhere; requiring a ruling for every shared symbol would make
   composition unusable.)
2. A symbol two sources define **differently** MUST have a ruling naming the
   winning source and giving a **non-empty reason**. Absent one, composition
   fails (`E_REGISTRY_INVALID`) naming the symbol and every source defining it.
3. A ruling whose named source does not define the symbol is an error.
4. A ruling that settles no contest is an error, symmetric with §7.1's
   `prefix_collisions`: a stale entry would hide the next real contest.
5. Each source's `prefix_collisions` (§7.1) carry forward — they are rulings
   their authors made, and dropping them would fail the load.
6. The result MUST itself be a valid registry; composition cannot emit
   something unloadable.

The reason is required for the same purpose as `prefix_collisions`' reason, one
level up, and for a second party: the consumer of a published table has no other
place to learn *why* a contested symbol means what it does.

The effective registry carries its own provenance — the constituents under
`[registry.composed_from.<name>]`, and the rulings under
`[registry.reconciliation.<symbol>]` with `from` and `reason`:

```toml
[registry]
name = "merged"
schema_version = 1
version = "2026.07"

[registry.composed_from.oil]
hash = "sha256:f9e651ecd5a67356…"
version = "1.4"

[registry.composed_from.water]
hash = "sha256:015960d3b7d632ac…"
version = "2.1"

[registry.reconciliation.bbl]
from = "oil"
reason = "this deployment is upstream oil production, not water management"
```

Note the key order — `name` before `schema_version` before `version`, `hash`
before `version`, `from` before `reason`. That is the ascending-byte-order rule
above, not a stylistic choice: it is what makes the hash reproducible.

**Serialization MUST be deterministic.** A composed registry has no authored
file, so its identity is the hash of its generated bytes, and two conforming
implementations composing the same sources with the same rulings MUST produce
byte-identical output. Keys are emitted in ascending byte order at every level,
scalars precede sub-tables within a table, and no other ordering is permitted.

Because the rulings live *inside* the hashed artifact, changing a reason changes
the hash — so a seal over the pin (§9) covers the reasoning transitively, and a
justification cannot be rewritten without detection.

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

- identical canonical units **and the same registry pin** → result keeps the
  unit. Across differing pins the boundary check of §5.6 applies first, and its
  outcome governs;
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
| `E_UNIT_SYNTAX` | Unit string does not parse under the declared grammar (§6.1) |
| `E_UNKNOWN_UNIT` | Symbol does not resolve against the loaded registry (strict) |
| `E_DIM_MISMATCH` | Add/compare/concat across incommensurable dimensions |
| `E_UNIT_MISMATCH` | Strict-mode concat/join across commensurable but different units, including two registries that disagree about a symbol's factor or offset (§5.6) |
| `E_AFFINE_COMPOUND` | Affine unit inside a compound expression |
| `E_INTERVAL_MISUSE` | Absolute unit on an `interval: true` quantity kind (or vice versa) |
| `E_BASE_MISSING` / `E_BASE_MISMATCH` | `pu` without base; p.u. arithmetic across different bases |
| `E_CAST_LOSSY` | Conversion would lose precision under the requested storage policy (§5.1) |
| `E_BAD_PLACEMENT` | Unit on an invalid nesting position (§5.1) |
| `E_KIND_UNSUMMABLE` | Aggregation across quantity kinds not `summable_with` |
| `E_TEMPORAL_MISMATCH` | §8.5 violations: summing a rate over rows, mixing instant/interval, differing periods |
| `E_GRAMMAR_VERSION` | Wire `grammar` newer than implementation supports |
| `E_BAD_METADATA` | Extension metadata absent, not valid UTF-8 or JSON, missing a required key, or containing an escape sequence in a unit-valued field (§5.2) |
| `E_REGISTRY_INVALID` | A loaded registry or vocabulary artifact violates a load-time validation rule (§7.2, §7.4), or a composition is unresolvable — an unruled contest, a stale or misdirected ruling, a missing reason (§7.5) |

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

- **Grammar v2 term syntax** — three pending changes all alter what a `term`
  may contain, and each migration rewrites every stored unit string, so they
  should land together rather than in three bumps:
  1. **rational exponents** (`m^1/2`, signal-processing units);
  2. **namespaced unit symbols** (`hydro:acre_ft`), so a domain vocabulary can
     mint units without colliding globally;
  3. a **prefix separator** (`k:W`), which would make prefix/authored-symbol
     collisions impossible by construction instead of by load-time check —
     at the cost of a canonical form nobody recognises.

  Items 2 and 3 both want `:`. That parses without a second separator —
  namespace prefixes are lowercase (§7.4), so reserving the fifteen lowercase
  SI prefix symbols (`a c d da f h k m n p q r u y z`) makes `X:Y`
  deterministic, and none of `core`, `cf`, `hydro`, `cim` is affected. But
  parsing is a lower bar than reading: `:` already means *namespace membership*
  in quantity-kind CURIEs, and reusing it for *scale composition* gives one
  character two jobs in the same document. If both ship, they should use
  different separators — `:` for namespaces, `.` for prefixes (`k.W`).

  Whether either is still needed is the prior question. §7.4 now restricts unit
  definitions to the single PR-gated registry, which largely retires item 2's
  motivation; item 3 duplicates a guarantee §7.2's load-time collision check
  already provides, at the cost of a canonical form nobody recognises. Dropping
  item 3 leaves `:` with exactly one meaning, which is the most usable outcome
  of the three.
- **Tolerance semantics** for storage-preserving casts (§5.1): absolute vs
  relative, per-value vs aggregate, and whether a tolerance used should be
  recorded in provenance.
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
