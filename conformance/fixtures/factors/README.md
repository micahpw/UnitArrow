# `factors` — conversion factors and offsets

**Spec:** §6.4 (affine conversion), §7.2 (registry `factor` / `offset`)
**Spec §11 category:** 2 · **Milestone:** M1
**Status:** shaped, empty.

Spec §11: *"(from, to) → exact rational + f64, including affine and delta pairs
and p.u. bases."*

## What a case will look like

```json
{
  "id": "factors.linear.001",
  "spec_ref": "§7.2",
  "status": "normative",
  "input": { "from": "MW", "to": "kW" },
  "expect": {
    "outcome": "ok",
    "factor": [1000, 1],
    "f64": 1000.0
  }
}
```

The `expect` shape is not in `schema/fixture.schema.json` yet — add it when the
first fixture lands, along with `factors` cases to the schema's conditional
branches.

## What these fixtures must pin

- **Exact rationals, reduced.** The expected `factor` is `[num, den]` in lowest
  terms. A binding that returns `[1000000, 1000]` for MW→kW is non-conformant
  even though the value is right: unreduced fractions overflow under
  exponentiation and hash differently.
- **`f64` is derived, not authoritative.** It is present so bindings can check
  the final conversion-to-float step, and it is the *only* place a float
  appears. See the exact-rational rule in [CLAUDE.md](../../../CLAUDE.md).
- **Affine round-trips.** degC↔degF↔K in both directions, and the delta
  counterparts alongside — 20 delta_degC must give 36 delta_degF, not 68.
- **p.u. bases** (§5.4): `pu` → physical multiplies by the base value and the
  result carries the base's unit.
- **Composite factors.** `MW*h` → `Btu` exercises rational arithmetic across a
  compound expression, where float intermediates visibly diverge between
  bindings.

## Blocked on

- **AMB-016** — §6.4's conversion formula and §7.2's offset convention
  contradict each other, and they disagree by 200 °F on boiling water. **No
  affine factor fixture can be written until this is decided.** The fixture
  registry currently encodes the §6.4 reading; if AMB-016 resolves the other
  way, every offset in `registry/conformance-core.toml` changes together with
  the formula.
- **AMB-018** — delta units' factors are implied but never specified.
- **AMB-008 / AMB-015** — exponent range and its missing error code.

M1's own exit criterion — *"the BTU-variant question is answered in the
registry"* — lands here: IT vs thermochemical Btu need distinct canonical
symbols with distinct factors, and a fixture proving they do not compare equal.
