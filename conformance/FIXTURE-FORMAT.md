# Conformance fixture format

Version **1** (`fixture_version: 1`). Normative for every file under
`fixtures/`. The machine-checkable form is
[schema/fixture.schema.json](schema/fixture.schema.json); this document is the
prose that explains why the fields exist.

## Design constraints

1. **Language-neutral.** Every binding — Rust, Python, TypeScript, WASM — runs
   the same bytes. JSON, no comments, no schema features beyond Draft 2020-12.
2. **Self-describing.** A fixture names the spec version, spec sections, and
   registry it was written against. A fixture that outlives its spec version
   should say so rather than silently mean something else.
3. **One assertion per case.** A case has one input and one expected outcome.
   No case both succeeds and errors depending on a flag.
4. **Open questions are visible, not hidden.** A case whose expected value
   depends on an unresolved spec ambiguity is marked, not omitted. See
   `status` below.
5. **Never edited to make an implementation pass.** Fixtures move only when the
   spec moves. (`provisional` cases are the exception, and only when their
   `AMB-nnn` entry is decided.)

## File structure

One JSON object per file. `cases` is a flat array — no nesting, no
cross-references between cases.

```json
{
  "fixture_version": 1,
  "category": "parsing",
  "spec": {
    "document": "unitarrow-spec",
    "version": "0.21.0-draft",
    "sections": ["6.1", "6.3"]
  },
  "registry": "conformance-core@0.1.0",
  "description": "Term ordering within canonical form.",
  "cases": [ ... ]
}
```

### Envelope fields

| Field | Required | Meaning |
|---|---|---|
| `fixture_version` | yes | Format version of *this document*. Currently `1`. |
| `category` | yes | One of `parsing`, `factors`, `propagation`, `wire`, `type-functions`, `plans`, `seals`, `closure`, `advisories`. Must match the containing directory. Named rather than numbered — see AMB-023. |
| `spec.document` | yes | `unitarrow-spec` or `provenance-companion-spec`. |
| `spec.version` | yes | Spec version the expectations were derived from. |
| `spec.sections` | yes | Section numbers as strings (`"6.3"`, `"8.5"`), no `§`. |
| `registry` | no | `<name>@<version>` of the registry symbols resolve against. Required for any category whose cases resolve symbols. |
| `description` | yes | One line. What this file is for. |
| `cases` | yes | Non-empty array. |

## Case fields

```json
{
  "id": "parsing.ordering.002",
  "spec_ref": "§6.3 step 3",
  "status": "provisional",
  "blocked_on": ["AMB-003"],
  "mode": "permissive",
  "grammar": 1,
  "input": { "unit": "m*kW*h*MW*W*Btu" },
  "expect": {
    "outcome": "ok",
    "canonical": "Btu*MW*W*h*kW*m",
    "dimension": { "mass": 4, "length": 9, "time": -10 }
  },
  "rationale": "Bytewise UTF-8 collation puts all uppercase before all lowercase."
}
```

| Field | Required | Meaning |
|---|---|---|
| `id` | yes | `<category>.<theme>.<nnn>`, globally unique across the suite. Stable forever: runners report it, the ambiguity register cites it. Never renumber. |
| `spec_ref` | yes | Human-readable pointer, e.g. `"§6.3 step 3"`. Free text. |
| `status` | yes | `normative` or `provisional`. |
| `blocked_on` | when provisional | Array of `AMB-nnn` IDs from [AMBIGUITIES.md](AMBIGUITIES.md). Required iff `status` is `provisional`; forbidden otherwise. |
| `mode` | no | `strict` \| `permissive` \| `untracked`. Present only where §6.3/§8 make behavior mode-dependent. Absent means the case is mode-independent — *not* that the default applies (the default is itself unresolved; see AMB-020). |
| `grammar` | no | Unit-string grammar version the input claims. Defaults to `1`. |
| `input` | yes | Category-specific; see below. |
| `expect` | yes | Exactly one of the two shapes below. |
| `rationale` | no | Why this is the expected answer. Write one whenever the case would otherwise look arbitrary. |

### `status`

- **`normative`** — the expected value follows from the spec as written. If an
  implementation disagrees, the implementation is wrong.
- **`provisional`** — the spec does not determine the answer, and the expected
  value encodes the resolution recommended in `AMBIGUITIES.md`. A runner may
  filter these out (`--skip-provisional`) while the questions are open. When the
  underlying entry is decided, the case is updated to match the decision and
  flipped to `normative`.

Provisional cases are the reason this suite is worth writing before there is any
code: each one is a spec decision made concrete enough to argue with.

### `input`

For `category: "parsing"`:

| Field | Required | Meaning |
|---|---|---|
| `unit` | one of | The **decoded** unit string. May be non-canonical, malformed, or empty — that is frequently the point. |
| `unit_json` | one of | The **raw JSON token** for the `unit` key as it appears on the wire, including its surrounding quotes. |
| `base` | no | The §5.4 `base` object, for `pu` cases. |
| `quantity` | no | CURIE, for cases where the quantity kind affects the outcome (§6.4 `interval`). |

Exactly one of `unit` and `unit_json` is present.

### Why `unit_json` exists

A fixture file is itself JSON, so `"\u004dW"` written in a fixture decodes to
`MW` before any runner sees it. The format therefore **could not express** cases
whose outcome depends on the byte encoding rather than the decoded value — which
is precisely what AMB-039 is about.

