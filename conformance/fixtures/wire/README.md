# `wire` — extension metadata and IPC round-trips

**Spec:** §5 (extension type, metadata JSON, untagged columns, p.u., display
name, provenance block)
**Spec §11 category:** 4 · **Milestone:** M2
**Status:** shaped, empty.

Spec §11: *"IPC fixtures that MUST re-serialize byte-stable metadata, including
unknown-key preservation and degradation behavior."*

## What these fixtures must pin

- **Byte-stable re-serialization.** Read an IPC file, write it back, compare
  bytes. This is the category's whole reason to exist.
- **Unknown-key preservation** (§5.2). Unknown keys of several JSON types
  (object, array, `null`, number) must survive untouched. Also unresolved: is
  key *order* preserved or canonicalized? It matters, because the companion's
  data digest is computed over serialized bytes (AMB-024) — an implementation
  that quietly drops or reorders an unknown key breaks the seal of every
  downstream artifact, and the failure surfaces as "this table was modified
  after it was published" pointing at an innocent consumer.
- **Graceful degradation** (§1 goal 3). A reader that ignores the extension
  sees the storage type plus legible metadata, and untagged tables pass through
  bit-identical.
- **Storage types and the cast rule** (§5.1). float32/64, decimal128/256, and
  all integer widths are allowed; converting an integer or decimal column must
  either promote to float64 or fail with `E_CAST_LOSSY`. Null validity is
  orthogonal to units and must survive every operation untouched.
- **Nested placement** (§5.1). A unit on a list field's *child* is valid; on the
  list field itself it is `E_BAD_PLACEMENT`. The reserved
  `struct{value, stderr}` layout must pass through unmodified with no
  conversion semantics attached to the children.
- **Display name, description, references** (§5.5). `unitarrow:display_name`,
  `unitarrow:description` (field *and* schema level), and schema-level
  `unitarrow:references` are all *plain* metadata keys, outside the extension
  blob — and therefore outside §5.2's unknown-key preservation rule, which
  covers only the blob. Fixtures must assert they survive a round trip anyway,
  because they are inside the companion's data digest.
- **Description propagation** (§5.5). A column's description survives only when
  the column "passes through unchanged"; converted, integrated, and renamed
  columns get none. Fixtures must cover every plan-IR operation, including the
  ones that make the rule ambiguous — filter, sort, concat, storage-only cast.
- **Provenance merge** (§5.6). Merging tables takes the weakest mode
  (`strict` > `permissive` > `untracked`). Merge behavior for `description` and
  `references` is undefined and needs fixtures once it is decided.
- **Metadata size lint** (companion §5.2 step 5). A per-field threshold warns in
  permissive and fails in strict. Needs a number before a fixture is possible.

## Blocked on

- **AMB-014** — there is no error code for malformed or missing extension
  metadata: absent blob, invalid UTF-8, invalid JSON, JSON scalar instead of an
  object, missing required `unit` or `grammar`. Proposed: `E_BAD_METADATA`.
- **AMB-024** — key-order stability under the digest.
- **AMB-029** — the metadata size norm has no number, no unit, and no defined
  scope, in either document.
- **AMB-031** — "passes through unchanged" is undefined, and the companion
  explicitly rules out the byte test. Every propagation fixture depends on this.
- **AMB-032** — the plain `unitarrow:*` keys have no round-trip or merge rules.
- **AMB-033** — `unitarrow:references` has no defined encoding or validation.

M2's exit criterion lands here: a cross-language round trip must be byte-stable,
and pyarrow-, polars-, and pandas-produced tables must tag and read identically
through the PyCapsule interface.
