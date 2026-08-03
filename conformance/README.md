# UnitArrow conformance suite

Golden files. Spec §11 calls this "a spec artifact, not an afterthought," and
roadmap decomposition principle 4 says golden files land *with* the feature,
never after.

**Every binding runs the full suite. A new binding is conformant when it passes
without modification to the golden files.** Editing a fixture so an
implementation passes is backwards — the correct move is a spec change that
updates the fixture first.

## Status

**M0.** Only the `parsing` category is populated (73 cases over §6.3 canonical
form and §5.2 value encoding). Every other category is shaped and empty, with a `README.md` naming its
spec section and owning milestone. There is no runner yet — `unitarrow-core`
does not exist.

Writing these fixtures **is** M0's exit criterion: *"canonical-form rules
reviewed against 20 hand-written unit strings with no ambiguity found."* The
review did not come back clean — **46 findings** are recorded in
[AMBIGUITIES.md](AMBIGUITIES.md) — but as of 2026-07-29 **every M0 blocker is
ruled and the editorial pass is applied**: spec **0.21.0-draft** / companion
**0.5.0-draft** carry all 13 canonical-form rulings, the kelvin offset
convention, the escape prohibition, `E_UNIT_SYNTAX` / `E_BAD_METADATA` /
`E_REGISTRY_INVALID`, and the `vocabularies` seal pin. 62 of 73 cases are
`normative`; the 11 still `provisional` are blocked on M1-scope entries only
(affine compound rules, exponent-range code, `pu`, reader-mode default).

Fixtures are pinned to spec **0.21.0-draft** / companion **0.5.0-draft** —
the first revisions whose §§6.1–6.4 are the *ruled* canonical form rather
than the ambiguous 0.14.0 text. The `wire/` and `companion/` categories
remain empty, blocked on the open entries among AMB-028 … AMB-046.

## Layout

```
README.md                     this file
FIXTURE-FORMAT.md             normative fixture format
AMBIGUITIES.md                spec ambiguity register (AMB-001 … AMB-046)
DECISIONS.md                  worked resolutions + a 49-item decision checklist
schema/fixture.schema.json    JSON Schema every fixture validates against
registry/conformance-core.toml  the symbol set fixtures resolve against
fixtures/
  parsing/                    §6, populated
  factors/                    §6.4, §7.2 — M1
  propagation/                §8.1–8.5 — M4
  wire/                       §5 — M2
  type-functions/             §8.6 — M4
  companion/                  provenance-companion §9 — M2.5, M6, M7
```

Two categories are missing and neither document plans for them: **`registry/`**
(malformed third-party catalogs — AMB-043) and wide-table cases under `wire/`
(AMB-041). Both are gaps in the spec's own conformance section, not just in
this suite.

## Category numbering

Directories are named, not numbered, because the three documents number them
incompatibly (AMB-023). Use this table to resolve a roadmap or spec reference:

| Directory | Spec §11 | Roadmap | Companion §9 | Milestone |
|---|---|---|---|---|
| `parsing/` | 1 | 1 | — | M0 / M1 |
| `factors/` | 2 | 2 | — | M1 |
| `propagation/` | 3 | 3 | — | M4 |
| `wire/` | 4 | 4 | — | M2 |
| `type-functions/` | 5 | — | — | M4 |
| `companion/plans/` | — | 6 | 1 | M4 |
| `companion/seals/` | — | 5 | 2 | M2.5 / M6 |
| `companion/closure/` | — | — | 3 | M7 |
| `companion/advisories/` | — | — | 4 | M7 |

Note the collision: spec §11 assigns 5 to type functions, roadmap M6 assigns 5
to seals, and the companion restarts at 1. Renumber during the editorial pass
to 1.0-rc; until then, cite directories by name.

## The fixture registry

Fixtures resolve symbols against
[registry/conformance-core.toml](registry/conformance-core.toml) — a
deliberately tiny registry that exists only for this suite. It is **not** the
shipping `core:` registry, which lands at M1 as a separately versioned,
PR-gated artifact. Do not copy factors out of it without re-deriving them; its
affine offsets in particular encode a recommended resolution to AMB-016, not
settled spec text.

## Running the suite

There is nothing to run yet. When a runner exists it should:

1. Load every `fixtures/**/*.json`, validate against `schema/fixture.schema.json`.
2. Load the registry named in each fixture's envelope.
3. For each case, perform the category's operation on `input` and compare to
   `expect` — exact string match on `canonical`, exact code match on `code`.
4. Report by case `id`. Ids are stable forever; runners, CI, and the ambiguity
   register all cite them.
5. Support `--skip-provisional` for cases whose expected value depends on an
   unresolved `AMB-nnn`.

Until then, the fixtures are checkable structurally:

```bash
find conformance -name '*.json' -exec sh -c 'jq -e . "$1" >/dev/null || echo "BAD JSON: $1"' _ {} \;
```

```bash
jq -r '.cases[].id' conformance/fixtures/*/*.json | sort | uniq -d
```

## Adding fixtures

See [FIXTURE-FORMAT.md](FIXTURE-FORMAT.md). The short version: one assertion per
case, ids are permanent, and if you had to make a judgement call the spec does
not license, the case is `provisional` and you write the `AMBIGUITIES.md` entry
first.
