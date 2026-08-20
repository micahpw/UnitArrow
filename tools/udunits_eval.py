"""Evaluate UDUNITS unit definitions into (exact scale, dimension vector).

UDUNITS defines a unit by an *expression* over other units — `W = J/s` — while
a UnitArrow registry states a dimension and an exact factor to that dimension's
base unit. Bridging the two needs a small evaluator over UDUNITS' definition
grammar, which is regular: `.` and space multiply, `/` divides, `^` raises,
parentheses group, and a leading scalar scales.

Two deliberate departures from what the file says, both recorded in the register:

  `pi`  is stored upstream as a truncated 30-digit decimal. Inheriting that
        would put a rounding where §7.2 promises a definition, so it is carried
        as an exact exponent instead (AMB-066).

  `rad` is `dimensionless` upstream, following SI. UnitArrow makes angle a base
        dimension, because with a dimensionless radian `rad/s` and `Hz` share
        {time:-1} and a 2*pi error passes every dimensional check (AMB-066).
"""

import re
import xml.etree.ElementTree as ET
from fractions import Fraction

FILES = ["base", "derived", "accepted", "common"]

# UDUNITS base symbol -> UnitArrow base dimension (§6.2).
BASE = {
    "m": "length", "kg": "mass", "s": "time", "A": "current",
    "K": "temperature", "mol": "amount", "cd": "luminosity",
}
# Overridden: dimensionless upstream, a base dimension here.
ANGLE = "rad"

# The base units are referenced by name as often as by symbol — `einstein` is
# defined as `mole`, not `mol`.
BASE_NAMES = {
    "meter": "length", "metre": "length", "kilogram": "mass", "second": "time",
    "ampere": "current", "kelvin": "temperature", "mole": "amount",
    "candela": "luminosity", "radian": "angle",
}


def singularize(word):
    """UDUNITS pluralizes names on the fly, so a definition may reference a
    plural that appears nowhere in the file — `3 international_feet` against an
    entry whose only name is `international_foot`."""
    for plural, singular in (("feet", "foot"), ("inches", "inch"), ("ies", "y")):
        if word.endswith(plural):
            return word[: -len(plural)] + singular
    if word.endswith("es") and word[:-2].endswith(("s", "x", "z", "ch", "sh")):
        return word[:-2]
    if word.endswith("s") and not word.endswith("ss"):
        return word[:-1]
    return word


class Value:
    """scale * pi^pi_exp * PRODUCT(base ** exponent)"""

    __slots__ = ("scale", "dims", "pi")

    def __init__(self, scale=Fraction(1), dims=None, pi=0):
        self.scale, self.dims, self.pi = Fraction(scale), dict(dims or {}), pi

    def __mul__(self, o):
        d = dict(self.dims)
        for k, v in o.dims.items():
            d[k] = d.get(k, 0) + v
        return Value(self.scale * o.scale, {k: v for k, v in d.items() if v}, self.pi + o.pi)

    def __truediv__(self, o):
        d = dict(self.dims)
        for k, v in o.dims.items():
            d[k] = d.get(k, 0) - v
        return Value(self.scale / o.scale, {k: v for k, v in d.items() if v}, self.pi - o.pi)

    def __pow__(self, n):
        return Value(self.scale ** n, {k: v * n for k, v in self.dims.items()}, self.pi * n)


class Unresolvable(Exception):
    pass


def load_prefixes(root="vendor/udunits-2/lib"):
    """symbol/name -> decimal multiplier.

    UDUNITS applies prefixes at parse time rather than storing prefixed
    entries, so `bar = 1000 hPa` only resolves if `hPa` can be decomposed.
    """
    out = {}
    for p in ET.parse(f"{root}/udunits2-prefixes.xml").getroot().findall("prefix"):
        v = p.findtext("value")
        if v is None:
            continue
        val = Fraction(v) if "e" not in v.lower() else Fraction(float(v)).limit_denominator(10**30)
        # Values are exact powers of ten; parse them as such rather than via float.
        val = Fraction(v.replace("e", "E")) if "E" not in v.upper() else _pow10(v)
        for tag in ("symbol", "name"):
            for el in p.findall(tag):
                if el.text:
                    out[el.text] = val
        for el in p.findall("aliases/symbol") + p.findall("aliases/name"):
            if el.text:
                out[el.text] = val
    return out


def _pow10(text):
    """Exact value of a decimal-or-scientific literal, without float."""
    t = text.strip().upper()
    if "E" in t:
        mant, exp = t.split("E")
        return Fraction(mant or "1") * Fraction(10) ** int(exp)
    return Fraction(t)


