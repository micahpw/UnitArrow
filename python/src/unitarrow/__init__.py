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

    schema = table.schema if isinstance(table, pa.Table) else table
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
    return table.cast(new_schema) if isinstance(table, pa.Table) else new_schema


def units_of(table):
    """Read back the units on a table or schema.

    Returns ``{column_name: unit_string}`` for tagged columns only. An untagged
    column is absent rather than ``None``: §5.3 makes "no unit" the absence of
    a claim, not a claim of dimensionlessness, and the two must not be
    conflated.
    """
    import pyarrow as pa

    schema = table.schema if isinstance(table, pa.Table) else table
    key = EXTENSION_NAME.encode()
    out = {}
    for field in schema:
        blob = (field.metadata or {}).get(key)
        if blob is not None:
            out[field.name] = parse_metadata(blob.decode())["unit"]
    return out
