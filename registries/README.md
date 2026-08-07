# Registries

Shipping registries, as opposed to `conformance/registry/`, which holds a
deliberately tiny fixture the golden files resolve against and which is not
meant for use.

A registry is **data** on the tzdata model (§7): versioned, hashable, and
shippable independently of any library release. Nothing here is generated —
these are authored artifacts, reviewed as data, and every conversion factor in
a deployment flows from whichever one it loads.

| Registry | Scope |
|---|---|
| `power-systems.toml` | Electric power systems: generation, interchange, bus voltages, line flows, and the temporal and angular quantities that accompany them |

## Adding one

Load-time validation is strict by design, so most mistakes surface immediately:
a factor not in lowest terms, a misspelled key, a dimension that redefines one
of the ten base dimensions, or a prefix expansion that collides with an
authored symbol. Read `docs/generated/registry.md` for the guardrails as the
implementation actually applies them, and `docs/generated/ambiguity.md` before
making a unit prefixable — that decision silently claims one spelling per
prefix.
