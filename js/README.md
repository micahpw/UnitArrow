# UnitArrow

**Physical units and quantity semantics for Apache Arrow columns.**
Column-level, cross-language, and wire-native: a unit annotates a whole
column as Arrow extension metadata, survives IPC / Flight / Parquet, and
converts as a metadata-level operation — never a per-element object.

> **Status: pre-release.** This repository reserves the `unitarrow`
> name while the specification is finalized and the first implementation
> lands. The spec draft, roadmap, and reference implementation will be
> published here. UnitArrow is a community extension-type specification
> in the spirit of GeoArrow; it is not affiliated with or endorsed by
> the Apache Arrow project or the ASF.

## Why

Every existing approach to units dies at a boundary. In-memory unit
libraries (pint, astropy, unyt) get the algebra right and lose
everything at serialization — their units are properties of runtime
objects. File-format conventions (netCDF/CF, HDF5 attributes) survive
storage but carry *dead* metadata: no checking, no propagation, no
algebra. Compile-time unit systems are safest and can't cross a wire at
all.

UnitArrow puts units where all of those layers already meet: the Arrow
interchange layer. A tagged column keeps its unit across processes,
languages, and the network; a consumer that has never heard of
UnitArrow sees a plain float column with human-readable metadata beside
it. Adoption never needs to be coordinated: producers can tag before
their consumers understand units, consumers can upgrade before their
producers do, and each side gains value on its own schedule.

### Where existing tools stop

| Approach | What it gets right | Where it stops |
|---|---|---|
| pint / astropy.units / unyt / Unitful.jl | Correct unit algebra, array-level units, mature registries | Units are runtime-object properties: they do not survive serialization. As of pint-pandas 0.8 / pandas 3.0 / pyarrow 25, `pa.Table.from_pandas` on a pint column doesn't even degrade — it raises `ArrowTypeError`. Adoption is all-or-nothing: everything becomes a Quantity, and pandas operations that don't silently fall back to object dtype fail outright. |
| netCDF/CF, HDF5 attributes, FITS `BUNIT` | Units survive storage; CF's standard-name vocabulary is decades of accumulated wisdom | The metadata is *dead*: no algebra, no checking, no propagation through compute — and each convention is welded to one container format. |
| F# units of measure, mp-units, uom | Compile-time safety, zero runtime cost | Locked inside one language's type checker; cannot cross a wire, an FFI boundary, or into dynamic languages at all. |
| SQL engines / dataframe libraries | Where the computation actually happens | No unit concept whatsoever; column metadata is stripped by the first query. |

None of this is a criticism of the algebra in these tools — pint's is
excellent, and CF's vocabulary is a direct input to UnitArrow's. Their
shared limitation is *placement*: each lives in one layer and dies at
that layer's edge. UnitArrow's job is the layer none of them can reach
— units as a property of the *data interchange* itself — and adapters
to and from pint are a natural part of the roadmap, so in-process pint
users gain wire survival rather than losing anything.

## Adopt gradually — the whole design bends around this

UnitArrow is *gradually typed*, the way TypeScript is: untagged columns
are always legal, checking is a dial rather than a wall, and each step
is independently useful. You can stop at any step and keep everything
it gave you.

### Step 1 — Tag, ship, read (zero-cost drop-in)

Convert a pandas or polars dataframe to Arrow, attach units, and ship
it. Read a UnitArrow table back into pandas with units intact.
Conversion is one scalar rescale driven by column metadata. No code
changes anywhere else; untouched pipelines pass tagged tables through
bit-identical.

```python
# API sketch — subject to change before 1.0
import unitarrow as ua

table = ua.tag(df, {"power": "MW", "gas": "GBtu"})   # pandas / polars / pyarrow in
table = ua.convert(table, {"gas": "MWh"})            # metadata-level rescale
df2   = ua.to_pandas(table)                          # units survive the round trip
```

The same table read in a browser via arrow-js exposes its units from
field metadata — client-side unit toggling with no server round trip.

### Step 2 — Checked compute (IR + WebAssembly)

Opt columns into checking and computation becomes dimensionally sound
*before* it runs: multiplying composes units, adding requires
commensurability, and summing an instantaneous MW column into "energy"
is an error — the only path from MW rows to MWh is an explicit,
period-aware `integrate`. A small typed expression IR lowers to your
engine (arrow-rs, polars, DuckDB) and to WebAssembly, so the same
checked operations — convert, integrate, differentiate, aggregate —
run in the browser over arrow-js buffers.

### Step 3 — End-to-end validated data products

The final step of the journey: units verified from UnitArrow sources
all the way to UnitArrow published outputs. Tables finalize with
`publish()` — checked in strict mode, pinned to a registry version, and
sealed so consumers can verify who published a table and that it hasn't
changed. Full derivation chains (including external tools like LP
solvers) become attestable and reproducible. This trust layer is
specified separately (the Provenance Companion) and is entirely
optional — Steps 1 and 2 never require it.

## Design principles

- **Zero-cost drop-in.** Tagging is O(columns) and touches no data
  bytes. Untagged tables are silent everywhere.
- **Graceful degradation.** Unaware consumers read the storage type plus
  legible metadata. Nothing is ever gated on adoption.
- **Semantics as data.** Units, dimensions, display forms, and quantity
  vocabularies live in a versioned registry, federated by namespace —
  domains extend it without permission.
- **One implementation.** A single Rust core (`unitarrow-core`) does all
  parsing, dimension algebra, and conversion, exposed to Python
  (`unitarrow-py`, via the Arrow PyCapsule interface — no pyarrow
  dependency), TypeScript, and WASM (`unitarrow-wasm`). A conformance
  suite of golden files keeps every binding identical.

## Packages

| Package | Registry | Status |
|---|---|---|
| `unitarrow` / `unitarrow-core` / `unitarrow-py` / `unitarrow-wasm` | crates.io | reserved |
| `unitarrow` | PyPI | reserved |
| `unitarrow` | npm | reserved |

## Roadmap

Specification first, then `unitarrow-core`, then the Python and
browser codecs (Step 1), then checked semantics and the IR (Step 2),
then the Provenance Companion (Step 3). The full spec and milestone
roadmap will be published in this repository.

---

*UnitArrow began in energy-systems research — where BTU, MWh, and
per-unit quantities collide daily — but nothing in it is
energy-specific.*
