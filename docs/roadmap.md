# UnitArrow Roadmap: Milestones

**Companion to:** UnitArrow Specification 0.14.0-draft + Provenance Companion Specification 0.1.0-draft
**Status:** Draft for review

Decomposition principles, in priority order:

1. **Metadata layer before trust layer.** Everything in spec §§5–8.5 is
   buildable alone and useful alone; everything in §§8.6–9.8 gets cheaper
   once it exists. Nothing below reverses that arrow.
2. **Every milestone ends in something usable** — by Micah first (GAT,
   Meridian), by others second. No milestone exists only to enable the next.
3. **Dogfood gates are real gates.** A milestone whose exit criterion is
   "still tagging columns three weeks later" can fail, and failing it means
   ergonomics work, not proceeding.
4. **Conformance grows with every milestone.** Golden files land with the
   feature, never after.

---

## M0 — Ratify and name

**Scope:** Name RESOLVED: **UnitArrow**, identifier `unitarrow` everywhere
(register on crates.io / PyPI / npm immediately — all free as of
2026-07-28). Remaining: spec editorial pass to 1.0-rc; freeze
§6.3 canonical form and §6.2 base dimensions (the two hardest-to-change
decisions); scaffold the conformance repo layout.

**Deliverables:** spec 1.0-rc; registered names; empty-but-shaped
conformance suite.

**Exit:** canonical-form rules reviewed against 20 hand-written unit
strings with no ambiguity found.

## M1 — `unitarrow-core`

**Scope (spec §§6, 7.1–7.2):** registry TOML loading; unit-string parser
and canonical form; dimension algebra with named derived dimensions; exact-
rational factor computation; affine/delta units; base registry (SI + common
energy units + `household_yr` as the contextual-unit exemplar).

**Deliverables:** `unitarrow-core` crate; conformance categories 1–2 (parsing/
normalization, factors) passing; `unitarrow-core` compiled to WASM under a size
budget (target: tens of KB).

**Exit:** the BTU-variant question is answered *in the registry* (canonical
symbols for IT vs thermochemical Btu with distinct factors).

## M2 — Wire and codecs

**Scope (spec §5 minus provenance):** `unitarrow.quantity.v1` extension type in
`unitarrow`; tag/read/convert; display-name key; `unitarrow-py` via
pyo3-arrow (PyCapsule in/out, abi3 wheels); `unitarrow-wasm` + arrow-js reading
path; graceful-degradation behavior.

**Deliverables:** the conversation's Python→IPC→browser demo, formalized:
tag in Python, read units in the browser, convert client-side. Conformance
category 4 (wire round-trips).

**Exit:** cross-language round trip byte-stable; a pyarrow-, polars-, and
pandas-produced table each tag and read identically through PyCapsule.

## M2.5 — Minimal seals (Provenance Companion; adoption driver)

**Scope (companion §5.1–5.4 reduced):** integrity + attribution only.
`publish(identity)` computing the data digest and Ed25519 seal;
three-state `verify()` (non-negotiable even in minimal form —
boolean-to-three-state is a breaking behavioral change later); pinned
trust-store TOML + TOFU; `unitctl keygen`. Seals record
`mode: "untracked"` explicitly — never an absent field — so early
artifacts stay unambiguous once modes exist. The envelope ships with all
optional fields (mode, registry pin, derivation, environment) defined and
unknown-field preservation enforced, so minimal verifiers degrade
gracefully on future rich seals.

**Deferred from here to M6:** `.well-known` discovery, the `Destination`
interface and transit read-back, external leaves, checking-mode claims.

**Deliverables:** sealed tables demonstrable in a plain script; badge
logic usable by any consumer (Meridian wires it in at M5/M6).

**Exit:** conformance seal fixtures (valid / unknown-publisher / tampered)
pass with the required wordings; a sealed table survives IPC round-trip
through an unaware consumer with the seal intact.

**Rationale:** trust is a second adoption gateway aimed at people who do
not yet care about units — "tamper-evident, attributable data products"
recruits them onto the same metadata plumbing. Exempt from the dogfood
gates for exactly that reason; the rest of the trust layer is not.

## M3 — GAT dogfood I + ontology codegen v0

**Scope:** GAT accepts tagged tables everywhere it accepts plain ones;
tagged inputs get axis labels, unit display forms, and per-plot conversion.
Vocabulary file schema (§7.3 fields); `core:` namespace + generated `cf:`
subset; codegen emitting plain Python/TypeScript/Rust constants.

