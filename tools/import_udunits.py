#!/usr/bin/env python3
"""Generate a UnitArrow registry from the pinned UDUNITS-2 submodule.

    python3 tools/import_udunits.py > registries/udunits.toml

The submodule is a *codegen input*, never a vendored output: the generated TOML
is committed and reviewed, because §7 treats a registry as data that every
conversion in a deployment flows from. `git diff` on the generated file is the
review.

Four categories are deliberately excluded, each for a stated reason rather than
by silent omission — a generator that quietly drops what it cannot model will,
on some later upstream change, drop something nobody notices is missing.
"""

import argparse
import re
import subprocess
import sys
from fractions import Fraction

sys.path.insert(0, "tools")
from udunits_eval import (  # noqa: E402
    BASE, BASE_NAMES, evaluate, load, load_prefixes, Unresolvable,
)

I128 = 2**127 - 1
LOG = re.compile(r"\b(lg|ln|lb)\s*\(")
SYMBOL = re.compile(r"[A-Za-z][A-Za-z0-9_]*$")

# Dimension names for the vectors that have conventional ones. Anything else
# gets a generated name; readability is worth this much curation and no more.
NAMED = {
    (): "dimensionless",
    (("length", 1),): "length",
    (("mass", 1),): "mass",
    (("time", 1),): "time",
    (("current", 1),): "current",
    (("temperature", 1),): "temperature",
    (("amount", 1),): "amount",
    (("luminosity", 1),): "luminosity",
    (("angle", 1),): "angle",
    (("time", -1),): "frequency",
    (("length", 2),): "area",
    (("length", 3),): "volume",
    (("length", 1), ("time", -1)): "velocity",
    (("length", 1), ("time", -2)): "acceleration",
    (("length", 1), ("mass", 1), ("time", -2)): "force",
    (("length", 2), ("mass", 1), ("time", -2)): "energy",
    (("length", 2), ("mass", 1), ("time", -3)): "power",
    (("length", -1), ("mass", 1), ("time", -2)): "pressure",
    (("current", 1), ("time", 1)): "charge",
    (("current", -1), ("length", 2), ("mass", 1), ("time", -3)): "voltage",
    (("current", -2), ("length", 2), ("mass", 1), ("time", -3)): "impedance",
    (("current", 2), ("length", -2), ("mass", -1), ("time", 3)): "admittance",
    (("current", 2), ("length", -2), ("mass", -1), ("time", 4)): "capacitance",
    (("current", -2), ("length", 2), ("mass", 1), ("time", -2)): "inductance",
    (("length", -3), ("mass", 1)): "density",
    (("angle", 1), ("time", -1)): "angular_frequency",
}


# Units that take SI prefixes. Curated rather than blanket: making a unit
# prefixable silently claims one spelling per prefix (AMB-054), and UDUNITS
# applies prefixes at parse time to almost everything, which is not a model
# UnitArrow can adopt without minting hundreds of contested symbols.
#
# `g` rather than `kg`: the SI base unit already carries a prefix, so the gram
# is the prefixable entry and the kilogram derives from it.
PREFIXABLE = {
    "m", "g", "s", "A", "K", "mol", "cd", "rad",
    "Hz", "N", "Pa", "J", "W", "C", "V", "F", "ohm", "S", "Wb", "T", "H",
    "lm", "lx", "Bq", "Gy", "Sv", "kat", "L", "eV", "bar",
}


def dim_key(dims):
    return tuple(sorted((k, v) for k, v in dims.items() if v))


def dim_name(dims, invented):
    key = dim_key(dims)
    if key in NAMED:
        return NAMED[key]
    if key in invented:
        return invented[key]
    name = "d_" + "_".join(f"{k}{v}".replace("-", "m") for k, v in key)
    invented[key] = name
    return name


