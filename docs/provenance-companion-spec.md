# Provenance Companion Specification (working name)

**Version:** 0.1.0-draft
**Status:** Draft for review
**Editor:** Micah
**Hosted with:** the UnitArrow project (`unitarrow` org); applies to any
Arrow artifact, tagged or not

Content-addressed provenance for data products: sealed publication,
derivation DAGs, reproducibility, and revocable trust — built from three
primitives (digests, signatures, static feeds) with no required services.

**The general model, stated once:** every node in a derivation DAG is
either **digest-verifiable** (deterministic under its pinned tools and
parameters) or **equivalence-verifiable** (nondeterministic, with a
declared equivalence). Domains differ only in their tools and their
equivalence metrics; nothing in this specification is domain-specific,
and nothing in it requires unit metadata.

---

## 1. Goals

1. **Claims, not cryptography.** All user-facing surfaces speak in three
   claims — who published this; it hasn't changed; how it was produced —
   never in cryptographic nouns.
2. **Self-contained artifacts.** Seals and provenance travel inside the
   artifact's own metadata. No sidecar files, ever.
3. **The storage layer is never trusted.** Integrity is end-to-end;
   Destinations are interchangeable and untrusted.
4. **Zero-cost opt-in.** Unsealed artifacts flow through everything
   silently; every tier (sealing, derivations, closure, advisories) is a
   separate opt-in.
5. **Trust attaches to artifacts and toolchains, not sources.** Recovery
   from any defect is republication.

## 2. Non-goals

- **No compute kernels.** Plans lower to engines; nothing here executes.
- **No key escrow, CA hierarchy, or web-of-trust.** Trust distribution is
  pinning, TOFU, and `.well-known` discovery (§5.5).
- **Not units-specific.** UnitArrow supplies one optional type system for
  plans and one optional claim for seals (§11); this spec functions
  without either.

## 3. Terminology

- **Seal** — the signed provenance block a publisher attaches at
  `publish()` (§5).
- **Mode** — a checking-level claim carried in the seal, defined by the
  type-system layer (for UnitArrow: `strict` / `permissive` /
  `untracked`).
- **Closure / reproducibility levels** — §6.
- **Advisory** — a signed document revoking or downgrading trust (§7).
- **Destination** — an immutable, content-addressed store (§5.2).

---

## 4. Derivations and the plan IR

UnitArrow's checking rules need a place to run, and any DAG needs a
recordable form. Rather than a compute engine, this spec defines a small **expression plan IR**: a typed, serializable
description of a sequence of simple operations — the UnitArrow §8.1 arithmetic,
conversion, `integrate`/`differentiate`, filter, project/rename, aggregate,
concat, and join. Plans are data: they canonicalize and hash like unit
strings.

- **Typing**: `unitarrow-core` type-checks a plan against input schemas — units,
  dimensions, quantity kinds, temporal blocks, taint (UnitArrow §8) — before anything
  executes, and computes the full output schema symbolically.
- **Lowering**: the reference implementation lowers plans to eager arrow-rs
  kernels. Additional lowerings share the same IR — the plan is the portable
  artifact, the engine is a backend choice:
  - **polars `LazyFrame`** (server-side, intended extension);
  - **SQL** (DuckDB, including duckdb-wasm as the browser's heavy engine
    for joins and group-bys);
  - **typed-array** (browser/JS): for elementwise arithmetic, conversion,
    `integrate`/`differentiate`, and simple aggregates, `unitarrow-core` (WASM)
    type-checks the plan and emits lowering instructions (scale factors,
    output schema) that the JS side executes over arrow-js buffers. This
    keeps the WASM bundle arrow-free while making unit-correct computation
    available to browser consumers.
- **Machine-generated plans**: the IR is the intended boundary for
  agent-generated computation. Guardrails are structural: the operation
  vocabulary is closed (no UDFs, no I/O), every plan is type-checked
  against units, kinds, temporal blocks, and taint before execution, and
  rejections return the enumerated error codes (§8) as machine-legible
  feedback. Accepted plans hash and seal like any other, so agent-derived
  artifacts carry full provenance.
