"""Tests for `Plan` — the unit-aware expression layer that lowers to SQL.

`Plan` computes schemas, never values, so most of what matters is: does the
result carry the right unit, and is the emitted arithmetic the arithmetic the
unit implies. The DuckDB tests close the loop by actually running it, because a
fragment that types correctly and computes the wrong number is worse than one
that fails.
"""

import math

import pyarrow as pa
import pytest

import unitarrow


@pytest.fixture(scope="module")
def reg(repo_root):
    return unitarrow.Registry.from_path(str(repo_root / "registries/power-systems.toml"))


@pytest.fixture
def plan(reg):
    return unitarrow.Plan(
        reg,
        {"p_mw": "MW", "q_mvar": "MVAr", "hours": "h", "t_c": "degC", "theta_deg": "deg"},
    )


def test_a_column_carries_its_unit_and_no_arithmetic(plan):
    e = plan["p_mw"]
    assert e.sql == "p_mw"
    assert e.unit == "MW"


def test_a_missing_column_says_what_is_available(plan):
    with pytest.raises(KeyError) as excinfo:
        plan["p_MW"]
    msg = str(excinfo.value)
    assert "p_mw" in msg, "the typo should be findable from the message"


def test_products_compute_the_result_unit(plan):
    e = plan["p_mw"] * plan["hours"]
    assert e.unit == "MW*h"
    assert e.sql == "(p_mw * hours)"


def test_quotients_compute_the_result_unit(plan):
    e = plan["p_mw"] / plan["hours"]
    assert e.unit == "MW*h^-1"


def test_a_ratio_of_like_dimensions_is_dimensionless(plan, reg):
    """Power factor is MW / MVA — both power, so the result is a pure ratio."""
    pf = plan["p_mw"] / unitarrow.Plan(reg, {"s_mva": "MVA"})["s_mva"]
    assert reg.dimension(pf.unit) == {}


def test_conversion_emits_the_scaling(plan):
    e = plan["p_mw"].to("kW")
    assert e.unit == "kW"
    assert e.sql == "(p_mw * 1000)"


def test_an_affine_conversion_adds_rather_than_scales(plan):
    """The case an emitter that assumes multiplication gets wrong at every
    value."""
    e = plan["t_c"].to("K")
    assert e.unit == "K"
    assert "+" in e.sql and "5463" in e.sql


def test_pi_stays_symbolic_in_the_sql(plan):
    """`deg` has no rational factor, so a pre-computed literal would round it.
    Emitting the engine's own pi keeps "floats come last" true past this
    process."""
    e = plan["theta_deg"].to("rad")
    assert e.sql == "(theta_deg * pi() / 180)"


def test_exact_rationals_appear_as_rationals(reg):
    """Auditability: a reviewer sees the registry's definition in the query
    rather than a decimal they would have to check elsewhere."""
    e = unitarrow.Plan(reg, {"q": "Btu"})["q"].to("J")
    assert e.sql == "(q * 52752792631 / 50000000)"


def test_addition_emits_the_coercion(plan, reg):
    """§8.1 coerces the right operand to the left unit. Emitting it means the
    engine adds comparable numbers rather than raw ones."""
    kw = unitarrow.Plan(reg, {"small": "kW"})["small"]
    e = plan["p_mw"] + kw
    assert e.unit == "MW"
    assert e.sql == "(p_mw + (small / 1000))"


def test_adding_matching_units_emits_no_scaling(plan, reg):
    other = unitarrow.Plan(reg, {"p2": "MW"})["p2"]
    assert (plan["p_mw"] + other).sql == "(p_mw + p2)"


def test_adding_across_dimensions_raises_before_any_sql(plan):
    with pytest.raises(ValueError) as excinfo:
        plan["p_mw"] + plan["theta_deg"]
    msg = str(excinfo.value)
    assert "E_DIM_MISMATCH" in msg
    # The message must come from the addition check, not from the conversion it
    # would otherwise fall through to. Both raise E_DIM_MISMATCH, so asserting
    # the code alone does not pin which fired — and only the addition check can
    # say what the caller was actually trying to do.
    assert "cannot add MW to deg" in msg