def load(root="vendor/udunits-2/lib"):
    """symbol-or-name -> entry, plus every entry in file order."""
    table, entries = {}, []
    for f in FILES:
        for u in ET.parse(f"{root}/udunits2-{f}.xml").getroot().findall("unit"):
            d = u.find("def")
            entry = {
                "file": f,
                "def": d.text if d is not None else None,
                "symbols": [s.text for s in u.findall("symbol") if s.text]
                + [s.text for s in u.findall("aliases/symbol") if s.text],
                "names": [n.text for n in u.findall("name/singular") if n.text]
                + [n.text for n in u.findall("name/plural") if n.text]
                + [n.text for n in u.findall("aliases/name/singular") if n.text]
                + [n.text for n in u.findall("aliases/name/plural") if n.text],
                "dimensionless": u.find("dimensionless") is not None,
            }
            entries.append(entry)
            for key in entry["symbols"] + entry["names"]:
                table.setdefault(key, entry)
    return table, entries


TOKEN = re.compile(
    r"\s*(\d+\.?\d*(?:[eE][-+]?\d+)?|[A-Za-z_µ°Ω][A-Za-z_0-9µ°Ω]*|\^|\.|/|\(|\)|@|-|\+)"
)


def tokenize(s):
    s = s.strip()
    out, i = [], 0
    while i < len(s):
        m = TOKEN.match(s, i)
        if not m:
            raise Unresolvable(f"cannot tokenize at {s[i:]!r}")
        out.append(m.group(1))
        i = m.end()
    return out


def evaluate(text, table, memo, depth=0, prefixes=None):
    """Evaluate a definition string to a Value."""
    if depth > 24:
        raise Unresolvable("definition cycle")
    toks = tokenize(text)
    pos = [0]

    def peek():
        return toks[pos[0]] if pos[0] < len(toks) else None

    def take():
        t = peek()
        pos[0] += 1
        return t

    def primary():
        t = take()
        if t == "(":
            v = expr()
            if take() != ")":
                raise Unresolvable("unbalanced parentheses")
            return v
        if t == "-":
            return Value(-1) * primary()
        if re.fullmatch(r"\d+\.?\d*(?:[eE][-+]?\d+)?", t):
            return Value(Fraction(t))
        return lookup(t)

    def power():
        v = primary()
        while peek() == "^":
            take()
            neg = False
            if peek() == "-":
                take()
                neg = True
            e = int(take())
            v = v ** (-e if neg else e)
        return v

    def expr():
        v = power()
        while True:
            t = peek()
            if t == "." or (t is not None and t not in (")", "/", "^", "@")):
                if t == ".":
                    take()
                v = v * power()
            elif t == "/":
                take()
                v = v / power()
            else:
                return v

    def lookup(sym):
        # pi is carried exactly rather than as its stored decimal.
        if sym in ("pi", "π"):
            return Value(1, {}, 1)
        if sym in BASE:
            return Value(1, {BASE[sym]: 1})
        if sym in BASE_NAMES:
            return Value(1, {BASE_NAMES[sym]: 1})
        if sym == ANGLE:
            return Value(1, {"angle": 1})
        # UDUNITS writes a trailing integer as an exponent: `cm2` is cm^2.
        m = re.fullmatch(r"([A-Za-z_µ°Ω][A-Za-z_µ°Ω]*)(\d+)", sym)
        if m and m.group(1) in table or (m and m.group(1) in BASE):
            return lookup(m.group(1)) ** int(m.group(2))
        if sym in memo:
            return memo[sym]
        e = table.get(sym)
        if e is None and prefixes:
            # UDUNITS applies prefixes at parse time, so `hPa` is h + Pa and
            # never appears as an entry. Longest prefix first, so `da` is not
            # shadowed by `d`.
            for p in sorted(prefixes, key=len, reverse=True):
                if sym.startswith(p) and len(sym) > len(p):
                    rest = sym[len(p):]
                    try:
                        return Value(prefixes[p]) * lookup(rest)
                    except Unresolvable:
                        continue
        if e is None:
            # Re-enter lookup rather than using the entry directly, so a
            # singularized name still reaches the base-name and prefix paths:
            # `micromoles` is micro + moles is micro + mole is micro + the base.
            singular = singularize(sym)
            if singular != sym:
                return lookup(singular)
        if e is None:
            raise Unresolvable(f"unknown symbol {sym!r}")
        if e["def"] is None:
            raise Unresolvable(f"{sym!r} has no definition")
        if "@" in e["def"]:
            raise Unresolvable(f"{sym!r} is affine and cannot appear in an expression")
        v = evaluate(e["def"], table, memo, depth + 1, prefixes)
        memo[sym] = v
        return v

    v = expr()
    if peek() is not None:
        raise Unresolvable(f"trailing tokens {toks[pos[0]:]!r}")
    return v