- **Derivation trust**: `publish(derivation=plan)` extends the seal with
  `{plan_hash, engine: {name, version}, inputs: [...]}`. Each input is one
  of two kinds:
  - `{kind: "artifact", name, data_digest, seal?}` — an in-ecosystem
    table; sealable, so derivations chain into a verifiable DAG.
  - `{kind: "external", digest, description, media_type?, tool: {name,
    version}}` — an input from outside the ecosystem (a solver output
    file, an instrument dump, a vendor CSV), with the tool that crossed
    the boundary (solver, parser) recorded. `digest` (raw bytes, no
    canonicalization — the recorded tool is what interprets them) is
    REQUIRED whenever the input is a retrievable byte sequence; it MAY be
    omitted only for non-materializable sources (live streams, interactive
    sessions) and then MUST carry a `reason`. The publisher's seal covers
    these claims: it does not assert the external input was *correct*,
    it asserts *this is exactly what was ingested, with exactly this
    toolchain* — the trust boundary made visible and signed rather than
    hidden.

    An external leaf MAY additionally carry `produced_from`, recording the
    opaque transform that generated the bytes:

    ```json
    "produced_from": {
      "inputs": [{ "digest": "sha256:...", "description": "unit-commitment LP" }],
      "parameters": { "seed": 7, "threads": 8, "options_digest": "sha256:..." },
      "equivalence": { "metric": "objective", "rtol": 1e-6 }
    }
    ```

    Composite tools record their invoked components:
    `tool: {name, version, plugins?: [{name, version}]}` — the plugins
    *actually invoked* in this run, not those merely installed. Advisory
    `tool` matching (§7) flattens across host and plugins, so a
    defective translation plugin is advisable independently of its host's
    version. Parameters that are files (configuration files, options
    files) are recorded by digest and are inputs in disguise: they count
    toward closure (§6) exactly as inputs do.

    **Digesting policy (byte vs semantic identity):** wherever a canonical
    form exists, digests are computed over it, never over author bytes —
    unit strings, plan IR, and sealed tables are therefore immune to
    formatter churn by construction. For tool configurations, prefer
    digesting the tool's *effective* configuration (a parsed,
    canonically-serialized dump) over the config file's bytes: it is
    formatter-immune and captures defaults and overrides the file misses.
    Raw bytes remain the honest digest for materials a tool consumes raw.
    Where formatter churn does produce divergent input digests, correctness
    is unaffected — deterministic transforms yield identical output digests
    from either spelling, and content addressing collapses the chains
    downstream; the cost is cache and index noise, never a wrong
    verification verdict.

    A composite tool containing a nondeterministic stage (e.g. a
    translator that internally solves a siting LP) has two valid
    recordings. *Flattened*: one `produced_from` for the whole transform,
    which MUST then declare `equivalence` — absence asserts determinism
    for the entire composite. *Decomposed*: chained records exposing the
    intermediate (deterministic translate → nondeterministic solve →
    output), which is preferred where the tool cooperates: deterministic
    stages verify by digest, the equivalence declaration shrinks to the
    nondeterministic stage, and advisories become surgical.

    `inputs` are digested upstream materials — raw files (the LP file)
    *or in-ecosystem artifacts referenced by their `data_digest`*, which is
    what lets DAGs interleave sealed artifacts and opaque transforms
    (capacity-expansion outputs → translator → production-cost LP → solver
    → parsed outputs) into one walkable chain; `parameters`
    are whatever bounds the tool's nondeterminism (seeds, thread counts,
    options files by digest); `equivalence` declares what *same* means for
    a re-run of a nondeterministic transform. Absence of `equivalence`
    asserts the transform is deterministic under the recorded parameters.
    This is the recorded form of the chain `LP (content-addressed) →
    solver (pinned version + parameters) → output (content-addressed)`.

  An auditor can confirm that a published artifact is exactly
  `plan(inputs)` by re-executing on the pinned engine. Verification walks
  the DAG and reports external leaves distinctly — "verified to roots;
  bottoms out at N attested external inputs (tool, version)" — neither
  tainted nor silently clean. Trust is established through the sequence of
  checked operations — at the expense of restricting that sequence to the
  IR's vocabulary, and of computational speed where the reference lowering
  is used.
- **Determinism caveat (normative)**: bitwise re-execution equality is
  guaranteed only on the same engine name+version recorded in the
  derivation. Cross-engine verification uses the same declared-equivalence
  concept as nondeterministic external transforms (`equivalence`, above):
  an equivalence check, not a digest check. Cross-engine equivalence
  checking is out of scope for v1.

---

## 5. Seals: publishing and verification

### 5.1 Claims model (normative for UX)

All user-facing surfaces — APIs, badges, errors, docs — speak in three claims
and never in cryptographic nouns:

1. **Who published this** (publisher identity)
2. **It has not changed since publishing** (integrity)
3. **It was checked at a stated level against a stated registry** (mode +
   registry pin)

Ed25519, keys, and signatures appear only in this section and in an auditor
appendix. An error surface that says "signature verification failed" instead
of "this table was modified after it was published" is non-conformant.

