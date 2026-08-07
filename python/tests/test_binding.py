"""Tests for the binding boundary itself.

Deliberately not a second copy of the Rust suite — the core's behaviour is
covered by 156 Rust tests. What is only testable here is what happens *at the
boundary*: whether §10 error codes survive into Python exceptions, whether
128-bit factors survive the FFI intact, and whether the objects behave like
Python objects.
"""

import pytest

import unitarrow

REGISTRY = "registries/power-systems.toml"


@pytest.fixture(scope="module")
def reg(repo_root):
    return unitarrow.Registry.from_path(str(repo_root / REGISTRY))


def test_module_constants_match_the_core(reg):
    assert unitarrow.EXTENSION_NAME == "unitarrow.quantity.v1"
    assert unitarrow.GRAMMAR_VERSION == 1
    assert unitarrow.SPEC_VERSION.startswith("0.")
    # The function form and the constant must agree; two spellings of one fact
    # is how they drift.
    assert unitarrow.extension_name() == unitarrow.EXTENSION_NAME
    assert unitarrow.spec_version() == unitarrow.SPEC_VERSION


def test_canonicalization_crosses_the_boundary(reg):
    assert reg.canonicalize("MW/h") == "MW*h^-1"
    assert reg.canonicalize("h*MW") == "MW*h"
    assert reg.canonicalize("USD/MWh") == "USD*MWh^-1"


def test_errors_carry_their_spec_code(reg):
    """§10 codes are the wire-stable vocabulary, so they must not be lost when
    an error becomes a Python exception."""
    with pytest.raises(ValueError, match="E_UNKNOWN_UNIT"):
        reg.canonicalize("Zorkmid")
    with pytest.raises(ValueError, match="E_UNIT_SYNTAX"):
        reg.canonicalize("MW**h")
    with pytest.raises(ValueError, match="E_DIM_MISMATCH"):
        reg.conversion("MW", "h")


def test_a_declared_refusal_explains_itself_rather_than_suggesting(reg):
    with pytest.raises(ValueError) as e:
        reg.canonicalize("MBtu")
    msg = str(e.value)
    assert "refused rather than guessed" in msg
    assert "Roman thousand" in msg
    assert "MMBtu" in msg
    # A deliberate refusal must not read as a typo.
    assert "did you mean" not in msg


def test_large_factors_survive_the_ffi_intact():
    """Factors are i128 in Rust. Python ints are arbitrary precision, so the
    value must arrive exact rather than rounded through a float."""
    reg = unitarrow.Registry.from_toml(
        '[registry]\nschema_version = 1\nname = "big"\nversion = "1"\n'
        '[dimension.energy]\nvector = { mass = 1, length = 2, time = -2 }\n'
        '[unit.J]\ndimension = "energy"\nfactor = [1, 1]\n'
        '[unit.huge]\ndimension = "energy"\nfactor = [170141183460469231729, 7]\n'
    )
    c = reg.conversion("huge", "J")
    assert c.numerator == 170141183460469231729
    assert c.denominator == 7
    assert isinstance(c.numerator, int)


def test_pi_exponent_is_visible_so_callers_know_when_exactness_ends(reg):
    deg = reg.conversion("deg", "rad")
    assert deg.pi == 1
    assert deg.is_exact_rational is False
    assert deg.apply(180.0) == pytest.approx(3.141592653589793, abs=0)

    mw = reg.conversion("MW", "kW")
    assert mw.pi == 0
    assert mw.is_exact_rational is True
    assert mw.apply(1.0) == 1000.0


def test_affine_conversions_carry_their_offset(reg):
    c = reg.conversion("degC", "K")
    assert (c.offset_numerator, c.offset_denominator) == (5463, 20)
    assert c.apply(0.0) == 273.15


def test_dimension_is_a_sparse_mapping(reg):
    assert reg.dimension("MW") == {"mass": 1, "length": 2, "time": -3}
    assert reg.dimension("pu") == {}
    # The case AMB-066 exists for: angular frequency is not frequency.
    assert reg.dimension("rad/s") != reg.dimension("Hz")
    assert reg.commensurable("MW", "MVAr") is True
    assert reg.commensurable("rad/s", "Hz") is False


def test_reverse_lookup(reg):
    assert reg.describe("MW") == "megawatt"
    assert reg.describe("Zorkmid") is None


def test_metadata_is_built_by_the_core_not_by_python(reg):
    assert reg.metadata("MW") == '{"unit":"MW","grammar":1}'
    # The unit is canonicalized on the way in, so a column cannot be tagged
    # with a spelling the registry would not accept back.
    assert reg.metadata("MW/h") == '{"unit":"MW*h^-1","grammar":1}'
    assert '"quantity":"core:power"' in reg.metadata("MW", "core:power")


def test_parse_metadata_preserves_unknown_keys():
    """§5.2 requires it, and the companion's data digest depends on it
    (AMB-024) — a reader that drops an unrecognised key breaks a seal."""
    got = unitarrow.parse_metadata(
        '{"unit":"MW","grammar":1,"vendorX:calib":"2026-03-01"}'
    )
    assert got["unit"] == "MW"
    assert got["vendorX:calib"] == '"2026-03-01"'


def test_registry_repr_is_informative(reg):
    r = repr(reg)
    assert "power-systems" in r and "resolvable" in r


def test_registry_is_frozen(reg):
    with pytest.raises(AttributeError):
        reg.name = "something else"
