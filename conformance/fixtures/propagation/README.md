# `propagation` — expression trees, taint, and mode behavior

**Spec:** §8.1 (propagation), §8.2 (taint), §8.3 (concat and join),
§8.4 (table modes), §8.5 (temporal semantics)
**Spec §11 category:** 3 · **Milestone:** M4
**Status:** shaped, empty.

Spec §11: *"expression trees → result unit / error code, including taint and
mode behavior."*

## What these fixtures must pin

- **Arithmetic** (§8.1). Multiply and divide compose units symbolically and
  canonicalize the result. Add, subtract, and compare require equal dimensions
  (`E_DIM_MISMATCH`) and coerce the **right** operand to the left's unit — the
  result carries the *left* unit, which is asymmetric and easy to get wrong.
- **Taint** (§8.2). `unknown` is contagious. Warnings fire **once per taint
  introduction per lineage** and once at export — never per element, never per
  kernel call, and only inside opted-in contexts. A fixture asserting warning
  *counts* is the only way to catch an implementation that warns per row.
- **Concat and join** (§8.3). Identical units keep the unit; commensurable but
  different is `E_UNIT_MISMATCH` in strict and first-input coercion with one
  warning in permissive; incommensurable is `E_DIM_MISMATCH` in all modes;
  tagged + untagged yields untagged plus one warning. **Join keys never
  coerce** — joining on commensurable-but-different key units is an error in
  every mode, because silent key coercion invites wrong joins.
- **Modes** (§8.4). Per-column taint survives inside any table, so a strict
  table cannot launder a tainted column.
- **Temporal** (§8.5). The headline case: `sum` over an `instant` rate column is
  `E_TEMPORAL_MISMATCH` in all modes, and the only path from MW rows to MWh is
  `integrate` — value × period, valid only on an `interval` column with a known
  period, producing `time: +1`, `{kind: "interval", statistic: "sum", period}`,
  and the kind named by `rate_of`. Columns without a `temporal` block
  participate unchecked; checkers MUST NOT invent one.
- **Kind algebra** (§7.3). `summable_with`, and the `opposes`/`balance` pair:
  subtraction across opposing kinds yields the declared balance kind, while an
  unsigned `sum` mixing them stays `E_KIND_UNSUMMABLE`.

## Blocked on

- **AMB-025** — `period` admits `P1M` and `P1Y`, which have no fixed length, so
  `integrate` is undefined for them and period equality (`P1M` vs `P30D`) is
  undecidable.
- **AMB-020 / AMB-021** — §8.4 says working tables have no mode, yet mode
  selects behavior throughout §8.3 and §8.5. Every fixture here needs an
  explicit `mode` until the default is defined.
- **AMB-012** — whether `MW*h` and `MWh` concat cleanly follows from canonical
  form, but the consequence is unstated.