def toml_escape(s):
    return s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", " ")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default="vendor/udunits-2/lib")
    args = ap.parse_args()

    commit = subprocess.run(
        ["git", "-C", "vendor/udunits-2", "rev-parse", "HEAD"],
        capture_output=True, text=True, check=True,
    ).stdout.strip()
    date = subprocess.run(
        ["git", "-C", "vendor/udunits-2", "log", "-1", "--format=%cd", "--date=short"],
        capture_output=True, text=True, check=True,
    ).stdout.strip()

    table, entries = load(args.root)
    prefixes = load_prefixes(args.root)
    memo = {}

    units, skipped = [], []
    for e in entries:
        d, src = e["def"], e["file"]
        symbols = [s for s in e["symbols"] if SYMBOL.match(s)]
        names = [n for n in e["names"] if SYMBOL.match(n)]
        chosen = (symbols or names)
        label = (e["symbols"] + e["names"] + ["?"])[0]

        if not chosen:
            skipped.append((label, "no ASCII symbol or name (§6.1)"))
            continue
        symbol = chosen[0]

        # The SI base unit of mass already carries a prefix. Authoring `kg`
        # while `g` is prefixable would make one spelling reachable two ways,
        # which the load check rejects. `g` is authored at 1/1000 and `kg`
        # derives from it at exactly 1 — the same resolution the hand-written
        # power-systems registry uses.
        if symbol == "kg" and "g" in PREFIXABLE:
            skipped.append((symbol, "derives from the prefixable `g`; authoring it too would collide"))
            continue

        if d is not None and LOG.search(d):
            skipped.append((symbol, "logarithmic; §7.2 models scale and offset only (AMB-067)"))
            continue

        offset = None
        if d is not None and "@" in d:
            body, off = d.split("@", 1)
            try:
                v = evaluate(body, table, memo, prefixes=prefixes)
                offset = Fraction(off.strip()) * v.scale
            except (Unresolvable, ValueError) as ex:
                skipped.append((symbol, f"affine, unresolvable: {ex}"))
                continue
        elif d is None:
            # `rad` has no definition upstream because it is dimensionless
            # there. Here angle is a base dimension, so it is a base unit.
            base = BASE.get(symbol) or BASE_NAMES.get(symbol)
            if symbol == "rad":
                base = "angle"
            if base is None:
                skipped.append((symbol, "no definition and not a base unit"))
                continue
            v = type("V", (), {"scale": Fraction(1), "dims": {base: 1}, "pi": 0})()
        else:
            try:
                v = evaluate(d, table, memo, prefixes=prefixes)
            except Unresolvable as ex:
                skipped.append((symbol, str(ex)))
                continue

        if v.scale < 0:
            # `degree_west = -1 degree_east`. A negative scale is a direction
            # convention, not a unit: it flips a sign rather than changing the
            # size of anything, and §7.2 rejects it at load. Excluding it is the
            # honest outcome — the sign belongs to the datum, not the unit.
            skipped.append((symbol, "negative scale; a sign convention, not a unit (§7.2)"))
            continue
        if abs(v.scale.numerator) > I128 or v.scale.denominator > I128:
            skipped.append((symbol, "factor exceeds the exact-rational range"))
            continue
        if not -8 <= v.pi <= 8:
            skipped.append((symbol, f"pi exponent {v.pi} out of range"))
            continue
        units.append((symbol, v, offset, e, d, src))

    unprefixed = []
    invented = {}
    dims_used = {}
    for symbol, v, offset, e, d, src in units:
        dims_used[dim_key(v.dims)] = dim_name(v.dims, invented)

    out = []
    w = out.append
    w("# UnitArrow registry — generated from UDUNITS-2. DO NOT EDIT BY HAND.")
    w("#")
    w("#     python3 tools/import_udunits.py > registries/udunits.toml")
    w("#")
    w(f"# Source: Unidata/UDUNITS-2 @ {commit[:12]} ({date}), BSD-style licence,")
    w("# vendored as a submodule at vendor/udunits-2 and used as a codegen input.")
    w("# The generated file is committed and reviewed, because §7 treats a registry")
    w("# as data every conversion in a deployment flows from — `git diff` is the")
    w("# review.")
    w("#")
    w("# TWO DELIBERATE DEPARTURES FROM UPSTREAM, both recorded in the register:")
    w("#")
    w("#   `rad` is dimensionless in UDUNITS, following SI. Here angle is a base")
    w("#   dimension, because with a dimensionless radian `rad/s` and `Hz` share")
    w("#   {time:-1} and a 2*pi error passes every dimensional check (AMB-066).")
    w("#")
    w("#   `pi` is stored upstream as a truncated 30-digit decimal. Inheriting")
    w("#   that would put a rounding where §7.2 promises a definition, so it is")
    w("#   carried as an exact exponent on the factor instead (AMB-066).")
    w("#")
    w(f"# {len(units)} units imported, {len(skipped)} excluded. Every exclusion is listed")
    w("# at the foot of this file rather than dropped silently.")
    w("")
    w("[registry]")
    w("schema_version = 1")
    w('name = "udunits"')
    w(f'version = "{date}"')
    w("")
    w("# ---------------------------------------------------------------------------")
    w("# Dimensions")
    w("# ---------------------------------------------------------------------------")
    w("")
    for key, name in sorted(dims_used.items(), key=lambda kv: kv[1]):
        if name == "dimensionless":
            w(f"[dimension.{name}]")
            w("vector = { }")
        elif len(key) == 1 and key[0][1] == 1 and key[0][0] == name:
            continue  # a base dimension needs no declaration
        else:
            w(f"[dimension.{name}]")
            w("vector = { " + ", ".join(f"{k} = {v}" for k, v in key) + " }")
        w("")

    w("# ---------------------------------------------------------------------------")
    w("# Units")
    w("# ---------------------------------------------------------------------------")
    w("")
    seen = set()
    for symbol, v, offset, e, d, src in sorted(units, key=lambda u: u[0]):
        if symbol in seen:
            continue
        seen.add(symbol)
        w(f"[unit.{symbol}]")
        w(f'dimension = "{dim_name(v.dims, invented)}"')
        w(f"factor = [{v.scale.numerator}, {v.scale.denominator}]")
        if v.pi:
            w(f"pi = {v.pi}")
        if symbol in PREFIXABLE:
            # Prefixing is only declared when every prefix in the set stays
            # inside the exact-rational range. `eV` is the case that forces the
            # check: its factor already carries a denominator of 10^27, so
            # pico- overflows i128 and the registry would fail to load.
            #
            # The all-or-nothing outcome is a real limitation rather than a
            # tidy one — it costs keV and MeV, which do fit, to avoid peV, which
            # does not. A per-unit prefix RANGE would keep both; §7.2 offers
            # only named sets (AMB-053).
            if all(
                abs((v.scale * Fraction(10) ** p).numerator) <= I128
                and (v.scale * Fraction(10) ** p).denominator <= I128
                for p in range(-30, 31)
            ):
                w('prefixes = "si"')
            else:
                unprefixed.append(symbol)
        if offset is not None:
            w(f"offset = [{offset.numerator}, {offset.denominator}]")
        # Every other ASCII spelling becomes an input alias, so the entry is
        # reachable by the name a user actually types. UDUNITS lists the most
        # precise name first — `IT_Btu` before `Btu` — and that ordering is
        # kept for the canonical symbol, with the rest resolving to it.
        others = [
            x
            for x in (e["symbols"] + e["names"])
            if x != symbol and SYMBOL.match(x)
        ]
        alias = sorted(dict.fromkeys(others))
        if alias:
            w("aliases = [" + ", ".join(f'"{a}"' for a in alias) + "]")
        long = next((n for n in e["names"] if SYMBOL.match(n)), None)
        if long and long != symbol:
            w(f'display = {{ long = "{toml_escape(long)}" }}')
        prov = f'provenance = {{ system = "udunits", source = "udunits2-{src}.xml@{commit[:12]}"'
        if d:
            prov += f', note = "upstream def: {toml_escape(d)}"'
        w(prov + " }")
        w("")

    if unprefixed:
        w("# ---------------------------------------------------------------------------")
        w("# Not prefixable, though the unit kind normally would be: at least one")
        w("# prefix pushes the exact factor outside the i128 rational range (AMB-053).")
        w("# ---------------------------------------------------------------------------")
        for sym in sorted(unprefixed):
            w(f"#   {sym}")
        w("")

    w("# ---------------------------------------------------------------------------")
    w(f"# Excluded ({len(skipped)}) — listed so an upstream change that drops something")
    w("# is visible in the diff rather than silent.")
    w("# ---------------------------------------------------------------------------")
    for sym, why in sorted(skipped):
        w(f"#   {sym:<24} {why}")

    print("\n".join(out))
    print(f"\n# generated {len(seen)} units across {len(dims_used)} dimensions", file=sys.stderr)
    print(f"# excluded {len(skipped)}", file=sys.stderr)


if __name__ == "__main__":
    main()
