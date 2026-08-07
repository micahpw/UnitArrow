"""UnitArrow: physical units and quantity semantics for Apache Arrow columns.

Tagging happens at egress — after an engine is done aggregating, just before a
table is written or sent. That ordering is deliberate: DuckDB, pandas and
polars all discard Arrow field metadata on ingestion, so a tag applied earlier
would not survive the pipeline anyway. Applied here, it survives IPC and
Parquet intact.

    import pyarrow as pa, unitarrow

    reg = unitarrow.Registry.from_path("registries/power-systems.toml")
    tagged = unitarrow.tag(table, {"p_mw": "MW", "v_kv": "kV"}, reg)
    pq.write_table(tagged, "out.parquet")

Every unit is canonicalized and validated before anything is written, so a
column cannot be tagged with a spelling the registry does not accept.
"""

from ._unitarrow import (  # noqa: F401
    Conversion,
    Fragment,
    Registry,
    EXTENSION_NAME,
    GRAMMAR_VERSION,
    SPEC_VERSION,
    extension_name,
    parse_metadata,
    spec_version,
)

__all__ = [
    "Registry",
    "Conversion",
    "Fragment",
    "Plan",
    "tag",
    "units_of",
    "parse_metadata",
    "EXTENSION_NAME",
    "GRAMMAR_VERSION",
    "SPEC_VERSION",
]


def tag(table, units, registry, *, quantities=None, strict=True):
    """Attach UnitArrow metadata to columns of a pyarrow Table or Schema.

    Args:
        table: a ``pyarrow.Table`` or ``pyarrow.Schema``.
        units: ``{column_name: unit_string}``. Units are canonicalized, so
            ``"MW/h"`` is stored as ``"MW*h^-1"``.
        registry: a loaded :class:`Registry`.
        quantities: optional ``{column_name: curie}`` for §7.3 quantity kinds.
        strict: if True (default), naming a column that does not exist is an
            error. A silent no-op there is how a pipeline ends up shipping an
            untagged column that everyone believes is tagged.

    Returns:
        The same type that was passed in, with metadata attached.

    Raises:
        ValueError: if a unit does not resolve, or — under ``strict`` — if a
            named column is absent.
    """
    import pyarrow as pa

    # Engines return different shapes: DuckDB's `.arrow()` gives a streaming
    # RecordBatchReader, not a Table. Tagging is a schema operation, so all
    # three are supported and none is materialized — a reader is rewrapped with
    # the tagged schema and stays streaming.
    if isinstance(table, pa.Table):
        schema = table.schema
    elif isinstance(table, pa.Schema):
        schema = table
    elif isinstance(table, pa.RecordBatchReader):
        schema = table.schema
    else:
        raise TypeError(
            f"expected a pyarrow Table, Schema, or RecordBatchReader, got "
            f"{type(table).__name__}"
        )
    names = set(schema.names)

    if strict:
        missing = sorted(set(units) - names)
        if missing:
            raise ValueError(
                f"no such column(s): {', '.join(missing)}. "
                f"Available: {', '.join(sorted(names))}"
            )

    quantities = quantities or {}
    fields = []
    for field in schema:
        unit = units.get(field.name)
        if unit is None:
            fields.append(field)
            continue
        # Canonicalized and validated here — a bad unit raises before anything
        # is written, rather than producing a file nobody can read back.
        blob = registry.metadata(unit, quantities.get(field.name))
        md = dict(field.metadata or {})
        md[EXTENSION_NAME.encode()] = blob.encode()
        fields.append(field.with_metadata(md))

    new_schema = pa.schema(fields, metadata=schema.metadata)
    if isinstance(table, pa.Table):
        return table.cast(new_schema)
    if isinstance(table, pa.RecordBatchReader):
        return pa.RecordBatchReader.from_batches(new_schema, table)
    return new_schema


def units_of(table):
    """Read back the units on a table or schema.

    Returns ``{column_name: unit_string}`` for tagged columns only. An untagged
    column is absent rather than ``None``: §5.3 makes "no unit" the absence of
    a claim, not a claim of dimensionlessness, and the two must not be
    conflated.
    """
    import pyarrow as pa

    schema = table if isinstance(table, pa.Schema) else table.schema
    key = EXTENSION_NAME.encode()
    out = {}
    for field in schema:
        blob = (field.metadata or {}).get(key)
        if blob is not None:
            out[field.name] = parse_metadata(blob.decode())["unit"]
    return out


class Plan:
    """A unit-aware expression builder that lowers to SQL.

    It computes *schemas*, never values. Given the units of the input columns it
    decides whether an operation is legal, what unit the result carries, and
    what arithmetic the engine must perform — then hands the SQL to DuckDB,
    polars, or anything else, and tags the result from the plan.

    That last part is the point. Tags do not have to survive an engine that
    discards Arrow metadata, because the output unit is known *before* the query
    runs and the result is retagged from the plan rather than recovered from the
    input.

        plan = unitarrow.Plan(reg, {"p_mw": "MW", "hours": "h"})
        energy = plan["p_mw"] * plan["hours"]      # MW*h
        print(energy.sql)                          # (p_mw * hours)

        out = con.sql(f"SELECT {energy.sql} AS e FROM t").arrow()
        out = unitarrow.tag(out, {"e": energy.unit}, reg)

    Operators mirror §8.1: ``*`` and ``/`` combine dimensions, ``+`` and ``-``
    require matching dimensions and emit the coercion into the SQL, so the
    engine adds comparable numbers rather than raw ones.
    """

    def __init__(self, registry, units):
        self._reg = registry
        self._cols = {name: _Expr(registry.col(name, unit), registry) for name, unit in units.items()}

    @classmethod
    def from_table(cls, table, registry):
        """Build a plan from a table's existing UnitArrow tags."""
        return cls(registry, units_of(table))

    def __getitem__(self, name):
        try:
            return self._cols[name]
        except KeyError:
            raise KeyError(
                f"{name!r} is not a tagged column in this plan. "
                f"Tagged: {', '.join(sorted(self._cols)) or '(none)'}"
            ) from None

    def __contains__(self, name):
        return name in self._cols

    def __repr__(self):
        cols = ", ".join(f"{k}:{v.unit}" for k, v in sorted(self._cols.items()))
        return f"<Plan {cols}>"


class _Expr:
    """A typed fragment with Python operators. Wraps the Rust `Fragment`."""

    __slots__ = ("_f", "_reg")

    def __init__(self, fragment, registry):
        self._f = fragment
        self._reg = registry

    @property
    def sql(self):
        """The expression text, for an engine to run."""
        return self._f.sql

    @property
    def unit(self):
        """The canonical unit of the result — what to tag the output with."""
        return self._f.unit

    def to(self, unit):
        """Convert, emitting the scaling (or the intercept, for affine units)."""
        return _Expr(self._f.to(unit, self._reg), self._reg)

    def __mul__(self, other):
        return _Expr(self._f.mul(other._f, self._reg), self._reg)

    def __truediv__(self, other):
        return _Expr(self._f.div(other._f, self._reg), self._reg)

    def __add__(self, other):
        return _Expr(self._f.add(other._f, self._reg), self._reg)

    def __sub__(self, other):
        return _Expr(self._f.sub(other._f, self._reg), self._reg)

    def __repr__(self):
        return f"<{self.sql} :: {self.unit}>"
