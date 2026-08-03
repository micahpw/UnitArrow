# Decision memo — resolving the ambiguity register

Companion to [AMBIGUITIES.md](AMBIGUITIES.md). That document records *what is
wrong*; this one works each finding through to a decision you can make in one
pass.

**Status 2026-07-29: all 13 M0 decisions and both affine-adjacent data
decisions are RULED** — see the ✅ marks in the checklist. Fixtures and the
fixture registry are updated to the rulings (62/73 normative). **Caution: the
*Staged artifacts* blocks in Clusters A and B stage the A1 (slash) form and are
superseded by the A4 ruling** — when writing the spec editorial pass, take
canonical-form serialization text from the AMB-001 resolution note in the
register, not from Cluster A's staged block. Everything not marked ✅ remains
unapplied — those register entries still say `open`, and
`registry/conformance-core.toml` still encodes the register's original
recommendations. On approval each entry's *Staged artifacts* section says
exactly what changes.

## Changes and reversals so far

1. **AMB-016's recommendation is reversed.** The register recommends the
   source-units offset convention. Deeper analysis says the *kelvin* convention
   is better — it makes offsets auditable against each other, and it requires
   changing one line of §6.4 instead of that line plus §7.2's registry data.
   See [Cluster G](#cluster-g--the-affine-contradiction-amb-016-amb-018).
2. **AMB-019's recommendation is reversed.** The register recommends deleting
   "or vice versa". Both directions of the interval/absolute mismatch are
   equally severe bugs; the right fix is to make the reverse *declarable*.
   See [Cluster M](#cluster-m--kind-and-temporal-edges-amb-019-amb-025-amb-022).
3. **AMB-005's recommendation is revised** — ASCII stays, but only for
   *canonical* symbols. Input tokenization becomes permissive (a symbol is a
   maximal run of characters that are not `*`, `/`, `^`, whitespace, or a
   control), which lexes `°C` and `µs` with **no Unicode tables**, and lets
   aliases resolve them to ASCII. Keeps every safety property; removes the
   ergonomic cost.
4. **AMB-029's size limit is revised** — the proposed schema-wide 64 KiB budget
   trips at **274 columns** on a documented table. Per-key and per-column only.
5. **Three new findings**, all from working the existing ones through:
   **AMB-036** (registry vs. vocabulary artifact identity), **AMB-037** (unit
   symbols are not namespaced, so federation does not protect the one thing
   canonical form depends on), and **AMB-038** (§5.5 and companion §5.2 give
   opposite answers on whether a unit conversion keeps a description). AMB-038
   is a direct contradiction between the two documents, not a gap.

Register total is now **46**; five entries decided (001, 002, 004, 016, 039), one direction ruled (044).

## How to read an entry

- **Evidence** — the quoted text, and what it does and does not settle.
- **Options** — every reading that is actually defensible. Options that look
  plausible but are eliminated by other spec text are shown with why, because
  those are the ones an implementer will reach for.
- **What breaks** — the concrete failure under each option.
- **Recommendation** — one option, with the reason it wins.
- **Staged artifacts** — the spec prose, fixture changes, and registry changes
  that follow, ready to apply.

Depth is deliberately uneven. The M0 and M1 blockers get full treatment because
they are urgent, consequential, and hard to reverse. The wire, seal, and
type-function entries get tighter treatment because their categories are empty
and their decisions can follow the features.

---

# Decision checklist

Rule on these; everything else in the memo is supporting material.

## M0 — canonical form — ✅ ALL 13 RULED 2026-07-29

| # | Decision | Recommended |
|---|---|---|
| 1 | Negative terms: one `/` each, or no slashes at all? | ✅ **RULED 2026-07-29: A4** — no slashes; `W*K^-1*m^-2`. Reverses the memo's A1. `/` and `1/…` remain input forms; display is a separate path. Fixtures re-goldened |
| 2 | Exponent sign under `/` | ✅ Mooted for output by ruling on 1; input typo-guard retained (`MW/s^-1` invalid) |
| 3 | Collation | ✅ **RULED: bytewise UTF-8**, ascending |
| 4 | Spelling a pure inverse | ✅ Mooted by ruling on 1 — canonical is `s^-1`; `1/s` stays legal input |
| 5 | Empty string | ✅ **RULED: `E_UNIT_SYNTAX`** — not `"1"`, not untagged |
| 6 | Symbol lexical class | ✅ **RULED: strict ASCII everywhere** (F1) — the permissive-input revision was declined; `°C` is a syntax error |
| 7 | Whitespace | ✅ **RULED, then REVISED 2026-07-30**: accepted at edges and around operators; rejected between two bare symbols, where collapsing `m s` → `ms` would silently mean millisecond |
| 8 | Exponent literal form | ✅ **RULED: strict** — no `+`, no leading zeros; `^0`/`^1` legal and normalized |
| 9 | Aliases | ✅ **RULED: per-unit `aliases` field** in §7.2; ASCII-only per the AMB-005 ruling |
| 10 | Registry identity | ✅ **RULED: required `name`** in `[registry]`, carried in every pin |
| 11 | Parse-failure code | ✅ **RULED: `E_UNIT_SYNTAX` added** to §10 |
| 12 | Registry vs. vocabulary artifacts (AMB-036) | ✅ **RULED: K2** — two formats; registry = units/dimensions (one, PR-gated), vocabularies = kinds (many); seal gains `vocabularies: [{name, version, hash}]` |
| 13 | Namespacing unit symbols (AMB-037) | ✅ **RULED: L2** — cross-artifact symbol collision is `E_REGISTRY_INVALID` at load; namespaced units deferred to grammar v2 beside rational exponents |

## M1 — factors, affine, reader behavior

| # | Decision | Recommended |
|---|---|---|
| 14 | Exponent range and its code | **i8, checked per-term and post-merge**; add `E_EXP_RANGE` |
| 15 | Affine offset convention (AMB-016) | ✅ **RULED: kelvin convention** (delegated; recorded because registry offset *data* depends on it). Registry updated; `C`/`F` aliases added |
| 16 | Delta units in the registry | **Full entries**, same dimension and factor, no offset |
| 17 | "Compound" for affine | **Check parsed terms before merge**; exactly one term, exponent 1 |
| 18 | Default mode at read time | **`permissive`**, and split §6.3's sentence into two independent axes |
| 19 | Permissive + unresolvable symbol | **Parser always errors; reader warns once, taints, preserves the string** |

## M2 and later

| # | Decision | Recommended |
|---|---|---|
| 20 | Malformed metadata code | **Add `E_BAD_METADATA`** |
| 21 | Metadata key order under the digest | **Canonicalize (sorted)**, not "preserve author order" |
| 22 | Plain `unitarrow:*` key preservation and merge | **Extend §5.2 to all such keys**; `description` drops on conflict, `references` unions |
| 23 | `references` encoding | **UTF-8 JSON array**; `url` an absolute URI; rename inner `description` → `title` |
| 24 | Description propagation (AMB-031) | **A table over the plan IR** — and resolve AMB-038 first |
| 25 | Conversion vs. description (AMB-038) | **Conversion keeps the description**; §5.5's list is wrong |
| 26 | Metadata size norm | **4 KiB per key, 64 KiB per schema, UTF-8 bytes**; `display_name` 256 B |
| 27 | Column digest canonicalization | **Digest logical values, not buffers** |
| 28 | New seal codes | **`E_METADATA_SIZE`, `E_DESCRIPTION_STALE`, `W_DESCRIPTION_STALE`** |
| 29 | Prose-only republication | **State the asymmetry**; add a `documentation-only` disposition |
| 30 | `interval` reverse check (AMB-019) | **Add explicit `interval = false`** — *reversed from the register* |
| 31 | `period` subset | **Fixed-length designators only**; compare as exact rational seconds |
| 32 | `pu` | **Dimensionless, non-composable**, `E_BASE_MISSING` at metadata validation |
| 33 | Document hygiene (011, 012, 023, 027, 028) | Accept as written in the register |

## Encoding — [Cluster O](#cluster-o--escapes-and-code-points-amb-039-decided-amb-040-open)

| # | Decision | Recommended |
|---|---|---|
| 34 | Escapes in units | **Prohibit** — ✅ decided and applied |
| 35 | Escapes in other machine-readable values | **Prohibit** — free everywhere it applies |
| 36 | Escapes in free prose | **Permit** — a quotation mark requires one |
| 37 | Control / bidi / zero-width code points in prose | **Forbid** |
| 38 | NFC in prose | **Validate, do not normalize** — pending a WASM size measurement |
| 39 | `quantity` CURIE character class | Undefined today; settle alongside 35 |
| 40 | Registry `display.latex` escapes | TOML literal strings; settle with AMB-027 |

## Scale and extensibility — [Cluster P](#cluster-p--wide-tables-amb-041-amb-042), [Cluster Q](#cluster-q--guardrails-for-extensible-catalogs-amb-043)

| # | Decision | Recommended |
|---|---|---|
| 41 | Bulk tagging interface | **Patterns at authoring time**; nothing pattern-shaped on the wire |
| 42 | Schema-level unit defaults | **Reject** — new §2 non-goal, three reasons |
| 42b | Enum / integer unit codes | **Reject** — saves 8 of 160 B/col and costs degradation, self-containedness, federation; enums belong in-memory and in `onto.*` constants |
| 43 | State the per-column byte cost | **Yes**, in §1 goal 5 |
| 44 | Bulk republish dispositions | **Add** |
| 45 | `column_digests` guidance for wide tables | **Add**, plus a degradation note |
| 46 | Registry load-time validation | **Add §7.5**, 23 checks |
| 47 | Registry error code | **`E_REGISTRY_INVALID`** with a structured reason |
| 48 | Validation vs. sealing | **Independent** — a Verified catalog may still be invalid |
| 49 | `registry/` conformance category | **Add** — malformed-catalog fixtures |

---

# Cluster A — What does §6.3 step 3 actually emit? (AMB-001, AMB-002, AMB-003)

One decision in three parts. Settle the shape of the negative group and the
other two fall out.

## AMB-001 — slash repetition

### Evidence

> negative exponents rendered with `/`, positive-exponent terms sorted
> lexicographically by symbol, then negative-exponent terms sorted
> lexicographically **after all `/` terms**

The final clause is circular — it locates the `/` terms relative to the `/`
terms. What the sentence does establish: there are two groups, positives sort
first, negatives sort second, and `/` is involved. What it does not establish:
how many slashes.

§6.1 supplies the missing constraint. Division binds left to right and there is
no grouping construct, so a single leading slash cannot cover more than one
term.

### Options

| | Emits | Verdict |
|---|---|---|
| **A1** | `W/K/m^2` — one `/` per negative term | Viable |
| **A2** | `W/K*m^2` — single `/`, rest joined by `*` | **Eliminated.** Parses as W·K⁻¹·m**⁺²** — a different unit, not a different spelling. `parsing.division.003` vs `.004` is the demonstration: the two inputs differ by one operator and denote units four orders apart in length |
| **A3** | `W/(K*m^2)` — parenthesized denominator | **Eliminated** by §6.1's "No parentheses", and reintroduces the grouping question |
| **A4** | `W*K^-1*m^-2` — no slashes in canonical output at all | Viable, and cheaper than it looks |

### The A4 note, which is the interesting part

A4 deserves a real hearing because it **eliminates AMB-001, AMB-002, and
AMB-004 outright** — three of the eleven M0 blockers:

- there is no slash, so slash repetition is moot;
- the sign is always written explicitly, so sign absorption is moot;
- `s^-1` is directly spellable, so the "unspellable pure inverse" hole closes
  and `"1"` stays purely the empty product.

The serializer becomes: sort all terms, join with `*`, render `symbol` or
`symbol^exponent`. Three lines, no grouping, no special cases. `/` remains in
the *input* grammar (producers still write `MW/h`); it simply never appears in
canonical output. Grammar as the input language, canonical form as a subset of
it, is a normal and clean split.

The argument against is not aesthetics, it is §1 goal 3. §5.2 makes `unit` the
canonical string, so the canonical string *is* what an unaware consumer reads
in field metadata — the "plain float column with human-readable metadata beside
it" that graceful degradation promises. `USD*MWh^-1` is meaningfully worse
there than `USD/MWh`, and there is no compound display form to fall back on
(§7.2's `display` is per-unit, not per-expression).

So: A4 is right if canonical form is ever decoupled from the wire string, and
A1 is right while they are the same string. They are the same string today.

### Recommendation

**A1.** One `/` per negative term. Accept the three-finding cost as the price
of a human-readable wire value.

If you would rather have the simplification: choosing A4 closes decisions 1, 2,
and 4 on the checklist at once, and the follow-on work is defining a compound
display form so goal 3 survives.

---

## AMB-002 — exponent sign under `/`

### Evidence

Nothing in §6.3 states whether `m^-2` in the negative group renders `/m^2` or
`/m^-2`. Under A1 this must be decided, because `/m^-2` read literally is
"divide by m⁻²" = multiply by m².

### Options

| | Rule | Consequence |
|---|---|---|
| **B1** | Slash owns the sign; rendered exponent is the absolute value; `/m^-2` is invalid input | One spelling; the input language matches the output language |
| **B2** | Same output, but `/m^-2` accepted on input and normalized | `MW/s^-1` silently means `MW*s`. Forgiving in a way that reads as a bug |
| **B3** | Render `/m^-2` | Denotes the opposite of the intent. Not defensible |

### What breaks

Under B2, a producer who writes `MW/s^-1` meaning "megawatts per inverse
second" gets `MW*s` with no diagnostic. The double negative is almost always a
typo for `MW/s`, and B1 catches it.

### Recommendation

**B1.** Costs one error code (`E_UNIT_SYNTAX`, decision 11) that is needed
anyway.

---

## AMB-003 — collation

### Evidence

"Sorted lexicographically by symbol", with symbols case-sensitive per §6.1. At
least three incompatible orderings answer to "lexicographic", and they produce
different canonical strings — therefore different hashes — for the same
expression.

### Options

| | Ordering | `Btu MW W h kW m` sorts as |
|---|---|---|
| **C1** | Bytewise UTF-8 | `Btu, MW, W, h, kW, m` |
| **C2** | Unicode code point | identical to C1 for all of ASCII |
| **C3** | Case-folding / locale-aware | `Btu, h, kW, m, MW, W` — and needs a tiebreak, since `mW` and `MW` fold together while remaining distinct units |

### The interaction worth noticing

If decision 6 restricts symbols to ASCII (AMB-005), then C1 and C2 are
*identical*, and both agree with a bare `<` on strings in JavaScript. No custom
comparator in any binding, no encoding-dependent behavior, nothing to get wrong
in the WASM build. Deciding AMB-005 the recommended way makes AMB-003 free.

If symbols were ever allowed above the BMP, C1 and JavaScript's `<` diverge:
JS compares UTF-16 code units, so surrogate pairs (0xD800–0xDFFF) sort below
U+E000–U+FFFF while their code points are higher. That is a real trap, and it
is another reason to keep symbols ASCII.

C3 is eliminated on its own terms: locale-aware collation is by definition not
locale-independent, and canonical form must be identical in every process on
every machine.

### Recommendation

**C1**, ascending, stated as "bytewise comparison of the UTF-8 encoding".

## Staged artifacts — Cluster A

**Spec §6.3, replacing steps 3–4:**

> 3. Serialize as a single product of two groups.
>
>    a. **Positive group** — every term whose exponent is positive, sorted
>       ascending by bytewise comparison of the UTF-8 encoding of its symbol,
>       joined by `*`. A term renders as `symbol` when its exponent is 1, and
>       `symbol^exponent` otherwise.
>
>    b. **Negative group** — every term whose exponent is negative, sorted
>       ascending by the same comparison, each rendered as `/` followed by the
>       term carrying the **absolute value** of its exponent: `/m` for exponent
>       −1, `/m^2` for −2. The `/` carries the sign; a negative exponent MUST
>       NOT appear after a `/`.
>
>    The negative group follows the positive group with no separator between
>    them. Each negative term requires its own `/`, because §6.1 binds division
>    left to right: `W/K*m^2` denotes W·K⁻¹·m⁺², not W·K⁻¹·m⁻².
>
> 4. When the positive group is empty and the negative group is not, the
>    positive group is written as the literal `1` — `1/s`, `1/K/m^2` (§6.1).
>
> 5. When both groups are empty — the empty product — the canonical form is
>    `1`.
>
> Canonicalization normalizes *spelling*, never the producer's choice of unit.
> `MW*h` and `MWh` are both canonical and are not equal, though they are
> commensurable and numerically identical (§1 goal 4). Two units are equal iff
> their canonical strings are identical.

That last paragraph also closes AMB-012.

**Fixtures:** `parsing.ordering.005`, `.006`, `parsing.division.001`, `.002`,
`.004`, `.005`, `.006`, `.007`, `.008`, `parsing.merging.006`,
`parsing.ordering.002`, `.003`, `.004` → `normative`, expectations unchanged.
13 cases clear. **No expected value moves** — the reference canonicalizer
already reproduces all 63 under exactly these rules.

**Registry:** unchanged.

---

# Cluster B — What can grammar v1 spell? (AMB-004, AMB-009)

## AMB-004 — the unspellable pure inverse

### Evidence

```
unit-string  = term *( ("*" / "/") term ) | "1"
term         = symbol [ "^" integer ]
```

`"1"` is an alternative for the whole string, not a `term`, and no production
allows a leading `/`. An expression with no positive-exponent terms therefore
has no spelling.

This is not a corner case. §8.1: "`a / b` … result unit is the canonicalized
symbolic product of input units." A dimensionless column divided by a seconds
column is ordinary arithmetic. §8.5's `differentiate` divides by a period. Both
produce a unit the spec cannot write down, which means they produce a value
that cannot be tagged, serialized, or hashed.

### Options

| | Rule | Cost |
|---|---|---|
| **D1** | Leading `1` permitted **only** when no positive term exists | Canonical form stays unique by construction |
| **D2** | Leading `1` permitted always; `1*MW` canonicalizes to `MW` | Also unique (the `1` is dropped), and simpler to parse. Adds an input spelling that no writer should emit |
| **D3** | Permit a leading `/`: `/s` | Introduces the only unary operator in the grammar |
| **D4** | Leave it; require a positive term | **Eliminated** — §8.1 generates the case |

### Recommendation

**D1.** Expressible directly in ABNF without a side condition:

```
unit-string  = product / inverse / "1"
product      = term *( ( "*" / "/" ) term )
inverse      = "1" 1*( "/" term )
```

D2 is harmless and slightly cheaper if you prefer permissive input; the only
reason to prefer D1 is that rejecting what you would never emit catches
generator bugs at the producer.

---

## AMB-009 — the empty string

### Evidence

`""` is ungrammatical under every reading. §6.3 step 4 carefully says the
*empty product* serializes as `"1"` — which is about an expression that
cancelled, not about an empty field.

### Options

| | Treatment | Verdict |
|---|---|---|
| **E1** | `E_UNIT_SYNTAX` | Recommended |
| **E2** | Equivalent to `"1"` | **Eliminated** by §5.3. An empty metadata field would become an explicit dimensionless claim nobody made — exactly the conflation §5.3 forbids |
| **E3** | Equivalent to untagged | **Eliminated.** The extension is present, so a claim *was* made; downgrading to untagged hides a producer bug and loses the fact that the producer tried |

### Recommendation

**E1.** Add a sentence to §6.1: "The empty string is not a valid unit string
and is not equivalent to `1`."

## Staged artifacts — Cluster B

**Spec §6.1**, replacing the grammar block, and folding in Cluster C's lexical
decisions:

```
unit-string  = product / inverse / "1"
product      = term *( ( "*" / "/" ) term )
inverse      = "1" 1*( "/" term )
term         = symbol [ "^" integer ]
symbol       = ALPHA *( ALPHA / DIGIT / "_" / "@" )
integer      = [ "-" ] ( "0" / ( NZDIGIT *DIGIT ) )
NZDIGIT      = %x31-39
```

> No whitespace is permitted anywhere, including leading and trailing. The
> empty string is not a valid unit string and is not equivalent to `1`. The
> leading `1` of an `inverse` is the multiplicative identity and contributes no
> dimension; it is permitted only in that position — `1*MW` is invalid.

**Fixtures:** `parsing.division.007`, `.008`, `parsing.merging.006`,
`parsing.dimensionless.003`, `.004` → `normative`. `.003` and `.004` retain
`E_UNIT_SYNTAX`, so they also depend on decision 11.

**Registry:** unchanged.

---

# Cluster C — What is a symbol? (AMB-005, AMB-006, AMB-007)

## AMB-005 — the lexical class

### Evidence

> `symbol = registry-resolved identifier (case-sensitive)`

This defines a symbol by what resolves, not by what lexes. A tokenizer cannot
be written from it: given `MWh`, it cannot know whether to emit one token or
try `MW` then `h`, and given `°C` it cannot know whether `°` starts a symbol.

Two places in §7.2 pull opposite ways. `variants = ["household_yr@CO", …]`
implies `@` and `_` are legal. `display.unicode = "°C"` raises whether display
forms are also accepted as input — §6.3 step 1 says "aliases → canonical
symbol" without saying where aliases come from (AMB-010).

### Options

| | Class | Consequence |
|---|---|---|
| **F1** | ASCII: `ALPHA *(ALPHA / DIGIT / "_" / "@")` | Covers every symbol either spec mentions. `°C`, `Ω`, `μs` become syntax errors, not unknown units |
| **F2** | Unicode identifiers (XID_Start/XID_Continue + `_@`) | Allows `°C`, `Ωm`. Costs a Unicode property table in a WASM build budgeted at "tens of KB", and opens normalization |
| **F3** | Unicode-permissive **input** tokenization, ASCII-only **canonical** symbols | The safe middle: `°C` tokenizes, resolves via alias to `degC`, canonical output stays ASCII |

### Why F2 is worse than it looks

Without a Unicode normalization rule, visually identical symbols become
distinct: U+03BC GREEK SMALL LETTER MU and U+00B5 MICRO SIGN both render as µ,
and `℃` (U+2103) is a single character that renders identically to `°C` (U+00B0
U+0043). Under F2 these are different symbols with different canonical forms
and different hashes — a canonical-form failure that is invisible on screen.
Fixing it means specifying NFC or NFKC, which is more Unicode machinery in the
WASM module and a normalization step before every lookup.

F3 contains the damage nicely: because canonical symbols are ASCII, a
normalization mistake can only produce `E_UNKNOWN_UNIT`, never two spellings of
one expression. Errors degrade to "I don't know that symbol" instead of
"equality is broken."

### Recommendation

**F1** for grammar v1 — smallest, safest, and no Unicode tables in WASM. Record
**F3** as the upgrade path, since it is compatible: widening the input class
later is backward-compatible, and narrowing it is not.

---

## AMB-006 — whitespace

### Evidence

The grammar has no whitespace production, which strictly means `MW * h` is
invalid — but spec grammars are often written assuming implicit whitespace, so
an implementer could reasonably go either way.

### Options

| | Rule | Consequence |
|---|---|---|
| **G1** | Forbidden everywhere, rejected not trimmed | A stray space surfaces at the producer |
| **G2** | Allowed around operators, stripped on parse | Canonical form must still say "no whitespace", so this only adds an input variant |
| **G3** | Allowed and preserved | Breaks uniqueness outright |

The realistic source of stray whitespace is a hand-edited config or a CSV
column of unit strings. G2 makes those pass silently; G1 makes them fail at the
point where the mistake was made, which is what §5.2's "writers MUST emit
canonical form" already implies.

### Recommendation

**G1.**

---

## AMB-007 — exponent literal form

### Evidence

`"^" integer` leaves `^+2`, `^02`, `^-0`, and `^0` undecided, each of which
would be a second spelling of something.

### Options

| | Accepts | Notes |
|---|---|---|
| **H1** | `["-"] ( "0" / NZDIGIT *DIGIT )` — no `+`, no leading zeros, no `-0` | `^0` legal and eliminated by step 2; `^1` legal and normalized to a bare symbol |
| **H2** | Also `+` and leading zeros, normalized away | More input variants for no benefit; every one is a chance for two producers to disagree before normalization |
| **H3** | Reject `^0` and `^1` as redundant | **Not recommended.** Both are natural outputs of programmatic construction — a loop that emits `sym^n` should not have to special-case n ∈ {0,1} |

### Recommendation

**H1.** Note the cross-reference: whether `degC^1` is "compound" is AMB-017,
not this entry — H1 only settles that `degC^1` *lexes*.

## Staged artifacts — Cluster C

Spec §6.1 as drafted at the end of Cluster B.

**Fixtures:** `parsing.normalization.004`, `.005`, `parsing.errors.007`,
`.009`, `.010` → `normative`. `parsing.errors.009` (`°C`) becomes normative
*and* correct for the stated reason — under F1 the degree sign cannot begin a
symbol, so it is a syntax error rather than an unknown unit.

**Registry:** unchanged. `conformance-core.toml` already uses only ASCII
symbols; `display.unicode` values stay as they are, since they are output-only
under F1.

---

# Cluster D — Registry identity, aliases, and federation (AMB-010, AMB-026, AMB-036, AMB-037)

Working AMB-010 and AMB-026 through surfaced two structural questions the
register had not caught. All four belong together because they are all about
*what a registry artifact is*.

## AMB-010 — aliases have nowhere to live

### Evidence

§6.3 step 1: "Resolve every symbol against the registry (**aliases → canonical
symbol**)."

§7.2's unit schema: `dimension`, `factor`, `offset`, `delta`, `display`,
`provenance`, `variants`. There is no `aliases` field. The first step of the
canonicalization algorithm reads data the registry format cannot express.

### Options

| | Shape | Notes |
|---|---|---|
| **I1** | `aliases = [...]` on each `[unit.X]` | A unit's spellings live with the unit. Validation is a global pass over all entries |
| **I2** | A flat `[alias]` table: `hr = "h"` | Trivially validated for collisions in one place; scatters a unit's definition across the file |
| **I3** | No aliases; delete step 1's parenthetical | **Eliminated.** Roadmap M1's exit criterion — "canonical symbols for IT vs thermochemical Btu with distinct factors" — needs `Btu` to map somewhere, and there is no migration path when a canonical symbol is renamed |

### Validation rules the spec must state either way

1. An alias MUST NOT equal any canonical symbol, in any loaded artifact.
2. An alias MUST NOT equal any other alias.
3. Aliases are input-only and are never emitted in canonical form.
4. Violations are **load-time** errors, matching §7.4's treatment of prefix
   collisions.

Rules 1 and 2 have to hold across the union of loaded artifacts, not per file —
which is the thread that leads to AMB-036 and AMB-037.

### Recommendation

**I1.**

---

## AMB-026 — registries have a version but no identity

### Evidence

```toml
[registry]
schema_version = 1
version = "2026.07"
```

§5.6 pins a registry as "version + content hash"; companion §5.3's seal carries
`registry: { version, hash }`. Two unrelated registries released as `"2026.07"`
are indistinguishable by version, so a consumer holding the wrong one sees a
hash mismatch with no way to learn *which* artifact it should have fetched.

### Options

| | Shape | Notes |
|---|---|---|
| **J1** | Required `name` in `[registry]`, `[a-z0-9-]+` | One field, closes it |
| **J2** | Reuse §7.4's namespace prefix as identity | Only works if registries and vocabularies are the same kind of artifact — which is AMB-036 |

### Recommendation

**J1**, and carry `name` alongside `version` and `hash` everywhere a registry
is pinned, including the seal. `conformance-core.toml` already does this and
flags it as a deviation.

---

## AMB-036 — "the registry" (singular) vs. "vocabulary artifacts" (plural) — NEW

### Evidence

§7 is titled *Registry format*. §7.1 gives a `[registry]` header, §7.2 units,
§7.3 quantity kinds. Then §7.4:

> Every **vocabulary file** declares a namespace prefix … A deployment's
> effective ontology is the **union of the vocabulary artifacts it loads**;
> prefix collisions are a load-time error.

Meanwhile §5.6 and companion §5.3 pin **one** `registry` — a single version and
a single hash.

Unresolved:

- Are a "registry" and a "vocabulary file" the same format? A vocabulary file
  has a namespace prefix; §7.1's header has no prefix field. A registry has
  `[unit.…]` and `[dimension.…]`; can a vocabulary file?
- If a deployment loads `core@2026.07` plus `hydro@2.1` plus `cf@…`, what does
  the seal's single `registry: {version, hash}` name? The core one only? Then
  the seal does not pin the ontology that was actually used, and a consumer
  cannot reconstruct it — which defeats claim 3 of companion §5.1, "checked at
  a stated level **against a stated registry**".
- §7.4 says vocabularies are individually "versioned, hashable, optionally
  sealed". Nothing says how their identities compose into the one pin.

### Options

| | Model | Consequence |
|---|---|---|
| **K1** | One format, two roles. A file MAY carry units/dimensions, quantity kinds, or both; `[registry]` gains `name` and an optional `prefix`. The seal pins a **list** of `{name, version, hash}` | Honest about federation; changes the seal shape |
| **K2** | Two formats. "Registry" = units and dimensions, PR-gated, exactly one loaded. "Vocabulary" = quantity kinds only, many loaded, separately pinned by a new seal field | Matches §7.4's "only two things are PR-gated: this registry *schema* and the `core:` namespace". Keeps the existing single `registry` pin meaningful |
| **K3** | Leave it | The seal's registry pin is not reproducible whenever more than one vocabulary is loaded |

K2 is closer to what §7.4 seems to intend, and it has a pleasing property: unit
symbols are global and unnamespaced (AMB-037), which is tolerable exactly
because the unit registry is singular and PR-gated. Quantity kinds are
federated and namespaced, which is what `core:` / `cf:` / `hydro:` already
encode.

### Recommendation

**K2**, stated explicitly, plus a `vocabularies: [{name, version, hash}]` field
alongside `registry` in the provenance block (§5.6) and the seal (companion
§5.3). This is a wire-visible change and should land before M2.5 freezes the
envelope — the roadmap already requires the minimal seal to ship with all
optional fields defined so minimal verifiers degrade gracefully.

---

## AMB-037 — unit symbols are not namespaced — NEW

### Evidence

§7.4: "every **term** is namespace-qualified (`core:power`,
`cf:air_temperature`, `hydro:streamflow`)."

But unit symbols are bare everywhere they appear: `[unit.MW]` in §7.2,
`"unit": "MW"` in §5.2, `MW*h` in every unit string. And they *cannot* be
qualified — `:` is not in the symbol lexical class under any reading of AMB-005,
so `core:MW` does not lex as one symbol.

So §7.4's "every term" is either false, or units are not terms. Either way the
consequence is the same and it is not stated: **federation protects quantity
kinds and does not protect units.** If a domain vocabulary defines `bbl` and
another defines `bbl` with a different factor, there is no prefix to
disambiguate and no rule saying what happens. That is worse than the quantity-
kind case, because canonical form — the equality and hashing primitive — is
built directly on unit symbols.

### Options

| | Approach | Notes |
|---|---|---|
| **L1** | Units stay global and unnamespaced; only the PR-gated core registry may define them (per K2); domain vocabularies may define quantity kinds only | Smallest change. Costs domains the ability to mint units, which is a real limitation — `household_yr@CO` in §7.2 is exactly a domain-minted contextual unit |
| **L2** | Units stay global, but any collision across loaded artifacts is a load-time error, symmetric with §7.4's prefix collisions | Domains may mint units; conflicts surface loudly at load rather than silently at compare |
| **L3** | Namespace unit symbols in grammar v2, e.g. `hydro:acre_ft` | Correct long-term, but changes the symbol lexical class and therefore the grammar integer, and touches every canonical string |

### Recommendation

**L2 now, L3 deferred to grammar v2.** L2 is a load-time validation rule and a
sentence in §7.4; it costs nothing on the wire and closes the silent-collision
hole. Record L3 in §12 next to rational exponents — both are grammar-v2 changes
to the symbol/term syntax and should be considered together, since doing them
in separate grammar versions means two migrations.

Also correct §7.4's "every term is namespace-qualified" to say "every
**quantity kind**", which is what it means.

## Staged artifacts — Cluster D

**Spec §7.1**, replacing the header block:

```toml
[registry]
schema_version = 1
name = "core"                  # required; [a-z0-9-]+; identity for pinning
version = "2026.07"
prefix = "core"                # vocabulary artifacts only; §7.4
# content hash is computed over the canonicalized file, not stored in it
```

**Spec §7.2**, adding to the unit schema:

> `aliases` — a list of additional input-only spellings that resolve to this
> entry's canonical symbol. Aliases are never emitted. Across the union of
> loaded artifacts, an alias MUST NOT equal any canonical symbol or any other
> alias, and a canonical symbol MUST NOT be defined twice; all three are
> load-time errors.

**Spec §7.4**, two corrections:

> - Every vocabulary file declares a namespace prefix; every **quantity kind**
>   is namespace-qualified … Unit symbols are **not** namespaced: they are
>   global, defined by the PR-gated unit registry, and a symbol defined twice
>   across loaded artifacts is a load-time error. Namespaced unit symbols are
>   deferred to grammar v2 (§12).

**Spec §5.6 and companion §5.3**, adding beside `registry`:

```json
"vocabularies": [ { "name": "cf", "version": "2026.07", "hash": "sha256:..." } ]
```

**Spec §12**, adding a bullet:

> - **Namespaced unit symbols** in grammar v2, alongside rational exponents —
>   both change the term syntax and should migrate together.

**Fixtures:** `parsing.normalization.001`, `.002` → `normative`.

**Registry:** `conformance-core.toml` gains nothing — it already declares
`name` and uses `aliases`. Remove the "NOT in spec §7.1" comment on `name`
once §7.1 is updated.

---

# Cluster E — A code for what the parser rejects (AMB-013)

### Evidence

§10's `E_UNKNOWN_UNIT` is raised when a "symbol does not resolve against the
loaded registry". That is *resolution*. These never reach resolution:

`MW**h` · `MW^` · `MW*` · `(m*s)` · `m/` · `^2` · `""` · `MW * h` · `0.5*MW` ·
`°C` · `MW^+2` · `m^1/2`

Twelve of the fourteen cases in `07-errors.json` are blocked on this one
decision, and so are `parsing.dimensionless.003`, `.004` and
`parsing.division.006`.

### Options

| | Approach | Verdict |
|---|---|---|
| **M1** | Add `E_UNIT_SYNTAX` | Recommended |
| **M2** | Reuse `E_UNKNOWN_UNIT` | **Eliminated.** "Unknown unit `MW**h`" sends the user to check their registry for a typo. The remediation is wrong, which is the same failure mode companion §5.1 declares non-conformant for seals |
| **M3** | A family — `E_UNIT_SYNTAX`, `E_UNIT_EMPTY`, `E_UNIT_BAD_EXPONENT`, … | Over-granular. The code selects the remediation; the message carries the detail |

### A sub-gap worth noting

§10 enumerates codes but says nothing about the error **payload**. "Bindings
map to native error types but preserve the code" — preserve what else? A
conformance fixture can currently assert only the code, which is why
`fixture.schema.json`'s error shape has just `code`. If offending-symbol and
byte-offset are meant to be available (they should be, for a syntax error to be
actionable), §10 needs one sentence and the fixture format needs one field.
Not raised as a separate register entry because it is a natural amendment to
whichever §10 edit lands.

### Recommendation

**M1.** And note that decisions 11, 14, and 20 are the same edit to §10 — add
three rows.

## Staged artifacts — Cluster E

**Spec §10**, three new rows:

| Code | Raised when |
|---|---|
| `E_UNIT_SYNTAX` | Unit string does not parse under the declared grammar (§6.1) |
| `E_EXP_RANGE` | Exponent outside the representable range, per term or after §6.3 step 2 merging |
| `E_BAD_METADATA` | Extension metadata absent, not valid UTF-8, not valid JSON, not a JSON object, or missing a required key (§5.2) |

Plus one sentence:

> Errors carry, in addition to the code, a human-readable message and — where
> applicable — the offending symbol and its byte offset in the input. Bindings
> MUST preserve the code verbatim and SHOULD surface the message.

**Fixtures:** `parsing.errors.003`–`.010`, `.014`, `parsing.division.006`,
`parsing.dimensionless.003`, `.004` → `normative`. 12 cases clear.

**Registry:** unchanged.

---

# Cluster F — Exponent range and its code (AMB-008, AMB-015)

### Evidence

§6.2 fixes dimension vectors as **signed 8-bit** exponents. §6.1 puts no bound
on unit-string exponents. So `m^300` is grammatically valid and dimensionally
unrepresentable, and `m^100*m^100` overflows during step 2's summation although
each input term is in range.

### Options

| | Bound | Where checked |
|---|---|---|
| **N1** | i8, `[-128, 127]` | Per term **and** after step 2's merge |
| **N2** | i8 | After merge only — so `m^200*m^-100` (result m^100) is legal |
| **N3** | Widen dimension exponents to i16 | **Eliminated.** §6.2's base-dimension representation is one of the two things M0 froze, and i8 is not the constraint in practice: m^127 is already absurd |

N2 is defensible — the result is representable, so why reject? — but it forces
the parser to carry a wider accumulator and to accept individual terms that can
never appear in a canonical form. N1 keeps every intermediate in the same width
as the final representation and rejects nonsense at the point it is written.

The two checks are genuinely both needed, and it is easy to implement only one:

- per-term only misses `m^100*m^100`, which silently wraps to −56;
- post-merge only accepts `m^200` as an input token before arithmetic.

The fixtures already assert both — `parsing.errors.011` is the per-term case,
`.012` the post-merge case.

### Recommendation

**N1**, with `E_EXP_RANGE` (decision 11's §10 edit covers it).

## Staged artifacts — Cluster F

**Spec §6.1**, one sentence after the grammar:

> An exponent MUST lie in the signed 8-bit range [−128, 127], both as written
> and after the merge in §6.3 step 2; `E_EXP_RANGE` otherwise.

**Fixtures:** `parsing.errors.011`, `.012` → `normative`.

**Registry:** unchanged.

---

# Cluster G — The affine contradiction (AMB-016, AMB-018)

The most consequential entry in the register, and the one where the memo
departs from it.

## AMB-016 — offset convention

### Evidence

§6.4:

> convert as `y = (x + offset_from) * factor - offset_to`

The offset is added **before** scaling, which means it is expressed in the
source unit's own scale.

§7.2, annotating the same field:

```toml
offset = [27315, 100]          # exact rational offset to kelvin
```

"Offset to kelvin" reads as *expressed in kelvin*, i.e. added **after** scaling.

For `degC` the two readings coincide, because degC's factor is 1. That is
precisely why the spec's own example cannot distinguish them — and there is no
`degF` entry anywhere in the document, so nothing in the spec exercises the
difference.

Separately, the formula's single unqualified `factor` cannot stand for both
`factor_from` and `factor_to`, which a conversion between two affine units
needs.

### The two self-consistent conventions

Both work correctly *if applied consistently*. The bug is only in mixing them —
which is exactly what an implementer reading §6.4's formula and §7.2's comment
together will do.

**O1 — source-units offset** (§6.4's structure)

```
base = (x + offset_from) * factor_from
y    = base / factor_to - offset_to
```
with `degF.offset = 459.67 = [45967, 100]`.

**O2 — kelvin offset** (§7.2's comment), the standard affine form

```
base = x * factor_from + offset_from
y    = (base - offset_to) / factor_to
```
with `degF.offset = 255.372… = [45967, 180]`.

Verified numerically, both give 100 °C → 212 °F, 0 → 32, −40 → −40, and 20
delta_degC → 36 delta_degF. Mixing them — §6.4's formula with §7.2's offset
value — gives **416.2978 °F** for boiling water.

### Why the memo now recommends O2, against the register

Four reasons, in order of weight:

1. **Offsets become comparable.** Under O2 every unit's `offset` is expressed
   in the same scale — the dimension's base unit — so the numbers can be
   audited against each other and against a reference. Under O1, degC's offset
   is 273.15 in °C-sized steps and degF's is 459.67 in °F-sized steps, and
   comparing them is meaningless. Given that the entire point of exact
   rationals (§7.2) is auditability, a field whose values are not mutually
   comparable is working against that.

2. **It is the smaller spec edit.** O2 makes §7.2's existing comment *true as
   written*, so the registry data in the spec needs no change at all — only
   §6.4's one-line formula is wrong. O1 requires changing the formula **and**
   §7.2's comment, and the comment is the thing registry authors will read
   while typing numbers in.

3. **The field name matches the operation.** `offset` in the form
   `base = x*factor + offset` is an intercept, which is what "offset"
   ordinarily denotes. In `(x + offset)*factor` it is a pre-multiplied
   intercept, which is a less usual thing to call an offset and easier to
   mis-enter.

4. **Registry data outlives spec prose.** Whichever convention is chosen, the
   numbers get typed into registry files once and copied thereafter. O2's
   numbers are the ones a physical-constants reference will hand you for "the
   offset to kelvin", so a copying error is less likely.

Against O2: §6.4's formula is operative normative text, and a `#` comment in an
example is not. If you weight "the normative text wins" above the four points
above, O1 is the answer, and the register's original recommendation stands. It
is a genuine judgement call; the numbers are correct either way.

**Whichever is chosen, both edits must land together.** The failure mode here
is a half-applied fix.

### Recommendation

**O2.** Requires updating `conformance-core.toml`'s degF offset from
`[45967, 100]` to `[45967, 180]` and rewriting the AFFINE UNITS comment block.

---

## AMB-018 — delta units are referenced but never specified

### Evidence

`delta = "delta_degC"` appears in §7.2's example; §6.4 says delta units are
"purely multiplicative and freely composable". Nothing states that `delta_X`
needs its own entry, what its dimension or factor is, whether it may carry an
offset, whether `delta` is symmetric, or what a non-affine unit's delta is.

`delta_degF`'s factor is the one that matters and is nowhere written: it must
be 5/9 — degF's factor with the offset dropped — or a 20-degree swing converts
wrongly, which is the exact bug §6.4 exists to prevent.

### Options

| | Model | Notes |
|---|---|---|
| **P1** | `delta_X` is a full `[unit.…]` entry; same dimension, same `factor`, no `offset`. `delta` is a one-way pointer, validated at load | Explicit; a reader of the registry sees every symbol that can appear in a canonical form |
| **P2** | The loader synthesizes `delta_X` from `X` | Less registry text, but synthesizes symbols that then appear in canonical strings and on the wire. A symbol nobody wrote is hard to audit and impossible to grep for |
| **P3** | No delta symbols; route conversion off the quantity kind's `interval` flag | **Eliminated** — §6.4 names `delta_degC` as a unit, and §5.2 tags columns with units, not kinds |

### Two sub-questions the register did not separate

**Is `delta` symmetric?** Conversion does not need the reverse pointer. But
§6.4's rule — "a kind with `interval: true` MUST use the delta unit" — needs to
answer "is this unit a delta?" given only the unit. Cheapest answer with no new
field: a unit is a delta iff some unit names it in `delta` and it does not
itself carry an `offset`. That is a derived index built at load, not a
declaration.

**What is a non-affine unit's delta?** `K`'s delta is `K`. Making `delta`
optional and defaulting to self for any unit without an `offset` is the
smallest rule and is what `conformance-core.toml` already assumes (it states
`delta = "K"` explicitly, because the spec does not say which).

### Recommendation

**P1**, with `delta` optional (defaulting to self for non-affine units) and the
delta-of index derived at load rather than declared.

## Staged artifacts — Cluster G

**Spec §6.4**, replacing the first bullet:

> - convert between each other through the dimension's base unit:
>
>   ```
>   base = x * factor_from + offset_from
>   y    = (base - offset_to) / factor_to
>   ```
>
>   `offset` is an exact rational expressed **in the dimension's base unit**,
>   so offsets are directly comparable across units of the same dimension. A
>   unit without an `offset` is not affine and `offset` is 0.

**Spec §7.2**, adding to the unit schema:

> `offset` — exact rational intercept, in the dimension's base unit (§6.4).
> Presence makes the unit affine.
>
> `delta` — the symbol of this unit's multiplicative counterpart. The named
> unit MUST exist, MUST share this unit's `dimension` and `factor`, and MUST
> NOT carry an `offset`; all three are load-time errors. `delta` MAY be omitted
> for a unit with no `offset`, in which case it is the unit itself.

Add `degF` to §7.2's example, since it is the entry that makes the convention
unambiguous:

```toml
[unit.degF]
dimension = "temperature"
factor = [5, 9]
offset = [45967, 180]          # exact rational offset, in kelvin
delta = "delta_degF"
```

**Fixtures:** `parsing.affine.007`, `.008` → `normative` (they depend only on
AMB-018, not on the offset convention). The affine *conversion* fixtures are
still unwritten and land in `factors/` at M1 — AMB-016 is what unblocks them.

**Registry:** under O2, `conformance-core.toml` changes
`[unit.degF].offset` from `[45967, 100]` to `[45967, 180]`, and the AFFINE
UNITS comment block is rewritten to the O2 formula. `degC`, `K`,
`delta_degC`, and `delta_degF` are unchanged — degC's `[27315, 100]` is the
same number under both conventions, and the deltas carry no offset.

---

# Cluster H — What counts as "compound"? (AMB-017)

### Evidence

> any compound expression containing an affine unit is invalid
> (`E_AFFINE_COMPOUND`)

"Compound" is never defined. The undecided cases are not exotic:

| Input | Under Q1 | Under Q2 |
|---|---|---|
| `degC` | valid | valid |
| `degC^1` | valid | valid |
| `degC^2` | error | error |
| `degC/degC` | **error** | **valid**, dimensionless |
| `degC*h/h` | **error** | **valid**, canonical form `degC` |

### Options

| | Rule | |
|---|---|---|
| **Q1** | Check the **parsed term list, before** §6.3 step 2's merge. Valid iff exactly one term whose symbol is affine and whose exponent is 1 (written or implicit) | Recommended |
| **Q2** | Check **after** merging | `degC*h/h` returns success with canonical form `degC` — a temperature that was computed through a product, which is the thing the rule forbids |
| **Q3** | Check after merging, but specially forbid an affine symbol that cancelled | Ad hoc, and still lets `degC*h/h` through |

### Why the ordering is the whole question

Under Q2 the two cancelling cases pass, and they are the ones that matter.
`degC/degC` is harmless in isolation but means an implementation is willing to
carry an affine symbol through multiplication. `degC*h/h` is the same
willingness with a visible consequence: the result is `degC`, but it was
produced by multiplying a Celsius reading by hours and dividing again, and
whether that is meaningful depends on whether the intermediate was ever
materialized.

Q1 also produces better diagnostics: the error names the string the user wrote,
not a merged form they never saw.

### Recommendation

**Q1.** Note the consistency check with §8.1: multiplying a `degC` column by an
`h` column yields "the canonicalized symbolic product", i.e. `degC*h`, which is
`E_AFFINE_COMPOUND` under Q1 — the string rule and the arithmetic rule agree,
which they would not under Q2.

## Staged artifacts — Cluster H

**Spec §6.4**, replacing the second bullet:

> - are **non-composable**. A unit string containing an affine symbol is valid
>   only when it consists of exactly one term, whose exponent is 1 whether
>   written or implicit. This is checked against the parsed term list **before**
>   the merge in §6.3 step 2, so `degC/degC` and `degC*h/h` are
>   `E_AFFINE_COMPOUND` even though they would cancel. The same rule applies to
>   units produced by §8.1 arithmetic.

**Fixtures:** `parsing.affine.003`, `.004`, `.005`, `.006` → `normative`.
`.003` also depends on decision 8 (`^1` lexes).

**Registry:** unchanged.

---

# Cluster I — Mode at read time (AMB-020, AMB-021)

## AMB-020 — §6.3 conditions behavior on a mode that does not exist yet

### Evidence

§6.3:

> Readers MUST accept non-canonical input in `permissive` mode (normalizing
> internally, tainting nothing) and MUST reject unresolvable symbols in
> `strict` mode (`E_UNKNOWN_UNIT`).

§8.4:

> Mode is a property recorded at finalization (companion `publish()`) —
> **working tables have no mode.**

So the normative reader behavior is selected by a property that is absent for
the artifact readers most often encounter.

### The decomposition that actually fixes it

§6.3's sentence conflates two independent axes, and separating them removes
most of the problem:

| Input condition | strict | permissive | untracked |
|---|---|---|---|
| **Non-canonical but resolvable** (`h*MW`) | accept, normalize, no taint, no warning | same | same |
| **Unresolvable symbol** (`Zorkmid`) | reject, `E_UNKNOWN_UNIT` | AMB-021 | silence |

The first row is mode-independent. §6.3 says "readers MUST accept
non-canonical input in permissive mode", which invites the reading that strict
readers reject it — but nothing supports that, and it would contradict the same
sentence's "Readers MUST accept". Non-canonical input is accepted everywhere;
only *writers* are constrained to emit canonical form (§5.2).

Once that is separated, the only thing needing a default is the second row.

### Options

| | Default when no mode is recorded | |
|---|---|---|
| **R1** | `permissive` | Recommended — warns rather than failing, and preserves data |
| **R2** | `untracked` (silence) | §1 goal 5's "untagged tables flow through silent" is about *untagged* columns. This row is about a tagged column with an unusable string, where silence hides a real defect |
| **R3** | No default; every parsing API takes an explicit mode | Honest, but adds a parameter to every call site and pushes the decision onto callers who have no basis for it |

### Recommendation

**R1**, plus the two-axis restatement, which is the more valuable half.

---

## AMB-021 — permissive plus an unresolvable symbol

### Evidence

§6.3 pairs "accept non-canonical in permissive" with "reject unresolvable in
strict" and never states the fourth cell.

### Options

| | Behavior | |
|---|---|---|
| **S1** | Warn once per column, mark the column tainted (§8.2), preserve the original string verbatim | Recommended |
| **S2** | Accept and pass the string through opaquely, no dimension available | Indistinguishable from S1 except that it does not taint — which would let an unusable unit flow into checked arithmetic unflagged |
| **S3** | Discard the metadata; treat the column as untagged | **Eliminated** — loses data §5.2 requires preserved, and destroys the evidence of what the producer meant |
| **S4** | Reject in all modes | Not an alternative to S1; see below |

### S4 is not a competing option — it is a different layer

The cleanest resolution splits parser from reader:

- the **parser** always fails on an unresolvable symbol, in every mode. It has
  one job and no policy;
- the **reader** catches that failure and applies mode policy: strict
  propagates `E_UNKNOWN_UNIT`, permissive warns once and taints, untracked is
  silent.

Under that split S4 describes the parser and S1 describes the reader, and they
are complementary rather than exclusive. This is the layering
`parsing.errors.002` already asserts, and it is why that fixture expects
`E_UNKNOWN_UNIT` even with `mode: "permissive"`.

The warning needs a code: `W_UNRESOLVED_SYMBOL`, mirroring §10's convention.

### Recommendation

**S1 at the reader, S4 at the parser**, stated as an explicit layering.

## Staged artifacts — Cluster I

**Spec §6.3**, replacing the closing paragraph:

> Writers MUST emit canonical form.
>
> Parsing and mode are separate layers. The parser is mode-independent: it
> accepts any well-formed input, normalizes it, and fails with
> `E_UNKNOWN_UNIT` on a symbol that does not resolve. Readers apply mode
> policy to that failure:
>
> | | `strict` | `permissive` | `untracked` |
> |---|---|---|---|
> | Non-canonical but resolvable | accept and normalize; no taint, no warning | same | same |
> | Unresolvable symbol | `E_UNKNOWN_UNIT` | `W_UNRESOLVED_SYMBOL` once per column; the column is tainted (§8.2) and the original string is preserved verbatim for round-trip | silent; column tainted |
>
> A table with no recorded mode (§8.4 — every working table) is read as
> `permissive` unless the caller requests otherwise.

**Spec §10**, add `W_UNRESOLVED_SYMBOL` to the warnings sentence.

**Fixtures:** `parsing.errors.002` → `normative`.

**Registry:** unchanged.

---

# Cluster J — Metadata: codes, preservation, encoding (AMB-014, AMB-024, AMB-032, AMB-033)

Four entries, one theme: what the metadata layer guarantees. All four block
`wire/`, none blocks M0 or M1.

## AMB-014 — no code for malformed metadata

§5.2 makes `unit` and `grammar` required and says nothing about an absent blob,
invalid UTF-8, invalid JSON, a JSON scalar where an object is required, or a
missing required key. **Add `E_BAD_METADATA`**, covering all of them, naming
the offending key. Same §10 edit as decisions 11 and 14.

## AMB-024 and AMB-032 — preservation

§5.2's "Unknown keys MUST be preserved on round-trip" governs the extension
metadata **blob**. It does not reach `unitarrow:display_name`,
`unitarrow:description`, or `unitarrow:references`, which §5.5 deliberately puts
outside the blob. Nothing else reaches them either — yet all of them are inside
the companion's data digest, so dropping one breaks every downstream seal and
surfaces as "this table was modified after it was published" pointing at an
innocent consumer.

### The key-order sub-decision, which is the interesting one

§5.2 says keys are "preserved". Preserved *in order*, or merely present? It
matters because the digest is over serialized bytes, so two producers writing
the same logical metadata in different key order produce different digests for
identical data.

| | Rule | |
|---|---|---|
| **T1** | Preserve author order | Literal reading of "preserved". Digest depends on authoring order — an implementation that round-trips through a hash map silently changes it |
| **T2** | Canonicalize: serialize keys sorted, bytewise | Digest is a function of content alone. Costs the word "preserved" its literal meaning for order, which is worth it |

T2 is what companion §4's own digesting policy asks for — *"wherever a
canonical form exists, digests are computed over it, never over author bytes …
immune to formatter churn by construction."* The metadata blob has no canonical
form today; defining one makes that sentence true for tables as well as for
unit strings and plans.

**Recommend T2**, and extend the preservation rule to every `unitarrow:*` key.

### Merge

§5.6 defines merge for `mode` only. For the §5.5 keys:

- `description` — **drop on conflict.** Matches §5.5's own stance on derived
  columns: honest emptiness over prose that may now be wrong.
- `references` — **union, deduplicated by `url`.** Losing a methodology link
  because two tables were concatenated is pure loss, and references are
  additive by nature.
- `display_name` — drop on conflict, same reasoning as `description`.

## AMB-033 — `references` encoding

"a list of `{url, description, digest?}` entries", with no serialization, no
required/optional marking, no digest format, and no behavior on a malformed
blob. Also a naming trap: `references[].description` and the field-level
`unitarrow:description` are different things one word apart. And the field is
called `url` while the prose recommends DOIs — a DOI is not a URL.

**Recommend:** UTF-8 JSON array; `url` required and constrained to an absolute
URI (which admits `doi:10.1234/foo` as a scheme and resolves the tension);
`title` required (renamed from `description` to kill the collision); `digest`
optional and self-describing per companion §6's `sha256:` convention.
Malformed → `E_BAD_METADATA`.

## Staged artifacts — Cluster J

**Spec §5.2**, replacing the closing sentence:

> Unknown keys MUST be preserved on round-trip and ignored otherwise. This
> requirement extends to every `unitarrow:*` field and schema metadata key
> (§5.5), which are outside this blob but inside the data digest that seals it
> (companion §5.2). On serialization the blob's keys are emitted sorted
> bytewise, so the digest is a function of content and not of authoring order.

**Spec §5.5**, replacing the `references` paragraph's schema sentence:

> … as a UTF-8 JSON array of `{url, title, digest?}` entries. `url` is a
> required absolute URI — `doi:10.1234/foo` and `https://doi.org/10.1234/foo`
> are both valid, and durable identifiers are RECOMMENDED. `title` is required.
> `digest` is optional and self-describing (`sha256:…`). A malformed value is
> `E_BAD_METADATA`.

**Spec §5.6**, adding to the merge rules:

> `unitarrow:display_name` and `unitarrow:description` are dropped when the
> inputs disagree; `unitarrow:references` is the union of the inputs,
> deduplicated by `url`.

**Fixtures:** none yet — `wire/` is empty. These decisions are what let it be
written.

---

# Cluster K — Descriptions: propagation and size (AMB-038, AMB-031, AMB-029)

AMB-038 is new and must be settled before AMB-031, because it decides one of
AMB-031's rows.

## AMB-038 — the two documents disagree about conversion — NEW

### Evidence

Spec §5.5:

> derived columns (**converted**, integrated, renamed) get no description
> unless one is explicitly authored — honest emptiness over stale prose

Companion §5.2:

> byte change is not meaning change (**a unit conversion changes every byte
> while the description stays true**; a small value correction changes meaning
> while the bytes barely move)

§5.5 drops the description on conversion. The companion uses conversion as its
worked example of a transformation through which the description *stays true* —
and that example is load-bearing, because it is the justification for never
wiping descriptions automatically.

This is a direct contradiction, not a gap: the same operation, opposite
outcomes, in documents published together.

### Options

| | Rule | |
|---|---|---|
| **U1** | Conversion **keeps** the description; §5.5's list is wrong | Recommended. The companion's reasoning is correct — MW → kW does not change what the column is a description of |
| **U2** | Conversion **drops** it; the companion's parenthetical is wrong | Then the companion loses its clearest illustration, and users lose documentation on every unit toggle — which is the single most common operation the library performs |
| **U3** | Conversion keeps it only when the quantity kind is unchanged | Sounds principled; a conversion never changes the quantity kind, so it reduces to U1 with extra words |

### Recommendation

**U1.** Remove "converted" from §5.5's derived list, and note the reason
inline so it does not get re-added.

While there: **"renamed" is questionable too.** Renaming changes no data and no
meaning; it usually reflects a naming convention, not a repurposing. The
conservative case for dropping is that a rename *sometimes* signals reuse of a
column for a different purpose — but that is equally true of a filter, which
the list does not mention. Recommend removing `renamed` as well, leaving the
derived set as the operations that genuinely produce a different quantity.

---

## AMB-031 — "passes through unchanged" is undefined

### Evidence

`rename` is listed as derived even though it changes no data, so "unchanged" is
not about bytes. And the companion explicitly rules bytes out from the other
side. Neither identity criterion is available, and the rule is stated in terms
of one of them.

### The fix: a table over the plan IR

Companion §4's operation vocabulary is closed and short, which makes this
completely decidable. Proposed, incorporating AMB-038:

| Plan-IR operation | Description survives | Why |
|---|---|---|
| conversion | **yes** | Same quantity, different scale (AMB-038) |
| filter | **yes** | A subset of the same rows |
| project, no rename | **yes** | Identity |
| project with rename | **yes** | Same data, same meaning |
| concat | **yes iff all inputs' descriptions are identical**, else none | Consistent with §5.6's drop-on-conflict |
| join, carried-through columns | **yes** | The column is unchanged; only its neighbours differ |
| arithmetic (`a+b`, `a*b`, …) | no | A new quantity |
| aggregate | no | A new quantity over a different row space |
| `integrate` / `differentiate` | no | A new quantity, and the dimension changes |

Eight rows settle it completely and are directly testable in `type-functions/`.

Note what the table does *not* need: any notion of "unchanged". Each row is a
decision about a named operation, which is the only form an implementation can
actually check.

### Recommendation

Adopt the table; delete the "passes through unchanged" phrasing entirely.

---

## AMB-029 — the size norm has no number

Spec §5.5 invokes "the size norm"; companion §5.2 step 5 says "a generous
per-field threshold warns in permissive mode and fails in strict". Neither
states a value, a unit, or a scope — and "free-text descriptions **and
similar**" does not say what "similar" covers.

The unit matters more than it looks: bytes, Unicode scalars, and grapheme
clusters differ by roughly 3× for non-Latin prose, so a limit stated as
"characters" penalises a Japanese description against an English one.

### Recommendation

| Key | Limit |
|---|---|
| `unitarrow:description` (field and schema) | 4 KiB |
| `unitarrow:references` (whole value) | 16 KiB |
| `unitarrow:display_name` | 256 B |
| All `unitarrow:*` keys, per schema, summed | 64 KiB |

Measured in **UTF-8 bytes**. Over the limit: `W_METADATA_SIZE` in permissive,
`E_METADATA_SIZE` in strict (decision 28). Numbers belong in the spec, not the
registry — they are a wire-format norm, not registry data.

## Staged artifacts — Cluster K

**Spec §5.5**, replacing the propagation sentence:

> Whether a transformation carries a column's description is decided per
> operation, not by any notion of the column being "unchanged" — a unit
> conversion changes every byte while leaving the description true, and a small
> value correction changes the meaning while barely moving the bytes. Over the
> plan IR (companion §4): conversion, filter, project (with or without rename),
> and carried-through join columns keep the description; arithmetic, aggregate,
> `integrate`, and `differentiate` produce a new quantity and get none;
> `concat` keeps it only when every input's description is identical. Derived
> columns get no description unless one is explicitly authored — honest
> emptiness over stale prose.

Plus the size limits table, and the `references` schema from Cluster J.

**Companion §5.2 step 5**, replacing "a generous per-field threshold":

> … lints metadata size against the limits in UnitArrow §5.5: `W_METADATA_SIZE`
> in permissive mode, `E_METADATA_SIZE` in strict.

**Fixtures:** none yet; unblocks `wire/` and `type-functions/`.

---

# Cluster L — Column digests and republication (AMB-035, AMB-030, AMB-034)

## AMB-035 — "canonically serialized" is undefined for buffers

Companion §5.4: column digests are computed "over its data — validity and value
buffers, **canonically serialized**". No canonicalization is given, and Arrow
offers several ways for logically identical columns to differ physically: a
sliced array carries an offset into a longer parent buffer; dictionary-encoded
and plain encodings of the same strings; a validity buffer that is absent
versus present-and-all-true; 64-byte padding whose contents are unspecified;
big-endian hosts.

This is not cosmetic. Both features that consume these digests — the
what-changed report (§5.4) and the republish disposition flow (§5.2) — key off
digest **equality**, so an unstable digest produces spurious
`E_DESCRIPTION_STALE` prompts and a report that flags untouched columns.

| | Approach | |
|---|---|---|
| **V1** | Digest **logical values**: a canonical value encoding independent of Arrow physical layout — little-endian fixed-width, length-prefixed variable-width, a distinguished null marker, no padding | Recommended. Stable across slicing, dictionary encoding, and null representation, which is what the feature means |
| **V2** | Normalize the **physical layout** before digesting: materialize slices to offset 0, decode dictionaries, canonical null representation, zero-filled padding, little-endian | Same outcome by a longer route, and every new Arrow layout is a new normalization rule |

**Recommend V1**, spelled out per Arrow type.

## AMB-030 — codes for the new strict-mode failures

Companion §5.2 introduces two conditions that "fail in strict"; §8's taxonomy is
unchanged. The second is not a variant of `E_SEAL_BROKEN` — nothing is broken,
the publisher is being asked a question, and reusing a code makes the
remediation ("rewrite, or explicitly keep") unreachable from the code alone.

**Recommend** `E_METADATA_SIZE`, `E_DESCRIPTION_STALE`, and the permissive-path
warnings `W_METADATA_SIZE` and `W_DESCRIPTION_STALE`.

## AMB-034 — a prose fix is operationally a data correction

The two digests disagree about metadata, both deliberately: the data digest
includes it (or descriptions would be forgeable), the column digests exclude it
(or they could not localize data changes). Together, fixing a typo breaks the
seal, mints a new `data_digest`, and requires a republication with a
`supersedes` chain — while every column digest is unchanged, so the
what-changed report says nothing changed.

**Recommend** stating the asymmetry in §5.5, and adding `documentation-only` to
the republish dispositions so the what-changed report can say what actually
happened. Without it, publishers are discouraged from fixing prose, which is
the opposite of what §5.5 is for.

## Staged artifacts — Cluster L

**Companion §5.4**, replacing the `column_digests` sentence:

> `column_digests` (optional) are computed per column over its **logical
> values**, not its Arrow buffers: fixed-width values little-endian,
> variable-width values length-prefixed, nulls as a distinguished marker, no
> padding, and **concatenated across record batches in order** — the digest is
> a function of the column's values alone, independent of slicing, dictionary
> encoding, null-buffer representation, and chunking. Re-batching a table
> (8760 rows in one batch vs 365 daily batches) MUST NOT change any column
> digest. Field metadata is excluded — the seal
> signature already covers it (§5.5, and the asymmetry noted there).

**Companion §8**, four new rows: `E_METADATA_SIZE`, `E_DESCRIPTION_STALE`,
`W_METADATA_SIZE`, `W_DESCRIPTION_STALE`.

**Companion §5.2**, adding to the republish paragraph:

> Dispositions are `rewrite`, `keep`, and `documentation-only` — the last
> asserting that only prose changed, which the what-changed report surfaces as
> such.

**Fixtures:** none yet; unblocks `companion/seals/`.

---

# Cluster M — Kind and temporal edges (AMB-019, AMB-025, AMB-022)

## AMB-019 — "or vice versa" — recommendation reversed

### Evidence

§10: `E_INTERVAL_MISUSE` is "Absolute unit on an `interval: true` quantity kind
(**or vice versa**)". §7.3 offers only `interval = true`; its absence is the
absence of a claim, exactly as untagged is for units (§5.3) and as §8.5 rule 4
requires for temporal. So a checker has no basis for the reverse direction, and
`cf:air_temperature` — which declares only `parent` — cannot reject
`delta_degC`.

### Why the register's recommendation was wrong

The register recommends deleting "or vice versa" on the grounds that "the
dangerous bug is one-directional". Working the arithmetic shows it is not:

- **Forward** — a temperature *difference* kind tagged `degC`: 20 converts to
  68 °F instead of 36. This is §6.4's stated motivating bug.
- **Reverse** — an *absolute* temperature tagged `delta_degC`: 20 converts to
  36 °F instead of 68.

Both are wrong by the same offset, in opposite directions, and neither is
detectable by dimensional analysis. Deleting the clause discards half the guard
that §6.4 exists to provide.

### Options

| | Approach | |
|---|---|---|
| **W1** | Add explicit `interval = false`. Absence still means no claim; only an explicit `false` triggers the reverse check | Recommended. Three-valued and honest, consistent with §8.5 rule 4 |
| **W2** | Delete "or vice versa" | The register's original. Simpler, catches half the bug |
| **W3** | Treat absence as `false` | **Eliminated** — contradicts §5.3 and §8.5 rule 4's shared principle that absence is not a claim |

### Recommendation

**W1.** The `core:` namespace should then declare `interval = false` on the
absolute temperature kinds, which is where the value actually lands.

## AMB-025 — non-fixed-length periods

`temporal.period` is "ISO-8601" unrestricted, so `P1M` and `P1Y` are legal —
but §8.5 rule 1 defines `integrate` as value × period, and a month has no fixed
length without a calendar and an anchor. Rule 2 compares periods for equality,
and `P1M` versus `P30D` is neither clearly equal nor clearly unequal.

**Recommend:** restrict to fixed-length designators (`PnW`, `PnD`, `PTnH`,
`PTnM`, `PTnS` and combinations), excluding `Y` and `M`; compare by normalized
total seconds as an exact rational, so `PT60M` equals `PT1H`. Calendar-relative
periods need an anchor timestamp and are a separate feature — §12, not a patch.

## AMB-022 — `pu`

Already argued in the register and unchanged here: `pu` is **dimensionless**
(not a new base dimension — §6.2's list is frozen and extending it for a §5.4
feature is the wrong trade), **non-composable** like an affine unit, and
`E_BASE_MISSING` is raised by *metadata validation*, never by the unit-string
parser, which has no access to `base`.

The consequence worth stating in §5.4: a `pu` column and a `"1"` column are
dimensionally equal and will pass a dimension check against each other.
Anything stronger has to come from the quantity kind.

## Staged artifacts — Cluster M

**Spec §7.3**, adding to the flat-fields list:

> `interval` — `true` when the kind denotes a difference and MUST use the delta
> unit; `false` when it denotes an absolute value and MUST NOT. Absence is the
> absence of a claim and is not equivalent to `false` (§5.3, §8.5 rule 4).

**Spec §5.2**, constraining `period`:

> … and ISO-8601 `period`, restricted to fixed-length designators (`PnW`,
> `PnD`, `PTnH`, `PTnM`, `PTnS`, and combinations). `Y` and `M` designators are
> not permitted: they have no fixed length, so `integrate` is undefined for
> them. Periods compare by normalized total seconds as an exact rational.

**Spec §5.4**, adding:

> `pu` is dimensionless and non-composable: a compound expression containing it
> is `E_AFFINE_COMPOUND`. `E_BASE_MISSING` is raised by metadata validation,
> not by the unit-string parser. A `pu` column and a `"1"` column are
> dimensionally equal; distinguishing them is the quantity kind's job.

**Fixtures:** `parsing.dimensionless.005`, `.006` → `normative`.

**Registry:** `conformance-core.toml` adds `interval = false` to
`core:temperature`.

---

# Cluster N — Document hygiene (AMB-011, AMB-012, AMB-023, AMB-027, AMB-028)

No fixture depends on these; each is a short editorial decision.

| | Finding | Recommendation |
|---|---|---|
| **AMB-011** | `variants` semantics undefined | Non-normative cross-reference only: each variant MUST have its own full `[unit.…]` entry, canonicalizes as itself, and is commensurable-but-unequal to its parent. No inheritance. Say so in §7.2 |
| **AMB-012** | Canonical form does not reduce across unit choices | Closed by Cluster A's §6.3 note. No separate edit |
| **AMB-023** | Conformance category numbering collides three ways | Renumber in the 1.0-rc pass; until then cite categories by name. Spec §11 gains `6. Seals` and `7. Plans` or drops numbers entirely — the latter is cleaner, since the companion has its own list |
| **AMB-027** | §7.2's example is not valid TOML (multi-line inline tables) | Reformat to single-line inline tables or `[unit.X.display]` sub-tables. Separately, define what "canonicalized file" means in §7.1's hash comment — most likely a normalized re-serialization, matching companion §4's digesting policy. Add a CI step that parses every ```toml``` block |
| **AMB-028** | Both documents claim "exactly three" interfaces while cross-referencing a fourth | Enumerate it as interface 4 in spec §9 and companion §11 — documentation staleness ↔ column digests and the republish disposition — and give the companion a type-system-agnostic phrasing so a units-free implementation has no dangling pointer |

---

# Cluster O — Escapes and code points (AMB-039 DECIDED, AMB-040 open)

## AMB-039 — units: prohibit escapes. **Decided.**

### Why it is the right call, stated once

§6.3 buys one *spelling* per unit expression. §5.2 then hands that spelling to
JSON, which gives every string many *byte encodings*: `"MW"`, `"\u004dW"`,
`"\u004D\u0057"`. `MW/h` can additionally be written `MW\/h` — JSON permits an
escaped solidus, and several writers emit one by default.

Canonical form is only canonical if the encoding layer is too. Without this
rule, two producers tagging bit-identical columns can emit different
`data_digest`s (companion §5.2 step 3), and a reader that merely re-serializes
changes the digest without changing the data — surfacing to the next consumer
as "this table was modified after it was published."

### The cost is zero, and that is the unusual part

JSON mandates escaping only `"`, `\`, and U+0000–U+001F. Under AMB-005's ASCII
symbol class a unit string may contain only `A-Za-z0-9_@` and `^ * / - 1`.
The two sets do not intersect: **no character that can legally appear in a unit
string requires an escape.** The prohibition removes a whole class of encoding
ambiguity without removing a single expressible unit.

It also closes routes the grammar has no opinion on, because it never expected
to see them: control characters (`\u0001` — impossible to write literally in
JSON, so escapes are the only way in), unpaired surrogates (`\ud800`, valid
JSON, invalid UTF-8 once decoded), and a BOM.

### Layer, and the one thing to watch

The check is at the **metadata** layer, not the unit parser: detecting an escape
requires the raw JSON token, and a JSON parser decodes before the unit string
exists. So the code is `E_BAD_METADATA`, and `E_UNIT_SYNTAX` keeps its meaning
of "this is not a unit". `parsing.encoding.005` pins the ordering — `"M\\W"`
decodes to `M\W`, which is *also* a grammar violation, and the metadata layer
must reject it first.

**Coupling worth noting:** the prohibition is *free* only under AMB-005's ASCII
class. Under a Unicode symbol class it still holds but stops being sufficient,
because NFC/NFD would reintroduce several byte encodings of one rendered symbol.
That is a third independent argument for keeping symbols ASCII, alongside
"AMB-003 becomes free" and "no Unicode tables in the WASM build."

### Fixture-format consequence

A fixture file is itself JSON, so `"\u004dW"` in a fixture decodes to `MW`
before any runner sees it — **the format could not express these cases.**
`input.unit_json` was added: the raw JSON token including its quotes, with
backslashes doubled. A case carries `unit` or `unit_json`, never both. Cases
about the *decoded* value keep using `unit`; only cases whose outcome depends on
the byte encoding use `unit_json`.

### Applied

`fixtures/parsing/08-encoding.json`, 8 cases, 7 `normative` and 1 provisional
(`parsing.encoding.008`, a literal BOM — outside AMB-039's reach because it is
not an escape, so it depends on AMB-005 and AMB-013 instead). Suite is now **71
cases**, and the reference canonicalizer reproduces all of them.

**Spec §5.2**, appended:

> The `unit` value, and every other unit-valued field, MUST be written without
> JSON escape sequences: a `\` anywhere in the raw token is `E_BAD_METADATA`.
> Every character permitted in a unit string (§6.1) is representable literally,
> so this constrains encoding and never expression. Canonical form (§6.3) fixes
> one spelling per unit; this fixes one byte sequence per spelling, which is
> what the data digest and cross-language hashing actually require.

---

## AMB-040 — the rest of the metadata. **Open, and it is two questions.**

"Prohibit escape characters" names only one of them, because the carriers
differ in whether an escape syntax exists at all. `unitarrow:display_name` and
`unitarrow:description` are *plain Arrow metadata strings* (§5.5), not JSON —
raw UTF-8 map values with no escape layer to prohibit. Everything else rides in
JSON or TOML.

### Question 1 — escapes, where a syntax exists

| Field | Content | Prohibit? |
|---|---|---|
| `unit`, `base.unit` | unit strings | **Decided: yes** (AMB-039) |
| `grammar`, `temporal.kind` / `.statistic` / `.period` | enumerated + ISO-8601 | Yes — all ASCII |
| `quantity` | CURIE | Yes *if* CURIEs are ASCII — **their character class is undefined**, a sub-gap this exposes |
| `references[].url`, `.digest` | absolute URI, `sha256:…` | Yes — RFC 3986 URIs are ASCII by construction |
| `unitarrow:provenance` | mode, versions, hashes, timestamp | Yes — all ASCII |
| seal (companion §5.3) | key_id, base64, digests, timestamp | Yes, and it matters more here: the signature is over canonical JSON |
| seal `publisher`, `references[].title` | human-facing | **No** — may need `\"` |
| `unitarrow:description` | prose | **No** |
| registry `display.*` | LaTeX and similar | Only by switching to TOML *literal* strings — §7.2 already carries an escaped backslash in `display.latex` |

The rule that falls out: **prohibit escapes in every machine-readable value;
permit them only in free prose, where a quotation mark legitimately appears.**

The registry row is the awkward one. `display.latex` genuinely needs
backslashes, so either it moves to a TOML literal string (`'^{\circ}C'`, which
cannot then contain a single quote) or escapes stay and the registry hash
inherits the ambiguity. This ties to AMB-027, which already needs §7.1 to define
what "canonicalized file" means — settle both together.

### Question 2 — permitted code points, where no escape syntax exists

This is the larger exposure, it applies to exactly the two prose fields, and
nothing constrains it today:

- **Control characters.** A JSON-carried value cannot hold one without an
  escape. A raw Arrow metadata string has no such gate, so a `display_name`
  containing U+0007 or a stray CR is legal and will break terminal output, CSV
  export, and log lines.
- **Bidi overrides** (U+202A–U+202E, U+2066–U+2069) and **zero-width
  formatters** (U+200B–U+200D) make a string render as something other than what
  it contains. These strings appear on badges and provenance surfaces, which
  companion §5.1 makes normative for UX — so this is a display-spoofing surface
  with a *signature over it*, which is worse than an unsigned one: the seal
  attests to bytes the reader cannot see.
- **Non-NFC forms** — the quiet one, and the most likely to actually bite.
  Descriptions are inside the data digest, so a table that passes through any
  normalizing layer emerges with a different digest and a broken seal, reported
  as "modified after publishing" while the prose looks identical on screen.
  Nobody will diagnose that from the error message.
- **Byte-vs-character counting** for AMB-029's size limits stays ambiguous while
  escapes are permitted: `\u00e9` is six bytes as JSON and two as UTF-8. The
  limits are stated in UTF-8 bytes, so escapes must at minimum be counted
  post-decode.

### Options

| | Scope | |
|---|---|---|
| **X1** | No constraint | Status quo. Leaves the NFC seal-break and the bidi surface open |
| **X2** | Forbid C0/C1 controls, unpaired surrogates, bidi and zero-width formatters; allow LF in `description` only | Closes the display and terminal hazards. Cheap: a code-point range check, no Unicode tables |
| **X3** | X2 **plus** require NFC | Also closes the seal-break. Costs a normalization dependency — significant in the WASM build, which is why it deserves a separate decision from X2 |
| **X4** | Restrict prose to a printable ASCII subset | **Not recommended.** §5.5 exists to make tables self-documenting, and non-English documentation is a first-class case |

### Recommendation

**Question 1: prohibit escapes in every machine-readable value**, exactly as
AMB-039 does for units. It is free everywhere it applies, and it makes the
seal's "canonical JSON" phrasing (companion §5.3) mean something concrete.

**Question 2: X2 now, X3 when the cost is measured.** X2 is a range check and
closes the spoofing and terminal hazards immediately. X3 is the one that
prevents a silent seal break, but requires a normalization table in
`unitarrow-core` and therefore in the WASM module, against a budget the roadmap
puts at "tens of KB" — so it needs a size measurement before it is committed to,
not a decision in the abstract.

Note that X3 becomes cheaper if prose is *validated* as NFC rather than
*normalized* to it: rejecting non-NFC input needs a quick-check table, which is
much smaller than full normalization data. That is probably the right shape.

Decisions 34–40 are in the checklist at the top of this memo.

---

# Cluster P — Wide tables (AMB-041, AMB-042)

## The shape

An hourly generation table: one timestamp column, one `MW` column per
generator, hundreds to thousands of them, one unit across all of them. Common
enough in energy systems to be the default case rather than an edge case, and
the roadmap's own dogfood target (GAT, Meridian) works on exactly this data.

Two costs, and they need different answers.

## Cost 1 — authoring, which is the one that decides adoption

The only interface either document describes is one unit per column. For this
table that is a dict literal with N identical values, regenerated whenever the
generator set changes.

**This is what M3's gate fails on.** *"After three weeks of normal GAT use,
columns are still being tagged voluntarily"* does not survive an interface that
charges a line per generator, and the roadmap is explicit that failing M3 routes
work back to ergonomics rather than forward.

### The answer, and the thing to be careful about

Patterns at authoring time, expanded to per-field metadata by the writer:

```python
ua.tag(table, {"gen_*": "MW", "timestamp": None})
ua.tag(table, "MW", skip=["timestamp"])            # broadcast + exceptions
ua.tag(table, {"timestamp": None, ua.REST: "MW"})  # explicit remainder
```

**Nothing pattern-shaped reaches the wire.** That is the whole design: patterns
are a binding-level convenience over an unchanged format.

This is where the encoding decisions already made start paying for themselves.
A library generating 2000 identical metadata blobs from one pattern must produce
bytes that another implementation's expansion matches exactly, or every
`data_digest` diverges by producer. Canonical form (§6.3), no escapes
(AMB-039), and sorted JSON keys (AMB-024 option T2) are together what make
pattern expansion safe. Without them, the ergonomic feature would quietly break
the trust layer.

### And the non-goal that has to go with it

Someone will propose schema-level defaults — one `unitarrow:defaults` blob that
columns inherit. It is the obvious optimisation and it is wrong three times
over:

1. **Projection.** `select(["gen_0007"])` must keep its unit. A schema-level
   default survives no column selection in any engine, and projection is the
   most common thing anyone does to a wide table.
2. **Graceful degradation** (§1 goal 3). An unaware consumer reads *field*
   metadata; the promise is "a plain float column with human-readable metadata
   beside it". Under schema-level defaults it sees 2000 bare columns and one
   blob it cannot interpret.
3. **§5.3's tagged/untagged distinction.** Is an inheriting column tagged?
   Every rule in §8 — taint, concat, mode — keys off that, and neither answer
   is defensible.

Add it to §2 so it is rejected once instead of re-litigated. §2 is already the
declared scope tie-breaker.

### The second proposal that will follow it: enum unit codes

The other obvious shrink — replace `"unit": "MW"` with an integer code into the
registry — was measured and should be rejected too, on arithmetic before
principles:

| Per-column tag | Bytes | vs spec |
|---|---|---|
| Current (`{"grammar":1,"unit":"MW"}`) | 160 | — |
| Unit as int code (`{"grammar":1,"u":7}`) | 152 | **−8 (5 %)** |
| Int code + shortened extension name (`ua.q1`) | 132 | −28 (17.5 %) |

The unit string is **2 bytes of the 160**. The tag's cost is the extension
mechanism itself — metadata keys 44 B, extension name 21 B, flatbuffer framing
~70 B — so enum-ing the unit attacks the smallest slice. (If bytes ever
mattered enough to change identity encoding, the extension *name* is the
bigger lever, and even both together save 17.5 %.) Meanwhile the 2000
identical blobs are exactly what transport compression eats: the 424 KB tagged
schema message zlib's to **36 KB**.

And the costs are structural, not stylistic:

1. **Graceful degradation** (§1 goal 3). `"unit": "MW"` is the entire
   legibility story — an unaware consumer reads it with no UnitArrow anywhere.
   `"u": 7` is meaningless without the registry, converting every consumer
   into an aware consumer.
2. **Self-containedness** (§1 goal 2). An int code is only resolvable against
   one specific registry version; the table stops being a self-contained
   artifact and starts being a foreign key.
3. **Federation** (§7.4). Codes need an assignment authority. Domain
   vocabularies mint units on their own cadence in their own repositories —
   there is nobody to hand out non-colliding integers, and AMB-037 is hard
   enough for *strings*, which at least collide visibly.
4. **A second identity.** Canonical form (§6.3) is the cross-language equality
   and hashing primitive. A numeric alias creates a parallel identity that
   must be kept in bijection with it forever, and every place they can drift
   is a new AMB-016-shaped bug.

Where enums genuinely belong — and two of the three are already in the design:

- **In-memory interning** inside `unitarrow-core`: resolve symbols once at
  load, use integer handles internally, emit strings at the boundary. Pure
  implementation detail, never on the wire.
- **The codegen'd `onto.*` constants** (§4): enums at the *authoring* layer —
  typo-safe named constants that resolve to canonical strings before anything
  is written.
- **Authoring-side unit constants** in bindings (`ua.MW`), same idea one layer
  down. A typo'd constant is an import error; a typo'd string is
  `E_UNKNOWN_UNIT` at runtime.

The pattern in both rejected proposals is the same: the wire format is the
wrong layer to optimise, because its redundancy is what compression removes
for free and what legibility and federation depend on.

## Cost 2 — wire size, which is real but bounded

Measured, pyarrow 21, IPC stream, float64:

| Shape | Untagged | Tag only | + display_name + description |
|---|---|---|---|
| 2000 × 8760 (annual) | 140.4 MB | +320 KB (**0.2 %**) | +800 KB (0.6 %) |
| 2000 × 24 (daily slice) | 0.58 MB | +320 KB (**54.8 %**) | +800 KB (136.8 %) |
| 500 × 24 | 0.15 MB | +80 KB (54.6 %) | +200 KB (136.3 %) |
| 50 × 8760 | 3.58 MB | +8 KB (0.2 %) | +20 KB (0.6 %) |

160 bytes per tagged column; 400 with documentation.

The honest reading: **negligible on whole tables, dominant on small slices.**
The pathological case is many small independent messages — a service returning a
day at a time pays the whole schema on every response. Flight streams and
Parquet files send the schema once and amortise it away.

No format change is warranted. This is the price of per-field metadata, and
per-field metadata is what buys projection-safety and graceful degradation. But
say so in §1 goal 5, because "zero-cost drop-in" currently reads as a promise
that a 24-row slice does not keep.

## AMB-042 — the seal side of the same shape

`column_digests` carries 86 bytes per column — 172 KB on this table — inside
the signed envelope, but **once per artifact**, in schema metadata. Measured
across granularities: 0.12 % on the annual file, 0.10 % on the same file split
into 365 daily record batches (batching is free — batches never repeat schema
metadata), and 19 % per file only when every day becomes its own sealed
artifact. Even in that worst case the digests are the *third*-largest overhead:
each daily file is 2.8× its data, and the repeated tagged schema (320 KB/file,
117 MB/year) outweighs the digests (172 KB/file, 63 MB/year). The lever for
sliced publishing is artifact granularity, not the digest feature.

The same measurement surfaced a correctness requirement now folded into
Cluster L's staged text: digests must be **chunk-invariant**, or a no-op
re-batch changes every column digest and spuriously triggers the republish
disposition flow.

The sharper problem is the republish flow: strict mode *"refuses to seal without
an explicit per-column disposition"* for changed columns carrying inherited
descriptions. Re-run the generation model and every column changes — two
thousand dispositions, one at a time. Nobody will do that; they will drop to
permissive, which is the opposite of what the feature is for.

There is also a perverse incentive: `column_digests` is optional, and omitting
it makes the disposition prompt disappear. So the current design pushes wide-
table publishers away from digests precisely where a what-changed report would
be most valuable.

**Recommend:** bulk dispositions (glob, or all-affected-at-once), a documented
guideline for when omitting `column_digests` is reasonable, and a sentence in
§5.4 admitting that the what-changed report degrades to table granularity when
they are absent.

## Staged artifacts — Cluster P

**Spec §2**, new non-goal:

> - **No schema-level unit defaults.** A unit is always carried on the field it
>   describes. Table-level defaults that columns inherit would not survive
>   column projection, would be invisible to an unaware consumer reading field
>   metadata (§1 goal 3), and would leave §5.3's tagged/untagged distinction
>   undefined for inheriting columns. Bulk tagging is an authoring-interface
>   concern: bindings SHOULD accept patterns and expand them to per-field
>   metadata at write time.

**Spec §1 goal 5**, appended:

> Tagging is O(columns) in work and in bytes — roughly 160 bytes of schema
> metadata per tagged column. On a whole table this is immaterial; on a small
> slice of a very wide table it can exceed the data. That is the cost of
> per-field metadata, which is what makes units survive projection.

**Companion §5.2 / §5.4:** bulk dispositions and the `column_digests` guidance
above.

**Fixtures:** a `wire/` case tagging 1000 identical columns and asserting
byte-stable re-serialization — the regression test for pattern expansion being
deterministic across producers.

---

# Cluster Q — Guardrails for extensible catalogs (AMB-043)

## Why this is urgent rather than tidy

§7.4 makes federation the normal case, and deliberately: domain vocabularies
ship on their own cadence, reviewed by their own experts, with only the schema
and `core:` PR-gated. That means arbitrary third-party artifacts get loaded into
the resolution path that canonical form, dimension algebra, and every conversion
factor depend on.

The spec states **one** validation rule for those artifacts: *"prefix collisions
are a load-time error."*

### The trap that will be hardest to explain

§7.4 makes vocabularies **sealable**, and companion §11 interface 3 confirms the
seal machinery applies to them. A seal proves *attribution and integrity*. It
proves nothing about *conformance*.

So a vocabulary can be Verified — "published by a known org, unchanged since
publishing" — and still redefine `MW`. Validation must therefore run on every
artifact regardless of seal state, and its failure must **not** be phrased as a
trust failure. Companion §5.1 is already normative that user-facing surfaces
speak in claims rather than cryptographic nouns; the same discipline applies
here in reverse. "This catalog is signed by an organisation you trust and is not
a valid registry" is a sentence the software has to be able to say.

## The checklist

Normative load-time validation. Every check is cheap, and every one of them is
currently absent.

### Structural

1. Every canonical symbol and every alias matches the §6.1 `symbol` production.
   Rejects `M*W`, `MW h`, and the empty key — the §6.1 production constrains
   unit *strings* today and nothing enforces it on registry *keys*.
2. No canonical symbol is defined twice across the union of loaded artifacts
   (AMB-037).
3. No alias equals any canonical symbol or any other alias (AMB-010).
4. Every `dimension` reference resolves.
5. Dimension vectors use only the nine §6.2 base dimensions as keys, and every
   exponent is in i8.
6. No artifact redefines a base dimension name.
7. Under AMB-036's model: a *vocabulary* artifact MUST NOT declare `[unit.…]`
   or `[dimension.…]`.

### Numeric

8. `factor` is `[num, den]`, both integers, `den ≠ 0`, `num ≠ 0`, and
   `num/den > 0` — a zero or negative scale is never a unit.
9. `offset` is `[num, den]` with `den ≠ 0`.
10. Both are reduced to lowest terms, or normalized at load with a warning.
    Unreduced pairs overflow fast under exponentiation, and §7.2's whole
    auditability claim assumes a canonical numeric form.
11. Magnitude bounds chosen so composing factors across the i8 exponent range
    cannot overflow the implementation's integer width.

### Affine and delta

12. A `delta` target exists, shares the unit's `dimension` and `factor`, and
    carries no `offset` (AMB-018).
13. A unit carrying an `offset` has a `delta`, or the loader derives one.
14. No delta chains: a delta unit does not itself point elsewhere.

### Quantity kinds

15. `parent`, `opposes`, `balance`, `rate_of`, `same_as`, `broader`, and every
    `summable_with` entry resolve.
16. The `parent` and `same_as` graphs are acyclic. §7.3 says resolution is
    lookup with no inference engine — but `parent` is still a graph, and a cycle
    hangs a naive walker.
17. `interval` is boolean (and see AMB-019 on the third state).
18. `rate_of` relates kinds whose dimensions differ by exactly `time: 1`,
    otherwise §8.5's `integrate` produces nonsense with full type-checker
    blessing.

### Federation

19. `prefix` is present, matches `[a-z][a-z0-9-]*`, and is not `core` unless the
    artifact *is* core — prefix squatting is the federated analogue of symbol
    shadowing.
20. No prefix collision (the one rule §7.4 already has).
21. `variants` targets exist (AMB-011).

### Hygiene

22. Display strings and provenance notes are bounded in size, and constrained in
    code points on the same terms as table prose (AMB-040 question 2). A
    registry is not exempt from the bidi and control-character hazards; it is
    *more* exposed, because its display strings are rendered by every consumer.
23. Unknown keys inside known tables are rejected — typo protection, since
    `factr = [1000, 1]` would otherwise load silently as a unit with no factor.
    Unknown *top-level* tables are preserved for forward compatibility, gated on
    `schema_version`.

### Reporting

24. `E_REGISTRY_INVALID`, carrying the failed check, the artifact name, and the
    offending key. One code with a structured reason beats twenty codes.
25. Validation is independent of seal verification, and its message never
    implies a trust problem.

## Staged artifacts — Cluster Q

**Spec §7**, new subsection §7.5 *Load-time validation (normative)* containing
the checklist above, and a sentence stating that a valid seal on a vocabulary
attests to its origin and not its conformance.

**Spec §10**, one row: `E_REGISTRY_INVALID` — "A loaded registry or vocabulary
artifact fails a §7.5 validation check."

**Conformance:** a new `registry/` category of deliberately malformed catalogs —
shadowed core symbol, alias collision, symbol containing `*`, `factor = [1, 0]`,
negative factor, i8 overflow in a dimension vector, `parent` cycle, dangling
`delta`, delta with an offset, `rate_of` off by a dimension, prefix squat,
misspelled key. Each asserting `E_REGISTRY_INVALID` and its reason. **No
document currently plans for this category** — spec §11 and companion §9 both
stop at artifacts that are already valid.

Decisions 41–49 are in the checklist at the top of this memo.

---

# Cluster R — From the editor review session (AMB-044, AMB-045, AMB-046)

Registered with full treatment in [AMBIGUITIES.md](AMBIGUITIES.md) directly;
no separate memo analysis yet. AMB-044 (cast rule vs `E_CAST_LOSSY`) carries a
ruled *direction* — storage policies `promote` / `preserve` /
`preserve(tolerance=…)` — pending staged spec text. AMB-045 (warning taxonomy
is an ellipsis) and AMB-046 (Arrow temporal types silently untaggable) are
open; both block later categories, neither blocks M0.

---

# What clears when

Counted from the fixtures' own `blocked_on` sets, not estimated.

| Stage | Decisions | Provisional cases cleared | Left |
|---|---|---|---|
| M0 | 1–13 | **28** of 39 | 11 |
| M1 | 14, 16, 17, 18, 19 | 9 more | 2 |
| Later | 32 (`pu`) | the last 2 | 0 |

The 11 that survive M0: `parsing.affine.003`–`.008` (decisions 16 and 17),
`parsing.errors.002` (18, 19), `parsing.errors.011`–`.012` (14), and
`parsing.dimensionless.005`–`.006` (32).

**No expected value moves anywhere in that sequence.** The scratchpad reference
canonicalizer already reproduces all 63 cases under exactly these
recommendations, so applying them is a `status` flip and nothing else. If a
decision goes the other way — A4 on slashes, O1 on the affine convention, W2 on
`interval` — the affected expectations change and the memo's *Staged artifacts*
sections say which.

Nothing in the M0 set touches `conformance-core.toml`. Decision 15 (AMB-016) is
the only one that moves a number in it, and it is an M1 decision, so **M0 can
close without editing the fixture registry at all**.
