# Spec ambiguity register

Places where the UnitArrow Specification (now **0.21.0-draft**) and the
Provenance Companion Specification (now **0.4.0-draft**) are **silent**,
**circular**, or **self-contradictory**. The 0.18.0 / 0.4.0 revisions ARE the
editorial pass applying this register's ruled entries. AMB-001 … AMB-027 were found while writing the §6.3
conformance fixtures; AMB-028 … AMB-035 came from the 0.17.0 / 0.3.0 revision;
AMB-036 … AMB-038 surfaced while working the others through to decisions in
[DECISIONS.md](DECISIONS.md). See [History](#history) at the end.

This register exists because roadmap **M0**'s exit criterion is *"canonical-form
rules reviewed against 20 hand-written unit strings with no ambiguity found."*
Writing the fixtures **is** that review, and it did not come back clean.

Each entry records a recommended resolution. Where a fixture depends on one, the
case carries `"status": "provisional"` and `"blocked_on": ["AMB-nnn"]`, so every
open decision is greppable from the fixtures and every fixture is traceable to
the decision it presumes. **Provisional cases are recommendations, not
ratified goldens** — they change when the entry is decided, and they are the one
exception to the rule that fixtures never move to accommodate an implementation.

| Severity | Meaning |
|---|---|
| **contradiction** | Two parts of the spec (or spec and registry example) cannot both hold |
| **gap** | A rule that fixtures need is simply absent |
| **circular** | The text defines a term using itself |
| **implicit** | Correct as written, but load-bearing and unstated — will be re-litigated |

Status is `open` unless an entry says otherwise. **Decided so far (16):**
AMB-001 (slash-free canonical form, A4), AMB-002/004 (mooted; input rules
retained), AMB-003 (bytewise UTF-8 collation), AMB-005 (strict ASCII symbols),
AMB-006 (whitespace by position — **revised 2026-07-30**), AMB-007 (strict
exponent literals), AMB-009
(empty string is a syntax error), AMB-010 (per-unit `aliases` field), AMB-013
(`E_UNIT_SYNTAX` added), AMB-014 (`E_BAD_METADATA`, by implication of 039),
AMB-016 (kelvin offset convention), AMB-026 (required registry `name`), AMB-036 (registry/vocabulary split; seal pins the set),
AMB-037 (unit-symbol collision is a load-time error; namespaces deferred to
grammar v2), AMB-039 (no escapes in unit values). AMB-044 has a ruled
*direction* pending spec text. The rest await rulings.

**With these, every M0 blocker is decided.** The frozen surface: grammar
(strict ASCII symbols, strict integers, no whitespace, `1/…` inverse input),
canonical form (slash-free, sign-in-exponent, positives-then-negatives, each
group bytewise), alias and identity model, `E_UNIT_SYNTAX`, and the
registry/vocabulary split. Remaining open entries block M1+ only.

---

## Canonical form and grammar (§6.1, §6.3)

### AMB-001 — Rule 3 is circular, and slash repetition is undetermined
**§6.3 step 3 · contradiction + circular · RESOLVED 2026-07-29**

The text reads:

> negative exponents rendered with `/`, positive-exponent terms sorted
> lexicographically by symbol, then negative-exponent terms sorted
> lexicographically **after all `/` terms**

"Negative-exponent terms sorted after all `/` terms" defines the position of
the `/` terms in terms of the `/` terms. It cannot be executed as written.

Worse, it never says whether canonical form emits **one** slash or **one per
negative term**. §6.1 settles it: division binds left-to-right, so `a/b/c` is
a·b⁻¹·c⁻¹ and there is no grouping construct. A single leading slash therefore
cannot express more than one negative term — `W/K*m^2` parses as W·K⁻¹·m**⁺²**,
which is a *different unit*, not a different spelling. Only repeated slashes
preserve meaning.

**Recommended resolution.** Restate step 3 as: emit all positive-exponent terms
first, sorted, joined by `*`; then each negative-exponent term, sorted, each
introduced by its own `/`. Negative exponents are rendered with their sign
removed (see AMB-002). So thermal conductance is **`W/K/m^2`** — note that the
negative terms sort among themselves (`K` before `m`), which is easy to get
backwards.

**RESOLVED 2026-07-29 — the other way: A4** (DECISIONS.md Cluster A). Canonical
output contains **no slashes**; every term carries its sign in the exponent —
`W*K^-1*m^-2`, `MW*s^-1`, `USD*MWh^-1`. Positive terms still precede negative
terms, each group sorted per AMB-003 (still open). `/` remains legal *input*
(left-to-right per §6.1) and normalizes away. Human-readable rendering is the
display layer's job, not canonical form's. This ruling also resolves AMB-002
and AMB-004 by construction.

*Cases:* `parsing.division.001`, `parsing.division.004`, `parsing.division.007`, `parsing.division.008`, `parsing.merging.006`, `parsing.ordering.005`, `parsing.ordering.006`

---

### AMB-002 — Sign rendering under `/` unstated
**§6.3 step 3 · gap · RESOLVED 2026-07-29**

Given `m^-2` in the negative group, canonical form could be `/m^2` or `/m^-2`.
The latter is a double negative (divide by m⁻²  = multiply by m²) and would make
the string mean the opposite of the intent, but the spec never rules it out.

**Recommended resolution.** The `/` carries the sign; the rendered exponent is
always the absolute value, and exponent `1` is omitted there as everywhere.
`/m^-2` is then not merely non-canonical but *invalid* on input, since it would
parse to a positive exponent and round-trip to `m^2`.

**RESOLVED 2026-07-29 — mooted for output by the AMB-001 ruling** (canonical
form contains no `/`). The input rule is retained as a typo guard: a negative
exponent after `/` (`MW/s^-1`) is a syntax error rather than a double negative
that silently means `MW*s`. Only its error code (AMB-013) is still pending.

*Cases:* `parsing.division.002`, `parsing.division.005`, `parsing.division.006`, `parsing.ordering.005`

---

### AMB-003 — "lexicographically" has no defined collation
**§6.3 step 3 · gap · RESOLVED 2026-07-29**

Symbols are case-sensitive (§6.1), so `mW` and `MW` are distinct units that must
have a defined relative order. "Lexicographic" spans at least three incompatible
answers: bytewise, Unicode code point, and locale-aware collation. Case-folding
collations (`Btu` < `h` < `kW` < `m` < `MW` < `W`) and bytewise
(`Btu` < `MW` < `W` < `h` < `kW` < `m`) produce different canonical strings for
the same expression — which means different hashes, which is exactly what §6.3
exists to prevent.

**Recommended resolution.** Bytewise comparison of the UTF-8 encoding. It is the
only option that is locale-independent, trivially identical in Rust
(`Ord for str`), Python (`bytes` comparison), and JS (`<` on strings is UTF-16
code-unit order — equal to UTF-8 byte order for the BMP, so this must be pinned
explicitly if non-ASCII symbols are ever allowed; see AMB-005). Consequence for
the fixture registry: `Btu < J < K < MW < MWh < USD < W < degC < degF < delta_degC < delta_degF < h < kW < m < mW < pu < s`.

**RESOLVED 2026-07-29 — bytewise UTF-8, ascending.** With AMB-005's ASCII
symbol class this equals code-point order and JavaScript's `<` for free; no
binding needs a custom comparator.

*Cases:* `parsing.division.001`, `parsing.division.004`, `parsing.division.008`, `parsing.division.010`, `parsing.ordering.002`, `parsing.ordering.003`, `parsing.ordering.004`, `parsing.ordering.006`

---

### AMB-004 — `s^-1` has no canonical spelling
**§6.1, §6.3, §8.1 · contradiction · RESOLVED 2026-07-29**

The grammar is:

```
unit-string  = term *( ("*" / "/") term ) | "1"
term         = symbol [ "^" integer ]
```

`"1"` is an alternative for the *whole string*, not a `term`, and a string
cannot begin with `/`. So a unit expression with **no positive-exponent terms
is unspellable**: `1/s` is ungrammatical, `/s` is ungrammatical, and `s^-1`
canonicalizes (per AMB-001/002) into the negative group with nothing in front
of it.

This is not hypothetical. §8.1 says `a / b` yields "the canonicalized symbolic
product of input units", and a dimensionless column divided by a `s` column is
an ordinary operation. §8.5's `differentiate` divides by a period. Both produce
a result the spec cannot write down.

**Recommended resolution.** Permit `1` as a leading term when the expression has
no positive-exponent terms, and require it there:

```
unit-string  = ( term / "1" ) *( ("*" / "/") term )
```

Canonical form for the empty positive group is then `1/s`, `1/K/m^2`, and so
on, while `"1"` alone remains the empty product. `1*MW` stays invalid — `1` is
legal only as the leading term and only when no positive term exists (otherwise
`1*MW` and `MW` would be two spellings of one expression, reopening AMB-001's
problem).

**RESOLVED 2026-07-29 — mooted for output by the AMB-001 ruling:** a pure
inverse is directly spellable as `s^-1`, so canonical form never needs a
leading `1`. On *input*, the `1/term…` form is accepted (users may write `1/s`,
`1/K/m^2`) and normalizes to exponent form; `1*MW` remains invalid.

*Cases:* `parsing.dimensionless.004`, `parsing.division.007`, `parsing.division.008`, `parsing.merging.006`

---

### AMB-005 — The symbol character class is undefined
**§6.1 · gap · RESOLVED 2026-07-29**

`symbol = registry-resolved identifier (case-sensitive)` defines a symbol by
what resolves, not by what lexes. A tokenizer cannot be written from this: it
does not know where a symbol ends.

Two places in the spec push in opposite directions:

- §7.2 shows `variants = ["household_yr@CO", "household_yr@TX"]`, implying `@`
  and `_` are legal symbol characters.
- §7.2 shows `display.unicode = "°C"`. If display forms are also accepted as
  input aliases (§6.3 step 1 says "aliases → canonical symbol" without saying
  where aliases come from — see AMB-010), then symbols include non-ASCII, and
  AMB-003's bytewise collation needs an explicit UTF-8-not-UTF-16 statement.

**Recommended resolution (revised).** Split the input class from the canonical
class:

- **Canonical symbols** — registry keys, and everything §6.3 emits — are ASCII:
  `ALPHA *( ALPHA / DIGIT / "_" / "@" )`.
- **Input tokenization** is permissive: a symbol is a maximal run of characters
  that are not `*`, `/`, `^`, whitespace, or an ASCII control. That lexes `°C`,
  `µs`, and `Ωm` with **no Unicode tables** — no XID properties, nothing that
  costs WASM bytes. Validity is then decided by registry lookup, so `°C`
  resolves through an alias (AMB-010) to `degC`.

This keeps every safety property of ASCII-only while removing the ergonomic
cost: a user may type the symbol they know, and canonical form is still ASCII,
so equality and hashing are unaffected. A tokenization or normalization mistake
can only produce `E_UNKNOWN_UNIT` — never two spellings of one expression.

It also absorbs the Unicode normalization problem without a normalization
table. `µ` (U+00B5) and `μ` (U+03BC) render identically and are different code
points; under a Unicode *canonical* class they would be different symbols with
different hashes. Here they are simply two aliases for one ASCII symbol, and the
set of spellings people actually type is small and enumerable.

The original narrower recommendation — ASCII everywhere, `°C` a syntax error —
remains the fallback if permissive lexing proves to surface confusing errors.

**RESOLVED 2026-07-29 — the original strict form, F1: ASCII everywhere.**
`symbol = ALPHA *( ALPHA / DIGIT / "_" / "@" )` for input and output alike;
`°C` is `E_UNIT_SYNTAX` at the lexer. The permissive-input revision above was
offered and declined — predictable errors won over input breadth. Display
forms remain output-only. If the ergonomic pressure returns, the permissive
lexer is a backward-compatible widening; the reverse is not.

*Cases:* `parsing.errors.009`

---

### AMB-006 — Whitespace has no production
**§6.1 · gap · RESOLVED 2026-07-29**

The grammar contains no whitespace rule, which strictly means `MW * h` is
invalid — but ABNF-style grammars in specs are often written assuming implicit
whitespace, so an implementer could reasonably go either way. Permitting it
would also make canonical form ambiguous unless the spec said where spaces go.

**Recommended resolution.** No whitespace anywhere, including leading and
trailing. Reject it as a syntax error rather than trimming, so a stray space in
a hand-written tag is caught at the producer rather than silently normalized.

**RESOLVED 2026-07-29 — as recommended: forbidden, rejected, never trimmed.**

**REVISED 2026-07-30 — split by position.** The blanket ban was wrong, and the
argument for it was weak: "catches the mistake at the producer" is not much of a
benefit when the producer's intent was never in doubt. The grammar already
treats `/` and `1/s` as input sugar normalized away on output; whitespace at the
edges and around operators belongs in exactly that category.

But blanket *stripping* would have been worse, and this is the argument the
original entry failed to make. Whitespace between two bare symbols cannot be
collapsed safely:

| Input | Collapsed | Dimension |
|---|---|---|
| `m*s` | — | `{length: 1, time: 1}` |
| `m s` | `ms` | `{time: 1}` — **millisecond** |
| `m m` | `mm` | `{length: 1}` — **millimetre**, not m² |

Measured against a registry with SI prefix expansion, so this is not
hypothetical: `ms` and `mm` are real units, and the substitution is silent.
Reading the space as multiplication instead (the UDUNITS/pint convention) is
also rejected, for a different reason — it gives the product a second spelling,
which is what §6.3 exists to prevent.

**The rule, as implemented:**

- **Leading and trailing whitespace** — accepted, normalized away. The realistic
  source is a CSV column or a hand-edited config.
- **Whitespace around `*` and `/`** — accepted. The operator makes the intent
  explicit, so a space cannot change meaning.
- **Whitespace between two bare symbols** — `E_UNIT_SYNTAX`, with a message that
  names both readings: *"a space is not multiplication and joining them would
  change the meaning — write `m*s` (or `ms`, if that is one symbol)"*.
- **Whitespace inside a term** (`m ^2`) — rejected; it reads as two tokens and
  would need a rule of its own for no benefit.

Canonical form contains no whitespace under any spelling, so none of this
affects equality or hashing.

*Fixture impact:* `parsing.errors.007` previously asserted that `MW * h` is
rejected. Its input is now `m s` — the case that is still rejected — and
`parsing.normalization.008` … `.010` cover the accepted forms. A golden moving
is correct here: the spec moved.

*Cases:* `parsing.errors.007`

---

### AMB-007 — Exponent literal form unstated
**§6.1 · gap · RESOLVED 2026-07-29**

`"^" integer` leaves open whether `^+2`, `^02`, `^-0`, and `^0` are legal, and
whether they are distinct from `^2`, `^2`, `^0`, and omission. Each admits
multiple spellings of one expression unless pinned.

**Recommended resolution.** `integer = ["-"] 1*DIGIT` with no leading zeros
except the literal `0`; no `+`; no `-0`. `^0` is legal input and is eliminated
by §6.3 step 2, so `m^0` canonicalizes to `"1"`. `^1` is legal input and
canonicalizes to a bare symbol.

**RESOLVED 2026-07-29 — as recommended.** `^+2`, `^02`, and `^-0` are syntax
errors; `^0` and `^1` are legal and normalize away.

*Cases:* `parsing.affine.003`, `parsing.errors.010`, `parsing.normalization.004`, `parsing.normalization.005`
`parsing.errors.009`

---

### AMB-008 — Exponent range is unbounded in the grammar but bounded in dimensions
**§6.1 vs §6.2 · contradiction · open**

§6.2 fixes dimension vectors as **signed 8-bit** exponents. §6.1 places no bound
on unit-string exponents. `m^300` is therefore grammatically valid and
dimensionally unrepresentable, and `m^100*m^100` overflows during step 2's
exponent summation even though each input term is in range.

**Recommended resolution.** Bound unit-string exponents to the i8 range
[-128, 127], and check for overflow *after* step 2's merge, not per-term.
Requires a new error code (AMB-015).

*Cases:* `parsing.errors.011`, `parsing.errors.012`

---

### AMB-009 — The empty string is not covered
**§6.1, §6.3 step 4 · gap · RESOLVED 2026-07-29**

`""` is ungrammatical (the grammar requires at least one term or `"1"`), and
§6.3 step 4 is careful to say the *empty product* serializes as `"1"` — but the
spec never states that `""` on the wire is an error rather than a spelling of
dimensionless. Given §5.3's insistence that dimensionless is an explicit claim,
accepting `""` as `"1"` would let an empty metadata field silently become a
claim nobody made.

**Recommended resolution.** `""` is a syntax error (AMB-013), explicitly not
equivalent to `"1"` and explicitly not equivalent to untagged.

**RESOLVED 2026-07-29 — as recommended.**

*Cases:* `parsing.dimensionless.003`

---

### AMB-010 — §6.3 requires alias resolution; §7.2 has nowhere to record aliases
**§6.3 step 1 vs §7.2 · contradiction · RESOLVED 2026-07-29**

Step 1 of canonicalization is "Resolve every symbol against the registry
(aliases → canonical symbol)." The registry unit schema in §7.2 has
`dimension`, `factor`, `offset`, `delta`, `display`, `provenance`, and
`variants`. **There is no `aliases` field.** The first step of the
canonicalization algorithm depends on data the registry format cannot express.

**Recommended resolution.** Add `aliases = [...]` to the §7.2 unit schema, as a
list of additional input-only spellings that resolve to the entry's canonical
symbol. Load-time validation must reject an alias that collides with any
canonical symbol or with another alias. Aliases are never emitted.

**RESOLVED 2026-07-29 — as recommended: per-unit `aliases` field.** Note the
interaction with AMB-005's strict-ASCII ruling: aliases are symbols, so they
are ASCII too — `°C` cannot be an alias; `degreeC` and `C` can.

*Cases:* `parsing.normalization.001`, `parsing.normalization.002`

---

### AMB-011 — `variants` semantics undefined
**§7.2 · gap · open**

`variants = ["household_yr@CO", "household_yr@TX"]` appears in the
`household_yr` example with no accompanying prose. Unanswered: are variants
independently declared `[unit.…]` entries, or implicitly created by this list?
Do they resolve as symbols in a unit string? Do they canonicalize to themselves
or to the parent? Do they inherit `dimension` and override only `factor`? Is a
column tagged `household_yr` commensurable *and equal* to one tagged
`household_yr@CO`?

They cannot be equal — they have different factors by construction — so the
list is either documentation or an inheritance mechanism, and the difference is
observable in canonical form.

**Recommended resolution.** Treat `variants` as a non-normative cross-reference
only: each variant MUST have its own full `[unit.…]` entry, resolves and
canonicalizes as itself, and is commensurable-but-unequal to its parent. No
inheritance. (Deferred past M0; no fixtures depend on it. Flagged because M1
defines the base registry and will have to decide.)

**Concretely.** §7.2 shows `variants = ["household_yr@CO", "household_yr@TX"]`
and says nothing more. Three readings, all consistent with the text:

| Reading | `household_yr@CO` is… | Consequence |
|---|---|---|
| separate units | its own registry entry with its own factor | `household_yr` alone is incomplete |
| a parameterised unit | the base unit plus a region tag | canonical form must decide whether `@CO` survives |
| documentation | a note that regional values exist | the variants are unresolvable symbols |

Canonical form is built on symbols, so this is not cosmetic: whether
`household_yr@CO` and `household_yr` compare equal depends on the answer.

*Cases:* none yet

---

### AMB-012 — Canonical form deliberately does not reduce across unit choices
**§6.3, §1 goal 4, §8.3 · implicit · open**

If the registry defines both `MWh` and the components `MW` and `h`, then `MW*h`
and `MWh` are distinct canonical strings that are commensurable and numerically
identical. Under §8.3, concatenating two such columns is `E_UNIT_MISMATCH` in
strict mode and a silent coercion-with-warning in permissive.

This follows correctly from §1 goal 4 ("data is never silently canonicalized to
SI") and is almost certainly intended. But it is nowhere stated, it will
surprise every implementer, and "canonical form" is a name that actively
suggests the opposite.

**Recommended resolution.** State it in §6.3 as a note: canonicalization
normalizes *spelling*, never *choice of unit*. Two commensurable units are equal
iff their canonical strings are identical.

*Cases:* none blocked on it — cited in the rationale of `parsing.normalization.007` and `parsing.dimensionless.008`, which are determined by the spec as written.

---

## Error taxonomy (§10)

### AMB-013 — No error code for a unit-string syntax error
**§10 · gap · RESOLVED 2026-07-29**

`E_UNKNOWN_UNIT` is raised when a "symbol does not resolve against the loaded
registry." That is symbol *resolution*. It does not cover strings that never
reach resolution because they do not parse: `MW**h`, `MW^`, `MW*`, `(m*s)`,
`m/`, `^2`, `""`.

Every one of these needs an expected code before an error fixture can be
written, and mapping them to `E_UNKNOWN_UNIT` would be wrong in a
user-visible way — "unknown unit `MW**h`" tells the user to check their
registry when the problem is a typo.

**Recommended resolution.** Add **`E_UNIT_SYNTAX`** — "unit string does not
parse under the declared grammar." Distinct from `E_GRAMMAR_VERSION` (which is
about a *newer* grammar the implementation cannot handle) and from
`E_UNKNOWN_UNIT`.

**RESOLVED 2026-07-29 — as recommended.** `E_UNIT_SYNTAX` joins §10. This was
the single largest unblock: 14 fixture cases flipped to normative on this
ruling alone.

*Cases:* `parsing.dimensionless.003`, `parsing.dimensionless.004`, `parsing.division.006`, `parsing.errors.003`, `parsing.errors.004`, `parsing.errors.005`, `parsing.errors.006`, `parsing.errors.007`, `parsing.errors.008`, `parsing.errors.009`, `parsing.errors.010`, `parsing.errors.014`
`parsing.dimensionless.003`, `parsing.dimensionless.005`

---

### AMB-014 — No error code for malformed extension metadata
**§5.2, §10 · gap · RESOLVED 2026-07-29**

§5.2 makes `unit` and `grammar` required. Nothing says what is raised when the
`ARROW:extension:metadata` blob is absent, is not valid UTF-8, is not valid
JSON, is a JSON scalar rather than an object, or omits a required key. Wire
round-trip fixtures (category `wire`, M2) cannot be written without this.

**Recommended resolution.** Add **`E_BAD_METADATA`** covering all of the above,
with the offending key named in the message.

**RESOLVED 2026-07-29 — by implication of the AMB-039 ruling.** The escape
prohibition required the code to exist; the 0.18.0 editorial pass added it to
§10 covering the full enumeration (absent blob, invalid UTF-8/JSON, missing
required key, escape in a unit-valued field).

**Concretely.** The metadata blob is UTF-8 JSON, so it can fail four ways
before any unit string exists:

```
b"\xff\xfe..."          -> not UTF-8
'{"unit": "MW"'          -> not JSON
'{"quantity": "core:power"}'  -> JSON, no `unit` key
'{"unit": "\u004dW"}'    -> valid, but escaped (AMB-039)
```

None of them is `E_UNIT_SYNTAX` — nothing was parsed as a unit. Resolved by
adding `E_BAD_METADATA` to §10, covering all four.

*Cases:* the `parsing.encoding.*` error cases assert it; wire fixtures to come

---

### AMB-015 — No error code for exponent overflow
**§6.2, §10 · gap · open**

See AMB-008. Neither the grammar's unbounded integer nor step 2's exponent
summation has a defined failure mode when the result leaves i8.

**Recommended resolution.** Add **`E_EXP_RANGE`** — "exponent outside the
representable range after canonicalization."

*Cases:* `parsing.errors.011`, `parsing.errors.012`

---

## Affine and delta units (§6.4, §7.2)

### AMB-016 — The affine conversion formula contradicts the registry's offset convention
**§6.4 vs §7.2 · contradiction · open**

This is the most consequential entry in the register: the two readings disagree
by 200 °F on an everyday conversion, and the spec's own example cannot
distinguish them.

§6.4 gives:

> convert as `y = (x + offset_from) * factor - offset_to`

The offset is added **before** scaling, which means it is expressed in *source
units*. §7.2 annotates the same field the other way:

```toml
offset = [27315, 100]          # exact rational offset to kelvin
```

"Offset to kelvin" most naturally means the offset is expressed *in kelvin*,
i.e. added **after** scaling. For `degC` the two readings coincide, because
`degC`'s factor is 1 — which is precisely why the example hides the conflict.
`degF` is the discriminating case, and there is no `degF` entry in the spec.

- Offset in source units: `degF.offset` = 459.67 = `[45967, 100]`
- Offset in kelvin: `degF.offset` = 255.372… = `[45967, 180]`

Take §7.2 literally (`[45967, 180]`) and evaluate §6.4's formula for
100 °C → °F, with the single unqualified `factor` read as `fac_from / fac_to`
= 9/5:

```
y = (100 + 27315/100) * 9/5 - 45967/180
  = 373.15 * 1.8 - 255.3722
  = 416.2978            ← should be 212
```

Separately, the formula's bare `factor` is itself underspecified: converting
between two affine units involves both `fac_from` and `fac_to`, and a single
symbol cannot stand for both.

**Recommended resolution.** Fix the convention to **source units** (matching
§6.4's structure, which is the operative text) and correct §7.2's comment to
"exact rational offset, in this unit's own scale, to the dimension's base
unit." Restate the formula with both factors explicit:

```
base = (x + offset_from) * factor_from
y    = base / factor_to - offset_to
```

Check: 100 °C → (100 + 273.15) × 1 = 373.15 K → 373.15 ÷ (5/9) − 459.67 =
671.67 − 459.67 = **212** °F. And 20 delta_degC → 36 delta_degF via the delta
units, which carry no offset.

*Cases:* none blocked on it — cited in the rationale of `parsing.normalization.007` and `parsing.dimensionless.008`, which are determined by the spec as written.
parsing fixtures depend only on composability. The fixture registry
`registry/conformance-core.toml` encodes the recommended convention and says so
in a comment.

---

### AMB-017 — "compound expression" is undefined
**§6.4, §10 · gap · open**

> any compound expression containing an affine unit is invalid
> (`E_AFFINE_COMPOUND`)

Undecided edge cases, all of which a parser must answer:

| Input | Compound? |
|---|---|
| `degC` | no — clearly valid |
| `degC^1` | one term, redundant exponent |
| `degC^2` | one term, but squared |
| `degC/degC` | two terms that cancel to `"1"` |
| `degC*h/h` | two terms that cancel to `degC` |

The cancelling cases matter because §6.3 step 2 runs *before* serialization: if
the affine check runs after merging, `degC/degC` is dimensionless and passes; if
it runs on the parsed terms, it fails.

**Recommended resolution.** Check on the **parsed term list, before merging**: a
unit string is valid iff it contains no affine symbol, or consists of exactly
one term whose symbol is affine and whose exponent is 1 (whether written or
omitted). So `degC` and `degC^1` are valid; `degC^2`, `degC/degC`, and
`degC*h/h` are all `E_AFFINE_COMPOUND`. Checking before merging is the
conservative choice and keeps the error message pointing at what the user wrote.

*Cases:* `parsing.affine.003`, `parsing.affine.004`, `parsing.affine.005`, `parsing.affine.006`

---

### AMB-018 — Delta units are referenced but never specified
**§6.4, §7.2 · gap · open**

`delta = "delta_degC"` appears in the registry example and §6.4 says delta units
are "purely multiplicative and freely composable." Nothing states:

- that `delta_X` requires its own `[unit.…]` entry (it must, to have a factor);
- that it shares X's dimension;
- what its factor is — for `delta_degC` it is 1, but for `delta_degF` it is
  5/9, i.e. equal to `degF`'s factor with the offset dropped, and this is never
  written down;
- whether `delta` is symmetric (does `delta_degC` point back via some field?);
- whether a delta unit may itself carry an `offset` (it must not);
- whether a unit may be its own delta (for `K`, whose delta is itself — the
  example registry does not show `K` at all).

**Recommended resolution.** Normative text in §7.2: a unit named by another
unit's `delta` MUST exist, MUST share its dimension, MUST carry the same
`factor`, and MUST NOT carry an `offset`. A non-affine unit's `delta` is itself
and MAY be omitted. `delta` is one-directional; validation checks it at load.

*Cases:* `parsing.affine.007`, `parsing.affine.008`

---

### AMB-019 — `E_INTERVAL_MISUSE`'s "or vice versa" is unenforceable
**§6.4, §7.3, §10 · contradiction · open**

§10 defines the code as "Absolute unit on an `interval: true` quantity kind
(**or vice versa**)". The reverse direction requires knowing that a kind is
*absolute* — but §7.3 offers only `interval = true`. Its absence is the absence
of a claim, exactly as untagged is for units (§5.3), not an assertion of
absoluteness. `cf:air_temperature` declares `parent = "core:temperature"` and
nothing else; a checker has no basis to reject `delta_degC` on it.

Treating absence as an absolute claim would also contradict §8.5 rule 4, which
tells checkers not to invent a temporal kind from absence.

**Recommended resolution.** Either add an explicit `interval = false` (making
absence still mean "no claim", and only an explicit `false` triggering the
reverse check), or delete "or vice versa" from §10 and enforce one direction
only. Prefer the latter: fewer states, and the dangerous bug is one-directional.

**Concretely.** `interval: true` marks a difference; nothing marks a kind
*absolute*. So the reverse check has nothing to test:

| Column kind | Unit | Detectable? |
|---|---|---|
| `interval: true` | `degC` (absolute) | yes — the flag is present and contradicted |
| *(no flag)* | `delta_degC` (difference) | **no** — absence of `interval` is absence of a claim, not a claim of absoluteness |

The second row is the "or vice versa" half of `E_INTERVAL_MISUSE`, and it is
unenforceable without an explicit `interval: false`.

*Cases:* none yet (blocks `type-functions`)

---

## Modes, dimensionless, and per-unit (§5.3, §5.4, §8.4)

### AMB-020 — §6.3 conditions reader behavior on a mode that working tables do not have
**§6.3 vs §8.4 · contradiction · open**

§6.3:

> Readers MUST accept non-canonical input in `permissive` mode … and MUST reject
> unresolvable symbols in `strict` mode

§8.4:

> Mode is a property recorded at finalization (companion `publish()`) —
> **working tables have no mode.**

So the normative reader behavior in §6.3 is selected by a property that is
absent for exactly the artifacts readers most often encounter: unsealed working
tables. Reading such a table has no defined behavior at all.

**Recommended resolution.** Define `permissive` as the default when no mode is
recorded, and state that `strict` reader behavior is entered only by an explicit
caller request or a sealed `mode: "strict"` claim. Every fixture that exercises
mode-dependent behavior carries an explicit `mode` field so it does not depend
on the default.

*Cases:* `parsing.errors.002`

---

### AMB-021 — Permissive-mode behavior on an unresolvable symbol is unspecified
**§6.3, §5.3, §8.2 · gap · open**

§6.3 pairs "accept non-canonical input in permissive" with "reject unresolvable
symbols in strict" — but non-canonical and unresolvable are different
conditions, and the permissive half of the *unresolvable* case is missing. Four
plausible behaviors, all defensible:

1. accept and pass the string through opaquely, no dimension available;
2. warn once and mark the column tainted (`unknown`, §8.2);
3. treat the column as untagged entirely, discarding the metadata;
4. reject in all modes.

These differ observably: (3) loses data that §5.2 says must survive round-trip.

**Recommended resolution.** Behavior (2): warn once per column
(`W_UNRESOLVED_SYMBOL`), treat the column as tainted for checking purposes, and
**preserve the original string verbatim** for round-trip. This keeps §5.2's
preservation guarantee, keeps the taint model honest, and never silently
upgrades an unknown symbol to a claim.

*Cases:* `parsing.errors.002`

---

### AMB-022 — `pu` has no declared dimension or composability rule
**§5.4 · gap · open**

`"unit": "pu"` is "valid only when `base` is present," and conversion multiplies
by the base value. Unstated:

- What is `pu`'s dimension? If dimensionless, a p.u. column silently passes
  dimension checks against any `"1"` column, and `pu + ratio` type-checks —
  almost certainly a bug the spec means to catch.
- Is `pu` composable? Is `pu*MW` legal? `pu/h`? `pu^2`?
- Does `pu` appear in the registry at all, and if so under what `dimension`?
- Is `E_BASE_MISSING` raised at parse time or at tag-validation time? A bare
  unit string has no access to the `base` key, so a parser cannot raise it.

**Recommended resolution.** The obvious fix — give `pu` a dedicated `per_unit`
dimension so it never unifies with `"1"` — should be **rejected**: §6.2's
base-dimension list is one of the two things M0 froze, and extending it to
solve a §5.4 problem is a bad trade for a narrow feature.

Instead: `pu` is **dimensionless** in the vector sense, and the guard lives at
the metadata level, where the information actually is. A p.u. column is one
whose metadata carries `base`; `E_BASE_MISSING` is raised by *metadata
validation*, never by the unit-string parser, which has no access to `base` and
therefore cannot raise it. `pu` is **non-composable** like an affine unit
(`pu*MW` → `E_AFFINE_COMPOUND`), so a compound expression can never smuggle a
p.u. factor into an otherwise physical unit.

The consequence worth stating in §5.4: a `pu` column and a `"1"` column *are*
dimensionally equal and will pass a dimension check against each other.
Anything stronger has to come from the quantity kind.

*Cases:* `parsing.dimensionless.005`, `parsing.dimensionless.006`

---

## Cross-document

### AMB-023 — Conformance category numbering collides across three documents
**spec §11 vs roadmap M4/M6 vs companion §9 · contradiction · open**

| Source | 1 | 2 | 3 | 4 | 5 | 6 |
|---|---|---|---|---|---|---|
| Spec §11 | parsing | factors | propagation | wire | type functions | — |
| Roadmap M4 | | | propagation | | | plans |
| Roadmap M6 | | | | | **seals** | |
| Companion §9 | plans | seals | closure | advisories | | |

Spec §11 assigns #5 to "type functions"; roadmap M6's exit criterion assigns #5
to "seals"; companion §9 restarts at 1 with its own four. Any directory named
`05-*` is wrong under at least one document.

**Recommended resolution.** Key conformance directories by **name**, not
number — `parsing/`, `factors/`, `propagation/`, `wire/`, `type-functions/`,
`companion/{plans,seals,closure,advisories}/` — and keep the mapping table in
[README.md](README.md) so roadmap references still resolve. Then renumber in
the spec editorial pass to 1.0-rc.

*Cases:* none blocked on it — cited in the rationale of `parsing.normalization.007` and `parsing.dimensionless.008`, which are determined by the spec as written.

---

### AMB-024 — Unknown-key preservation is silently load-bearing for seals
**§5.2 vs companion §5.2 · implicit · open**

§5.2: "Unknown keys MUST be preserved on round-trip and ignored otherwise."
Companion §5.2 step 3 computes the data digest over "the canonical IPC
serialization of (schema with all metadata *except* the seal itself, followed by
record batches in order)."

Together these mean an implementation that quietly drops an unknown extension
key does not merely lose metadata — it **breaks the seal of every downstream
artifact**, and the failure surfaces as "this table was modified after it was
published" (companion §5.4) pointing at an innocent consumer. The coupling is
real and correct, but neither document mentions the other here.

**Recommended resolution.** A note in §5.2 stating the consequence, and a wire
fixture with unknown keys of several JSON types (object, array, null) that
asserts byte-stable re-serialization. Also unspecified and needed for that
fixture: whether key *order* is preserved or canonicalized, which matters
because the digest is over serialized bytes.

**Update (spec 0.17.0-draft / companion 0.3.0-draft).** Unresolved, and now
broader: §5.5's new `unitarrow:description` and `unitarrow:references` are
*plain* metadata keys outside the extension blob, so §5.2's preservation rule
does not even reach them (AMB-032). Companion §5.4 confirms the hazard from the
other side — column digests exclude field metadata "which the seal signature
already covers", i.e. metadata is inside the signed digest.

**Concretely.** §5.2 says a reader preserves unknown metadata keys. The
companion hashes the schema. So dropping a key nobody understood changes a
digest:

```
producer writes : {"unit": "MW", "vendorX:calib": "2026-03-01"}
naive reader     : {"unit": "MW"}                     # key dropped
                   -> data_digest changes -> seal fails to verify
```

The coupling is real and stated in neither document: §5.2's preservation rule is
load-bearing for companion §5.4, and a reader that treats it as politeness
breaks verification.

*Cases:* none yet (blocks `wire`)

---

### AMB-025 — `period` admits ISO-8601 durations that have no fixed length
**§5.2, §8.5 · gap · open**

`temporal.period` is "ISO-8601 `period`" with no subset restriction, so `P1M`
and `P1Y` are legal. §8.5 rule 1 defines `integrate` as "value × period" — but
a month has no fixed length, so the product is undefined without a calendar and
an anchor timestamp. `differentiate` has the same problem inverted. §8.5 rule 2
also compares periods for equality across operands, and `P1M` vs `P30D` is
neither clearly equal nor clearly unequal.

**Recommended resolution.** Restrict `period` to fixed-length designators
(`PnW`, `PnD`, `PTnH`, `PTnM`, `PTnS` and combinations thereof, excluding `Y`
and `M`), and compare periods by normalized total seconds as an exact rational
so `PT60M` equals `PT1H`. Calendar-relative periods, if ever needed, are a
separate feature requiring an anchor.

**Concretely.** §8.5 allows any ISO-8601 duration in `period`. Two of them
have no fixed length:

| `period` | Seconds | `integrate` over it |
|---|---|---|
| `PT1H` | 3600 | defined |
| `P1D` | 86400 (ignoring DST) | defined-ish |
| `P1M` | 28–31 days | **undefined** |
| `P1Y` | 365 or 366 days | **undefined** |

`MW` × `P1M` has no single answer in MWh, so either `period` is restricted to
fixed-length durations or `integrate` must reject the variable ones.

*Cases:* none yet (blocks `type-functions`)

---

### AMB-026 — Registries have a version but no identity
**§7.1, §5.6 · gap · RESOLVED 2026-07-29**

The registry header is:

```toml
[registry]
schema_version = 1
version = "2026.07"
```

There is no name. §5.6 pins a registry into the provenance block as "version +
content hash", and companion §5.3's seal carries
`registry: { version, hash }` — so two unrelated registries both released as
`"2026.07"` are indistinguishable by version, and a consumer holding the wrong
one sees a hash mismatch with no way to say *which* registry it should have
fetched. §7.4 gives vocabulary files a namespace prefix but says nothing about
the registry artifact itself.

This also blocks the conformance suite mechanically: a fixture must name the
registry it resolves against, and `version` alone is not a name.

**Recommended resolution.** Add a required `name` to `[registry]` (a short
lowercase-and-hyphen identifier), and carry it alongside `version` and `hash`
everywhere a registry is pinned. `registry/conformance-core.toml` uses it
already, marked as a deviation.

**RESOLVED 2026-07-29 — as recommended.** `name` is required; the
conformance-core deviation comment can be dropped once §7.1 is updated.

*Cases:* none blocked on it — cited in the rationale of `parsing.normalization.007` and `parsing.dimensionless.008`, which are determined by the spec as written.
`"registry": "conformance-core@0.1.0"`

---

### AMB-027 — The §7.2 registry example is not valid TOML
**§7.2 · contradiction · open**

§7.1 states the registry format is TOML. The §7.2 example block does not parse:

```
tomllib.TOMLDecodeError: Invalid initial character for a key part
  (at line 17, column 66)
```

TOML v1.0.0 forbids newlines inside an inline table — *"No newlines are allowed
between the curly braces unless they are valid within a value."* Two entries in
the example wrap one:

```toml
display = { unicode = "°C", latex = "^{\\circ}C", ascii = "degC",
            long = "degree Celsius" }

provenance = { source = "EIA RECS 2020", scope = "US average",
               note = "site energy, electricity + gas" }
```

(Arrays *may* span lines, so the neighbouring `factor = [...]` entries are fine.
It is specifically the inline tables.)

This is cosmetic in the document and not cosmetic downstream: registries are
hashed and pinned (§5.6), and the natural fix — reformatting to one line, or to
`[unit.degC.display]` sub-tables — changes the file's bytes. §7.1's comment says
the content hash is computed "over the canonicalized file," but the spec never
defines what canonicalizing a TOML file means, so any reformatting today is a
registry-identity change.

Found by running the spec's own code blocks through a TOML parser; blocks 1 and
3 pass, block 2 fails.

**Recommended resolution.** Fix the example to single-line inline tables (what
`registry/conformance-core.toml` does) or to explicit sub-tables. Separately,
define the registry canonicalization the hash is computed over — most likely a
normalized re-serialization rather than author bytes, matching the companion's
digesting policy (companion §4: "wherever a canonical form exists, digests are
computed over it, never over author bytes").

**PARTIALLY RESOLVED 2026-07-29:** the 0.18.0 editorial pass fixed the §7.2
example (single-line inline tables; every ```toml``` block now parses). The
hash-canonicalization definition in §7.1 **remains open**.

*Cases:* none — this is a document defect, not a behavioral one. Guard it with
a CI step that parses every ```` ```toml ```` block in the spec.

---

## Descriptions, references, and column digests (added in spec 0.17.0-draft / companion 0.3.0-draft)

Spec §5.5 grew from "Display name" to "Display name and description", adding
`unitarrow:description` and `unitarrow:references`. The companion added
`column_digests` (§5.3, §5.4), a republish-disposition flow (§5.2), and a
metadata size lint (§5.2 step 5).

Both error taxonomies (spec §10, companion §8) and both conformance-suite
sections (spec §11, companion §9) are **unchanged**, so the new behavior has no
codes and no category. None of AMB-001 … AMB-027 were addressed.

### AMB-028 — "exactly three interfaces" is now false
**spec §9, §5.5 vs companion §11, §5.2 · contradiction · open**

Spec §9: *"The two specifications meet at exactly three optional interface
points … **Nothing else in either document references the other.**"*
Companion §11: *"Exactly three, all optional in both directions."*

Both statements are now untrue, in both directions:

- Spec §5.5 references the companion's seal, its "optional column-level
  digests", and "its republish flow requires a disposition for descriptions
  inherited onto changed columns."
- Companion §5.2 references *"UnitArrow's §5.5 propagation rule"* by section
  number.

That is a fourth interface — documentation staleness ↔ column digests and the
republish disposition — and unlike the other three it is not enumerated,
not marked optional, and not described from both sides consistently. It also
strains companion non-goal 3 ("Not units-specific"): a units-free
implementation now has a normative-sounding pointer into a document it does not
implement.

**Recommended resolution.** Either enumerate it as interface 4 in both
documents (spec §9 and companion §11) with an explicit statement of what each
side owns, or sever the cross-references: give the companion a
type-system-agnostic phrasing ("the type layer's description-propagation rule,
if it has one") and have the spec describe staleness without naming the
companion's mechanisms.

**Concretely.** §9 says the two specs touch at "exactly three interfaces",
then the companion adds more:

```
1. provenance block (§5.6)      <- named
2. registry pin                  <- named
3. checking mode                 <- named
4. description survival (§5.5 vs companion §5.2)   <- NOT named
5. metadata size lint (companion §5.2 step 5)      <- NOT named
```

A reader auditing the boundary against the stated count will miss two.

*Cases:* none — structural.

---

### AMB-029 — the metadata size norm has no number
**spec §5.5 vs companion §5.2 step 5 · gap · open**

Both documents gesture at a threshold that neither defines:

- Spec §5.5: *"they are paragraphs, not documents — longer material belongs
  behind a link"* and *"not an escape hatch from the size norm."*
- Companion §5.2 step 5: *"a generous per-field threshold warns in permissive
  mode and fails in strict."*

"The size norm" has no antecedent. Unspecified: the number; its unit (UTF-8
bytes, Unicode scalars, or grapheme clusters — they differ by 3× for non-Latin
prose); its scope ("free-text descriptions **and similar**" — does it cover
`display_name`? `references[].description`? the whole `references` list?); and
whether there is a table-level total in addition to a per-field cap.

A conformance fixture for the strict-mode failure cannot be written without a
number, and two implementations picking different "generous" values will
disagree about whether a table publishes.

**Recommended resolution (revised).** State a concrete limit **per key and per
column**, in UTF-8 bytes, and enumerate exactly which keys the lint covers. Put
the numbers in the spec, not the registry — they are a wire-format norm.

**Do not state a schema-wide total.** The obvious 64 KiB budget trips at **274
columns** on a documented table (measured: 400 bytes per column with
`display_name` and `description`), so a schema-wide cap silently forbids
documenting exactly the wide tables that need it most — see AMB-041. If a
whole-schema guard is wanted it must scale with column count, not be a
constant.

**Concretely.** The size norm is prose in both documents:

> "documentation is paragraphs, not embedded documents"

with a "generous per-field threshold" that warns in permissive and fails in
strict. No number appears, so two implementations disagree about whether a
600-character description passes — and one of them fails the publish.

*Cases:* none yet (blocks `wire` and `companion/seals`)

---

### AMB-030 — no error codes for the two new strict-mode failures
**companion §5.2 vs companion §8 · gap · open**

Companion §5.2 introduces two conditions that "fail in strict" but §8's
taxonomy is unchanged:

1. metadata size over the (undefined, AMB-029) threshold;
2. republishing a table whose column digest changed while carrying an
   unmodified inherited description, with no per-column disposition supplied.

The second is not a variant of `E_SEAL_BROKEN` — nothing is broken; the
publisher is being asked a question. Reusing an existing code would make the
remediation ("rewrite the description, or explicitly keep it") unreachable from
the code alone.

**Recommended resolution.** Add `E_METADATA_SIZE` and `E_DESCRIPTION_STALE` to
companion §8, and a matching warning `W_DESCRIPTION_STALE` for the permissive
path (§5.2 says permissive "publishes with a warning naming the affected
columns" — that warning has no code either).

**Concretely.** Companion §5.2 introduces two strict-mode failures with no
§10 code:

| Failure | Code today |
|---|---|
| metadata field exceeds the size lint | *none* |
| transit read-back verification fails | *none* |

§10 is the wire-stable vocabulary, so a binding surfacing these has to invent a
string — and two bindings will invent different ones.

*Cases:* none yet (blocks `companion/seals`)

---

### AMB-031 — "passes through unchanged" is undefined
**spec §5.5 · gap · open**

> Transformations carry a column's description only when the column passes
> through unchanged; derived columns (converted, integrated, renamed) get no
> description unless one is explicitly authored

`rename` is listed as derived even though it changes no data at all, so
"unchanged" is not a statement about bytes. But nothing says what it *is* a
statement about, and the companion §5.2 explicitly rules bytes out from the
other direction: *"byte change is not meaning change (a unit conversion changes
every byte while the description stays true; a small value correction changes
meaning while the bytes barely move)."*

So neither identity criterion is available, and these are all undecided:

| Operation | Description survives? |
|---|---|
| filter (fewer rows, values untouched) | ? |
| sort / reorder | ? |
| concat of two columns with identical descriptions | ? |
| storage-only cast (int64 → float64 under §5.1's cast rule) | ? |
| unit conversion with no value change (MW → MW) | ? |
| project with no rename | ? |

**Recommended resolution.** Define it per operation over the companion §4 plan
IR, whose vocabulary is closed and short — arithmetic, conversion,
`integrate`/`differentiate`, filter, project/rename, aggregate, concat, join.
A table of eight rows settles it completely and is directly conformance-testable
in `type-functions/`. Anything less precise will diverge between bindings.

*Cases:* none yet (blocks `type-functions` and `wire`)

---

### AMB-032 — the new plain metadata keys have no round-trip or merge rules
**spec §5.2, §5.5, §5.6 · gap · open**

§5.2's *"Unknown keys MUST be preserved on round-trip"* governs the
**extension metadata JSON** — the blob under `ARROW:extension:metadata`.
`unitarrow:display_name`, `unitarrow:description`, and `unitarrow:references`
are deliberately *outside* that blob (§5.5: "plain field-metadata keys"), so
the preservation rule does not reach them, and no other rule does either.

Two consequences, neither stated:

1. **Round-trip.** Nothing requires an implementation to preserve these keys.
   Yet they are inside the data digest (companion §5.2 step 3 digests "schema
   with all metadata except the seal itself"), so an implementation that drops
   a description breaks every downstream seal — the AMB-024 trap, on a second
   surface, now with keys this spec defines rather than unknown ones.
2. **Merge.** §5.6 defines merge behavior for the provenance block's `mode`
   only. Merging two tables with different schema-level `unitarrow:description`
   values, or different `unitarrow:references` lists, is undefined — first
   wins? concatenate? drop? The union of two reference lists is the obviously
   useful answer and the obviously unstated one.

**Recommended resolution.** Extend §5.2's preservation requirement to all
`unitarrow:*` field and schema metadata keys, and give §5.6 merge rules for
`description` (drop on conflict — honest emptiness over a wrong claim, matching
§5.5's own stance on derived columns) and `references` (union, deduplicated by
`url`).

**Concretely.** `unitarrow:display_name` and `unitarrow:description` are
plain metadata with no merge rule. Concatenating two tables:

```
left  : display_name = "Site power"
right : display_name = "Plant power"
result: ?   -- left wins? dropped? error?
```

§5.6 defines merging for `mode` (weakest wins) and now for `composed_from` and
`vocabularies` (union). These keys were never given one.

*Cases:* none yet (blocks `wire`)

---

### AMB-033 — `unitarrow:references` has no defined encoding
**spec §5.5 · gap · open**

> Schema-level `unitarrow:references` holds … a list of
> `{url, description, digest?}` entries.

Undefined: the serialization (JSON is implied, but `unitarrow:display_name` is
a bare string, so `unitarrow:*` keys are not uniformly JSON); whether `url` and
`description` are required; the digest format (presumably the companion's
self-describing `sha256:` per its §6, but the spec never says so); and the
behavior on a malformed blob, which folds into AMB-014's missing
`E_BAD_METADATA`.

There is also a naming trap: `references[].description` and the field-level
`unitarrow:description` are different things one word apart.

And a small internal tension — the field is named `url`, but the text says
*"durable identifiers (DOIs, archived URLs) are RECOMMENDED."* A DOI is not a
URL. Is `doi:10.1234/foo` legal in a `url` field, or must it be written
`https://doi.org/10.1234/foo`?

**Recommended resolution.** Specify UTF-8 JSON array; `url` required and
constrained to an absolute URI (which admits `doi:` as a scheme, resolving the
tension); `description` required; `digest` optional and self-describing per the
companion's convention. Rename the inner field to `title` to kill the
collision.

**Concretely.** §5.5 defines `unitarrow:references` as a list of
`{url, description, digest?}` without saying how it is encoded:

```
{"unitarrow:references": "[{\"url\": \"https://…\"}]"}   -- JSON in a string?
```

Arrow metadata values are strings, so a list needs an encoding. `unit` is a bare
string and the provenance block is JSON — this key says neither.

*Cases:* none yet (blocks `wire`)

---

### AMB-034 — a prose typo fix is operationally a data correction
**spec §5.5 vs companion §5.2, §5.4 · implicit · open**

The two digests deliberately disagree about metadata, and the spec never says
so:

- the **data digest** (companion §5.2 step 3) covers "schema with all metadata
  except the seal itself" — descriptions **included**;
- the **column digests** (companion §5.4) are computed "excluding field
  metadata, which the seal signature already covers" — descriptions
  **excluded**.

Both choices are right on their own terms: the signature must cover the prose
or a description is forgeable, and column digests must ignore it or they could
not localize *data* changes. But together they mean fixing a spelling mistake
in a description breaks the seal, produces a new `data_digest`, and requires a
republication with a `supersedes` chain — indistinguishable, at the artifact
level, from correcting the numbers. Meanwhile every `column_digest` is
unchanged, so the what-changed report says nothing changed.

**Recommended resolution.** State the asymmetry explicitly in §5.5, and
consider a `documentation-only` republish disposition that the what-changed
report can surface as such — otherwise publishers will be discouraged from
fixing prose, which is the opposite of what §5.5 is for.

**Concretely.** Fixing a typo in a description changes the schema, which
changes the digest, which invalidates the seal:

```
before : description = "Net power at the point of interconection"
after  : description = "Net power at the point of interconnection"
         -> data_digest changes -> republish required
```

Editorially trivial, operationally a data correction with a `supersedes` chain.
Neither document says so, and a publisher will discover it the first time.

*Cases:* none yet (blocks `companion/seals`)

---

### AMB-035 — "canonically serialized" is undefined for column buffers
**companion §5.4 · gap · open**

> `column_digests` (optional) are computed per column over its data —
> validity and value buffers, **canonically serialized**

No canonicalization is defined, and Arrow offers several ways for two columns
with identical logical values to have different buffers:

- a **sliced** array carries a nonzero offset and a full-length parent buffer;
- **dictionary-encoded** vs plain encoding of the same strings;
- a validity buffer that is **absent** vs **present and all-true**;
- trailing **padding** bytes in a buffer (Arrow pads to 64 bytes; the contents
  are unspecified);
- **endianness** on a big-endian host.

Without a normalization rule, column digests are not reproducible across
implementations or even across two runs on the same one. That is not a cosmetic
problem: the republish disposition flow (§5.2) *and* the per-column
what-changed report (§5.4) both key off digest equality, so an unstable digest
produces spurious `E_DESCRIPTION_STALE` prompts and a what-changed report that
flags untouched columns.

**Recommended resolution.** Either define the normalization explicitly
(materialize slices to offset 0, decode dictionaries, canonical
all-valid = absent validity buffer, zero-filled padding, little-endian), or —
simpler and more robust — digest the **logical values** rather than the
buffers, which is what the feature actually means and what makes it stable
across encodings.

**Concretely.** The data digest is "SHA-256 over the canonical IPC
serialization". Arrow leaves choices that change bytes without changing values:

| Choice | Affects bytes |
|---|---|
| record batch size | yes |
| dictionary encoding | yes |
| buffer padding / alignment | yes |
| compression codec | yes |

Two writers emitting the same logical table can produce different digests, so
"canonically serialized" needs pinning down before the seal envelope freezes.

*Cases:* none yet (blocks `companion/seals`)

---

## Found while writing the decision memo

These three surfaced from working the existing findings through to decisions
(see [DECISIONS.md](DECISIONS.md)), not from a fresh reading.

### AMB-036 — "the registry" (singular) vs. "vocabulary artifacts" (plural)
**§7.1, §7.4 vs §5.6, companion §5.3 · contradiction · RESOLVED 2026-07-29**

§7.4 describes a federated world: *"Every vocabulary file declares a namespace
prefix … A deployment's effective ontology is the union of the vocabulary
artifacts it loads."* §5.6 and companion §5.3 pin **one** registry — a single
`version` and a single `hash`.

So when a deployment loads `core` plus `cf` plus `hydro`, the seal names one of
them and the consumer cannot reconstruct the ontology that was actually used.
That defeats claim 3 of companion §5.1 — *"checked at a stated level against a
stated registry"* — for every deployment that uses federation at all, which
§7.4 presents as the normal case.

Also undefined: whether a registry and a vocabulary file are the same format.
§7.1's header has no `prefix` field; §7.4 says every vocabulary file has one.
Can a vocabulary file declare `[unit.…]` entries?

**Field evidence that the definitions are missing:** the project editor,
reading the finished documents, could not say what a vocabulary is versus a
registry. §3's Terminology defines *Registry* as "the versioned data artifact
defining units, dimensions, display forms, and quantity kinds" and *Vocabulary*
as "a namespaced registry fragment contributed by a domain" — "fragment"
being the only word distinguishing them, and it is never given content.

**Proposed definitions** (the K2 model, stated plainly):

- **Registry** — the artifact that defines *measurement itself*: `[dimension.…]`
  and `[unit.…]` entries — vectors, factors, offsets, deltas, aliases, display
  forms. Exactly **one** is loaded per deployment. PR-gated, because every
  conversion factor in the deployment flows from it. Answers: *what is a MW,
  and what is it worth in W?*
- **Vocabulary** — an artifact that defines *domain meaning*:
  `[quantity."px:…"]` entries only — kinds, parents, `summable_with`,
  `opposes`, `rate_of`. **Many** loaded per deployment, each owning a namespace
  prefix, each on its own release cadence. Answers: *what does this column
  mean, and what may it be combined with?*

A worked example. A hydro group ships `hydro.toml` declaring `prefix = "hydro"`
and `[quantity."hydro:streamflow"] parent = "core:volume_flow"` — legal, theirs
to define. If the same file also declares `[unit.acre_ft]`, that is (under this
model) not a vocabulary any more: units belong to the registry, because a unit
definition carries a conversion factor that every deployment loading the file
would silently trust. The current spec text permits no such distinction because
it never says which tables may appear in which artifact — which is also what
makes AMB-037's `bbl` collision possible and AMB-043's `[unit.MW]` shadowing
attack silent.

**Recommended resolution.** Two formats with distinct roles: a **registry**
(units and dimensions, PR-gated, exactly one loaded) and **vocabularies**
(quantity kinds, federated, many loaded), each versioned and hashable. Add
`vocabularies: [{name, version, hash}]` beside `registry` in §5.6 and companion
§5.3. This is wire-visible and should land before M2.5 freezes the seal
envelope.

**RESOLVED 2026-07-29 — as recommended (K2).** A vocabulary declaring
`[unit.…]` or `[dimension.…]` is `E_REGISTRY_INVALID` (AMB-043 check 7). The
hybrid option — a namespace-guarded unit whitelist so domains can mint
contextual units without a core PR — was noted as a backward-compatible
extension to revisit if `household_yr@CO`-style units create pressure; it was
not adopted now. §3's Terminology gets the two plain-language definitions
recorded above.

**Concretely.** The failing case is a deployment loading several
artifacts:

```
loads: core@2026.07 + cf@2026.07 + hydro@2.1
seal records: registry = { name = "core", version = "2026.07", hash = "…" }
```

The seal names one artifact and the ontology came from three, so companion
§5.1's claim 3 — *checked at a stated level against a stated registry* — cannot
be reconstructed. Resolved by splitting the two roles and adding a
`vocabularies` list beside `registry`.

*Cases:* none — structural. Every parsing fixture depends on the registry
half via AMB-026.

---

### AMB-037 — unit symbols are not namespaced
**§7.4 vs §6.1, §7.2 · contradiction · RESOLVED 2026-07-29**

§7.4: *"every **term** is namespace-qualified (`core:power`,
`cf:air_temperature`, `hydro:streamflow`)."*

Unit symbols are bare everywhere — `[unit.MW]`, `"unit": "MW"`, `MW*h` — and
they *cannot* be qualified, because `:` is not in the symbol lexical class under
any reading of AMB-005. So either §7.4's "every term" is false, or units are not
terms; either way the consequence is unstated and it is the more dangerous of
the two cases.

Quantity-kind collisions are prevented by prefixes and caught at load. Unit
collisions have neither. If two loaded vocabularies both define `bbl` with
different factors, nothing says what happens — and canonical form, the equality
and hashing primitive, is built directly on unit symbols. §7.2's
`household_yr@CO` is exactly a domain-minted contextual unit, so this is not
hypothetical.

**Recommended resolution.** Now: units stay global, and a symbol defined twice
across loaded artifacts is a **load-time error**, symmetric with §7.4's prefix
collisions. Correct "every term" to "every quantity kind". Later: namespaced
unit symbols in grammar v2, tracked in §12 alongside rational exponents — both
change term syntax and should migrate together rather than in two grammar
versions.

**RESOLVED 2026-07-29 — as recommended (L2 now, L3 deferred to grammar v2).**
Under the AMB-036 ruling the collision surface shrinks anyway — only the
single PR-gated registry defines units, so a cross-artifact symbol collision
can only arise between registry versions or via a vocabulary illegally
declaring units, both of which `E_REGISTRY_INVALID` now catches.

**Re-examined 2026-07-31**, asked directly: would namespaces let a user fall
back to `oil:bbl` vs `water:bbl` and so fix AMB-058/059?

*What they fix.* Composition, largely — a union of two registries stops
colliding, exactly as it already does for quantity kinds. And AMB-059's silent
case becomes **a correct conversion rather than an error**, because two
distinctly-spelled units of the same dimension are commensurable, which is the
best available outcome and better than a load failure.

*What they do not fix.* Namespacing makes the string carry the publisher's
chosen **label**, not the artifact's **identity**. Three residual cases stay
silent, and they are the ones that bite:

- a lookalike `core` registry redefining `core:MW` (AMB-047's open hole);
- version drift — `core@1.2` and `core@1.3` both say `core:therm`, one with a
  corrected factor;
- two independent publishers both claiming the `oil:` prefix, which §7.4 makes
  a load-time error with no lever — AMB-058 again, moved from symbol to prefix.

**This is the AMB-055 argument, recurring.** Long-name canonical form was
rejected because two registries can both spell it `foot`: the ambiguity lives
*across artifacts*, so enriching the string cannot resolve it. `oil:bbl` is the
same move and fails the same way — `oil@1.4` and `oil@2.0` both spell it
`oil:bbl`. Only the pin distinguishes them. Namespaces reduce how often
AMB-059's boundary check fires; they do not remove the need for it.

*A cheaper interim, measured.* `_` is **already** in the symbol lexical class
and `:` is not, so qualified spellings need no grammar change at all — a
composition artifact (AMB-058 option 3) can rename on import:

```
     oil_bbl -> canonical "oil_bbl"
   water_bbl -> canonical "water_bbl"
   distinct canonical forms? true
   and they CONVERT rather than error: y = x × 13250941244/9936705933
```

That is the whole user-facing benefit — a fallback spelling when two registries
collide, with conversion between them — available under grammar v1 today. What
it lacks is *interoperability*: a rename is local, so two deployments importing
the same registry may pick different local names, where a publisher-declared
namespace would give both the same spelling. That interop gain is the real case
for L3, and it is contingent on namespace labels being globally trustworthy,
which is governance (§12's vocabulary index) rather than mechanism.

Term ordering is not an argument either way: `:` (0x3A) and `_` (0x5F) produce
identical relative orderings under §6.3's bytewise sort, since the separator
only matters between symbols already sharing a prefix.

**Standing recommendation unchanged: L3 stays deferred to grammar v2**, now for
a sharper reason than "it changes canonical form" — it is an ergonomic and
interop improvement, not a correctness fix, and shipping it as though it closed
AMB-059 would leave the silent case in place while looking solved.

*Cases:* none — structural.

---

### AMB-038 — the two documents disagree about whether conversion keeps a description
**§5.5 vs companion §5.2 · contradiction · open**

Spec §5.5: *"derived columns (**converted**, integrated, renamed) get no
description unless one is explicitly authored."*

Companion §5.2: *"byte change is not meaning change (**a unit conversion
changes every byte while the description stays true**; a small value correction
changes meaning while the bytes barely move)."*

The same operation, opposite outcomes, in documents published together. And the
companion's version is load-bearing: it is the justification for never wiping
descriptions automatically, so it cannot simply be dropped.

**Recommended resolution.** The companion is right — MW → kW does not change
what the column is a description of. Remove "converted" from §5.5's derived
list. Remove "renamed" too: renaming changes neither data nor meaning, and if a
rename can signal repurposing then so can a filter, which the list does not
mention. See [AMB-031](#amb-031--passes-through-unchanged-is-undefined), whose
propagation table this settles one row of.

**Concretely.** The same operation, two rules:

| Document | Says |
|---|---|
| spec §5.5 | a conversion **keeps** the description |
| companion §5.2 | a conversion **drops** it |

`MW → kW` on a column described as "net export at the meter" therefore either
keeps that sentence or loses it, depending on which document a binding was
written from.

*Cases:* none yet (blocks `type-functions` and `wire`)

---

## Encoding of metadata values

### AMB-039 — canonical form is undone one layer down by JSON escapes
**§5.2, §6.3 · gap · DECIDED — prohibit**

§6.3 guarantees one spelling per unit expression. §5.2 then carries that
spelling inside UTF-8 JSON, and JSON gives every string many byte encodings:
`"MW"`, `"\u004dW"`, and `"\u004D\u0057"` all decode to a value a reader
accepts. `MW/h` may be written `MW\/h` or `MW\u002Fh`, since JSON permits an
escaped solidus.

Nothing in §5.2 or §6.3 says whether "Writers MUST emit canonical form"
constrains the decoded value or the bytes, and both readings break something:

- **decoded value only** — the wire bytes are not a function of the unit, so two
  producers tagging identical columns produce different `data_digest`s
  (companion §5.2 step 3), and a reader that merely re-serializes changes the
  digest without changing the data;
- **bytes** — then §5.2's round-trip preservation requirement extends to the
  *escape form*, which no JSON library preserves by default.

Escapes also admit inputs the unit grammar has no opinion on because it never
expected to see them: control characters (`\u0001`), unpaired surrogates
(`\ud800` — valid JSON, not valid UTF-8 once decoded), and a byte-order mark
(`\ufeff`).

**Decision: escape sequences are prohibited outright in the `unit` value and in
every other unit-valued field (`base.unit`).** A `\` anywhere in the raw JSON
token is `E_BAD_METADATA` — detected at the metadata layer, before the unit
grammar applies.

The prohibition is **expressively free**. JSON mandates escaping only `"`, `\`,
and U+0000–U+001F; under AMB-005's ASCII symbol class a unit string may contain
only `A-Za-z0-9_@`, `^`, `*`, `/`, `-`, and `1`. No character that can legally
appear in a unit string requires an escape, so this removes an entire class of
encoding ambiguity at zero cost.

Note the coupling: it is *free* only under AMB-005's ASCII class. Under a
Unicode symbol class the prohibition still holds but stops being sufficient,
because NFC/NFD would reintroduce several byte encodings of one rendered symbol
— one more reason to keep symbols ASCII.

**Concretely.** Both of these decode to the identical unit string, and
only the raw token distinguishes them:

```
decoded value : Some("MW")
raw token     : "\"MW\""        vs   "\"\u004dW\""
```

Canonical form is a function of the *decoded* value, so both canonicalize to
`MW` — while the wire bytes differ, and so does any digest over them. Decided:
escapes are prohibited in unit-valued fields, checked against the raw token.

*Cases:* see `fixtures/parsing/08-encoding.json`

---

### AMB-040 — escapes and permitted code points in the rest of the metadata
**§5.2, §5.5, §5.6, §7.2, companion §5.3 · gap · open**

AMB-039 settles the unit value. Everywhere else is open, and there is no single
answer, because the carriers differ in whether an escape syntax even exists:

| Carrier | Contents | Escape syntax | Free to prohibit? |
|---|---|---|---|
| `ARROW:extension:metadata` | `unit`, `base.unit` | JSON | **Decided** — AMB-039 |
| same | `grammar`, `temporal.*`, `quantity` | JSON | Yes if all ASCII — but `quantity`'s CURIE character class is itself undefined |
| `unitarrow:display_name` | short label | **none** — raw Arrow metadata string | N/A; the question is *which code points* |
| `unitarrow:description` | free prose | **none** | N/A; same |
| `unitarrow:references` | `url`, `title`, `digest` | JSON | `url` and `digest` yes (ASCII); `title` is prose |
| `unitarrow:provenance` | mode, versions, hashes, timestamp | JSON | Yes — all ASCII |
| seal (companion §5.3) | publisher, key_id, base64, digests | JSON, **signed** | Yes except `publisher` |
| registry | `display.latex` etc. | TOML basic strings — §7.2 already uses an escaped backslash | Only by switching to literal strings |

So "prohibit escape characters" names one of two questions:

**1. Escapes.** Applies to JSON- and TOML-carried values. Free to prohibit for
every machine-readable field. **Impossible for free prose**, which legitimately
contains `"` and therefore requires `\"` in JSON.

**2. Permitted code points.** Applies to the raw-string values —
`display_name` and `description` — where there is no escape syntax to prohibit.
This is the larger exposure and nothing constrains it today:

- **Control characters** in a display name break terminal output, CSV export,
  and log lines. They cannot appear in a JSON-carried value without an escape,
  but a raw Arrow metadata string has no such gate.
- **Bidi overrides** (U+202A–U+202E, U+2066–U+2069) and zero-width formatters
  (U+200B–U+200D) let a description render as something other than what it
  contains. These strings appear on badges and provenance surfaces, which
  companion §5.1 makes normative for UX — so this is a spoofing surface with a
  signature over it.
- **Non-NFC forms** are the quiet one. Descriptions are inside the data digest,
  so a table passing through any normalizing layer comes out with a different
  digest and a broken seal — reported as "modified after publishing" while the
  prose is visually unchanged.
- **Byte-vs-character counting** for AMB-029's size limits stays ambiguous while
  escapes are permitted: `\u00e9` is six bytes of JSON and two of UTF-8.

**Recommended resolution.** Prohibit escapes in every non-prose JSON and TOML
value. For prose, permit escapes but constrain code points: forbid C0 and C1
controls except LF in `description`, forbid unpaired surrogates, forbid bidi
and zero-width format characters, and require NFC. See
[DECISIONS.md](DECISIONS.md) Cluster O.

*Cases:* none yet (blocks `wire` and `companion/seals`)

---

## Scale and extensibility

### AMB-041 — tagging a wide table costs one authored line per column
**§1 goal 5, §5.1 · gap · open**

§1 goal 5 promises tagging is "O(columns)" and a "zero-cost drop-in". That is
true of the *operation*; it is not true of the *authoring*. The only interface
the documents describe is one unit per column — §5.1's per-field extension, and
the README's `ua.tag(df, {"power": "MW", "gas": "GBtu"})`.

The shape this breaks on is routine in energy systems: an hourly generation
table with one column per generator and one timestamp column. Hundreds to
thousands of columns, all `MW`, one exception. The authored form is a dict
literal with N identical values, regenerated whenever the generator set
changes.

This is precisely the shape roadmap **M3's dogfood gate** is most likely to fail
on — *"after three weeks of normal GAT use, columns are still being tagged
voluntarily"* is not a plausible outcome if tagging a routine table means
emitting a thousand-entry literal.

**Measured wire cost** (pyarrow 21, IPC stream, float64 columns):

| Shape | Untagged | Tag only | + display_name + description |
|---|---|---|---|
| 2000 cols × 8760 rows | 140.4 MB | +320 KB (**0.2 %**) | +800 KB (0.6 %) |
| 2000 cols × 24 rows | 0.58 MB | +320 KB (**54.8 %**) | +800 KB (136.8 %) |
| 500 cols × 24 rows | 0.15 MB | +80 KB (54.6 %) | +200 KB (136.3 %) |
| 50 cols × 8760 rows | 3.58 MB | +8 KB (0.2 %) | +20 KB (0.6 %) |

160 bytes per tagged column, 400 with documentation. Negligible on an annual
table; dominant on a daily slice. The pathological case is many small
independent messages — a service returning per-day slices pays the full schema
every time — while Flight streams and Parquet files amortise it once.

**Recommended resolution.** Two parts, and the split matters.

*Authoring:* patterns that expand to per-field metadata at write time —
`ua.tag(table, {"gen_*": "MW", "timestamp": None})`, a `rest` sentinel, a skip
set. Purely a binding concern; **nothing pattern-shaped reaches the wire**.

*Spec:* add a §2 non-goal, **"No schema-level unit defaults"**, so the
apparently obvious optimisation is rejected once rather than re-proposed. Three
reasons, all fatal:

1. **Projection.** `select(["gen_0007"])` must keep its unit. A schema-level
   default survives no column selection in any engine, and projection is the
   most common operation on a wide table.
2. **Graceful degradation** (§1 goal 3). An unaware consumer reads *field*
   metadata. Under schema-level defaults it sees 2000 untagged columns and one
   blob it cannot interpret.
3. **§5.3's tagged/untagged distinction.** Is a column with an inherited default
   tagged? Every rule in §8 keys off that answer, and there is no good one.

The per-column bytes are the price of those three properties. State the trade in
§1 goal 5 rather than leaving it to be rediscovered.

*Cases:* none yet (blocks `wire`)

---

### AMB-042 — column digests and republish dispositions do not scale
**companion §5.2, §5.3, §5.4 · gap · open**

`column_digests` is an object with one entry per column (companion §5.3):
measured, **86 bytes per column, 172 KB** on the 2000-column table, inside the
signed envelope. How much that costs depends entirely on artifact granularity
(pyarrow 21, IPC stream, 2000 float64 columns × 8760 rows):

| Scenario | Unsealed | Sealed + column digests | Digest overhead |
|---|---|---|---|
| One file, one 8760-row batch | 140.8 MB | 140.9 MB | 172 KB (**0.12 %**) |
| One file, 365 daily record batches | 175.8 MB | 175.9 MB | 172 KB (**0.10 %**) |
| 365 separately sealed daily files | 330 MB | 393 MB | 63 MB (**19 %/file**) |

Inside one artifact the digests are carried **once**, in schema metadata — record
batches never repeat them, so batching is free. The cost appears only when each
slice becomes its own sealed artifact, and even there the digests are not the
main offender. One sealed daily file decomposes as: 384 KB data, 104 KB base
schema, **320 KB unit tags**, 172 KB digests, 96 KB batch framing — 1.08 MB
total, **2.8× the data**, of which the repeated *tagged schema* is the largest
metadata line (117 MB/year vs 63 MB/year for digests).

A second, sharper consequence of the daily-batch scenario: digests MUST be
**chunk-invariant** — computed over a column's logical values across all record
batches. Scenarios A and B above contain byte-identical data; any per-buffer or
per-batch digest gives them different `column_digests`, breaking both the
what-changed report and the republish-disposition trigger on a no-op re-chunk.
This strengthens AMB-035's logical-values recommendation and must be stated
explicitly.

Worse is the republish flow (§5.2): strict mode *"refuses to seal without an
explicit per-column disposition"*. A systematic correction across a wide table —
a re-run of the whole generation model — demands a disposition for every
affected column. Two thousand of them, one at a time.

Neither is wrong in a narrow table; both are unusable in a wide one, and the
spec offers no guidance on either. `column_digests` is marked optional, but
nothing says when omitting it is the right call, and the disposition flow
*depends* on it — so on a wide table a publisher is pushed toward omitting
digests precisely where the what-changed report would be most valuable.

**Recommended resolution.** Bulk dispositions (glob or all-affected), a
documented default for omitting `column_digests` above some column count, and an
acknowledgement in §5.4 that the what-changed report degrades to table
granularity when they are absent.

*Cases:* none yet (blocks `companion/seals`)

---

### AMB-043 — a custom catalog can break the spec, and only one guardrail exists
**§7.4, §7.2, §7.3 · gap · open**

§7.4 makes federation the normal case: domain vocabularies live in their own
repositories, on their own release cadence, reviewed by their own experts, and
only the registry *schema* and the `core:` namespace are PR-gated. Arbitrary
third-party artifacts are therefore loaded into the resolution path that
canonical form, dimension algebra, and every conversion factor depend on.

The spec states exactly **one** load-time validation rule: *"prefix collisions
are a load-time error."* Everything else is unchecked. A careless or hostile
catalog can currently:

- **redefine a core unit** — `[unit.MW] factor = [1, 1]`, and every MW column in
  the deployment converts wrong, silently (there is no collision rule for unit
  symbols at all — AMB-037);
- **shadow one by alias** — `aliases = ["MW"]` on an unrelated entry, which is
  the same attack and harder to spot;
- **break canonical form** — define a symbol containing `*`, `/`, `^`, or
  whitespace, which the §6.1 production forbids in unit *strings* but nothing
  enforces on registry *keys*;
- **overflow the dimension representation** — `vector = { length = 200 }`, or
  redefine a base dimension name;
- **supply a degenerate factor** — `[1, 0]`, `[0, 1]`, or a negative scale;
- **cycle the kind graph** — `a.parent = b`, `b.parent = a`, hanging a naive
  walker; likewise `same_as`;
- **dangle a `delta`, `variants`, `rate_of`, or `summable_with` reference**.

The load-bearing point: **§7.4 makes vocabularies sealable, and a seal proves
attribution, not conformance.** Companion §5.4's *Verified* state says the
artifact is unchanged and came from a known publisher. It says nothing about
whether the artifact is a valid registry. So validation MUST run on every
artifact regardless of seal state, and its failure MUST NOT be phrased as a
trust failure — a valid signature over a spec-breaking catalog is exactly the
case the user will find hardest to reason about.

There is also no error code: §10 has nothing for an invalid registry.

**Recommended resolution.** A normative load-time validation section in §7 — the
full checklist is in [DECISIONS.md](DECISIONS.md) Cluster Q — plus
`E_REGISTRY_INVALID` in §10 carrying the failed check and the offending key.

**Concretely.** A third-party catalog can redefine measurement itself:

```toml
[dimension.length]
vector = { time = 1 }        # length is now time

[unit.MW]
dimension = "power"
factor = [1, 1]              # a megawatt is one watt
```

The first is caught (§6.2's base dimensions are frozen). The second **loads
cleanly** and every conversion in the deployment is wrong by 10⁶ — see
AMB-047.

*Cases:* none yet; warrants its own `registry/` conformance category of
deliberately malformed catalogs, which no document currently plans for.

---

## Casting, warnings, and temporal storage

### AMB-044 — the cast rule forbids conversions its own error code calls legal
**§5.1 vs §10 · contradiction · open (direction ruled)**

§5.1:

> converting a column whose storage is an integer or decimal type MUST either
> (a) promote storage to `float64`, or (b) fail with `E_CAST_LOSSY` if the
> caller requested storage preservation.

There is no third branch — so an **exact** storage-preserving conversion is
forbidden. But int64 kWh × 1000 → Wh is exact wherever it does not overflow,
and decimal128 rescaling by a power of ten is a pure scale change. §10 then
describes the code as firing only when conversion "requires **lossy** storage
change" — implying exact preservation succeeds, which §5.1 does not permit.

**Editor's direction (2026-07-29):** casting should be supported with explicit
lossiness handling — exact, lossy-permitted, or within a declared tolerance.
Proposed shape: conversion takes a storage policy —

- `promote` (default): result is `float64`, current behavior;
- `preserve`: keep storage; succeed when the rescale is exact and overflow-free,
  else `E_CAST_LOSSY`;
- `preserve(tolerance=…)`: keep storage; succeed when the per-value rounding
  error is within the declared tolerance, emitting `W_CAST_LOSSY`; else
  `E_CAST_LOSSY`.

This makes §10's description true, unlocks the exact integer/decimal cases, and
gives the "close enough" case an explicit opt-in instead of a silent rounding.
Open sub-questions: tolerance semantics (absolute vs relative, per-value vs
aggregate) and whether a tolerance used belongs in provenance (a seal claiming
`strict` over tolerance-cast data should probably say so).

**Concretely.** §5.1 forbids a storage-changing cast; §10 defines
`E_CAST_LOSSY` for a cast that *would* lose precision:

| Conversion | §5.1 | §10 implies |
|---|---|---|
| `MW → kW` on `float64` | allowed | allowed |
| `MW → kW` on `int32` | **forbidden outright** | allowed unless lossy |

If storage-preserving casts are forbidden, `E_CAST_LOSSY` can never fire for
them — the rule and its error code describe different systems.

*Cases:* none yet (blocks `factors` and `wire`)

---

### AMB-045 — the warning taxonomy is an ellipsis
**§10 · gap · open**

§10's entire specification of warnings:

> Warnings mirror the codes (`W_TAINT_INTRODUCED`, `W_COERCED`, …).

The `…` is doing normative work. Warnings are not a sealing-layer nicety — they
are the **permissive-mode half of the whole semantics layer**: everywhere
strict errors, permissive warns, and §8.2 constrains their *cardinality* (once
per taint introduction per lineage, once at export — never per element). The
conformance fixture format already asserts warning codes (`expect.warnings`),
and this register has been minting them out of necessity —
`W_UNRESOLVED_SYMBOL` (AMB-021), `W_METADATA_SIZE` / `W_DESCRIPTION_STALE`
(AMB-030), `W_CAST_LOSSY` (AMB-044) — with no table to land in.

"Mirror the codes" also under-determines the mapping: `W_COERCED` mirrors no
`E_COERCED`; nothing says which codes have warning counterparts.

**Recommended resolution.** Give §10 a second table enumerating warning codes
with the same wire-stability rule as errors, each row naming its trigger and
its cardinality. Core set: `W_TAINT_INTRODUCED`, `W_COERCED`,
`W_UNRESOLVED_SYMBOL`, `W_CAST_LOSSY`, plus the companion's `W_ADVISORY` and
(pending AMB-030) `W_METADATA_SIZE`, `W_DESCRIPTION_STALE`.

**Concretely.** §10 lists warnings as `W_TAINT_INTRODUCED`, `W_COERCED`,
"…". The implementation currently emits four:

```
W_TAINT_INTRODUCED   W_COERCED   W_UNRESOLVED_SYMBOL   W_CAST_LOSSY
```

Two of those appear in no spec text, so a binding written from §10 alone would
not know to produce them — and warnings are as wire-visible as errors.

*Cases:* none yet (blocks `propagation`, which asserts warning counts)

---

### AMB-046 — Arrow's temporal types are silently untaggable
**§5.1, §8.5 · gap · open**

§5.1's allowed storage types are floats, decimals, and integers. Arrow's
`timestamp`, `duration`, `date`, and `time` types are absent — so a tag on any
of them is presumably `E_BAD_PLACEMENT`, though the text never says so, and the
rationale is never stated.

For **timestamps** the exclusion is right: a timestamp is a point on a
calendar, not a quantity; its second/millisecond granularity is encoding
resolution already carried by the Arrow type, and timezone conversion is
Arrow's job, not a unit rescale. This should be *stated*, not implied.

For **duration** the exclusion is inconsistent: a duration is a genuine
physical quantity (dimension `{time: 1}`), and the same values stored as plain
`int64` seconds CAN be tagged `"s"` — so taggability depends on which Arrow
type the producer happened to choose, and the better-typed choice gets less
capability. A `duration` column also cannot carry a `quantity` kind or
`temporal` block, both of which are meaningful for it.

There is also an §8.5 interaction worth stating: `integrate` takes its period
from the *metadata* (`temporal.period`), never from timestamps sitting in a
neighbouring column. An implementation that infers per-row periods from
timestamp deltas is inventing a claim, which §8.5 rule 4 forbids in spirit.

**Recommended resolution.** State the timestamp/date/time exclusion and its
rationale in §5.1 explicitly. For `duration`: either admit it as a storage type
with a load-time consistency check (the tagged unit must be commensurable with
time, and conversion must respect the type's own TimeUnit), or state the
workaround (cast to integer storage) and defer to §12. No position taken here —
but silence is the one wrong option, because implementers will guess
differently.

**Concretely.** A duration column is a quantity, and Arrow already types
it:

| Arrow type | Carries a unit? | Taggable under §5.1? |
|---|---|---|
| `Float64` | no | yes |
| `Duration(SECOND)` | **yes, in the type** | unstated |
| `Timestamp(MILLI, "UTC")` | **yes, in the type** | unstated |

Tagging `Duration(SECOND)` with `unit: "h"` would be two units disagreeing in
one column. §5.1 neither permits nor forbids it.

*Cases:* none yet (blocks `wire`)

---

### AMB-047 — nothing constrains a replacement registry's *contents*
**§7.2, §7.4, §5.6 · gap · open**

Found by generating the guardrail table on the docs site: of sixteen hostile
registry mutations, fifteen are caught at load time. One is **accepted** —
redefining a core unit's factor.

```toml
[unit.MW]
dimension = "power"
factor = [1, 1]        # a megawatt is now one watt
```

This loads cleanly and every conversion in the deployment is then wrong by
10⁶, silently. No check fires, and correctly so under the current rules: within
a single artifact, `MW = 1 W` is simply *a definition*. There is nothing for it
to contradict.

AMB-037's cross-artifact collision rule does not help. That rule catches a
symbol defined **twice** across loaded artifacts; a wholesale *replacement*
registry defines each symbol once. And the AMB-036 ruling — only the PR-gated
registry defines units — constrains *which kind of artifact* may define units,
not what the artifact says.

What exists today is only identity: §5.6 and companion §5.3 pin the registry by
name, version, and content hash, so a consumer can tell **which** registry
produced a table. Nothing tells them whether that registry is *the* `core`
registry or a lookalike with the same name and a different hash. And per
AMB-043, a seal makes a hostile catalog *attributable*, not *correct*.

**Recommended resolution.** Three options, in increasing strength:

1. **Trust-store pinning for registries** — reuse companion §5.5's mechanism:
   an org commits the expected `core` registry hash, and a mismatch is a loud
   event rather than a silent substitution. Cheapest, and it composes with
   machinery that already exists.
2. **Self-consistency checks** — assert relationships a real registry must
   satisfy regardless of provenance: a unit whose symbol carries an SI prefix
   must have the corresponding factor relative to its unprefixed sibling
   (`kW`/`W` = 1000, `mW`/`W` = 1/1000). Catches the `MW = 1 W` case directly,
   costs nothing at runtime, and needs the prefix table to be data.
3. **A signed canonical `core`** — ship the core registry's hash compiled into
   `unitarrow-core`, and warn when a loaded registry claiming `name = "core"`
   does not match. Strongest, but couples library releases to registry
   releases, which §7's whole tzdata-model premise exists to avoid.

Option 1 plus 2 is probably right: pinning for deliberate substitution,
self-consistency for accidents and typos.

*Cases:* none yet; the live guardrail table on the docs site
(`docs/generated/registry.md`) shows the gap with a ⚠️ and will keep showing it
until this is closed — the generator reports what the implementation actually
does, so the row cannot silently become a false reassurance.

---

### AMB-048 — §6.1 requires prefixed units as distinct entries and says nothing about authoring them
**§6.1, §7.2 · gap · open (direction implemented)**

§6.1, in a parenthesis:

> prefixed units (`kW`, `GBtu`) are distinct registry entries

That is the right rule — it keeps resolution a flat lookup, so canonical form
never has to decide whether `ft` is *foot* or *femto-tonne*. But it is the whole
of what either document says about prefixes, and it leaves the authoring problem
untouched.

**The scale.** SI has 24 prefixes since the 2022 CGPM additions. At one entry
each, a prefixable unit costs **25 registry entries**; twenty prefixable units
cost **500**. Every one carries a hand-typed exact-rational factor, and a
mistyped prefix factor is exactly the silent-corruption case
[AMB-047](#amb-047--nothing-constrains-a-replacement-registrys-contents)
describes: `[1000000, 1]` where `[1000000000, 1]` was meant loads cleanly and is
wrong by 10³ forever.

**Why not parse prefixes instead.** Splitting an unresolved symbol into
prefix + base would keep the registry tiny and is genuinely ambiguous. Measured
against a modest unit set, each of these is a real unit *and* a real prefix
reading:

| Symbol | As a unit | As prefix + unit |
|---|---|---|
| `cd` | candela | centi + day |
| `ft` | foot | femto + tonne |
| `min` | minute | milli + inch |
| `Gy` | gray | giga + year |
| `PS` | metric horsepower | peta + siemens |

Canonical form cannot tolerate that — one spelling per expression is its entire
job — so parse-time prefixes are not an option, and §6.1's rule stands.

**Implemented direction.** A unit declares a prefix set, and prefixed forms are
**derived at resolution time** from one authored value by exact rational
arithmetic:

```toml
[unit.W]
dimension = "power"
factor = [1, 1]
prefixes = "si-engineering"   # 12 entries; "si" gives all 24
```

Registry source stays small and the ambiguity above becomes a **load-time error
naming both readings** rather than a machine-dependent resolution. Guardrails
shown live on the docs site: collision with an explicit unit, with an alias, or
with another expansion; affine units refused (a prefixed degree scale is not a
unit); unknown prefix set; exact-rational overflow.

**Revised 2026-07-30 — derived, not stored.** The first implementation
materialised every combination into the registry. Measured on a 20-unit SI-ish
registry, that turned a 3 KB source file into **500 stored units and 1049
aliases** — a 50× amplification, all heap-allocated strings, in a module that
must fit a WASM budget.

Resolution is now its own branch: authored symbols are looked up first, and only
on a miss is a prefix split attempted, with the unit synthesized from its base.
Verbose names split the same way (`petawatt` → `peta` + `watt`), so spelling a
prefixed unit out works without storing its name either.

The correctness property is unchanged, and this is the part worth being precise
about: **the load-time enumeration still happens**. Every prefix × base
combination is checked for collisions and for factor overflow at load; only the
*storage* was dropped. So a symbol can never be reachable both as an authored
entry and as a prefix split — which is what makes trying explicit entries first
a fast path rather than a tie-break, and what keeps `ft` from ever being
ambiguous between foot and femto-tonne.

`Registry::unit_count()` now reports what the registry *resolves*;
`authored_count()` reports what it stores. The gap between them is the feature.

**Two things the implementation surfaced that were not obvious:**

1. **Micro must be `u`, not `µ`.** Symbols are ASCII under the AMB-005 ruling, so
   the Greek mu cannot appear in a unit string — `µW` is a display form only.
   Nothing in §6.1 or §7.2 says this, and it is exactly the kind of detail a
   registry author will get wrong once.
2. **Overflow is rarer than arithmetic suggests, because rationals
   cross-reduce.** 10³⁰ × 52752792631 is 5.3 × 10⁴⁰, past i128 — but `Btu`'s
   denominator (5 × 10⁷) cancels 10⁷ of the prefix, landing at 1.06 × 10³³, so
   the full SI range on Btu loads fine. It overflows only when the denominator
   is 1 and gives cross-reduction nothing to work with. A rough estimate said
   `QBtu` would overflow; it does not.

**Open questions this leaves.**

- Which prefix sets are normative, and are they registry data or spec constants?
  (Implemented as constants `"si"` and `"si-engineering"` — but see AMB-053: the
  boundaries were arbitrary, and an explicit range may be the better shape.)
- Binary prefixes (`Ki`, `Mi`, `Gi`) for `count`-dimension units — a separate
  set, or out of scope?
- Should the spec bound prefix range per unit rather than relying on an overflow
  error, so a registry cannot *nearly* overflow and then break when a factor is
  re-derived more precisely?
- Does a prefixed unit inherit its base's `delta`? Currently no — deltas are
  temperature-shaped and affine units cannot be prefixed at all, so the question
  has not bitten, but §7.2 should say.

*Cases:* none in `parsing/` — expansion is registry-load behaviour, so it belongs
in the proposed `registry/` conformance category (AMB-043). Demonstrated live at
`docs/generated/conversions.md`.

---

### AMB-049 — nothing lets a user type or verify a unit in words
**§6.1, §7.2, §5.5 · gap · open (direction implemented)**

Symbols are terse, ASCII, and case-sensitive. Those are the right properties for
a wire format and poor ones at a keyboard, and the specification provides
nothing to bridge the two:

- `display` forms exist (§7.2) but are **output-only** — §6.1's symbol class
  cannot lex `°C`, so a user who types what the chart axis shows gets
  `E_UNIT_SYNTAX` with no indication that `degC` is the answer.
- Under the AMB-005 ASCII ruling, **micro is `u`**, not `µ`. Nothing states this
  and a user typing `µW` has no path to `uW`.
- `mW` and `MW` differ by 10⁹ and by one keystroke, and **nothing offers the
  reverse direction** — no way to ask "what did I just tag this as?" and read
  back *milliwatt* rather than *megawatt*. That check is the cheapest possible
  guard against a class of error that is otherwise entirely silent.
- §10 defines error *codes* but says nothing about whether errors may carry
  suggestions, so "unknown unit" is as much as an implementation is obliged to
  say.

**Implemented direction — three mechanisms, all registry-driven so they stay
correct for whatever units are loaded:**

1. **Verbose input.** Any unit whose display name is a single word also answers
   to that name and its plural: `megawatt`, `megawatts`, `kilowatt`, `hour`.
   These are ordinary aliases — input-only, never emitted, collision-checked.
   Prefix expansion composes them, so `prefixes = "si-engineering"` on a unit named
   *watt* yields *microwatt* through *terawatt* for free. Multi-word names
   (`British thermal unit (IT)`) are **not** turned into aliases: inventing a
   spelling would be guessing at what a user will type.
2. **Reverse lookup.** `Registry::describe(symbol)` returns the display name, so
   `MW → megawatt` and `mW → milliwatt` can be shown back after tagging.
3. **Suggestions.** An unresolved symbol gets case-insensitive matches first
   (`Megawatt` → `megawatt`, the predictable stumble once verbose names exist)
   then near-misses by edit distance (`megwatt` → `megawatt`). A display form
   appearing anywhere in the input is named against its symbol: *"`°C` is a
   display form, not an input symbol — write `degC` (degree Celsius)"*.

**What this needs from the spec.** Whether verbose aliases are normative or an
implementation courtesy matters, because a producer that tags with `megawatt`
expects any conformant reader to resolve it. Either §7.2 states that a
single-word `display.long` is also an input alias — making it part of the
registry contract and conformance-testable — or verbose input stays a local
convenience and producers must not rely on it. The current implementation
assumes the former.

**Open questions.**

- Are verbose aliases case-insensitive? Currently **no** — they are lowercase
  symbols like any other, and `Megawatt` is an error *with a suggestion*. Making
  them case-insensitive would be the only case-insensitive resolution path in
  the system, against §6.1's case-sensitivity.
- Should plurals be aliases at all? Implemented as yes; harmless, and users type
  them.
- Do suggestions belong in §10 alongside the codes, given AMB-013's note that
  §10 defines codes but never the error *payload*?
- Localization: §5.5 says localization is a client concern keyed on `quantity`
  or field name. Verbose input is implicitly English-only, and a non-English
  registry would define its own display names — worth stating rather than
  discovering.

**Concretely.** Both directions now work, and the reverse one is what lets
a user check a tag they did not write:

| Input | Resolves to |
|---|---|
| `megawatt` | `MW` |
| `MW` reads back as | *megawatt* |

Without the reverse lookup a user confirming `MW` means what they think has
nothing to consult but the registry file.

*Cases:* none in `parsing/` — resolution depends on registry contents, so these
belong in the proposed `registry/` category (AMB-043). Demonstrated live at
`docs/generated/errors.md`.

---

### AMB-050 — the "survives the wire" claim has no executable demonstration
**§1 goal 2, roadmap M2 · gap · open**

The project's central claim is that units survive IPC / Flight / Parquet. Every
artifact that asserts it — the spec, this register, the docs site — asserts it in
prose. Nothing lets a reader *check* it.

Roadmap M2 already names the fix as a deliverable: *"the conversation's
Python→IPC→browser demo, formalized: tag in Python, read units in the browser,
convert client-side."* Worth recording what building the first half revealed.

**What exists now.** `unitarrow-core` compiled to WebAssembly, driving a live
playground: canonicalization, dimensions, conversions with exact rationals,
error codes with suggestions. Measured at **159 KB raw / 53 KB gzipped**,
depending on nothing — which is §4's rule ("arrow-rs stays out of the WASM
build") paying off. This proves the *unit engine* in a browser.

**What is missing, and why.** The round trip — build a tagged table, download a
real Arrow IPC file, upload it back, watch the metadata survive — needs
something that can write real IPC bytes. Three options, none free:

| Approach | Cost |
|---|---|
| **arrow-js on the page** | ~1 MB vendored. Real Arrow files any reader can open. This is what §4 already anticipates: "arrow-js owns table handling" |
| **Hand-rolled IPC writer** | Flatbuffers encoding by hand. High risk of producing something only *this* code can read, which would demonstrate nothing |
| **Pyodide + pyarrow** | ~10 MB runtime, and it needs a Python implementation — `unitarrow-py` is still a stub. Also cuts against §4, which specifies JS + WASM for the browser precisely so arrow-rs and Python stay out of it |

Recommended: **arrow-js**, lazily loaded on the demo page only. It is the option
§4 already assumes, it produces files that are genuinely interoperable (which is
the entire point of the demonstration), and it keeps the WASM module unit-only.

**A related measurement worth recording.** Taking both crates `no_std + alloc`
(see AMB-051) moved the module from 171 KB to 159 KB — about 7 %. The bulk is
`core::fmt`, pulled in by the formatted error messages, not by std. Roadmap M1's
"tens of KB" target therefore needs a different lever than dependency removal:
either terser errors in the WASM build, or an acknowledgement that 53 KB
gzipped is the realistic figure for a module that explains its own failures.

*Cases:* none — this is a demonstration gap, not a behavioural one. The
playground lives at `docs/generated/playground.md`.

---

### AMB-051 — nothing states which environments the core must run in
**§4 · gap · open (resolved in the implementation)**

§4 says `unitarrow-core` depends on nothing and "compiles to a small WASM
module", and roadmap M1 budgets "tens of KB". Neither says whether the crate may
depend on an **operating system** — and "no dependencies" and "no std" are
different claims that are easy to conflate.

It turned out to matter less than expected and to be worth doing anyway. An
audit found `std` used for exactly two things in non-test code:
`std::collections::BTreeMap`, which `alloc` also has, and
`impl std::error::Error`, which `core::error::Error` has provided since Rust
1.81. Both crates are now `no_std + alloc`, with the WASM crate supplying a bump
allocator and panic handler behind `cfg(target_arch = "wasm32")` so host builds
still use std for tests and clippy.

The measured saving was ~7 % (171 → 159 KB), so the case for `no_std` is not
really size. It is that the constraint is now **enforced by the compiler**: the
core cannot acquire a filesystem, a clock, or a thread by accident, which is
exactly the kind of dependency that would be discovered late, in the binding
that could least afford it.

**Recommended resolution.** State it in §4: `unitarrow-core` is `no_std` and
requires only `alloc`. It is a one-line addition that turns an implicit
assumption into a checkable property, and it tells binding authors what they can
rely on.

**Concretely.** The core compiles for three targets with different
constraints, and the spec named none of them:

| Target | Constraint |
|---|---|
| host Rust | std available |
| `wasm32-unknown-unknown` | no allocator, no panic handler, no std |
| embedded / `no_std` | `alloc` only |

The middle one is binding: it forced `no_std`, a hand-written allocator, and the
absence of `f64::powi` — decisions invisible in the spec but not optional.

*Cases:* none — architectural.

---

### AMB-052 — nothing says which English spelling a user may type
**§7.2, §5.5 · gap · open (direction implemented)**

The SI brochure writes `metre`, `litre`, `deca`. US usage writes `meter`,
`liter`, `deka`. §7.2's `display.long` holds one string, and — under AMB-049's
verbose-input mechanism — that one string becomes the only word that resolves.
So a registry declaring `metre` silently rejects `meter`, and the user has no
way to know which was chosen except by trying.

Nothing in either document acknowledges this. §5.5 addresses localization but
scopes it to *display*: "Localization is a client concern, keyed on `quantity`
or field name — never on `display_name` string matching." That is about
rendering, and says nothing about what a producer may **type**.

The two halves need separating:

- **Input** is not localization. `metre` and `meter` are two spellings of one
  English word for one unit, exactly like `MW/h` and `MW*h^-1` are two spellings
  of one expression. Accepting both is forgiveness, not i18n.
- **Display** genuinely is a locale question, and remains open. The registry
  declares one `long` name and that is what `describe()` returns; a client
  wanting the other spelling has nowhere to express it. §5.5's "keyed on
  `quantity`" hook is the natural place, and is unspecified.

**Implemented direction.** Both spellings resolve, generated from a curated
table and collision-checked like any other alias. Prefixed forms compose, so
`kilometer` works whether the registry wrote `kilometre` or not, and plurals
come along.

**Why it must be curated rather than a rule.** A generic `-re`/`-er` swap is the
obvious implementation and is unsafe: `tonne` and `ton` are **different units**,
1000 kg against 907.18474 kg. Relating them would be a silent 10 % error on
every mass in the table. This is the same shape as the `ft` / femto-tonne
collision (AMB-048) and takes the same answer — enumerate what is provably safe,
and let load-time collision checks catch the rest.

**Open questions.**

- Is the spelling table normative? Same question AMB-049 raises for verbose
  names generally: a producer tagging `meter` needs to know whether every
  conformant reader resolves it.
- Which spelling *displays*, and can a client ask for the other? Currently
  whatever the registry declared, with no override.
- Are there other safe pairs? `gramme`/`gram` is plausible; `ampère`/`ampere`
  is moot under the ASCII ruling. Each addition needs the `ton`/`tonne` check
  run against it.

**Concretely.** Both spellings resolve to one canonical symbol:

| Input | Resolves to |
|---|---|
| `metre`, `metres` | `m` |
| `meter` | `m` |

The rule is curated rather than mechanical, because a generic `-re`/`-er` swap
would also relate `ton` and `tonne` — which are **different units**, 907.18 kg
against 1000 kg, a 10% error.

*Cases:* none in `parsing/` — resolution depends on registry contents, so these
belong in the proposed `registry/` category (AMB-043).

---

### AMB-053 — the prefix set boundaries were arbitrary
**§6.1, §7.2 · gap · open (corrected in the implementation)**

The prefix subset shipped with AMB-048 ran from `tera` (10¹²) down to `femto`
(10⁻¹⁵) — four steps up, five down. Asymmetric, and the boundary was not chosen
so much as landed on.

Both ends were wrong for this project's own domain. `TWh` is routine in power
systems, `PWh` appears in global generation statistics, and `EJ` is the standard
unit in international energy balances. A set stopping at `tera` fails on the
data the specification was written for.

**Corrected:** `si-engineering` is now symmetric, exa (10¹⁸) to atto (10⁻¹⁸),
powers of 10³ only — 12 prefixes. `hecto`, `deca`, `deci`, and `centi` exist for
`hPa`, `daL`, `dL`, `cm` and belong to registries that want them, reachable via
`"si"` (all 24).

**What this exposes.** The spec names no prefix sets at all — §6.1 only says
prefixed units are distinct registry entries. So every boundary here is an
implementation choice masquerading as a default, and a second implementation
would pick differently. Either the sets are normative and belong in §7, or
`prefixes` takes an explicit range and the named sets are conveniences. The
latter is probably right: `prefixes = { set = "si", from = -18, to = 18 }` keeps
the policy in the registry, where the domain knowledge is.

**Concretely.** The two prefix sets, as implemented:

| Set | Prefixes | Range |
|---|---|---|
| `si` | 24 | 10⁻³⁰ … 10³⁰ |
| `si-engineering` | 12 | 10⁻¹⁸ … 10¹⁸ |

The engineering set is symmetric by construction. An earlier asymmetric range
(tera down to femto) had no principle behind it and would have made `Gt`
available while `Gm` was not.

*Cases:* none yet; belongs in the `registry/` category alongside AMB-048.

---

### AMB-054 — an ambiguous spelling has no declared owner
**§6.1, §7.2 · gap · open (direction implemented)**

`ft` is *foot*, and in a registry that prefixes `t` (tonne) it is also
*femto-tonne*. AMB-048's load-time check catches that — and then offers the
registry author only bad options: drop `ft`, or narrow `[unit.t].prefixes` and
lose `kt`, `Mt`, `Gt`, which are the units global emissions data is reported in.

Neither is what the author wants. The author wants to say **`ft` means foot
here**, and get on with it.

**The design principle, stated once:** ambiguity is resolved **by the registry
author, at load**, never by the user at tag time. `ft` is overwhelmingly more
likely to mean foot than femto-tonne, and asking every user to disambiguate a
case that essentially never arises would tax the common path to serve the empty
one. So the registry declares the winner:

```toml
[registry.prefix_collisions]
ft = "foot is ubiquitous in this domain; femto-tonne is not a real quantity"
```

One table, so "where did this registry make a judgement call?" has one answer —
and the value is the **reason**, required rather than optional. A stale entry —
declaring a symbol with no collision — is itself an error, because it would hide
the next real one.

**Why the reason is mandatory.** The declaration sits in the registry header,
far from the `[unit.ft]` entry it silently affects, so a diff of that entry
cannot show the trade-off: a reviewer sees a foot added, not a femto-tonne
removed. Without a recorded reason the judgement has to be reconstructed from
two distant places, and a later maintainer facing a domain change has nothing to
weigh. Making it optional would have been the same as omitting it — the field
exists to force the sentence, and `Registry::collision_risks` surfaces it as a
column so the omission would have been visible anyway.

**What it costs, precisely.** The shadowed reading becomes **unavailable
entirely**, not merely inconvenient, and the reason is worth stating because the
first implementation got it wrong. A prefixed unit's canonical form (§6.3) *is*
its symbol. Once `ft` belongs to foot, femto-tonne has no canonical spelling —
so a verbose escape hatch (`femtotonne` → the derived unit) would hand back
canonical `ft`, which now means foot. That is a silent corruption of exactly the
kind canonical form exists to prevent. Shadowing therefore removes the reading;
it does not relocate it.

The error message says so rather than leaving it to be discovered:

> `"ft"` is ambiguous: it is an authored unit of `"length"` (foot), and expanding
> `[unit.t]` with prefix `"f"` would also produce it. Resolve it here rather than
> leaving it to readers — either declare it under `[registry.prefix_collisions]`
> as `ft = "<why the authored reading wins>"`, which gives the authored unit the
> symbol and makes `femtotonne` unavailable entirely (its
> canonical form would be `"ft"`, which now means something else), or narrow
> `[unit.t].prefixes`

**Open questions.**

- ~~Should the spec require the *reason* alongside the declaration?~~
  **Decided: yes, required.** The list became a table of `symbol = "reason"`,
  an empty or missing reason is a load error, and `CollisionRisk::reason`
  carries it into the review report. See "Why the reason is mandatory" above.
- Is "authored wins" the only resolution worth supporting? A registry could
  conceivably want the prefixed reading to win, which would need the authored
  unit renamed instead — probably not worth the machinery.
- Should a shadowed reading be reportable? A tool auditing a registry might
  reasonably want to list what became unreachable.

*Cases:* none yet; belongs in the `registry/` category (AMB-043) with the rest
of the load-time behaviour.

---

### AMB-055 — should canonical form use long names instead of symbols?
**§6.3 · design question · open (recommend no, with reasons)**

Raised on review: abbreviations like `ft` feel like they belong in the display
and ergonomic-input space, not in the identity primitive. Spelling canonical
form as `foot` / `femtotonne` would, the argument goes, remove ambiguity
entirely, at the cost of larger unit strings.

The instinct is right about *where abbreviations belong*. The conclusion does not
survive measurement, for three reasons — and the size cost, which was the
expected objection, is not one of them.

**1. Size is a non-issue.** Measured against a real tag: the unit string is a
few bytes of ~160.

| Canonical form | Tag | Long form | Tag | Cost |
|---|---|---|---|---|
| `MW` | 160 B | `megawatt` | 166 B | +6 B (4 %) |
| `MW*h^-1` | 165 B | `megawatt*hour^-1` | 174 B | +9 B |
| `W*K^-1*m^-2` | 169 B | `watt*kelvin^-1*metre^-2` | 181 B | +12 B |

The tag's cost is the extension mechanism, not the unit string (AMB-041). So
the trade the proposal offers is real: it is not paying meaningfully in bytes.

**2. It does not remove ambiguity — it moves it.** The decisive test, run
against two registries that both spell it `foot`:

| Registry | Canonical | Long name | Factor |
|---|---|---|---|
| international | `ft` | foot | 381/1250 = 0.304800000 m |
| US survey | `ft` | foot | 1200/3937 = 0.304800610 m |

Both are correct; they differ in the eighth digit, and surveying cares — 2 cm
over 10 km. Renaming the canonical form to `foot` changes nothing here, because
the ambiguity is **across registries**, not within one. Within a registry, `ft`
is already unambiguous (the shadow rule, AMB-054). Across registries, only the
registry pin — name, version, content hash (§5.6) — resolves it, and it resolves
it identically whichever spelling canonical form uses.

The same holds for `ton`/`tonne`, `gallon` (US vs imperial), `calorie` (IT vs
thermochemical, the same split as Btu). Long names collide on *meaning* exactly
where symbols do.

**3. Most long names are not legal symbols.** Measured on the fixture registry:
**8 of 17** units have a `display.long` that cannot be a canonical form —
"megawatt hour", "British thermal unit (IT)", "degree Celsius", "per unit".
Canonical form would need a mangling rule (`megawatthour`? `megawatt_hour`?),
every implementation would need the *same* rule, and joining words is precisely
the hazard AMB-006 just resolved — `m s` collapsing to `ms`.

**4. Symbols are language-neutral; names are not.** SI symbols are
internationally standardised: `m`, `kg`, `MW` are written identically in every
locale. SI *names* are translated, and even within English split `metre`/`meter`
(AMB-052). Canonical form is the cross-language equality and hashing primitive
(§6.3); making it depend on a spelling choice in one language is a regression
for exactly the property it exists to provide.

**Recommendation: keep symbols.** But the underlying instinct deserves to be
recorded as satisfied elsewhere: abbreviations *are* ergonomic, and the spec now
treats them that way — verbose names resolve on input (AMB-049), display forms
are output-only, and within-registry ambiguity is resolved by the author at load
(AMB-054) rather than by the user at tag time. What remains — the same spelling
meaning different things in different registries — is the registry pin's job,
and no naming scheme substitutes for it.

*Cases:* none — this would be a change to every canonical-form fixture. Recorded
so the question is not re-opened without the measurements.

---

### AMB-056 — a prefix separator, and the grammar-v2 migration it belongs to
**§6.1, §12 · design question · RULED 2026-07-30**

Raised alongside AMB-055: should there be a syntactic marker for prefix + base,
the way `^` marks an exponent — `k:W`, `f:t` — so a prefixed unit is
distinguishable from an authored symbol by *shape*?

This is a genuinely better answer to the collision problem than long names, and
it is worth recording properly rather than dismissing.

**What it would buy.** Collisions become impossible *by construction* rather
than by load-time check: `ft` is the authored foot, `f:t` is femto-tonne, and
nothing can produce both. Three consequences follow:

- AMB-048's load-time enumeration becomes unnecessary — the check exists only
  because the two namespaces currently overlap.
- AMB-054's `prefix_collisions` declaration becomes unnecessary.
- The shadowed reading stops being **unavailable**: femto-tonne regains a
  canonical spelling (`f:t`), which is the one real loss in the current design.

**What it would cost.** If the separator appears in *canonical* form, then
`MW` canonicalizes to `M:W` — and no one on earth recognises `M:W` as a
megawatt. That directly damages §1 goal 3, which promises an unaware consumer
sees "human-readable metadata". If instead the separator is **input-only** and
canonical form stays `MW`, the collision returns at canonicalization time and
nothing is actually gained.

So the design has a fork, and only one branch delivers:

| Variant | Collisions | Shadowed readings | Legibility |
|---|---|---|---|
| Input-only separator | still possible | still lost | unchanged |
| Canonical separator | impossible | representable | `M:W` is unrecognisable |

**Recommendation: defer to grammar v2, and decide it together with the other
two term-syntax changes.** There are now three pending changes that all alter
what a `term` may contain, and doing them in three separate grammar versions
would mean three migrations of every stored unit string:

1. **Rational exponents** (§12, original) — `m^1/2`
2. **Namespaced unit symbols** (AMB-037) — `hydro:acre_ft`
3. **A prefix separator** (this entry) — `k:W`

**Do the two uses of `:` actually collide?** Raised on review, and measured:
**no — one separator suffices.** §7.4 namespace prefixes are lowercase
identifiers, so only the *lowercase* SI prefix symbols can be confused with one.
Fifteen strings, all one or two characters:

```
a  c  d  da  f  h  k  m  n  p  q  r  u  y  z
```

The nine uppercase prefixes (`Q R Y Z E P T G M`) cannot collide with a
lowercase namespace at all. Reserving those fifteen as forbidden namespace names
makes `X:Y` deterministic: if `X` is a reserved prefix symbol it is a prefix,
otherwise it is a namespace. No second separator, no lookahead, no precedence
rule.

Checked against every namespace the two documents actually name — `core`, `cf`,
`hydro`, `cim`, `onto` — none is reserved. Note `cf` survives: `c` and `f` are
prefix symbols individually, but `cf` is not one.

One constraint follows: at most one `:` per symbol, so `hydro:k:acre_ft` is out.
That costs nothing, because a prefixed unit inside a namespace can still use the
separator-free form (`hydro:kacre_ft`), resolved by the ordinary prefix branch
within that namespace.

**But the prior question is whether either feature is still needed**, and the
honest answer has moved since both were raised:

- **Namespaced units (AMB-037).** The motivation was two independently-authored
  vocabularies both minting `bbl`. The AMB-036 ruling largely retired it: only
  the PR-gated registry defines units, and vocabularies define quantity kinds
  only, so cross-artifact unit collisions can now arise only between registry
  *versions*. This becomes necessary again if the hybrid option — letting
  vocabularies mint namespace-guarded units — is ever adopted.
- **A prefix separator (this entry).** It solves a problem that load-time
  collision checking plus `prefix_collisions` (AMB-054) already solves
  deterministically, and pays for it with a canonical form nobody recognises.

**Would two separators be *easier*, even though one parses?** A second review
question, and a sharper one: the analysis above establishes that one separator is
*unambiguous*, which is a lower bar than *usable*. Three costs of the
one-separator scheme that the parseability argument glosses over:

1. **`:` would carry two meanings.** The spec already uses `:` for CURIEs in
   quantity kinds — `core:power_generation`, `cf:air_temperature`. Adding
   `k:W` gives the same character a structurally different job in the same
   document: *namespace membership* versus *scale composition*. A reader seeing
   `:` would have to know which space they are in before they can read it. That
   is a real cognitive cost even where nothing is ambiguous.
2. **The reserved list is a hidden rule.** A vocabulary author who picks `da` or
   `c` as a namespace hits an error whose reason lives in the SI prefix table,
   not anywhere near their file. Fifteen invisible reserved words is a small
   trap, but it is a trap, and it is the price of the one-separator scheme.
3. **Error messages get vaguer.** With distinct separators, "that is not a known
   namespace" and "that is not a known prefix" are separable failures. With one,
   the parser must guess which the author meant before it can say what is wrong.

So on usability grounds the answer flips: **if both features ship, they should
use different separators** — `hydro:acre_ft` for namespaces (keeping `:`
consistent with the CURIEs already in the spec) and `.` for prefixes, `k.W`.
`.` is the best remaining candidate: visually light, no conflict with the
grammar (numeric literals are already forbidden, §6.1), and it reads as "part
of" in most notations. `-` collides visually with negative exponents and `_` is
already a symbol character.

**But the cleanest outcome is that the question does not arise.** The prefix
separator has almost no user-facing surface unless it appears in *canonical*
form — and there it costs more legibility than the overload costs clarity
(`M.W` is no more recognisable than `M:W`). Drop it, and `:` has exactly one
meaning everywhere, which is more consistent than either two-separator scheme.

**Recommendation, in order of preference:**

1. **No prefix separator.** `:` means namespace, everywhere, one meaning. The
   `ft` problem stays solved by load-time checking and `prefix_collisions`
   (AMB-054).
2. If a prefix separator is wanted anyway: **use `.`, not `:`** — the
   consistency cost of overloading is real, and the reserved-list trap
   disappears with it.
3. One separator with fifteen reserved namespace names is *correct* but the
   worst of the three to use, and should only be chosen if adding a character
   to the grammar is judged more expensive than the hidden rule.

**RULED 2026-07-30 — option 2: `.` for prefixes, `:` for namespaces.** `k.W`
reads more clearly than `k:W`, and the fifteen reserved namespace names go away
with the overload. Deferred to grammar v2 with the other term-syntax changes;
nothing in grammar v1 changes.

Note what this ruling does **not** retire. `.` would be an *optional explicit*
form — `MW` must keep working — so the separator-free spelling stays the common
one, and `ft` stays contested between *foot* and *femto-tonne* under it. The
load-time check and `prefix_collisions` remain load-bearing either way. What the
separator would add is a canonical spelling for a shadowed reading, if canonical
form ever adopts it (see the fork above, still open).

**Extension surfaced 2026-08-07 by AMB-069's `MBtu` case: an optional
separator does not close an ambiguous concatenation.**

`MBtu` means 10⁶ Btu under SI prefixes and 10³ in the US gas industry, where
`M` is the Roman thousand. Asked whether `M.Btu` would settle it — and it
would, *for the spelling that uses it*. But the ruling above makes `.`
**optional**, so making `Btu` prefixable still mints the bare `MBtu`, still at
10⁶, still colliding with the industry reading. **The separator adds an
unambiguous spelling; it does not subtract the ambiguous one.**

What would close it is a third state between prefixable and not — a per-unit
declaration that prefixes apply *only* through the explicit form:

```toml
[unit.Btu]
prefixes = "si-engineering"
prefix_separator = "required"    # M.Btu resolves; MBtu does not
```

That is narrower than making the separator mandatory everywhere (which would
break `MW` and every tagged column in existence) and stronger than leaving it
optional. It is the right shape for any unit whose concatenated prefix form is
already spoken for by another convention — which is a small set, but `Btu` is
in it and heat rate is not a niche quantity.

**RULED 2026-08-07 — the separator mode is accepted in principle for grammar
v2; the diagnostic half is implemented now.** A mandatory-separator unit is the
right tool and cannot ship before `.` is legal. But the reason a refusal exists
is independent of the syntax, and that half was the weaker one: `MBtu` used to
fail as `E_UNKNOWN_UNIT — did you mean MMBtu?`, which reads as a typo when it is
a deliberate refusal, and discards the thousand-fold reason entirely.

Registries may now declare refused spellings with a required reason:

```toml
[registry.ambiguous.MBtu]
reason = "under SI prefixes M is mega, so MBtu reads as 10^6 Btu; the US gas industry reads M as the Roman thousand and means 10^3 Btu."
use = ["MMBtu", "Btu"]
see = "https://unitarrow.org/generated/ambiguity/"
```

which produces, at the moment a user hits it:

```
"MBtu" is ambiguous and is refused rather than guessed: under SI prefixes M is
mega, so MBtu reads as 10^6 Btu; the US gas industry reads M as the Roman
thousand and means 10^3 Btu. Nothing in the string says which, and the two
differ by 1000x. Write one of `MMBtu`, `Btu` instead. See
https://unitarrow.org/generated/ambiguity/
```

An ordinary typo still gets suggestions rather than a lecture — the two paths
are distinguished by whether the registry declared the spelling.

Two load-time checks, both mutation-tested: a refusal without a reason is
rejected (same principle as `prefix_collisions`), and so is a refusal for a
spelling the registry also defines, since it could never fire while the author
believed it was protecting them.

**The documentation the error points at is generated from the same
declarations**, so the page and the message cannot disagree — neither is
written by hand.

Still deferred: `prefix_separator = "required"`, which needs grammar v2. Until
then `Btu` stays non-prefixable and `MBtu` is refused with the explanation
above.

*Cases:* none — grammar v2.

---

### AMB-057 — the reserved-symbol surface is invisible to registry authors
**§7.2 · gap · open (direction implemented)**

Raised on review alongside the separator ruling: the warnings about reserved
symbols need to be clear to users *and to registry developers*. They were not.

Under AMB-048, making a unit prefixable silently claims one spelling per prefix.
`t` with the engineering set claims twelve — `ft`, `kt`, `Mt`, `Gt`, `at`, `nt`
and so on — and nothing told the author that. They found out by declaring
`[unit.ft]` and watching the load fail, which is late and reads as an
implementation quirk rather than a consequence of their own earlier choice.

The set is also **not fixed**, which is the part that makes documentation alone
insufficient: it is derived from which units *this* registry made prefixable and
with which set, so it differs registry to registry and cannot be memorised or
published as a static list.

**Implemented direction — three queries, because three different people ask:**

| Question | Who asks | API |
|---|---|---|
| "Is `cd` safe to declare?" | author, mid-edit | `would_collide(symbol)` |
| "What did this registry decide?" | reviewer, at PR time | `collision_risks()` |
| "What is claimed at all?" | author, planning | `prefix_reserved()` |

`collision_risks()` is the one worth arguing for: it lists every contested
spelling and whether the author resolved it, so *every judgement call a registry
made*, and the reason for each, is visible in one place. A diff will not show it
— the decision lives in a `[registry.prefix_collisions]` entry far from the
`[unit.…]` entry it affects — and a reviewer would otherwise have to reconstruct
it.

An unresolved row cannot appear in a loaded registry, so the report always reads
as "here is what was decided" rather than "here is what is broken".

**Open questions.**

- ~~Should `prefix_collisions` entries carry a *reason*?~~ **Decided: yes, and
  required.** The report made the question answer itself — it was a column with
  nothing in it, and the missing "why" is what a reviewer actually needs.
  `prefix_collisions` is now a table of `symbol = "reason"`; an empty or missing
  reason fails the load, and `CollisionRisk::reason` fills the column. See
  AMB-054.
- Should a registry be able to *reserve* a spelling without declaring the unit —
  "nobody may define `ft` here" — so a future edit cannot quietly take it?
- Should the load warn on a prefixable unit whose base is one or two characters,
  since short bases are where the whole contested set comes from?

*Cases:* none yet; belongs in the `registry/` category (AMB-043). Shown live at
`docs/generated/ambiguity.md`.

---

### AMB-058 — no composition story for units across registries
**§7.4, §5.6, AMB-036, AMB-047 · gap · open**

Raised by the project editor, pushing on AMB-054: *"for a user loading two
third-party registries with conflicts, what choice does the user have?"*

Under the AMB-036 ruling (K2) they have none, because they cannot reach that
state: **exactly one registry defines units per deployment.** Vocabularies are
federated and many, but they define quantity kinds only, and namespaces make
their collisions structurally impossible — `hydro:streamflow` and
`cf:streamflow` coexist. Units have neither namespaces (AMB-037 L3 is deferred
to grammar v2) nor a merge operation.

That is not an answer to the question; it relocates it. The user who needs
`acre_ft` from a hydrology registry and `bbl` from an oil registry must produce
the union themselves — AMB-047 establishes that a replacement registry's
contents are unconstrained, so this is a supported thing to do — and then they
hit the load check with **no lever at all**, because both mechanisms this
project has built live upstream of them: `prefix_collisions` belongs to the
registry author, namespaces belong to the vocabulary author.

**Measured, not assumed.** Concatenating two registries that both define `bbl`:

```
E_REGISTRY_INVALID: not valid TOML: line 10: duplicate key "dimension"
```

A raw TOML parser error. It does not name `bbl`, does not mention units, does
not say two artifacts disagree, and offers no path forward — the opposite of the
prefix-collision message, which was written precisely to name the sacrificed
reading and state the cost. And the obvious reach fails informatively but
uselessly:

```
E_REGISTRY_INVALID: [registry.prefix_collisions] declares "bbl", but no
collision there exists; a stale entry hides the next real one
```

Correct — `prefix_collisions` resolves *authored vs prefixed*, and this is
*authored vs authored*. There is no analogue for the latter.

**Why not simply soften the load check.** Determinism is the product. A registry
that resolves `bbl` differently depending on load order, or that silently lets a
later artifact win, breaks canonical form as a cross-language equality primitive
— the one thing §6.3 exists to guarantee. "Refuses to load" is right; what is
missing is a lever for the person holding the failure.

**Options.**

1. **Hold the line** — one registry, PR the union upstream. Cheapest, and it
   works while the ecosystem is small. It fails exactly when the project
   succeeds: the core registry becomes a serialization point and every domain
   waits on a review by people who do not know the domain. AMB-036 already
   flagged this pressure and deferred the hybrid.
2. **Namespaced unit symbols** (AMB-037 L3, grammar v2) — `hydro:acre_ft`.
   Removes most conflicts by construction, exactly as it already does for
   quantity kinds. Costs a change to canonical form, which is why it is v2 and
   not now; AMB-056's `.` / `:` split was partly laying this groundwork. See
   AMB-037's 2026-07-31 re-examination: it fixes composition but not AMB-059,
   and most of its user-facing benefit is reachable under grammar v1 by renaming
   on import, since `_` is already a legal symbol character.
3. **A declarative composition step** — make the merge a first-class artifact
   rather than something a user does with `cat`:

   ```toml
   [registry]
   extends = ["core@1.2", "hydro@2.1"]

   [conflicts]
   bbl = { from = "hydro", reason = "this deployment is water, not oil" }
   ```

   This is `prefix_collisions` moved up one level, and the same principle
   answers it: **the conflict is resolved by whoever has the context, as early
   as possible, with a stated reason.** For an intra-registry collision that is
   the registry author; for a cross-registry one it is the person composing the
   deployment. The invariant survives — one *effective* registry, deterministic,
   hashable, still describable by §5.6's pin.

**Recommendation: 3 now, 2 later, never 1 alone.** Option 3 is additive, needs
no grammar change, and reuses machinery that exists (the required-reason shape
from AMB-054, the load-time enumeration from AMB-048). Option 2 removes the
cause rather than managing it, but cannot land before grammar v2. Option 1 is
the current state and is only tenable while there is effectively one registry
author.

**Regardless of which is chosen**, the duplicate-symbol diagnostic should stop
being a TOML parser error. A symbol defined twice is the single most likely
failure a registry composer will hit, and it currently produces the worst
message in the system.

*Cases:* none yet; belongs in the `registry/` category (AMB-043) alongside
AMB-047, whose trust-store question it shares — composing registries multiplies
the number of parties whose factors a deployment silently trusts.

---

### AMB-059 — combining columns across registries is silently wrong
**§8.1, §8.3, §5.6 · gap · RESOLVED 2026-08-01**

AMB-058 asks what a user does when two registries *fail to compose*. This is
what happens when they **succeed** — and it is much worse, because it is silent.

Canonical form is registry-**relative**: `bbl` canonicalizes to `"bbl"` in every
registry that defines it, whatever factor it carries. But §5.6 pins the registry
per **table**, so two tables can legitimately carry different pins. Measured, on
two registries that each load cleanly:

```
   oil: canonical="bbl"  dims=[("length", 1)]  factor=9938205933/62500000000
 water: canonical="bbl"  dims=[("length", 1)]  factor=29810117799/250000000000

 canonical strings equal? true   dimensions equal? true   values differ by 33.4%
```

Now trace it through the semantics:

- **§8.3 concat/join** — *"identical canonical units → result keeps the unit."*
  The strings are identical. The join succeeds; every row is wrong by a third.
- **§8.1 `a + b`** — *"if units differ, the right operand is coerced."* They do
  not differ, so nothing is coerced. Values are added directly.
- **§5.6 provenance merge** — merges `mode` (weakest wins) and says **nothing
  about merging `registry`**. The result table's pin is undefined: it names one
  registry while describing data resolved against two.

So the one thing §6.3 exists to guarantee — that equal canonical strings mean
the same quantity — does not hold across registry pins, and nothing in the
pipeline checks the pin before relying on it.

**This is AMB-036's defect, recurring one level down.** AMB-036 found that a
seal naming one vocabulary cannot describe an ontology assembled from several,
and fixed it by adding `vocabularies: [...]` to §5.6. The identical hole for
*registries* was closed by fiat instead — "exactly one is loaded" — which makes
the state unreachable rather than safe. AMB-047 then established that a
replacement registry's contents are unconstrained, and §5.2 lets any foreign
table arrive pre-tagged with its own pin. Units surviving IPC into a deployment
that did not tag them is the **premise of the project**, not an edge case.

**Recommended resolution — compose one effective registry, then check the
boundary.**

1. **Inside a deployment**, exactly one *effective* registry is in force —
   composed declaratively per AMB-058 option 3, so it stays deterministic,
   hashable, and describable by a single §5.6 pin. At operation time there is
   then only one registry and the question does not arise.
2. **At the boundary** — reading a foreign table whose pin differs from the
   effective registry's — compare the **symbols the schema actually uses**,
   resolving each in both and requiring identical dimension *and* factor.
   Disagreement is `E_UNIT_MISMATCH` in strict mode and taint plus one warning
   in permissive.

Why symbol-wise rather than pin equality: a pin comparison would reject every
version bump, which is the common case and must stay painless. The symbol check
is exact where it matters and silent where it does not. Measured, over the two
registries above:

```
    m: agree        km: agree (through the derived-prefix branch)
  bbl: DISAGREE — combining is unsafe
  acre_ft: present in only one
```

It costs one resolution per **distinct unit string in the schema** — a handful,
not per row, not per element. And it partially closes AMB-047 on the read path
as a side effect: a lookalike registry that redefines `MW` is caught the moment
a table tagged by the real one meets it.

**Namespacing is not the fix.** Asked directly (AMB-037, 2026-07-31): a
qualified `oil:bbl` vs `water:bbl` does turn this case from a silent error into
a correct conversion, which is a real gain. But it carries the publisher's
*label*, not the artifact's *identity*, so `oil@1.4` and `oil@2.0` — and a
lookalike `core` — remain identical strings with different factors. It lowers
the boundary check's firing rate; it does not replace it.

**What must not be done:** putting the registry hash into canonical form. That
would make the equality primitive unreadable, break every fixture, and couple
§6.3 to §5.6 permanently — while still not helping the version-bump case, since
the hash changes when nothing relevant did.

**RESOLVED 2026-08-01 — as recommended, and implemented.** §5.6 gains a
*Reading across a registry boundary* subsection; §8.3's first bullet now
requires the same registry pin before canonical-form equality may be relied on;
§10's `E_UNIT_MISMATCH` scope names the case. Spec 0.20.0-draft.

The ratified rule: when an incoming table's `registry.hash` differs from the
reader's effective registry, resolve **the distinct symbols the incoming schema
uses** in both registries and require **dimension, factor, and offset** all to
match. Symbols compared per pin comparison — never per row.

| Outcome | `strict` | `permissive` |
|---|---|---|
| pins equal | proceed, nothing checked | proceed |
| every symbol agrees | proceed | proceed |
| dimension differs | `E_DIM_MISMATCH` | taint, one warning |
| factor or offset differs | `E_UNIT_MISMATCH` | taint, one warning |
| unresolvable locally | `E_UNKNOWN_UNIT` | taint, one warning |
| source registry unobtainable | `E_REGISTRY_INVALID` | taint, one warning |

Three points settled during implementation that the recommendation had not
stated:

- **Offset is part of the test.** An affine scale whose zero moved has an
  unchanged factor, so comparing scale alone passes a `degC` whose offset
  differs — and the column is then wrong at every value.
- **The derived-prefix branch must be traversed.** `km` is authored in no
  registry (AMB-048 derives it at resolution), so a check consulting only
  authored entries lets every prefixed symbol cross unexamined.
- **Unobtainable is not agreement.** §5.6's `pin` embedding can leave a reader
  unable to fetch the source registry. *Could not check* and *checked and
  agreed* are separate verdicts, and `unverifiable()` is a distinct function
  from `check_boundary()` so they cannot be collapsed by accident.

Implemented in `unitarrow_core::boundary`, 10 unit tests. Diagnostics carry the
ratio — *values differ by 33.4%* — because "factors differ" is not actionable.

*Cases:* `composition.cross-registry.001`–`006`, all **normative**: the silent
33% case, the equal-pin short-circuit, a version bump touching nothing in use,
an affine offset disagreement, a derived-prefix disagreement, and the
unobtainable registry.

---

### AMB-060 — a multi-registry table has no describable provenance
**§5.6, companion §5.3 · gap · RULED 2026-07-31 · blocks M2.5**

The end-to-end case, raised by the project editor: a pipeline reads several
UnitArrow tables, loads two or more registries, computes across them, and
publishes. **How does a consumer of the result understand the units?**

Today it cannot. §5.6's provenance block and companion §5.3's seal both carry:

```json
"registry":     { "name": "core", "version": "2026.07", "hash": "sha256:..." },
"vocabularies": [ { "name": "cf",  "version": "2026.07", "hash": "sha256:..." } ]
```

Registry singular, vocabularies plural — and **§5.6 states the argument for the
plural form in the same sentence**: the list covers *"every vocabulary loaded
when the claim was made — a checked-at-a-stated-registry claim is not
reproducible without them."* That argument applies verbatim to registries. It
was not applied only because AMB-036 declared exactly one is ever loaded, which
AMB-058 shows is a constraint without a mechanism behind it.

**The editor's proposal** — the metadata should reference every registry
involved plus how they were reconciled — is correct in substance and needs one
ordering constraint to be sound.

**Why reconciliation must precede computation, not describe it.** If a pipeline
computes first and records the registries afterwards, the output can hold two
columns both tagged `bbl` meaning different things. A *list* of registries does
not say which column drew on which, so resolution would need per-column registry
attribution in §5.2 field metadata — per column, on every column, against
AMB-041's wide-table budget. Worse, it breaks the core invariant directly: two
columns in one table would share a canonical form and denote different
quantities, so §6.3 would no longer be an equality primitive **within a single
table**, which is the one place it must hold unconditionally. That is not a
trade-off; it contradicts what canonical form is for.

So the order is **compose → compute → publish**, and the block records the
composition rather than the mess.

**Recommended resolution.**

1. `registry` keeps its shape and names the **effective** registry — the single
   artifact the composition produced. No wire change for the single-registry
   case, which stays the common one, and consumer resolution stays exactly one
   lookup in exactly one artifact.
2. Add `composed_from: [{name, version, hash}, …]`, mirroring `vocabularies`.
   Present only when the effective registry was composed; absent means it was
   authored directly.
3. Add the **reconciliation record** — the conflicts resolved and, per AMB-054,
   the required reason for each. This is `prefix_collisions`' reason field
   propagated one level up, and it answers the consumer's actual question:
   *why does `bbl` mean the oil barrel in this table?*

**The property this buys.** A consumer holding `composed_from` and the
reconciliation can **recompute the effective registry and compare the hash**.
The reconciliation stops being a claim and becomes checkable — and because the
effective hash is derivable from inputs plus recipe, sealing that one hash
transitively covers the recipe, so no one can swap the reconciliation without
breaking the seal.

**What a consumer then does**, in three tiers of effort:

| Question | What they need |
|---|---|
| What is this column's unit worth? | one lookup in the effective registry |
| Why does `bbl` mean *that* here? | the reconciliation entry's reason |
| Do I trust it? | fetch `composed_from`, replay, compare hash, check each seal |

**RULED 2026-07-31 — both embeddings, chosen by deployment shape.** The project
editor's framing: for a publisher shipping a 100 MB file, a few KB buys something
that lasts decades; for an enterprise emitting millions of small files totalling
TB, a hash-only form still gives verification a raw URL never could. So the modes
are not competitors —

| Mode | Per table | Resolves offline | Survives the sources disappearing |
|---|---|---|---|
| `Full` — embed the effective registry | 1143 B | yes | **yes** |
| `Pin` — embed the hash only | 375 B | no | no |

**They trade availability, not integrity.** Both carry the pin, so neither can
silently resolve against the wrong registry; only `Full` still resolves when the
constituents are gone. That is the sentence the spec should carry, because it is
what makes `Pin` a legitimate choice rather than a corner cut.

**Measured, on the compression question** (200 files, 1143 B block each, 8 KB
incompressible payload) — and the intuitive container is the one where the hope
fails:

| Container | Per-file cost of the embedded block |
|---|---|
| solid stream + zstd -19 | **14 B** |
| solid stream + gzip -9, 8 KB payloads | **37 B** |
| solid stream + gzip -9, 64 KB payloads | **702 B** |
| per-entry compression (zip, Parquet block) | **512 B** |

Three mechanisms: zstd's window is megabytes, so copies deduplicate almost
perfectly; gzip's is 32 KB, so the two gzip rows differ in nothing but payload
size and that alone moves the cost 19×; and per-entry compression — a zip member,
a Parquet metadata block — never deduplicates at all, because sibling files are
invisible to the codec by construction. *"Zip up the bundle"* is precisely the
case where embedded registries do not compress away.

**Implemented** in `unitarrow_core::compose` (`Embedding::{Full, Pin}`,
`verify_pin`) and shown live at `docs/generated/composition.md`. Still
unspecified: §5.6 must gain `composed_from`, the reconciliation record, and the
embedding mode itself.

*Cases:* none yet; needs `wire` fixtures (AMB-024, AMB-029) plus a
`companion/` case for the seal shape. Should land with the feature per the
roadmap's golden-files rule.

---

### AMB-061 — a composition can fail in a way no constituent could
**§7.5, §7.1 · gap · open (direction implemented)**

Found while writing the `composition` fixtures, by asking what breaks that no
single registry can break.

Two registries, each valid alone: `imperial` defines `ft` (foot); `metric` makes
`t` (tonne) prefixable. Neither has a collision. **Composed, femto+t produces
`ft` for the first time** and the effective registry will not load.

The diagnosis is right and the advice was not. The underlying message says to
declare `prefix_collisions`, which lives in a source header the composer does
not author — they hold two third-party files. `contested()` is no help either,
and correctly so: no symbol is defined twice, so this is not a definition
contest and only the final validation catches it.

There is a resolution, and it was undiscoverable: **a source that defines no
units is legal**, so the composer contributes their own, declaring only
`[registry.prefix_collisions]`. Verified — `ft` resolves as foot, `Gt` still
resolves, `femtotonne` becomes unavailable, and the ruling carries into the
effective registry where a consumer can read it. The load message now names this
move when it detects an emergent collision.

**What remains open.**

- Should the deployment-contributes-a-source pattern be *named* in §7.5 rather
  than left as a consequence of two rules? It is the only way a composer can
  make a registry-level ruling, which is more load-bearing than "legal".
- Should `contested()` report emergent prefix collisions too? It answers "what
  must I decide?", and this is a thing the composer must decide — but it is
  found by enumerating prefix × base across the merged set, not by comparing
  definitions, so it is a different computation behind the same question.
- Should a composition be able to carry `prefix_collisions` directly, rather
  than through a synthetic source? Cleaner, at the cost of a second ruling
  channel with its own stale-entry rules.

*Cases:* `composition.emergent.001` (the failure), `composition.emergent.002`
(the resolution, with `Gt` surviving and `femtotonne` gone), both normative.

---

### AMB-062 — engines that do not model column metadata are contagious, not graceful
**§1 goal 2, §1 goal 3, §5.2 · gap · open · affects M2**

Asked directly: *do these metadata tags survive ingestion into a DuckDB
database?* — and then, *do pandas and polars suffer the same?* Measured on
**DuckDB 1.4.4 / pandas 2.3.3 / polars 1.36.1 / pyarrow 21.0.0**, tagging a
column with
`unitarrow.quantity.v1` field metadata and a `unitarrow:provenance` schema
block:

| Path | Field metadata | Schema metadata |
|---|---|---|
| Arrow → `register()` → `SELECT *` → Arrow | **lost** | **lost** |
| Arrow → `CREATE TABLE AS SELECT` → Arrow | **lost** | **lost** |
| Parquet → `read_parquet()` → Arrow | **lost** | **lost** |
| DuckDB → `COPY TO … (FORMAT PARQUET)` | **not written** | **not written** |
| pyarrow → Parquet → pyarrow (no DuckDB) | kept | kept |

Nothing survives a pass through the engine. This is not a DuckDB defect —
Arrow field metadata is not part of any SQL type system, and most engines will
behave the same way — but it is a gap between what §1 promises and what a user
will experience, because DuckDB is one of the most common hops in the analytics
ecosystem.

**Why goal 3 does not cover it.** *"A consumer that does not implement this spec
sees ordinary storage-typed columns with human-readable metadata. Nothing
breaks."* That describes a **leaf**. DuckDB is a **hop**: it does not merely
ignore the metadata, it strips it for everyone downstream. Graceful degradation
is local; this is contagious, and silent — the table that comes out is untagged
with nothing to indicate it ever was tagged. Under §5.3 an untagged column is
"absence of a claim", so a strict consumer downstream cannot distinguish *never
tagged* from *tagged, then laundered*.

**What does survive, and where.** The bytes are still in the Parquet file. The
schema block lands as a first-class key; the *field* metadata lands inside the
base64 `ARROW:schema` blob Arrow writes for its own round-tripping. DuckDB can
read both — as opaque values — via `parquet_kv_metadata()`:

```
  pyarrow-written file kv keys: ['ARROW:schema', 'unitarrow:provenance']
  duckdb-written file kv keys : NONE
```

**Recovery works, and its limit is the interesting part.** Decoding
`ARROW:schema` and reattaching by field name restores the tags exactly —
verified. But:

```
  SELECT power * 2 AS doubled  ->  columns: ['doubled', 'site']
  'doubled' known to source schema: False
```

Name-keyed recovery fails the moment a projection renames or derives, and fails
*silently*. Which is correct rather than unfortunate: a derived column's unit is
a §8.1 propagation result, not something to recover. Recovery is sound only for
columns that passed through untouched, and an implementation MUST NOT reattach a
source unit to a column whose provenance it cannot establish — that would
manufacture a claim nobody made, the exact hazard §5.3 exists to prevent.

**A UnitArrow-aware DuckDB workflow is constructible today.** `COPY … (FORMAT
PARQUET, KV_METADATA {...})` writes Parquet key-value metadata, verified
working. So schema-level provenance can be carried across a DuckDB hop
deliberately. Per-column units have no such channel — `COMMENT ON COLUMN` is
readable in-session but does not survive `COPY TO` — so they need the
`ARROW:schema` reattachment path.

**The loss is at ingestion, not at write.** Worth stating precisely, because it
determines where any fix must intervene: by the time a user runs `COPY TO`, the
units are already gone. Nothing was checked during their SQL either. The write
does not discard the metadata; it has nothing to discard.

**No type-level channel exists.** Measured, since a carrier on the *type* rather
than the *field* would have been the clean answer:

| Attempt | Result |
|---|---|
| `pa.ExtensionType` (`unitarrow.quantity.v1`) through a scan | flattened to `double` |
| the same with `SET arrow_lossless_conversion = true` | flattened to `double` |
| `CREATE TYPE quantity_MW AS DOUBLE` | erased immediately — `duckdb_columns()` reports `DOUBLE` |

`arrow_lossless_conversion` governs DuckDB's own types, not user extension
types. A nominal alias does not survive its own catalog, let alone arithmetic.
So a type-level carrier needs a **native extension** with per-instance type
info, the mechanism `spatial`'s `GEOMETRY` uses — not a `CREATE TYPE`.

**The same measurement across the ecosystem** (pandas 2.3.3, polars 1.36.1),
because the question is whether DuckDB is special. It is not — but the engines
fail differently, and the differences matter:

| Hop | Field md | Schema md | `ARROW:schema` in the output file |
|---|---|---|---|
| pyarrow → Parquet → pyarrow | kept | kept | original, with the units |
| DuckDB | lost | lost | **absent** |
| pandas | lost | lost — replaced by its own `pandas` key | present, **rewritten without the units** |
| polars | lost | lost | present, **rewritten without the units** |

**Trap 1: a surviving `ARROW:schema` key is not surviving metadata.** pandas and
polars both write that key — describing *their* schema, which carries none of the
tags:

```
  original      ARROW:schema -> power field md: {b'unitarrow.quantity.v1': …}
  after pandas  ARROW:schema -> power field md: NONE
  after polars  ARROW:schema -> power field md: NONE
```

So a recovery tool that reads `ARROW:schema` **from the file it is repairing**
finds the key, decodes it, recovers nothing, and has no signal that anything is
wrong. Mitigation B is only sound because it reads the key from the **original**
input; that is a correctness requirement, not an implementation detail, and the
obvious simplification silently defeats it.

**Trap 2: `ARROW:schema` shadows the file-level KV for Arrow readers.**

| File | file-level KV | `read_table().schema.metadata` |
|---|---|---|
| polars sidecar (writes `ARROW:schema`) | `ARROW:schema`, `unitarrow:columns` | **empty** |
| DuckDB sidecar (writes none) | `unitarrow:columns` | `unitarrow:columns` |

When a writer emits `ARROW:schema`, pyarrow reconstructs the schema from it and
reports *its* metadata — so a sidecar key present in the file is invisible to the
ordinary reader path. Any consumer of the degraded-transit form MUST read
Parquet key-value metadata directly (`ParquetFile.metadata.metadata`), never
`read_table().schema.metadata`. Two engines writing the same sidecar are read
back differently otherwise.

**Polars does support the opt-in.** `write_parquet(metadata={...})` writes the
sidecar key correctly — verified — so mitigation A generalises beyond DuckDB.

**Two mitigations work today, measured.**

*A — a single sidecar key.* DuckDB has no per-field channel but does have
`KV_METADATA` on `COPY`. Per-column units collapse into one JSON blob under one
key, which DuckDB writes and reads:

```
  keys DuckDB wrote: ['unitarrow:columns']
  recovered: {'power': {'unit': 'MW', …}, 'temp': {'unit': 'degC', …}}
  hop 2 keys: ['unitarrow:columns']   -- survives arbitrarily many hops
```

Survives any number of hops — but only if **every** `COPY` opts in. One
forgetful query launders the table.

*B — retag from the source schema.* Decode `ARROW:schema` from the input file
and reattach, refusing anything the input cannot prove:

```
  passthrough : restored=['power','temp'] refused=[]
  derived     : restored=['temp']
                refused=['doubled (not in source)', 'power (type double->int32)']
```

`power` is refused despite an exact name match, because the type changed —
reattaching there would have manufactured a claim. Needs no cooperation from the
author of the query, which is what makes it the right default; it simply cannot
cover derived columns, and must say so loudly rather than return a quietly
partial result.

**Recommended resolution.**

1. Qualify §1 goal 3, or add a fourth category beside leaf and implementer: an
   engine that **transits** data without modelling column metadata. Name the
   consequence — everything downstream is untagged — rather than leaving it to
   be discovered.
2. Specify the **recovery** path normatively: decode `ARROW:schema`, reattach
   only to columns whose name and storage type are unchanged, and never to
   derived or renamed columns.
3. M2's graceful-degradation deliverable should include a DuckDB round trip, not
   only the Python→IPC→browser demo. The browser path preserves metadata, so it
   exercises the easy direction; DuckDB is where the promise is actually tested.
4. Ship mitigation B as `unitctl retag <out> --from <in>` in M3, reporting
   restored and refused columns per column rather than a summary — a partial
   restore that reads as a full one is worse than no restore.
5. Standardise mitigation A's key (`unitarrow:columns`) in §5.2 as the
   **degraded-transit form**: the same information, addressed by column name in
   one blob, for carriers with no per-field channel. It is strictly weaker (it
   cannot survive a rename) and should say so. Specify that readers resolve it
   from Parquet key-value metadata directly, because `ARROW:schema` shadows it
   on the ordinary reader path (trap 2 above).
6. A native DuckDB extension is the only path to units surviving *inside* the
   engine — and therefore the only path to dimensional checking on DuckDB
   arithmetic. Large: it means §8 propagation implemented against DuckDB's
   expression tree, calling `unitarrow-core`. Worth scoping only after M2 proves
   the wire format; mitigations A and B cover transit without it.
7. Consider whether the companion's seal should treat a metadata-stripping hop
   as a detectable event. It currently cannot be one: a seal does not survive
   transformation (§5.6), so a laundered table is indistinguishable from an
   untagged one — which is arguably the right answer, but it is unstated.

*Cases:* none — a `wire` fixture here would test DuckDB rather than UnitArrow.
The *recovery* algorithm is in scope and should get cases when it is specified.

---

### AMB-063 — the spec cannot read the most widely deployed unit syntax in science
**§6.1, §7.4 · gap · open · strategic**

Raised by the question *"what are we even adding if UDUNITS and CF already
exist?"* — which turns out to have a concrete, measurable component.

CF conventions have carried UDUNITS-parseable `units` attributes on netCDF
variables since 1998, across essentially all of climate, ocean, and atmospheric
science. §7.4 already plans to generate the `cf:` namespace from the CF standard
name table, so interoperation is intended. But the **unit strings themselves**
were never tested against the implementation. Measured, against real CF units:

| CF/UDUNITS string | UnitArrow verdict |
|---|---|
| `W m-2` | `E_UNIT_SYNTAX` |
| `kg m-2 s-1` | `E_UNIT_SYNTAX` |
| `m s-1` | `E_UNIT_SYNTAX` |
| `W m-2 K-1` | `E_UNIT_SYNTAX` |
| `J kg-1 K-1` | `E_UNIT_SYNTAX` |
| `s-1` | `E_UNIT_SYNTAX` — `expected \`*\` or \`/\`, found '-'` |
| `m2` | `E_UNKNOWN_UNIT` |
| `degree_Celsius` | `E_UNKNOWN_UNIT` |
| `K`, `1`, `degC` | ok |

**Nine of twelve fail.** Two independent causes, both deliberate decisions:

- **Space as multiplication.** CF writes `W m-2`; AMB-006 made whitespace
  between two bare symbols an error, because `m s` versus `ms` is a real trap.
  CF has thirty years of practice saying a space is a product, and the trap has
  evidently not been fatal there.
- **Bare-sign exponents.** CF writes `m-2`; §6.1 requires `m^-2`.

So the project intends to consume the CF vocabulary while being unable to parse
the corpus it annotates. For an energy-data deployment this is not a corner
case — wind resource, solar irradiance, and reanalysis data are all CF netCDF.

**Recommended resolution: a separate reader, not a grammar change.**
`parse_cf(s) -> CanonicalUnit` as an input adapter, consistent with how verbose
names and `/` sugar already work — flexible input, one canonical output. It
touches neither grammar v1, nor §6.3, nor any fixture, and it converts "we are
reimplementing UDUNITS" into "we can ingest the existing corpus."

Explicitly **not** recommended: relaxing §6.1 to admit space-multiplication.
That would reopen AMB-006 and put the `m s` / `ms` trap into the primary
grammar to serve a dialect a dedicated reader handles safely.

**Seeding from UDUNITS: measured 2026-07-31, and it is mechanical.** The user
identifies UDUNITS as the standard that matters most to them, and its licence
permits what UCUM's forbids (BSD-style, derivative works allowed with
attribution — see AMB-065). The whole database is **112 KB** across six XML
files:

| | Count |
|---|---|
| unit entries | **277** |
| base units (dimension-defining) | 7 |
| integer factor — exact | 119 |
| decimal literal — **exact as a rational** | 111 |
| purely symbolic (`J = N.m`) — exact by construction | 39 |
| affine, via `@` (`°C = K @ 273.15`) | 2 |
| **exceeding i128 — needing curation for representation** | **0** |

**The float concern was wrong.** A decimal literal like `1e-3 kg` or
`1.60217733e-19 J` is *exactly* a rational; nothing is lost converting it. Every
numeric literal in the database fits i128 as an exact rational, so the mapping
to §7.2 factors is mechanical.

**The real judgement is authority, not precision.** Concretely, UDUNITS carries

```
eV = 1.60217733e-19 J
```

which is the pre-2019 CODATA value. The 2019 SI redefinition fixed the
elementary charge at exactly `1.602176634e-19`, so UDUNITS' figure is superseded
by about 0.4 ppm. A blind import inherits that silently — and `π` stored as a
truncated 30-digit decimal is definitionally wrong however many digits it has.

**Recommended: submodule as a codegen input, never as a vendored output.**
Pin `Unidata/UDUNITS-2` as a submodule; generate the registry from it; review and
commit the *generated* TOML. That matches the two patterns the spec already has
— §4's code-generated ontology constants and §7's tzdata model — and keeps the
registry the reviewable artifact §7.2 assumes.

Every generated unit MUST carry §7.2 `provenance` naming the upstream file,
the submodule commit, and the original `def` string, so a stale constant is
traceable rather than silently inherited. That also makes AMB-047 partly
self-solving: a registry regenerated from a pinned upstream can be diffed
against its source, which is exactly what "nothing constrains a replacement
registry's contents" currently lacks.

**Upstream churn: the database is effectively frozen.** Measured from the
repository's commit history for `lib/*.xml` — 52 commits in nineteen years, of
which **43 fall in 2007–2014**. The last twelve years total nine commits, with
2019 and 2022–2025 seeing none at all. A pinned submodule would need
regeneration on the order of once every year or two, not continuously, which is
what makes the tzdata model appropriate here.

**Getting to ASCII is largely already done, upstream.** AMB-005 forbids
non-ASCII symbols, which excludes 18 UDUNITS spellings. But the file ships ASCII
names for almost all of them:

| non-ASCII | ASCII name already in UDUNITS |
|---|---|
| `Ω` (two code points) | `ohm` |
| `°C`, `℃` | `degree_Celsius`, `degC`, `deg_C` |
| `°F`, `℉` | `degree_fahrenheit`, `degF`, `deg_F` |
| `°K`, `°R` | `degK`, `degR` |
| `°` | `arc_degree`, `degree`, `arcdeg` |
| `'`, `′` / `"`, `″` | `arc_minute` / `arc_second` |
| `Å` (two code points) | `angstrom` |
| `%` | `percent` |
| `π` | `pi` |
| `BµV` | **none** |

**12 of 13 entries carry an ASCII spelling in the source**, so the
transliteration is a lookup rather than an invention — and note `degC` is
already a UDUNITS alias, meaning this project independently chose the spelling
the standard already uses. Only `BµV` has no ASCII name, and it is excluded on
other grounds (below).

**A third category the import cannot represent: logarithmic units.** Classifying
all 277 definitions:

| | Count |
|---|---|
| linear (scale only) — importable as-is | **267** |
| affine (`@` offset) — §6.4 already handles | 2 |
| **logarithmic (`lg(re …)`) — no §7.2 representation** | **8** (excluded, AMB-067) |

`dB`-family units (`lg(re 1 mW)`, `lg(re 20e-6 Pa)`, …) are neither a scale nor
a scale-plus-offset, so §7.2's `factor`/`offset` model cannot express them at
all. They must be excluded explicitly, the same way AMB-066 recommends for
angles, and for the same reason: silent omission invites someone to approximate
one later.

**So the answer to "all 277" is: 267 mechanically, 2 affine, and 8 excluded by
model** — plus the angle units AMB-066 sets aside, which overlap the `°`/`'`/`"`
rows above.

**IMPLEMENTED 2026-08-07.** `vendor/udunits-2` is pinned as a submodule and
`tools/import_udunits.py` generates `registries/udunits.toml` — **265 authored
units resolving 961 symbols, zero contested spellings.** The generated file is
committed and reviewed; the submodule is a codegen input, never a vendored
output.

Evaluating UDUNITS' definitions needed a small expression evaluator, because it
defines units by expression (`W = J/s`) where UnitArrow states a dimension and a
factor. The grammar is regular — `.` and space multiply, `/` divides, `^` raises,
parentheses group — plus three things the file does not spell out: prefixes are
applied at parse time (`bar = 1000 hPa` needs `hPa` decomposed), names are
pluralized on the fly (`3 international_feet` against an entry named
`international_foot`), and a trailing integer is an exponent (`cm2`). **257 of
259 definitions evaluate**; the two that do not are the arc-second symbol and a
non-ASCII alias of `degC`, both excluded anyway.

**Four categories the load check forced into the open**, none anticipated:

| | |
|---|---|
| `degree_west = -1 degree_east` | a negative scale is a **sign convention, not a unit** — it flips a sign rather than changing the size of anything. Excluded; the sign belongs to the datum |
| `kg` authored *and* derived | the SI base unit of mass already carries a prefix, so `g` is authored at 1/1000 and `kg` derives at exactly 1 |
| `eV` with `prefixes = "si"` | its factor already carries a denominator of 10²⁷, so pico- overflows i128 and the registry will not load |
| a stale loop variable in the generator | gave every unit the *last* entry's aliases, caught as `alias "molec" is claimed by both "A" and "Bd"` |

The `eV` case is the interesting one, and it argues AMB-053's open question.
Prefixing is all-or-nothing per unit, so avoiding `peV` — which overflows —
costs `keV` and `MeV`, which do not. A per-unit prefix **range**
(`prefixes = { set = "si", from = -9, to = 30 }`) would keep both; §7.2 offers
only named sets. Recorded rather than worked around: the generator detects the
overflow, declines to declare prefixes for that unit, and lists it.

Both departures from upstream are asserted by a test, so a regeneration cannot
silently undo them: `rad` must carry dimension `angle`, `rad/s` must not be
commensurable with `Hz`, and 180 arc-degrees must be π radians exactly.

Still open: whether the seeded set should stay the full import or be curated
down, and whether `udunits` or a curated subset becomes the `core` registry
§7.4 refers to — that is a governance question, not a technical one.

*Cases:* none yet. A `parsing` sub-theme keyed to the CF dialect should land
with the reader, per the golden-files-with-the-feature rule.

---

### AMB-064 — the handoff to in-memory unit libraries is unspecified, and nearly free
**§5.2, §6.3 · gap · open · strategic**

Follows AMB-063. If UDUNITS and CF already exist, the question is what an
extension to them looks like — and what a clean handoff to the libraries people
already use would buy. Measured against **pint 0.24.4 / pint-pandas 0.7.1**.

**pint parses UnitArrow canonical form verbatim — 11 of 11.**

| Canonical | pint reads it as |
|---|---|
| `MW*h` | `hour * megawatt` |
| `MW*h^-1` | `megawatt / hour` |
| `W*K^-1*m^-2` | `watt / kelvin / meter ** 2` |
| `degC`, `delta_degC` | `degree_Celsius`, `delta_degree_Celsius` |
| `m^2`, `1`, `Btu`, `kW*h` | all parse |

No adapter, no translation table. §6.3's slash-free form with `^` exponents
happens to sit inside pint's accepted grammar.

**The complementarity is the finding.** pint-pandas propagates units through
pandas arithmetic — `power * 2` stays `pint[megawatt]` — which is exactly the
in-engine propagation this project cannot get from DuckDB or polars. But:

```
  df.to_parquet(...) -> ArrowTypeError: Did not pass numpy.dtype object
```

**pint-pandas cannot write a Parquet file at all.** So pint has propagation and
no wire format; UnitArrow has a wire format and no propagation. They are not
competitors, and the gap each leaves is the other's contribution.

**pint-pandas is column-level, not per-value** — checked, because a per-element
object layout would disqualify it as a partner at any real scale, and because
§1 goal 1 makes exactly this commitment:

| | Storage | Memory (1M float64) | `MW -> kW` rescale |
|---|---|---|---|
| plain `float64` Series | ndarray | 8.0 B/value | 0.15 ms |
| **pint-pandas `PintArray`** | unit on the **dtype**, magnitudes in an ndarray | 9.0 B/value (1.1×) | 35.8 ms |
| bare pint `Quantity(ndarray)` | one unit, one ndarray | 8.0 B/value | 0.16 ms |
| object column of `Quantity` | one Python object per value | ≥56 B/value | ~34 **seconds** (extrapolated) |

The unit lives on `PintType`, one per column; `s[0]` boxes a `Quantity` lazily
on scalar access, which is `__getitem__` behaviour rather than storage. So pint
and UnitArrow made the same architectural choice, and the pairing survives
scale. The last row is what §1 goal 1 exists to forbid — a ~200,000× penalty
against the array path.

Worth recording honestly: pint-pandas' own wrapper costs ~36 ms per million
values against bare pint's 0.16 ms. Still O(n) array work rather than
per-element dispatch, but a real constant — an adapter doing bulk conversion
should prefer bare pint over the pandas layer.

**A full round trip works today**, via pint-pandas' own `dequantify()`:
`pint dtypes → flat Arrow + §5.2 field metadata → Parquet → read → retag →
pint dtypes`. Verified: dtypes restored, arithmetic still unit-aware, values
identical.

**Two requirements any such adapter MUST meet**, both found by building it:

1. **Canonicalize; never pass the foreign spelling through.** `dequantify()`
   emits pint's long names — `megawatt`, `degree_Celsius`. Writing those into a
   `unitarrow.quantity.v1` tag produces a tag UnitArrow cannot resolve
   (`degree_Celsius` is not a registry symbol or alias). The adapter must route
   through `canonicalize()`, making the tag registry-relative as §5.2 requires.
2. **Validate, because symbols collide across systems.**

   ```
   UnitArrow 'pu' -> pint interprets as 'picounified_atomic_mass_unit'
   ```

   §5.4's per-unit quantity read by pint as pico × unified atomic mass unit:
   silent, and dimensionally wrong (mass, not dimensionless). This is AMB-054's
   prefix-collision hazard occurring *between two registries in different
   systems*, where neither can see the other. A handoff that passes strings
   without checking dimension agreement will corrupt quietly.

**What extending UDUNITS should and should not mean.** Three layers, with what
each costs:

| Layer | Adopt from UDUNITS | Cost |
|---|---|---|
| Input dialect | its grammar, as a reader | **none** — see AMB-063 |
| Registry contents | its database as a seed | **corrected 2026-07-31** — the values are *decimal literals*, which are exactly rational, so conversion loses no precision (see AMB-063). The judgement is authority, not precision: whether a written value is the definition or a stale measurement |
| Canonical form | its strings as the equality primitive | **everything novel.** UDUNITS defines no stable canonical string, so there is no cross-language hash, so there is no content-addressed registry pin, so §5.6 and the entire companion collapse. Plus floats, plus a C/expat dependency with no `no_std` or WASM path |

Extend at the edges; keep the core. The two edge layers are pure gain and the
core layer is the only genuinely novel thing this project has.

**Recommended resolution.** Specify the handoff in §5.2 as a named
**interop contract** rather than leaving each binding to invent it: a foreign
unit system's string is *input*, canonicalized on the way in and re-emitted from
canonical form on the way out, with a dimension-agreement check at the boundary
and an error rather than a silent pass when it fails. `unitarrow-py` should ship
the pint adapter, since pint is where Python users already are.

*Cases:* none yet — needs a `wire` sub-theme once the contract is specified. The
`pu` collision is a good normative case the moment there is somewhere to put it.

---

### AMB-065 — canonical form checked against UCUM; UCUM unusable as a source
**§6.3 · gap · RESOLVED 2026-07-31 · strategic**

Raised by the GeoArrow analogy: GeoArrow did not reinvent projection names, it
carries EPSG codes and PROJJSON and defers to the authority. The question is
whether §6.3 is deferring where it should.

This conversation has repeatedly asserted that **content-addressed registry
identity plus a stable canonical form** is the piece with no prior art in
UDUNITS, CF, or pint. That is defensible for the *identity* half. It has **not
been checked for the canonical-form half**, and there is one obvious challenger:

**UCUM** — the Unified Code for Units of Measure — is a machine-oriented unit
code system with a formal grammar, mandated in HL7/FHIR and therefore carried by
a large share of the world's clinical data. To the best of the author's
knowledge it defines a case-sensitive code set and a notion of canonical or
normalized form, which is precisely what §6.3 claims as its contribution.

**RESOLVED 2026-07-31 — verified against the specification. §6.3 stands, and
UCUM cannot be used as a source.** Two findings, both consequential.

**1. UCUM defines no canonical form.** It deliberately went the other way:

> "The expression syntax of *The Unified Code for Units of Measure* generates an
> infinite number of codes with the consequence that it is impossible to compile
> a table of all valid units."

> "Programs that declare *full conformance* with *The Unified Code for Units of
> Measure* must compare unit expressions **by their semantics**, i.e. they must
> detect equivalence for different expressions with the same meaning."

Equivalence classes, not a normal form. So a UCUM consumer needs a parser and an
equivalence engine on both ends to answer *are these two columns the same unit?*
— which is precisely the cost §6.3 exists to remove. The novelty claim survives
in a sharper form than it was made: the contribution is not "a canonical form"
in the abstract, it is **comparison without a parser**, and the largest existing
unit-code standard explicitly declines to offer it.

Recorded as the thing §6.3 was chosen over, which the register previously
lacked. §6.3's freeze is confirmed rather than reopened.

**2. UCUM's license forbids what this project would need.** Verified against
`LICENSE.md` in the ucum-org repository. Licensees may not:

> "add, delete, or modify the Work's content including field names, field
> contents, descriptions, and comments"

> "use the Work to create Derivative Works"

> "use the Work for the purpose of developing or promulgating **a different
> standard for identifying units of measure**"

The third clause names this project's category directly. Transforming
`ucum-essence.xml` into a UnitArrow registry is a derivative work and is
prohibited; the broader clause may reach further. **This is a question for
counsel, not for engineering judgement** — recorded here as a constraint that
exists, not as a legal conclusion. It is not an OSI-approved licence.

**Consequence for AMB-063's "seed the registry from an existing database".**
UDUNITS-2 is BSD-style with a patent clause, permits modification and derivative
works with attribution, and is GPL-compatible. **UDUNITS is the licence-viable
source; UCUM is not.** Any vendoring or submodule work should target UDUNITS,
and a UCUM dependency should not be added without legal review.

Still open, and cheap: **QUDT** (semantic web, per-unit IRIs, Apache-2.0 to the
best of current knowledge) as a second cross-check, and whether any Arrow or
Parquet convention already carries UCUM codes.

**Concretely.** The two systems answer *are these the same unit?*
differently:

| | UnitArrow | UCUM |
|---|---|---|
| method | compare canonical strings | compare semantics |
| needs a parser at compare time | no | **yes** |
| `SELECT … WHERE unit = 'MW*h'` | works | meaningless |

That last row is the contribution: comparison without a parser. UCUM's own text
requires the other approach, so §6.3 is not re-treading it.

*Cases:* none — this changes what the fixtures mean, not what they assert.
Resolve before 1.0-rc, because a canonical-form change after that is the most
expensive change this format can make.

---

### AMB-066 — angle units are unrepresentable under the exact-rational rule
**§6.2, §7.2 · contradiction · RESOLVED 2026-08-01**

Found while scoping which UDUNITS units to seed. The project editor proposed
dropping `π` on the grounds that it is not a unit of anything. That is true, and
it does not help, because `π` is **load-bearing**:

```
arc_degree = (pi/180) rad
```

Two frozen decisions collide here.

- §7.2: factors are **exact rationals**. `π/180` is irrational, so a degree has
  no exact factor relative to a radian. Not awkward — *unrepresentable*.
- §6.2: the nine base dimensions contain no angle, and SI makes the radian
  dimensionless (`rad = m/m`). So `rad`, `°`, and `1` are all mutually
  commensurable, and converting between them demands the factor that cannot be
  written.

The consequence is worse than exclusion: `deg` and `rad` would be *silently
interconvertible with each other and with dimensionless* at whatever rational
approximation was stored, and §6.3 would treat a wrong-by-π/180 conversion as
dimensionally sound.

**Options.**

1. **Exclude angle units.** Honest, and cheap. Cost: wind direction, solar
   azimuth and zenith, and latitude/longitude are angle columns, and this is an
   energy-data project. Excluding them means those columns cannot be tagged at
   all — the untagged case §5.3 is careful to keep loud.
2. **Allow a declared rational approximation** with stated precision, so the
   registry says *this factor is approximate to N digits*. Requires a new field
   and admits the first inexact factor into a format whose auditability claim
   rests on exactness. Contaminates the hashing story only mildly (the rational
   is still exact bytes), but it breaks the promise that a factor is the
   definition rather than a rounding.
3. **Make angle a base dimension.** Deviates from SI, which is why §6.2 did not.
   But it makes `deg` and `rad` incommensurable with `1`, which is arguably the
   *correct* engineering behaviour — adding a radian to a dimensionless ratio is
   almost always a bug, and SI's choice to call the radian dimensionless is a
   known irritant in exactly this situation. Cost: §6.2's nine base dimensions
   were frozen at M0, and this is the change that decision was meant to prevent.

**Recommendation: 1 for now, 3 recorded as the grammar-v2 candidate.** Excluding
angles keeps every frozen promise and defers a base-dimension change to the only
release that can absorb one. But it should be excluded *explicitly*, with §7
stating why, rather than falling out of the import as an oversight — otherwise
the first person who needs solar azimuth will add `deg = 0.017453292519943295
rad` to a registry and reintroduce an inexact factor with no record.

Note this is independent of `π` as a spelling. Whether `π` is written `pi`,
`math:pi`, or omitted changes nothing: the irrationality is in the conversion,
not the notation.

**RESOLVED 2026-08-01 — option 3, plus an exact π exponent.** The recorded
recommendation was option 1 (exclude angles, defer 3 to grammar v2). It was
**reversed** once the motivating case was named: frequency stability analysis in
power systems.

**What changed the ruling.** Measured against a registry modelling SI faithfully
(radian dimensionless):

```
   rad*s^-1 dimension : [("time", -1)]
   Hz       dimension : [("time", -1)]
   COMMENSURABLE      : true
   conversion offered : y = x
   377 rad/s -> 376.99111843 Hz   (truth: 60 Hz)
```

`ω = 2πf`, so this is a **6.28× error that passes every dimensional check** — and
in power systems the Hz/rad-s⁻¹ confusion is routine, not exotic. Excluding
angles would not have prevented it; it would have left the conversion available
and untagged.

**Precedent settled the deviation-from-SI objection.** `BASE_DIMENSIONS` already
contained two non-SI base quantities:

```
   count        ** not an SI base quantity **
   currency     ** not an SI base quantity **
```

`count` is the exact analogue: dimensionless in SI too, separated so a tally
cannot be added to a ratio. Angle earns the same treatment on the same grounds,
so this is consistent with M0's decision rather than a departure from it.

**The freeze was cheaper to change than feared.** Six places hardcoded the arity,
and all 45 dimension-asserting fixtures use *sparse* maps — a tenth base
dimension with exponent 0 changes none of them. Dimension vectors are never on
the wire (§5.2 carries unit strings), so nothing serialized moved.

**Exactness solved with a π exponent, not an approximation.** Option 2 was
rejected because an approximate factor contradicts §7.2's whole auditability
claim. `Scale` = `Rational` × π^k: `deg = (1/180)·π¹ rad` composes, inverts and
exponentiates losslessly, and π reaches a float only at application — the same
"floats come last" rule everything else follows. Verified end to end:

```
   deg -> rad factor : 1/180·π
   180 deg           : 3.14159265358979312
   pi                : 3.14159265358979312
   EXACT             : true
   round trip 45 deg : 45
```

and after the ruling the hazard is refused:

```
   commensurable : false
   refused: E_DIM_MISMATCH — rad*s^-1 is {time: -1, angle: 1} but Hz is {time: -1}
```

§6.2 gains `angle` with the rationale; §7.2 gains the `pi` key, bounded to
`-8..=8`, with `offset` explicitly excluded. Spec 0.21.0-draft.

**Consequence for AMB-063.** The angle units UDUNITS carries — `arc_degree`,
`arc_minute`, `arc_second`, and the `°`/`'`/`"` spellings — are no longer
excluded from the import. They become ordinary entries with `pi = 1` and exact
rational parts, since UDUNITS defines them all against `(pi/180) rad`.

*Cases:* `parsing.angle.001`–`006`, all normative — the radian's dimension, the
degree's, angular frequency versus frequency (the pair that carries the whole
argument), composition, and angle-over-angle cancelling to dimensionless.

---

### AMB-067 — naming a logarithmic unit makes it silently convertible
**§7.2, §8.1, §8.3 · gap · DEFERRED 2026-08-01 · has a demonstrated hazard**

Asked directly: can the bel family not simply be canonicalized as `Bel` and
imported like anything else? Naming fixes the *spelling* problem (AMB-005) and
does nothing about the *model* problem — and the combination is worse than the
exclusion.

Registering `dBm` as an ordinary unit of power, as a naive import would:

```
It loads, and canonicalizes cleanly:
   dBm      -> dBm
   W/dBm    -> W*dBm^-1

And then it converts:
       0 dBm : unitarrow says      0.000 mW, truth      1.000 mW   ** WRONG **
      10 dBm : unitarrow says     10.000 mW, truth     10.000 mW   ok
      20 dBm : unitarrow says     20.000 mW, truth    100.000 mW   ** WRONG **
      30 dBm : unitarrow says     30.000 mW, truth   1000.000 mW   ** WRONG **

   `dBm` is commensurable with W: true
```

Note the 10 dBm row is **right by coincidence** — a spot check passes while the
column is wrong by 33× at the top of its range. §7.2 models a unit as
`base = x·factor + offset`; a bel is `10·log₁₀(x/ref)`, and no choice of those
two numbers expresses it. Because the unit declares a dimension, §8.3 will also
coerce a `dBm` column into watts without a warning.

**So naming alone is not available.** Three ways forward:

1. **Exclude**, as AMB-063 currently records. Safe; costs the tag. A `dBZ`
   column then cannot be labelled at all, and radar reflectivity is ordinary
   data next to wind and solar work.
2. **Tag without converting** — register the bel family as **non-composable and
   non-commensurable**: the column carries an honest label, `dBm*h` is rejected,
   and no conversion to watts is offered. This keeps the larger half of the
   value (*what is this column?*) while refusing the half that is wrong.
3. **Model it** — a third unit kind beside linear and affine, carrying
   `scale = "log10"` and a reference, making conversion correct rather than
   refused.

**Recommended: 2 now, 3 as a later widening.** Option 2 needs no new arithmetic
and is a strict improvement on an untagged column.

**It also fixes a smell the register already carries.** Non-composability is
currently a hardcoded string match:

```rust
pub fn is_non_composable(&self) -> bool {
    self.is_affine() || self.symbol == "pu"
}
```

`pu` is special-cased by name — which AMB-022 flags as undefined behaviour in
the spec. Option 2 wants exactly the same property, so the fix is to make
non-composability a **declared registry property** rather than a symbol
comparison, and let both `pu` and the bel family declare it. One small change,
two open entries.

Note the shared shape: `pu` and `dBm` are both *quantities relative to a stated
reference*. They differ in that `pu` is a linear ratio and `dBm` is the
logarithm of one — which is precisely where §7.2 stops.

**DEFERRED 2026-08-01 — modelling postponed for both `pu` and the bel family.**
Interim position, stated so it is not rediscovered: `pu` keeps its hardcoded
non-composability, and the eight bel units are **excluded from the UDUNITS
import** (AMB-063). Option 2 above is not implemented either — this is a
deferral of the whole question, not a partial adoption.

**The one guard that must not be skipped, and it is not where this entry first
put it.** The registry loader *cannot* detect this: given
`[unit.dBm] dimension = "power" factor = [1, 1000]`, nothing distinguishes a
mistaken logarithmic unit from an ordinary linear one — the TOML is
well-formed and the intent is gone. So the check belongs in the **importer**,
which does see `lg(re 1 mW)` in the source XML and must refuse to emit a linear
approximation of it rather than skipping the entry quietly.

That distinction matters for regeneration: an importer that silently drops what
it cannot model will, on some later upstream change, drop something nobody
noticed was missing. It should fail loudly with the list.

*Cases:* an importer case, not a registry case — asserting that a `lg(re …)`
definition produces a refusal naming the unit, never a linear factor. Belongs
with the import tooling whenever that lands.

---

### AMB-068 — one `count` dimension lets per-entity ratios cancel silently
**§6.2, §7.3 · gap · open**

Found while reviewing an external energy-forecasting pipeline's own unit layer
against §6.2. That pipeline makes `person` and `vehicle` **separate base
dimensions**, and its single headline guardrail is what that buys:

```
population × distance_per_vehicle           # vehicles_per_capita omitted
  = person · mi·vehicle^-1
  = person·mi/vehicle                       → not a length; rejected on insert
```

Under §6.2's vector both terms are `{count: 1}` and the check disappears:

```
count · length · count^-1 = {length: 1}     → commensurable with m, factor 1
```

The omitted term is a per-entity ratio, so the error is *exactly* that ratio and
nothing bounds it. In that pipeline's own numbers: light-duty vehicles per capita
≈ 0.85, so dropping it overstates vehicle-distance by 1.18×; medium/heavy-duty
vehicles per capita ≈ 0.03, so the same slip on the MHDV chain overstates by
**33×**. Both results land in `{length: 1}` and pass every check §6.3 can make.

This is AMB-066's argument with the nouns changed. §6.2's own text:

> With a dimensionless radian, `rad/s` and `Hz` both reduce to `{time: -1}`. They
> are then commensurable, and an implementation will offer a conversion between
> angular frequency and frequency at a factor of 1 — when the relationship is
> `ω = 2πf`. The error is 6.28×, and no dimensional check can see it.

Substituting: with one `count`, `person*m*vehicle^-1` and `m` both reduce to
`{length: 1}`; an implementation offers a conversion at factor 1; the true
relationship is the vehicles-per-capita ratio; the error is 1.18× or 33×, and no
dimensional check can see it.

**§7.3 does not rescue it.** Quantity-kind fields are `parent`, `interval`,
`summable_with`, `opposes`, `balance`, `rate_of`, `same_as`, `broader`,
`display`, `provenance` — guards about summation, intervals, and sign
conventions. None constrains a *multiplication chain*, and §7.3 states plainly
that "Resolution is lookup; there is no inference engine." A `core:population`
kind and a `core:vehicle_stock` kind would still multiply into a plain length.

**There is precedent on both sides.** AMB-022 proposed a dedicated `pu`
dimension so a per-unit column could never unify with a plain `"1"` column, and
the core registry records the rejection in a comment: "§6.2's base-dimension list
is one of the two frozen M0 decisions." AMB-066 then broke exactly that freeze
for `angle`, once a motivating case was named. Flavored counts are the same
request a third time, and the freeze has already proven negotiable when the
silent-wrong-number case is concrete.

**Options.**

1. **Do nothing; state the limit.** §6.2 says counts of different entities are
   not distinguished, so per-entity ratios cancel and an omitted ratio is
   invisible. Cheap and honest. Cost: every stock-and-flow model — vehicles,
   households, dwellings, generators, customers — is exactly the shape that loses
   its guardrail, and this is an energy-data project.
2. **Registry-declared count flavors.** A registry may name count dimensions
   (`count.person`, `count.vehicle`) extending the base vector; each is
   incommensurable with the others and with plain `count`, and cancels normally
   against itself. Keeps the vector as the sole commensurability authority, which
   §6.2 insists on. Cost: commensurability becomes registry-dependent, which is
   precisely what §6.2 forbids for `angle` — "a registry that defines the radian
   as dimensionless produces different commensurability answers, which is a
   different type system." The flavor set would have to be core-defined and
   MUST-level, and the vector stops being fixed-width.
3. **Flavor at the symbol layer.** `count@person` — the v1 grammar already admits
   `@` inside symbols (`symbol = ALPHA *( ALPHA / DIGIT / "_" / "@" )`) — with
   commensurability decided on symbol identity for count terms only. Cost: a
   special case in §6.3's comparison, colliding with the namespaced-symbol work
   deferred to grammar v2 (AMB-037, §12).

**Recommendation: 1 now, 2 recorded as the grammar-v2 candidate alongside
AMB-066's precedent.** Option 2 is the right answer in the wrong release: it
widens the dimension vector, which is the change M0's freeze exists to prevent,
and unlike angle it has no bounded flavor list — `person`, `vehicle`,
`household`, `dwelling`, `generator`, `customer` is an open set, and an open set
in the base vector is a different format. But the limit must be stated
*explicitly* in §6.2 rather than left to be discovered, because the failure is a
plausible number: an 18% or 33% overstatement of a state energy forecast,
arriving with every dimensional check green.

This is independent of whether `count` units are ever registered. The core
registry defines none today, so the first person to add `vehicle = 1 count`
introduces the cancellation with no record that it was considered.

*Cases:* none yet. Needs a `dimensions` case asserting that `count*m*count^-1`
canonicalizes to `m` and compares equal to `m` — pinning the behaviour as
deliberate rather than accidental — plus its incommensurability counterpart if
option 2 ever lands.

---

### AMB-069 — nothing records which measurement convention a unit follows
**§7.2 · gap · RULED 2026-08-07**

Raised on discovering that `MBtu` means 10⁶ Btu under SI prefixes and 10³ in
the US gas industry — the Roman thousand — a 1000× gap with nothing in the
symbol to say which. The proposal was a **top-level registry declaration**: SI,
US, or imperial.

**RULED 2026-08-07 — per unit, not per registry, and derived rather than
declared at the top.** Three reasons, the last decisive.

*Real registries are mixed.* `power-systems.toml`, written for one domain,
holds SI (`W`, `V`, `J`), IEC (`var`), US customary (`Btu`, `mi`), non-SI
metric (`t`, `h`, `deg`), ISO 4217 (`USD`) and industry spellings (`MVAR`).
Derived from the units themselves:

```
  si                         17 units
  non-si-accepted             8 units
  industry                    4 units
  us-customary                4 units
  iec                         2 units
```

No single top-level label is true for that file, and it is not an unusual file.

*An advisory flag would not fix the case that prompted it.* The ambiguity is in
the symbol; the remedy has to be at the symbol. Here `Btu` is simply not
prefixable, so `MBtu` does not resolve at all — refusing an ambiguous symbol
being the correct answer to one.

*A **behavioural** flag would be actively dangerous.* If `M` meant 10⁶ in one
registry and 10³ in another, the same canonical string would denote different
quantities in different registries — §6.3 would stop being an equality
primitive, which is AMB-059's hazard promoted from the unit table into the
prefix table, where no boundary check could see it. **Prefix semantics must be
universal.**

**Implemented.** §7.2's `provenance` key was already specified and shown in the
spec's own example — and the loader was **accepting it and throwing it away**,
so the documented field silently lost its data. It is now parsed into
`Unit::provenance` (`system`, `source`, `scope`, `note`), inherited by derived
prefixed forms (`kW` is as SI as `W`), and summarised by `Registry::systems()`.

The summary is **derived, never declared**, for the same reason the register's
entry count is computed: a hand-maintained summary drifts from its contents.
`Registry::units_without_a_stated_system()` reports the gaps rather than
defaulting them, because a unit whose reading depends on a convention should
say which one.

**Related.** AMB-056's `.` separator would give `M.Btu` an unambiguous
spelling, but as ruled it is *optional*, so the bare `MBtu` survives alongside
it. Closing the case needs a per-unit *separator required* mode — recorded as
an extension on AMB-056, and dependent on grammar v2.

**Still open.** Whether `system` should be a controlled vocabulary rather than
free text. Free text was chosen so a registry can say `"iec"` or `"iso-4217"`
without a spec change, but it means `"us"` and `"us-customary"` do not group.

*Cases:* none yet; belongs with the `registry/` category alongside AMB-043.

---

## Summary

| Severity | Count | IDs |
|---|---|---|
| contradiction | 15 | 001, 004, 008, 010, 016, 019, 020, 023, 027, 028, 036, 037, 038, 044, 066 |
| gap | 49 | 002, 003, 005, 006, 007, 009, 011, 013, 014, 015, 017, 018, 021, 022, 025, 026, 029, 030, 031, 032, 033, 035, 039, 040, 041, 042, 043, 045, 046, 047, 048, 049, 050, 051, 052, 053, 054, 057, 058, 059, 060, 061, 062, 063, 064, 065, 067, 068 |
| implicit | 3 | 012, 024, 034 |
| design question | 2 | 055, 056 |

69 entries total; **21 decided**, seven directions ruled (044, 048, 049, 051, 052, 053,
054), two open design questions (055, 056), the rest open. Two recommendations were **reversed** in
[DECISIONS.md](DECISIONS.md) after deeper analysis — AMB-016 (offset
convention) and AMB-019 (the `interval` reverse check). The text below is the
original recommendation in both cases; the memo argues the other way.

**Blocking M0's exit criterion** (canonical form cannot be implemented
deterministically without them) — 13: AMB-001, AMB-002, AMB-003, AMB-004,
AMB-005, AMB-006, AMB-007, AMB-009, AMB-010, AMB-013, AMB-026, AMB-036,
AMB-037. Deciding these clears **28 of the 39** provisional fixture cases with
no expected value changing.

**Blocking M1** (`unitarrow-core` factors, affine conversion, reader
behavior) — 7: AMB-008, AMB-015, AMB-016, AMB-017, AMB-018, AMB-020, AMB-021.

**Blocking M2 / `wire`** — 10: AMB-014, AMB-024, AMB-029, AMB-031, AMB-032,
AMB-033, AMB-038, AMB-040, AMB-041, AMB-062 (a DuckDB hop strips every tag —
measured — which qualifies §1 goal 3 and belongs in M2's degradation demo).

**Blocking M1 / M3 ergonomics and safety** — 3: AMB-041 (bulk tagging is what
M3's dogfood gate turns on), AMB-043 (nothing validates a third-party catalog),
AMB-058 (no way to compose two registries, and the worst diagnostic in the
system when someone tries).

**Blocking M2.5 / M6 / `companion/seals`** — 6: AMB-030, AMB-034, AMB-035,
AMB-042, AMB-029 again, and AMB-060 (the provenance block cannot describe a
multi-registry table at all, and M2.5 freezes the envelope).

**Blocking M4 / `type-functions`** — 4: AMB-019, AMB-025, AMB-031, AMB-022.

**Editorial only, no fixture depends on them** — 5: AMB-011, AMB-012, AMB-023,
AMB-027, AMB-028.

**Resolved by external verification** — AMB-065: UCUM defines no canonical form
(it mandates semantic comparison instead), so §6.3's freeze is confirmed. UCUM's
licence separately forbids derivative works and forbids use for developing a
different unit-identification standard, so it cannot be vendored — UDUNITS-2
(BSD-style) is the licence-viable source for AMB-063's registry seeding.

## History

| Spec | Companion | Entries | Notes |
|---|---|---|---|
| 0.14.0-draft | 0.1.0-draft | AMB-001 … AMB-027 | Initial review, written alongside the §6.3 fixtures |
| 0.17.0-draft | 0.3.0-draft | + AMB-028 … AMB-035 | §5.5 descriptions/references, companion column digests + republish flow + size lint |
| (same) | (same) | + AMB-036 … AMB-038 | Surfaced while writing [DECISIONS.md](DECISIONS.md): registry vs. vocabulary identity, unnamespaced unit symbols, and the conversion/description contradiction |
| (same) | (same) | + AMB-039, AMB-040 | Escape sequences in metadata values. **AMB-039 decided: prohibited in units** |
| (same) | (same) | + AMB-041 … AMB-043 | Wide-table tagging ergonomics and wire cost, seal-side scaling, and load-time guardrails for third-party catalogs. AMB-005 and AMB-029's recommendations revised |
| (same) | (same) | + AMB-068 | Reviewing an external forecasting pipeline's own unit layer: it makes `person` and `vehicle` separate base dimensions and catches a dropped `vehicles_per_capita` term; under §6.2's single `count` the same expression reduces to `{length: 1}` and the check vanishes. Error is whatever the dropped per-entity ratio is — 1.18× for light-duty, 33× for medium/heavy-duty. AMB-066's argument with the nouns changed; third time the frozen base-dimension list has been the obstacle (after AMB-022's `pu`) |
| **0.21.0-draft** | (same) | AMB-066 resolved | Recommendation **reversed**: angle becomes the tenth base dimension rather than being excluded, because `rad/s` and `Hz` were commensurable under SI's dimensionless radian — a silent 6.28x error in exactly the power-systems work that motivated it. Exactness solved by an exact π exponent on factors, not an approximation |
| **0.20.0-draft** | (same) | AMB-059 resolved | The boundary check ruled, implemented, specified and fixtured: differing registry pins now require per-symbol agreement on dimension, factor **and** offset, traversing the derived-prefix branch, with *unobtainable* kept distinct from *agreed*. Six normative cross-registry cases replace two provisional ones |
| (same) | (same) | + AMB-067 | Naming the bel family does not make it importable: registered as an ordinary unit, `dBm` converts to mW silently and is wrong by 33x at 30 dBm — while the 10 dBm spot check passes by coincidence. Recommends tag-without-converting, which also generalises `pu`'s hardcoded non-composability (AMB-022) into a declared property |
| (same) | (same) | + AMB-066 | Scoping the UDUNITS import surfaced a collision between two frozen M0 decisions: §7.2's exact rationals and §6.2's nine base dimensions leave angle units unrepresentable, since `arc_degree = (pi/180) rad` is irrational and SI makes the radian dimensionless. Dropping `pi` does not help — the irrationality is in the conversion, not the notation |
| (same) | (same) | AMB-065 resolved | Verified against the UCUM specification and licence: UCUM defines **no** canonical form — it mandates semantic comparison — so §6.3's contribution is *comparison without a parser* and its M0 freeze is confirmed. UCUM's licence forbids derivative works and forbids use for "developing or promulgating a different standard for identifying units of measure"; UDUNITS-2 (BSD-style) is the licence-viable seeding source instead |
| (same) | (same) | + AMB-065 | The GeoArrow analogy — carry EPSG, do not reinvent it — exposes that §6.3's novelty was never checked against UCUM, which is mandated in HL7/FHIR and may already define a canonical form. An instruction to verify, written without network access. Resolve before 1.0-rc |
| (same) | (same) | + AMB-064 | Measured: pint parses UnitArrow canonical form 11/11 with no adapter, and pint-pandas propagates units through pandas arithmetic — but cannot write Parquet at all. The two systems are complementary, not competing. Round trip demonstrated; two adapter requirements found by building it, including `pu` reading as pico-atomic-mass-unit in pint |
| (same) | (same) | + AMB-063 | Measured: nine of twelve real CF/UDUNITS unit strings fail to parse (`W m-2`, `kg m-2 s-1`, `s-1`…). The spec plans to generate the `cf:` namespace from CF while being unable to read the strings CF datasets carry. Recommends a separate CF reader, not a grammar change |
| (same) | (same) | + AMB-062 | Measured: DuckDB 1.4.4 drops all Arrow field and schema metadata on every ingestion path and writes none back. Not a leaf ignoring tags (goal 3) but a hop stripping them for everyone downstream — silently, and indistinguishable from never-tagged under §5.3 |
| **0.19.0-draft** | **0.5.0-draft** | + AMB-061; `composition` fixtures | §7.5 and §5.6 written; 24 normative + 2 provisional golden cases added under a new `composition` category, including a pin assertion that makes byte-identical output testable across languages. AMB-061 found by writing them: a composition can produce a prefix collision neither constituent had |
| (same) | (same) | + AMB-060 | A composed-registry table has no describable provenance: §5.6 and companion §5.3 pin `registry` singular while listing `vocabularies` plural, and the sentence justifying the plural form applies verbatim to registries. Recommends an *effective* registry pin plus `composed_from` and a reconciliation record, so the composition is recomputable and the seal covers it |
| (same) | (same) | + AMB-059 | Combining columns across registry pins is silently wrong: canonical form is registry-relative, §5.6 pins per table, and §8.3 compares canonical strings — two `bbl` from different registries concat cleanly and differ by 33%. AMB-036's defect recurring for registries, closed by fiat rather than solved |
| (same) | (same) | + AMB-058; `prefix_collisions` reasons required | Composing two third-party registries has no story: one registry defines units (AMB-036), units are not namespaced until grammar v2 (AMB-037), and `prefix_collisions` does not cover authored-vs-authored — the user holding the failure has no lever, and gets a raw TOML duplicate-key error |
| (same) | (same) | + AMB-057; AMB-056 ruled | Prefix separator ruled `.` (option 2); reserved-symbol surface made queryable — `would_collide`, `collision_risks`, `prefix_reserved` |
| (same) | (same) | + AMB-055, AMB-056 | Long-name canonical form measured and recommended against (it moves ambiguity rather than removing it); prefix separator deferred to grammar v2 with the other two term-syntax changes |
| (same) | (same) | + AMB-054 | Symbol collisions resolved by the registry author via `[registry.prefix_collisions]`, never by the user at tag time; each declaration carries a required reason, since the decision is invisible in a diff of the unit it affects; the shadowed reading is removed rather than relocated, because its canonical form would mean something else |
| (same) | (same) | AMB-048 revised | Prefixed units derived at resolution rather than stored: 20 authored units resolve 500 symbols while storing 20. Load-time collision enumeration retained |
| (same) | (same) | + AMB-052, AMB-053 | English spelling variants accepted on input (curated, because `ton`/`tonne` are different units); the engineering prefix set corrected from an asymmetric tera…femto to a symmetric exa…atto |
| (same) | (same) | AMB-006 revised | Whitespace ruling split by position: accepted at edges and around operators, rejected between bare symbols where collapsing would silently change the unit |
| (same) | (same) | + AMB-050, AMB-051 | WASM playground built (159 KB / 53 KB gz); the Arrow round-trip demo still needs arrow-js. Both crates taken `no_std + alloc` |
| (same) | (same) | + AMB-049 | Verbose unit names as input, reverse lookup for checking a tag, and suggestions on unresolved symbols. Implemented; normativity is the open question |
| (same) | (same) | + AMB-048 | SI prefix authoring: §6.1 mandates distinct entries but says nothing about generating them. Loader expansion implemented, with collision and overflow guards |
| (same) | (same) | + AMB-047 | Found by the docs generator: a replacement registry's *contents* are unconstrained — `MW = 1 W` loads cleanly |
| (same) | (same) | + AMB-044 … AMB-046; rulings | Editor review session: **AMB-001 ruled A4** (slash-free canonical form — reverses the memo; 002/004 mooted, fixtures re-goldened), **AMB-016 ruled O2**, cast-policy direction (044), warning taxonomy (045), temporal storage types (046). Registry-vs-vocabulary definitions added to AMB-036 |
| **0.19.0-draft** | **0.5.0-draft** | AMB-058/059/060 material | §7.5 *Composing registries* added (rulings with required reasons, deterministic serialization); §5.6 gains `composed_from` and the **full**/**pin** embedding modes; §3 defines *effective registry* and *ruling*; §10's `E_REGISTRY_INVALID` scope names §7.5; companion §5.2/§5.3 pin the constituents. The differing-pin merge rule (AMB-059) is explicitly marked unspecified rather than left silent |
| **0.18.0-draft** | **0.4.0-draft** | Editorial pass | All 13 M0 rulings + AMB-016/039/044-direction applied to the spec; `vocabularies` pin added to the seal; §7.2 example TOML fixed. AMB-014 resolved by implication. 16 entries decided |

**Nothing was resolved between those two revisions.** §§6.1–6.4, §7.2, §10, and
§11 of the spec are unchanged, as are the companion's §8 and §9 — so every
canonical-form, affine, error-code, and conformance-category finding still
stands exactly as written. The 39 provisional fixture cases are unaffected and
remain provisional.

The single highest-value fix is **AMB-016**: it is wrong in a way that produces
a plausible-looking number (416.3 °F for boiling water), it is invisible in the
spec's own example, and it lands in the registry data where it will be copied
rather than derived.
