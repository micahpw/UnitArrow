# `composition` — several registries, one effective registry

Spec **§7.5** (composing registries) and **§5.6** (the provenance block and its
embedding modes). Owned by roadmap **M2.5**, since §5.6 is wire-visible and the
seal envelope freezes there.

30 cases across six files. Unlike every other category here, these were written
*after* the feature — the register entries came first (AMB-058, AMB-059,
AMB-060), the implementation followed, and the fixtures were written last, which
is backwards from the roadmap's rule and is why AMB-061 was only found now.

| File | What it pins down |
|---|---|
| `01-merging.json` | Agreement is not a contest; the union; the single-source and empty cases |
| `02-rulings.json` | A contest must be ruled on, with a reason; stale, misdirected, and unknown-source rulings; contests in `[dimension.*]` and `[quantity.*]`, not just units |
| `03-determinism.json` | The pin. Source order must not affect it; the reason must |
| `04-emergent-collisions.json` | Failures no constituent could have alone (AMB-061) |
| `05-cross-registry.json` | Reading a table tagged against a different registry — the §5.6 boundary check (AMB-059, resolved) |
| `06-embedding.json` | `full` vs `pin`, and that a mismatched fetch is refused |

## The two cases worth reading first

**`composition.determinism.002`** shares `.001`'s expected pin with its sources
in the opposite order. If those ever diverge, source order has leaked into
registry identity, and two deployments composing the same registries would
disagree about what they built.

**`composition.emergent.001`** is a collision that exists only in the
composition: `imperial` defines `ft`, `metric` prefixes `t`, each loads
perfectly alone, and together femto+t contests foot. `.002` is how a composer
resolves it — by contributing their own source that defines no units and
declares only the ruling.

## No provisional cases remain

`05-cross-registry.json` was provisional and expected to fail while AMB-059 was
open. The entry was resolved on 2026-08-01, the boundary check implemented, and
all six cases are now normative. `provisional_cases_are_accounted_for` asserts
the suite has none left — so a provisional case reappearing is deliberate rather
than drift.

Run: `cargo test --test composition -- --nocapture`