def test_mw_and_mvar_type_check_because_they_share_a_dimension(plan):
    """Deliberate: no quantity kinds are declared, so real and reactive power
    are dimensionally identical and the checker does not separate them. If this
    starts failing, kinds were added and the registry's comment is stale."""
    assert (plan["p_mw"] + plan["q_mvar"]).unit == "MW"


def test_nesting_composes_without_precedence_surprises(plan):
    e = (plan["p_mw"] * plan["hours"]).to("kWh")
    assert e.sql == "((p_mw * hours) * 1000)"
    assert e.unit == "kWh"


def test_a_plan_can_be_built_from_a_table_s_own_tags(reg):
    t = unitarrow.tag(pa.table({"p_mw": [1.0]}), {"p_mw": "MW"}, reg)
    p = unitarrow.Plan.from_table(t, reg)
    assert "p_mw" in p
    assert p["p_mw"].unit == "MW"


def test_plan_repr_lists_its_columns(plan):
    assert "p_mw:MW" in repr(plan)


# --------------------------------------------------------------------------
# The loop closed: generate, run, retag.
# --------------------------------------------------------------------------


@pytest.fixture
def con():
    duckdb = pytest.importorskip("duckdb")
    return duckdb.connect()


@pytest.fixture
def tagged(reg):
    src = pa.table(
        {"p_mw": [100.0, 250.0], "hours": [1.0, 1.0], "t_c": [25.0, 31.0],
         "theta_deg": [0.0, 180.0]}
    )
    return unitarrow.tag(
        src, {"p_mw": "MW", "hours": "h", "t_c": "degC", "theta_deg": "deg"}, reg
    )


def test_duckdb_drops_the_input_tags(con, tagged):
    """The premise. If this ever stops being true, the reconstruct-from-plan
    design is no longer necessary — and that is worth noticing."""
    con.register("t", tagged)
    assert unitarrow.units_of(con.sql("SELECT * FROM t").arrow().read_all()) == {}


def test_the_output_is_tagged_from_the_plan_not_recovered(con, reg, tagged):
    con.register("t", tagged)
    plan = unitarrow.Plan.from_table(tagged, reg)
    energy = plan["p_mw"] * plan["hours"]
    out = con.sql(f"SELECT {energy.sql} AS e FROM t").arrow().read_all()
    assert unitarrow.units_of(out) == {}  # nothing survived the engine
    out = unitarrow.tag(out, {"e": energy.unit}, reg)
    assert unitarrow.units_of(out) == {"e": "MW*h"}


def test_the_generated_arithmetic_is_correct(con, reg, tagged):
    """A fragment that types correctly and computes the wrong number is worse
    than one that fails, so the numbers are checked against known truth."""
    con.register("t", tagged)
    plan = unitarrow.Plan.from_table(tagged, reg)
    exprs = {
        "kwh": (plan["p_mw"] * plan["hours"]).to("kWh"),
        "kelvin": plan["t_c"].to("K"),
        "radians": plan["theta_deg"].to("rad"),
    }
    sql = "SELECT " + ", ".join(f"{e.sql} AS {n}" for n, e in exprs.items()) + " FROM t"
    out = con.sql(sql).arrow().read_all()

    assert out.column("kwh").to_pylist() == [100_000.0, 250_000.0]
    assert out.column("kelvin").to_pylist() == [298.15, 304.15]
    # 180 degrees is pi exactly, because pi never became a rounded literal.
    assert out.column("radians").to_pylist()[1] == math.pi


def test_tagging_a_streaming_reader_keeps_it_streaming(con, reg, tagged):
    """DuckDB's `.arrow()` returns a RecordBatchReader — what an engine hands
    back. Materializing it here would quietly turn a stream into memory."""
    con.register("t", tagged)
    reader = con.sql("SELECT p_mw FROM t").arrow()
    assert isinstance(reader, pa.RecordBatchReader)

    out = unitarrow.tag(reader, {"p_mw": "MW"}, reg)
    assert isinstance(out, pa.RecordBatchReader)
    assert unitarrow.units_of(out) == {"p_mw": "MW"}
    # And it still yields its rows.
    assert out.read_all().column("p_mw").to_pylist() == [100.0, 250.0]


def test_tag_rejects_a_shape_it_cannot_handle(reg):
    with pytest.raises(TypeError, match="Table, Schema, or RecordBatchReader"):
        unitarrow.tag({"not": "arrow"}, {"a": "MW"}, reg)
