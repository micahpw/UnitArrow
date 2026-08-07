# `companion` — Provenance Companion fixtures

**Spec:** [provenance-companion-spec.md](../../../docs/provenance-companion-spec.md)
§9 · **Milestones:** M2.5, M4, M6, M7
**Status:** shaped, empty.

A **separate specification** that applies to any Arrow artifact and requires no
unit metadata. It lives in this suite because the two specs share a checker and
a claim, not because it is part of UnitArrow. Nothing under `plans/`, `seals/`,
`closure/`, or `advisories/` may depend on unit metadata being present.

The two specs meet at exactly three optional interface points (spec §9 /
companion §11): the `unitarrow:provenance` block travels as a claim inside the
seal envelope; the companion's plan IR embeds UnitArrow's §8 type system as its
checker; and UnitArrow's registry and vocabulary artifacts are sealable and
advisable. **Nothing else in either document references the other** — keep it
that way here.

## Subdirectories

| Directory | Companion §9 | Milestone | Covers |
|---|---|---|---|
| `plans/` | 1 | M4 | IR fixtures → output schemas, canonical plan hashes, type errors, and execution results on the pinned reference engine |
| `seals/` | 2 | M2.5, M6 | fixture keys, tables, and tampered variants → verification states **and their required wordings** |
| `closure/` | 3 | M7 | DAG fixtures → reproducibility levels, including parameter-file retrievability and `produced_from` chains |
| `advisories/` | 4 | M7 | feed fixtures → match results, severity bounds, transitive flags, supersession surfacing |

## Notes that will bite

- **`seals/` asserts wording, not just state.** Companion §5.1 is normative for
  UX: an error surface that says "signature verification failed" instead of
  "this table was modified after it was published" is non-conformant. The
  fixtures carry the required wordings.
- **`verify()` returns three states, never a boolean** (companion §5.4), plus
  *unsealed* as a fourth neutral display state. M2.5 calls this non-negotiable
  even in minimal form, because boolean-to-three-state is a breaking behavioral
  change later.
- **Minimal seals record `mode: "untracked"` explicitly** — never an absent
  field — so M2.5-era artifacts stay unambiguous once modes exist.
- **Advisory severity is bounded by matcher precision** (companion §7,
  normative): engine-only matchers MUST NOT carry `invalid` severity.
- **`column_digests` never upgrade a broken seal** (companion §5.4). The
  signature is all-or-nothing; digests only localize. A fixture must assert
  that a broken seal with all-intact column digests still reports *broken*.
- **The republish disposition flow** (companion §5.2). A changed column
  carrying an unmodified inherited description warns in permissive and refuses
  to seal in strict. Fixtures need an ancestor seal, a modified table, and both
  mode outcomes — plus the case that motivates the design: a unit conversion
  that changes every byte while the description stays true.
- **Column digests exclude field metadata; the data digest includes it.** Both
  are deliberate and they pull opposite ways — see AMB-034.
- **`plans/` fixtures are shared with `../type-functions/`.** The companion
  embeds UnitArrow's checker; the same schema-level cases must pass on both
  sides.

## Blocked on

Companion §10 tracks its own open questions, including this specification's
name. From [AMBIGUITIES.md](../../AMBIGUITIES.md):

- **AMB-035** — `column_digests` are "canonically serialized" with no
  canonicalization defined. Sliced arrays, dictionary encoding, absent-vs-
  all-true validity buffers, and padding all make the digest unstable, which
  breaks both the what-changed report and the republish disposition flow that
  key off it. **`seals/` cannot be written until this is decided.**
- **AMB-030** — the two new strict-mode failures (metadata size, missing
  republish disposition) have no error codes; companion §8 is unchanged.
- **AMB-029** — the size threshold has no number.
- **AMB-034** — a prose typo fix is operationally indistinguishable from a data
  correction, and the what-changed report will say nothing changed.
- **AMB-024** — unknown-key preservation silently determines whether downstream
  seals verify.
- **AMB-028** — both documents still claim "exactly three" interfaces while
  cross-referencing a fourth.
- **AMB-023** — the category numbering collision this directory works around.

Note that companion §9's conformance list is **unchanged** in 0.3.0-draft: it
gained `column_digests`, the republish flow, and the size lint without gaining
a place to test any of them. That is roadmap principle 4 ("golden files land
with the feature, never after") being violated at the spec level.
