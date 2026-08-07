# Registries

Shipping registries, as opposed to `conformance/registry/`, which holds a
deliberately tiny fixture the golden files resolve against and which is not
meant for use.

A registry is **data** on the tzdata model (§7): versioned, hashable, and
shippable independently of any library release. Nothing here is generated —
these are authored artifacts, reviewed as data, and every conversion factor in
a deployment flows from whichever one it loads.

| Registry | Scope |
|---|---|
| `power-systems.toml` | Electric power systems: generation, interchange, bus voltages, line flows, and the temporal and angular quantities that accompany them. Hand-written. |
| `udunits.toml` | General scientific units, **generated** from the pinned UDUNITS-2 submodule. 265 authored units resolving 961 symbols. |

## The generated one

```bash
python3 tools/import_udunits.py > registries/udunits.toml
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

## Adding one

Load-time validation is strict by design, so most mistakes surface immediately:
a factor not in lowest terms, a misspelled key, a dimension that redefines one
of the ten base dimensions, or a prefix expansion that collides with an
authored symbol. Read `docs/generated/registry.md` for the guardrails as the
implementation actually applies them, and `docs/generated/ambiguity.md` before
making a unit prefixable — that decision silently claims one spelling per
prefix.