**Deliverables:** GAT release consuming tags; `onto.*` constant packages.

**Exit (the carrot test):** after three weeks of normal GAT use, columns
are still being tagged voluntarily. Failure routes back to ergonomics, not
forward.

## M4 — Semantics and the plan IR (companion)

**Scope (spec §8):** propagation, taint, concat/join, table modes; temporal
semantics with `integrate`/`differentiate` and `rate_of`; kind algebra
(`opposes`/`balance`); plan IR definition + canonical hashing (Provenance Companion §4; typing supplied by `unitarrow-core`); eager
arrow-rs reference lowering; typed-array lowering instructions for the
browser.

**Deliverables:** checked arithmetic in `unitarrow-py` and `unitarrow-wasm`;
conformance categories 3 and 6 (propagation, plans).

**Exit:** plan hashes stable across Rust/Python/WASM for the full golden
set; the MW→MWh path works only via `integrate` in all bindings.

## M5 — Meridian integration

**Scope:** Meridian consumes tagged tables end-to-end: per-chart unit
toggles, locale-aware defaults (CLDR where applicable), display names,
SEDS panels in BTU/MWh/household units; extension contract updated so
Meridian extensions ship datasets + vocabulary files.

**Deliverables:** Meridian release with unit-aware charts; one internal
extension migrated to the contract as the reference example.

**Exit:** an extension author who is not Micah tags a dataset using only
the docs and the `onto.*` constants.

## M6 — Provenance Companion I: full seals

**Scope (companion §5 completing M2.5):** checking-mode claims in seals
(now that M4 semantics exist); `.well-known` discovery; `Destination`
interface with filesystem and S3-compatible implementations (SeaweedFS as
the first real deployment); transit read-back verification; external
leaves with `produced_from` for the GAT solver chain.

**Deliverables:** sealed GAT publish flow
(LP digest → solver params → parsed table → seal); Meridian badge UI
(verified / signed-unknown / broken / unsealed).

**Exit:** conformance category 5 (seals, tampered fixtures, required
wordings); a paper-walkthrough of one SEDS→Meridian and one GAT flow with
every field populated and none resented.

## M7 — Provenance Companion II: DAGs, closure, advisories

**Scope (companion §4 derivations, §6–7):** derivation sealing with
artifact + external inputs; closure computation and the reproducibility
ladder (reproducible / tol-reproducible / checkable / attested-only);
equivalence declarations; advisory feed format, matching (engine / ops /
tool / plan-hash / digest), precision-bounds-severity rule; self-retraction;
transitive DAG flagging.

**Deliverables:** static signed advisory feed in the project repo; verify()
reporting reproducibility level and advisory status; an advisory fire drill
(publish, advise, observe downstream flags, republish clean).

**Exit:** a GAT-published artifact reaches `tol-reproducible` with a
declared objective tolerance, verified by re-running HiGHS.

## M8 — Companion ecosystem (parallel, unordered)

Explicitly outside the mainline; each starts when demand appears:

- **Environment generator** — emit OCI/Nix environments from closed
  derivations; seed of the reproduction-attestation bot.
- **Reproduction attestations** — the affirmative advisory feed.
- **polars and SQL lowerings** — LazyFrame backend; DuckDB (incl.
  duckdb-wasm) backend; cross-lowering plan-hash question.
- **Vocabulary index** — the one-line-per-vocab discovery list + fuzzy-match
  linter.
- **Status index / resolver** — per-project indexer over Destinations +
  feeds answering status(digest) in one signed lookup; Meridian's backend
  as first host, badge as first consumer.
- **Ibis revisit** — the shadow-schema layer over Ibis expressions, now that
  the IR exists to power it.
- **Agent surface** — plan-generation API + docs for agent frameworks;
  Meridian MCP integration.

---

## Cross-cutting rules

- **Versioning:** `unitarrow-core` semver-stable from M1; `unitarrow`/`unitarrow-py`
  track arrow-rs majors behind minor bumps; the spec version and registry
  version move independently throughout.
- **Risk watch:** arrow-rs release churn (confined to M2+ crates); polars
  extension-API instability (does not block anything before M8); scope
  creep (the spec's non-goals section is the tie-breaker in every review).
- **Failure routing:** M3 and M5 are the two gates that can send work
  backward. The minimal seal (M2.5) is exempt from the dogfood gates — it
  is itself an adoption play. The rest of the trust layer (M6–M7:
  derivations, closure, advisories) never starts while a dogfood gate is
  failing — that machinery on top of un-adopted tagging is infrastructure
  for nobody.
