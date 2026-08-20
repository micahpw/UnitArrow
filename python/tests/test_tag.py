"""Tests for `tag()` and `units_of()` — the only real logic on the Python side.

These cover what the Rust suite cannot: metadata placement on a pyarrow schema,
survival through the two egress formats, and the failure modes of a helper that
takes a dict of column names.
"""

import io

import pyarrow as pa
import pyarrow.ipc as ipc
import pyarrow.parquet as pq
import pytest

import unitarrow


@pytest.fixture(scope="module")
def reg(repo_root):
    # The effective registry a deployment loads: core composed with the domain
    # extension (§7.5), which is not standalone.
    return unitarrow.Registry.compose_paths(
        [
            str(repo_root / "registries/core.toml"),
            str(repo_root / "registries/power-systems.toml"),
        ]
    )


@pytest.fixture
def table():
    return pa.table({"p_mw": [10.0, 12.0], "v_kv": [138.0, 345.0], "bus": ["A", "B"]})


def test_tagging_round_trips_through_parquet(reg, table, tmp_path):
    tagged = unitarrow.tag(table, {"p_mw": "MW", "v_kv": "kV"}, reg)
    path = tmp_path / "t.parquet"
    pq.write_table(tagged, path)
    assert unitarrow.units_of(pq.read_table(path)) == {"p_mw": "MW", "v_kv": "kV"}


def test_tagging_round_trips_through_ipc(reg, table):
    tagged = unitarrow.tag(table, {"p_mw": "MW"}, reg)
    sink = io.BytesIO()
    with ipc.new_stream(sink, tagged.schema) as w:
        w.write_table(tagged)
    back = ipc.open_stream(io.BytesIO(sink.getvalue())).read_all()
    assert unitarrow.units_of(back) == {"p_mw": "MW"}


def test_units_are_canonicalized_not_stored_as_typed(reg, table):
    tagged = unitarrow.tag(table, {"p_mw": "MW/h"}, reg)
    # What is stored is the equality primitive, not what someone typed.
    assert unitarrow.units_of(tagged)["p_mw"] == "MW*h^-1"


def test_an_untagged_column_is_absent_not_none(reg, table):
    """§5.3: no unit is the absence of a claim, not a claim of
    dimensionlessness. Returning None for `bus` would invite conflating them."""
    tagged = unitarrow.tag(table, {"p_mw": "MW"}, reg)
    units = unitarrow.units_of(tagged)
    assert "bus" not in units
    assert units == {"p_mw": "MW"}


def test_data_is_untouched(reg, table):
    """Tagging is O(columns) and touches no buffers (§1 goal 1)."""
    tagged = unitarrow.tag(table, {"p_mw": "MW"}, reg)
    assert tagged.column("p_mw").to_pylist() == [10.0, 12.0]
    assert tagged.schema.types == table.schema.types
    assert tagged.num_rows == table.num_rows


def test_existing_field_metadata_is_preserved(reg):
    """Other tools put things in field metadata too; clobbering them would make
    tagging destructive."""
    f = pa.field("p_mw", pa.float64(), metadata={b"vendor": b"keep me"})
    t = pa.table([pa.array([1.0])], schema=pa.schema([f]))
    tagged = unitarrow.tag(t, {"p_mw": "MW"}, reg)
    md = tagged.schema.field("p_mw").metadata
    assert md[b"vendor"] == b"keep me"
    assert unitarrow.EXTENSION_NAME.encode() in md


def test_schema_level_metadata_is_preserved(reg, table):
    t = table.replace_schema_metadata({b"unitarrow:provenance": b'{"mode":"strict"}'})
    tagged = unitarrow.tag(t, {"p_mw": "MW"}, reg)
    assert tagged.schema.metadata[b"unitarrow:provenance"] == b'{"mode":"strict"}'


def test_a_schema_can_be_tagged_without_a_table(reg, table):
    tagged = unitarrow.tag(table.schema, {"p_mw": "MW"}, reg)
    assert isinstance(tagged, pa.Schema)
    assert unitarrow.units_of(tagged) == {"p_mw": "MW"}


def test_retagging_replaces_rather_than_duplicates(reg, table):
    once = unitarrow.tag(table, {"p_mw": "MW"}, reg)
    twice = unitarrow.tag(once, {"p_mw": "kW"}, reg)
    assert unitarrow.units_of(twice) == {"p_mw": "kW"}


def test_a_misspelled_column_is_an_error_by_default(reg, table):
    """A silent no-op here is how a pipeline ships an untagged column that
    everyone believes is tagged."""
    with pytest.raises(ValueError) as e:
        unitarrow.tag(table, {"p_MW": "MW"}, reg)
    assert "no such column" in str(e.value)
    # ...and the message says what was available, so the typo is findable.
    assert "p_mw" in str(e.value)


def test_strict_can_be_opted_out_of(reg, table):
    tagged = unitarrow.tag(table, {"nope": "MW", "p_mw": "MW"}, reg, strict=False)
    assert unitarrow.units_of(tagged) == {"p_mw": "MW"}


def test_a_bad_unit_raises_before_anything_is_written(reg, table, tmp_path):
    with pytest.raises(ValueError, match="E_UNKNOWN_UNIT"):
        unitarrow.tag(table, {"p_mw": "MW", "v_kv": "Zorkmid"}, reg)
    # Nothing partially applied: the original table is unchanged.
    assert unitarrow.units_of(table) == {}


def test_a_declared_refusal_reaches_the_caller(reg, table):
    with pytest.raises(ValueError, match="refused rather than guessed"):
        unitarrow.tag(table, {"p_mw": "MBtu"}, reg)


def test_quantity_kinds_ride_along(reg, table):
    tagged = unitarrow.tag(
        table, {"p_mw": "MW"}, reg, quantities={"p_mw": "core:power_generation"}
    )
    blob = tagged.schema.field("p_mw").metadata[unitarrow.EXTENSION_NAME.encode()]
    assert unitarrow.parse_metadata(blob.decode())["quantity"] == "core:power_generation"


def test_tagging_nothing_is_a_no_op(reg, table):
    assert unitarrow.units_of(unitarrow.tag(table, {}, reg)) == {}
