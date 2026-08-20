# Registries

Shipping registries, as opposed to `conformance/registry/`, which holds a
deliberately tiny fixture the golden files resolve against and is not meant for
use.

A registry is **data** on the tzdata model (§7): versioned, hashable, and
shippable independently of any library release. Every conversion factor in a
deployment flows from whichever one it loads.

| Registry | Role |
|---|---|
| `core.toml` | **The core registry** (§7.4): one per deployment, PR-gated. Generated from UDUNITS-2. 265 authored units resolving 1033 symbols. |
| `power-systems.toml` | A **domain extension**: 12 units core lacks, composed with it (§7.5). Restates nothing. |

## Core is generated

```bash
python3 tools/import_udunits.py > registries/core.toml
```

`vendor/udunits-2` is a **codegen input**, never a vendored output: the
generated TOML is committed and reviewed, because a registry is data every
conversion flows from and `git diff` is the review. UDUNITS is BSD-style, so
this is permitted with attribution — unlike UCUM, whose licence forbids
derivative works and forbids use in developing a different unit standard
(AMB-065).

Upstream changes rarely: 52 commits to `lib/*.xml` in nineteen years, 43 of them
before 2015. Regeneration is a once-in-a-year-or-two act, not a build step.

**Two deliberate departures from upstream**, both asserted by a test so a
regeneration cannot silently undo them:

- `rad` is dimensionless upstream, following SI. Here angle is a base
  dimension, because otherwise `rad/s` and `Hz` share `{time:-1}` and a 2π
  error passes every dimensional check (AMB-066).
- `pi` is stored upstream as a truncated 30-digit decimal. It is carried as an
  exact exponent instead, so 180° is π rather than 3.141592653582.

Everything the import cannot model is listed in the generated file rather than
dropped silently — logarithmic units, sign conventions like `degree_west`, and
symbols with no ASCII spelling.

## Domain registries add; they do not restate

```
compose("core@2026-08-07", "power-systems@0.2.0")
  -> 277 authored, 1033 resolvable, 0 rulings required
```

A domain registry that restated core would force a ruling on every shared
symbol, because two definitions of one symbol is a contest (§7.5). Adding only
what is missing keeps composition free.

`power-systems.toml` shrank from 37 units to 12 when core arrived. Comparing the
two caught a real error: its hand-written `therm` was `1.05505585262e8` J — the
**EC** therm, 100,000 BTU(IT) — labelled as the US therm. UDUNITS has the US
therm at `1.054804e8` J. A 0.024% error, invisible without a second source.

That is the argument for importing from a curated database rather than
hand-authoring: not coverage, but the errors you do not know you have made.
