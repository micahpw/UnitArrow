# `type-functions` — schema-level checking

**Spec:** §8.6 (checked expression typing)
**Spec §11 category:** 5 (but see the numbering collision in
[../../README.md](../../README.md)) · **Milestone:** M4
**Status:** shaped, empty.

Spec §11: *"schema-level checking fixtures (§8.6) shared with the Provenance
Companion's plan conformance."*

§8.6 defines the *type system* only: pure functions that take input schemas —
units, dimensions, quantity kinds, temporal blocks, taint — and return either
the output schema or an error code. Plan serialization, hashing, and lowering
are the companion's (companion §4); this category tests the checker in
isolation, with no plan and no execution.

## What these fixtures must pin

Each case is (operation, input schemas) → (output schema | error code). No data,
no buffers, no engine. That is what makes them shareable with the companion's
plan conformance — the companion embeds this checker as a pluggable component
(companion §11 interface 2), so the same fixtures must pass on both sides.

- Purity: same inputs, same output, no registry mutation, no I/O.
- The full §10 code surface reachable from schema inspection alone.
- Interaction cases the individual sections do not cover on their own — a
  tainted operand *and* a dimension mismatch, an affine unit *and* an interval
  quantity kind, a `rate_of` kind *and* a missing period.

## Blocked on

- **AMB-019** — `E_INTERVAL_MISUSE`'s "or vice versa" is unenforceable. §7.3
  offers only `interval = true`; its absence is the absence of a claim, not an
  assertion of absoluteness, so a checker has no basis for the reverse
  direction. Either add an explicit `interval = false` or delete the clause.
- **AMB-025** — non-fixed-length periods.
- **AMB-022** — `pu` and `"1"` are dimensionally equal under the recommended
  resolution, so anything stronger must come from the quantity kind. Fixtures
  here decide what "stronger" means.