### 5.2 `publish()`

`table.publish(identity, check="strict"|"permissive", destination=None,
derivation=None)` finalizes a data product:

1. Run the requested check across all columns (strict failure aborts).
2. Pin the registry version + content hash and library version.
3. Compute the **data digest**: SHA-256 over the canonical IPC serialization
   of (schema with all metadata *except* the seal itself, followed by record
   batches in order).
4. Assemble the provenance block (for UnitArrow tables, UnitArrow §5.6) and sign it with the publisher's
   Ed25519 key.
5. Embed the seal in schema metadata. No sidecar files, ever.
6. If a `destination` is given: transmit the sealed artifact, then read it
   back from the host (or fetch the host's stored digest) and re-verify the
   seal end-to-end. `publish()` fails with `E_TRANSIT_ALTERED` if transit
   changed anything — the success return means "sealed, delivered, and
   verified at rest."

`Destination` is an interface, implemented separately from this spec. Its
required properties are deliberately abstract: storage MUST be **immutable**
(a published artifact is never overwritten in place), SHOULD be
**content-addressed** by `data_digest`, and MUST support retrieval by digest
through an opaque locator — artifact *identity* is the digest, never a
URL or hostname. Filesystem, S3-compatible object stores, and Flight
endpoints are the expected first implementations, but any store satisfying
these properties qualifies; because seals travel in-band and integrity is
verified end-to-end, the storage layer is never trusted. A mutable
"latest" naming layer atop immutable artifacts is explicitly a separate
concern (§10). Content addressing makes the transit check a key lookup and
makes overwriting a published artifact structurally impossible.

The seal binds the claim to the bytes: it cannot be lifted onto other data,
and any post-publish edit breaks it. Any transformation of a sealed table
drops the seal; provenance (mode, registry pin) survives per the artifact's provenance-block merge rules (UnitArrow §5.6).

### 5.3 Seal contents

```json
{
  "publisher": "nlr-pipelines",
  "key_id": "SHA-256 fingerprint of public key",
  "algorithm": "ed25519",
  "mode": "strict",
  "library_version": "1.4.2",
  "registry": { "version": "2026.07", "hash": "sha256:..." },
  "published_at": "2026-07-28T17:03:00Z",
  "supersedes": "sha256:... (optional: digest of the artifact this one corrects)",
  "data_digest": "sha256:...",
  "signature": "base64(sign(canonical-JSON of all fields above))"
}
```

### 5.4 Verification: three states

`verify(table)` runs implicitly on load when a seal is present and returns a
provenance object — never a boolean:

| State | Meaning | Required user-facing wording (or equivalent) |
|---|---|---|
| **Verified** | Digest matches; signature valid; key in trust store | "Published by {publisher} · unchanged · {mode} · registry {version}" |
| **Signed, unknown publisher** | Digest matches; signature valid; key not yet trusted | "Signed by an unrecognized publisher ({publisher}). Trust them?" |
| **Seal broken** | Digest or signature mismatch | "The data doesn't match its seal — it was modified after publishing, or the seal was copied from a different table." |

Unsealed tables are simply *unsealed* — a fourth, neutral display state, not
an error.

When a verified artifact declares `supersedes`, consumers SHOULD surface
the successor when displaying the superseded artifact's status — a flagged
artifact with a clean successor is a remediation notice, not just an
alarm.

Advisory status (§7) is **orthogonal** to seal status: a table can be
fully Verified and still carry an advisory flag — the seal proves what was
published by whom; the advisory says the publisher's toolchain was later
found defective.

### 5.5 Trust distribution

Three tiers, all shippable in v1; no CA, no web-of-trust:

1. **Pinned trust store** — a readable TOML file (`known_publishers`) an org
   commits to its config repo.
2. **Trust on first use** — first sight of a publisher's key is amber with a
   one-click "trust"; a *changed* key for a known publisher is the loud event.
3. **`.well-known` discovery** — publishers MAY host keys at
   `https://{org}/.well-known/unit-publishers.json`; trust grounds in the
   org's domain.

Key generation is one command with SSH-like defaults (`unitctl keygen`);
private keys live server-side or in CI secrets and are never embedded in
distributed binaries.

### 5.6 Threat model (must appear in user docs, plain language)

Protects against: accidental modification, corrupted transfers, forged
`strict` claims, seals transplanted between tables. Does **not** protect
against: a compromised publishing machine, a stolen key before rotation,
first-use spoofing under TOFU. Losing a key never loses data — nothing is
encrypted; republish with a new key and consumers re-trust.

## 6. Reproducibility closure

A digest *names* an input; a content-addressed Destination *retrieves* it
by that name. When both hold for every node, the derivation record becomes
an executable recipe (the Nix derivation model applied to data): fetch all
inputs by digest, fetch the plan by hash, run the recorded engine, compare
the output digest to the seal — fully machine-reproducible with no human
interpretation.

The general model, stated once: every node in a derivation DAG is either
**digest-verifiable** (deterministic under its pinned tools and
parameters) or **equivalence-verifiable** (nondeterministic, with a
declared equivalence). Domains differ only in their tools and their
equivalence metrics; nothing else in this chapter is domain-specific.

A derivation is **closed** when every input in its DAG (artifact and
external) *and every digested parameter* (configuration and options
files, §4) carries a digest and is retrievable from a declared
Destination — a transform is not re-runnable without its configuration.
Verification reports one of three reproducibility levels:

| Level | Condition | Meaning |
|---|---|---|
| `reproducible` | Closed + engine pinned + no nondeterministic transforms | Bit-reproducible: anyone can mechanically re-derive and compare digests |
| `tol-reproducible` | Closed, but the DAG contains attested transforms with declared `equivalence` | Tolerance-reproducible: independent parties re-run the transforms and verify by the declared equivalence (e.g. objective within rtol), not digest equality |
| `checkable` | All digests present; some inputs not retrievable | A party who independently holds the inputs can complete verification — **detection without disclosure**: digests are safe to publish even when bytes are proprietary |
| `attested-only` | Digests absent somewhere in the DAG | Verification rests on publisher attestation alone |

Tol-reproducibility is bounded by tool accessibility: a closed chain
through a proprietary solver can be re-run only by its license holders,
while the same chain through an openly available tool can be re-run by
anyone.
Since independent reproduction is the strongest trust signal this system
emits, closures over open tools yield more verifiable artifacts by
construction — an incentive, never a mandate.

Closure is opt-in per artifact and per input: fully public chains, digest-
only proprietary chains, and mixtures interoperate in one DAG. Digest
strings are self-describing (`sha256:` prefix); algorithm agility follows
the multihash convention.

Because a closed derivation record pins tools, engines, parameters, and
input digests, it is sufficient input to *generate* a verification
environment (an OCI image or Nix expression) mechanically; environment
generators are companion-ecosystem tooling, not part of this spec. A
derivation MAY additionally reference a publisher-built environment by
content address — `environment: {kind: "oci" | "nix", digest, locator?}` —
which validators SHOULD prefer over reconstructing one.

## 7. Advisories: revoking trust in computations

The seal records exactly the identifiers a post-hoc bug report needs:
`library_version`, registry hash, engine name+version, `plan_hash`, and
input digests. An **advisory** is a signed, published document that matches
on those identifiers — the security-advisory model (CVE/OSV/RustSec) applied
to data products:

```json
{
  "id": "UA-2027-0004",
  "severity": "invalid",
  "matches": [
    { "field": "engine", "name": "polars", "versions": "<1.44.2" },
    { "field": "ops", "any_of": ["aggregate.integrate"] }
  ],
  "//": "matchers may also target external-input tools: { field: 'tool', name: 'cplex', versions: '<22.1.2' }",
  "description": "Aggregation kernel produced wrong sums for grouped integrate",
  "remediation": "Republish with engine >= 1.44.2",
  "published_by": "unitarrow-project",
  "signature": "..."
}
```

- **Matchers have a precision ladder.** Because plans are canonical
  serializations over a small, enumerable operation vocabulary, matching on
  operations used is a static set intersection, not program analysis. From
  coarse to surgical: engine name+version only; engine + `ops_any_of`
  (plans whose IR contains listed operation kinds); external-input `tool`
  name+version (§4) — so a defective solver release is advisable exactly
  like a defective engine, transitively through the DAG; `input_digest` —
  matching any artifact whose DAG contains the digest among its inputs,
  including `produced_from` inputs, which is how a retracted upstream file
  (a redacted LP) flags every descendant; exact `plan_hash`; exact
  `data_digest`. Artifacts published without a recorded derivation
  can only be matched coarsely — recording derivations is what protects a
  publisher from collateral flagging.
- **Severity is graded** (`informational` / `degraded` / `invalid`): most
  bugs affect only certain plans or inputs, and a binary kill switch would
  train users to ignore advisories. **Precision bounds severity
  (normative):** engine-only matchers MUST NOT carry `invalid` severity;
  `invalid` requires an ops predicate, plan hash, or digest match. Bugs
  whose reachability depends on runtime parameters rather than operations
  used remain a coarse-match case (§10).
- **Self-retraction requires no additional trust.** An advisory signed by
  the same key that sealed an artifact is authoritative over that artifact
  automatically: the consumer already trusted that key for the seal, and a
  retraction can only downgrade, never forge validity. A `retracted`
  severity class exists for publisher-issued corrections (assumption
  errors, upstream data faults), matched by `data_digest` and carrying a
  free-text reason. Third-party advisories apply only through feeds the
  consumer subscribes to — the backstop for publishers who will not
  self-retract. Authorization rule: an advisory binds an artifact iff it is
  signed by the artifact's sealing key, or arrives via a subscribed feed.
- **Distribution mirrors trust distribution** (§5.5): pinned advisory feeds
  in the trust-store TOML, `.well-known` discovery per publisher, periodic
  sync for offline use. Feeds are themselves signed artifacts.
- **Verification integration**: `verify()` checks the artifact's recorded
  identifiers against loaded feeds and surfaces matches on the badge and the
  provenance object (`W_ADVISORY`; strict verification requests raise
  `E_ADVISORY_MATCH` for `invalid`-severity matches).
- **DAG propagation**: an artifact is *transitively affected* if any
  ancestor in its derivation DAG matches an advisory. Traversal requires the
  ancestors' seals; content-addressed Destinations make this a digest walk.
  Where ancestry is unretrievable, verification reports the chain as
  *unverifiable past depth N* rather than silently clean.
- **Recovery is per-artifact, not per-source**: the remediation path is
  republication — a fixed toolchain produces a new digest and a clean badge.
  Trust attaches to artifacts and toolchains, so one bug does not
  permanently burn a publisher.

---

## 8. Error taxonomy

| Code | Raised when |
|---|---|
| `E_PLAN_TYPE` | Expression plan fails type-checking (wraps the type layer's codes — for UnitArrow, its §10 taxonomy) |
| `E_SEAL_BROKEN` / `E_PUBLISHER_UNKNOWN` | §5.4 states (the latter is a status, not an exception, unless strict verification is requested) |
| `E_TRANSIT_ALTERED` | Destination read-back fails seal verification |
| `E_ADVISORY_MATCH` | Strict verification against an `invalid`-severity advisory match (§7); `W_ADVISORY` otherwise |

## 9. Conformance suite

Golden files cover:

1. **Plans** — IR fixtures → expected output schemas, canonical plan
   hashes, type errors, and (on the pinned reference engine) execution
   results.
2. **Seals** — fixture keys, tables, and tampered variants → expected
   verification states and wordings.
3. **Closure** — DAG fixtures → reproducibility levels, including
   parameter-file retrievability and `produced_from` chains.
4. **Advisories** — feed fixtures → match results, severity bounds,
   transitive flags, supersession surfacing.

## 10. Open questions (deferred, tracked)

- **Naming** — this specification's own name.
- **Sigstore-style keyless publishing** for orgs that refuse key custody.
- **Polars lowering** — shape of the LazyFrame backend for the plan IR,
  and whether plan hashes can be stable across eager and lazy lowerings.
- **Cross-engine derivation verification** — tolerance-based equivalence
  checking when re-execution happens on a different engine than recorded.
- **Equivalence metric vocabulary** — beyond scalar `{metric, rtol/atol}`:
  feasibility residuals, per-column tolerances, domain-specific checks for
  attested transforms.
- **Advisory predicates beyond ops matching** — bugs whose reachability
  depends on runtime parameters or data values, which static plan
  inspection cannot decide; and feed freshness / staleness policy for
  offline verification.
- **Reproduction attestations** — independent parties re-executing closed
  derivations and publishing signed confirmations to a feed; the
  environment-generator companion (§6) is its natural seed.
- **Status index / resolver** — precomputed transitive status per digest;
  a cache of derivable facts, never a trust root; OCSP-style stapling.
- **Mutable naming layer** — resolving "latest" to immutable digests,
  outside artifact identity.

## 11. Interfaces with UnitArrow

Exactly three, all optional in both directions:

1. **The mode claim.** The seal envelope carries extensible claims;
   UnitArrow defines one (`mode` + registry pin, UnitArrow §5.6). Sealing
   a plain untagged table omits it.
2. **Typed plans.** Plan structure, hashing, and lowering are defined
   here; the type system that checks a plan (units, kinds, temporal,
   taint) is supplied by UnitArrow's §8 as a pluggable checker. Untyped
   plans are legal and verify structurally only.
3. **Sealable vocabularies.** UnitArrow's registry and vocabulary files
   are artifacts; this spec's sealing and advisory machinery applies to
   them exactly as to tables.