`unit_json` carries the token verbatim, so backslashes are doubled in the
fixture source:

```json
{ "input": { "unit_json": "\"\\u004dW\"" } }
```

That is the eight-character token `"\u004dW"`, which decodes to `MW`. Under
AMB-039 it is `E_BAD_METADATA` — the decoded value is perfectly canonical, and
that is the problem: accepting it would mean the wire bytes are not a function
of the unit.

Use `unit` for everything else. A case that is about the grammar, the registry,
or canonical form should not pay the readability cost of a raw token. Note that
a literal non-ASCII character (a BOM, `°`) needs no escape and so belongs in
`unit`, not `unit_json` — the two rules cover different halves of the hazard.

### `input` for `category: "composition"`

| Field | Required | Meaning |
|---|---|---|
| `sources` | yes | Array of `{name, version, toml}` — the constituent registries, verbatim. May be empty, to assert that composing nothing fails. |
| `rulings` | yes | Array of `{symbol, from, reason}` (§7.5). May be empty. |
| `verify` | no | `"exact"` or `"tampered"` — asserts §5.6's rule that a fetched registry not matching the pin is refused. |
| `combine_with` | no | A second `{name, version, toml}`. Present only on AMB-059 cases, which model combining two independently-tagged tables. |
| `symbol` | with `combine_with` | The unit symbol both sides carry. |

Sources are inline rather than referenced by filename because a composition
case is *about* the relationship between two registries; splitting them into
files would put the interesting part somewhere the reader has to go find.

### `expect` for `category: "composition"`

Success adds:

| Field | Meaning |
|---|---|
| `pin` | The effective registry's `sha256:` content hash, exact. |
| `resolves` | Map of symbol → `{factor, dimension_name, describes, resolvable}`. Only the keys present are checked. |
| `composed_from` / `agreed` / `applied` | Arrays of names, order-sensitive. |

Failure adds:

| Field | Meaning |
|---|---|
| `names` | Tokens the diagnostic MUST mention. |

`names` is a **content** requirement, never a wording one — bindings phrase
errors natively and this suite is language-neutral. It exists because AMB-058's
original complaint was not that composition failed but that the failure never
said `bbl`. Asserting the code alone would let that regress.

#### On asserting `pin`

A pin has no oracle outside the algorithm §7.5 defines, so these values were
**recorded from a run**, not derived independently. That makes them a
regression lock and — more importantly — a *cross-language contract*: a second
implementation that orders keys differently, emits a trailing newline, or spaces
an inline table differently produces a different hash and fails here and nowhere
else. §7.5's "MUST produce byte-identical output" is otherwise untestable.

Two consequences. A pin case MUST be regenerated only when the serialization
rules themselves change, and such a change is a spec change. And
`composition.determinism.002` deliberately shares `.001`'s pin with its sources
in the opposite order — if that ever diverges, source order has leaked into
identity.

Other categories define their own `input` shape when they are populated; the
schema uses a permissive object there for now.

### `expect`

Success:

```json
{ "outcome": "ok", "canonical": "MW*h",
  "dimension": { "mass": 1, "length": 2, "time": -2 } }
```

Failure:

```json
{ "outcome": "error", "code": "E_AFFINE_COMPOUND" }
```

| Field | Required | Meaning |
|---|---|---|
| `outcome` | yes | `ok` or `error`. |
| `canonical` | when `ok`, parsing | The §6.3 canonical string. Exact match, byte for byte. |
| `dimension` | when `ok`, parsing | Sparse map over base dimensions; see below. |
| `code` | when `error` | A §10 error code, exact string. |
| `warnings` | no | Array of `W_*` codes the operation must emit. Absent means unconstrained, not "none" — assert emptiness with `[]`. |

An `error` case never carries `canonical` or `dimension`; an `ok` case never
carries `code`. The schema enforces this.

`E_UNIT_SYNTAX` and `E_BAD_METADATA` entered §10 with spec 0.21.0-draft.
One code remains **proposed, not yet in §10**: `E_EXP_RANGE` — every case
using it is `provisional` and blocked on AMB-015.

### `dimension`

A sparse map over the nine §6.2 base dimensions. Any omitted key is `0`; the
empty object `{}` is dimensionless. Values are integers in the signed-8-bit
range (see AMB-008).

```
length  mass  time  current  temperature  amount  luminosity  count  currency
```

Named derived dimensions (`energy`, `power`) never appear here — §6.2 decides
commensurability on the underlying vectors, so fixtures assert vectors. This
also means a fixture stays correct if the registry renames a derived dimension.

## Adding a case

1. Give it the next free number in its theme. Never reuse or renumber.
2. Decide `status` honestly. If you had to make a judgement call the spec does
   not license, it is `provisional` and needs an `AMB-nnn` — write the register
   entry first.
3. Write a `rationale` if the expected value is not self-evident from the input.
4. Confirm every symbol in `input.unit` either resolves in the named registry or
   is deliberately unresolvable in an `E_UNKNOWN_UNIT` case.
5. Validate against the schema before committing.

## Adding a category

Create `fixtures/<name>/`, add `<name>` to the schema's `category` enum, and
give the directory a `README.md` naming its spec section and owning milestone.
Directories are named, not numbered — see AMB-023.
