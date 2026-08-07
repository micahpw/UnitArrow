//! Guards for the documents this project treats as load-bearing.
//!
//! Every check here corresponds to a defect that **actually occurred**, not to
//! a hypothetical. They are `cargo test` rather than a CI-only script so they
//! fail on the machine where the mistake is made, seconds after it is made.
//!
//! | Guard | What went wrong |
//! |---|---|
//! | summary matches the document | the tally read 60 while there were 67 |
//! | no entry reads `open` over a ruling | 17 headers survived the M0 pass unchanged |
//! | `blocked_on` resolves | a dangling id turns a tracked question into an untracked one |
//! | every entry demonstrates itself | 40 entries had no concrete example |
//! | spec TOML parses | §7.2 shipped an example that was not valid TOML (AMB-027) |
//! | one spec version everywhere | four files drifted apart across three bumps |
//!
//! Zero dependencies, like the crate: the TOML and JSON readers are in-crate.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use unitarrow_core::json_lite::{self, Json};
use unitarrow_core::toml_lite;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn register() -> String {
    std::fs::read_to_string(root().join("conformance/AMBIGUITIES.md")).unwrap()
}

/// `(id, status line, body)` for every register entry.
fn entries(src: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let mut cur: Option<(String, String)> = None;
    let mut body = String::new();
    for line in src.lines() {
        if let Some(rest) = line.strip_prefix("### AMB-") {
            if let Some((id, status)) = cur.take() {
                out.push((id, status, std::mem::take(&mut body)));
            }
            let id = format!("AMB-{}", &rest[..3]);
            cur = Some((id, String::new()));
            continue;
        }
        if let Some((_, status)) = cur.as_mut() {
            if status.is_empty() && !line.trim().is_empty() {
                *status = line.to_string();
            }
            body.push_str(line);
            body.push('\n');
        }
    }
    if let Some((id, status)) = cur {
        out.push((id, status, body));
    }
    out
}

fn fixture_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root().join("conformance/fixtures")];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap().filter_map(Result::ok) {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "json") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn register_ids_are_contiguous_from_001() {
    let src = register();
    let ids: Vec<usize> = entries(&src)
        .iter()
        .map(|(id, _, _)| id[4..].parse().unwrap())
        .collect();
    for (i, n) in ids.iter().enumerate() {
        assert_eq!(
            *n,
            i + 1,
            "register ids must run AMB-001..N with no gaps; found {n} at position {}",
            i + 1
        );
    }
    assert!(
        ids.len() >= 68,
        "entries should never be removed, only resolved"
    );
}

/// The tally in the summary must match the document it summarises.
///
/// It said "60 entries total" while there were 67, because several count bumps
/// were `str::replace` calls that silently matched nothing.
#[test]
fn register_summary_matches_the_document() {
    let src = register();
    let all = entries(&src);
    let decided = all
        .iter()
        .filter(|(_, _, body)| {
            ["RESOLVED", "RULED", "DECIDED", "DEFERRED"]
                .iter()
                .any(|m| body.contains(&format!("**{m}")))
        })
        .count();

    let line = src
        .lines()
        .find(|l| l.contains("entries total"))
        .expect("the summary states an entry count");
    let stated_entries: usize = line.split_whitespace().next().unwrap().parse().unwrap();
    let stated_decided: usize = line
        .split("**")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse().ok())
        .expect("the summary states a decided count");

    assert_eq!(
        stated_entries,
        all.len(),
        "summary entry count is stale: {line}"
    );
    assert_eq!(
        stated_decided, decided,
        "summary decided count is stale: {line}"
    );
}

/// An entry that records a ruling must not still advertise itself as open.
///
/// Seventeen entries did, because the M0 editorial pass wrote resolutions into
/// bodies and never revisited the status lines. A reviewer scanning headers saw
/// decided work as unresolved.
#[test]
fn no_entry_reads_open_while_recording_a_ruling() {
    let src = register();
    let mut bad = Vec::new();
    for (id, status, body) in entries(&src) {
        let ruled = ["RESOLVED", "RULED", "DECIDED", "DEFERRED"]
            .iter()
            .any(|m| body.contains(&format!("**{m}")));
        let says_open = status.contains("· open") || status.contains("· OPEN");
        if ruled && says_open {
            bad.push(format!("{id}: {}", status.trim()));
        }
    }
    assert!(
        bad.is_empty(),
        "status lines contradict their bodies:\n  {}",
        bad.join("\n  ")
    );
}

/// Every `blocked_on` must name an entry that exists.
///
/// A dangling reference turns a tracked open question into an untracked one,
/// silently — the fixture still says "provisional" and nothing explains why.
#[test]
fn every_blocked_on_names_a_real_register_entry() {
    let src = register();
    let known: BTreeSet<String> = entries(&src).into_iter().map(|(id, _, _)| id).collect();
    let mut checked = 0;
    for path in fixture_files() {
        let doc = json_lite::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
        for case in doc.get("cases").and_then(Json::as_array).unwrap() {
            let id = case.get("id").and_then(Json::as_str).unwrap();
            let Some(bs) = case.get("blocked_on").and_then(Json::as_array) else {
                continue;
            };
            for b in bs.iter().filter_map(Json::as_str) {
                checked += 1;
                assert!(
                    known.contains(b),
                    "{id} is blocked on {b}, which is not in the register"
                );
            }
        }
    }
    println!("{checked} blocked_on references resolve");
}

/// Case ids are the suite's stable identifiers — runners report them and the
/// register cites them, so a duplicate makes a citation ambiguous forever.
#[test]
fn fixture_case_ids_are_unique_and_well_formed() {
    let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();
    for path in fixture_files() {
        let doc = json_lite::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let category = doc.get("category").and_then(Json::as_str).unwrap();
        for case in doc.get("cases").and_then(Json::as_array).unwrap() {
            let id = case.get("id").and_then(Json::as_str).unwrap().to_string();
            if let Some(prev) = seen.get(&id) {
                panic!(
                    "duplicate case id {id} in {} and {}",
                    prev.display(),
                    path.display()
                );
            }
            let parts: Vec<&str> = id.split('.').collect();
            assert_eq!(parts.len(), 3, "{id} must be <category>.<theme>.<nnn>");
            assert_eq!(
                parts[0], category,
                "{id} does not match its file's category"
            );
            assert!(
                parts[2].len() == 3 && parts[2].chars().all(|c| c.is_ascii_digit()),
                "{id} must end in three digits"
            );
            seen.insert(id, path.clone());
        }
    }
    assert!(
        seen.len() >= 100,
        "the suite should not shrink; found {}",
        seen.len()
    );
}

/// Every entry must demonstrate its problem concretely.
///
/// Forty entries once described a defect without showing one. An entry that
/// cites conformance cases counts: the docs build runs them and renders the
/// real input and result, which is a stronger example than prose.
#[test]
fn every_register_entry_demonstrates_its_problem() {
    let src = register();
    let mut case_ids = BTreeSet::new();
    for path in fixture_files() {
        let doc = json_lite::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
        for case in doc.get("cases").and_then(Json::as_array).unwrap() {
            case_ids.insert(case.get("id").and_then(Json::as_str).unwrap().to_string());
        }
    }

    let mut bare = Vec::new();
    for (id, _, body) in entries(&src) {
        let has_block = body.contains("```")
            || body.lines().any(|l| l.starts_with('|'))
            || body.lines().any(|l| l.starts_with("> "));
        let cites_case = case_ids.iter().any(|c| body.contains(c.as_str()));
        if !has_block && !cites_case {
            bare.push(id);
        }
    }
    assert!(
        bare.is_empty(),
        "these entries state a problem without showing one — add a snippet, a \
         table, or a conformance case:\n  {}",
        bare.join(", ")
    );
}

/// Both specifications' TOML examples must parse.
///
/// §7.2 once shipped an example that was not valid TOML (AMB-027). A reader
/// copying it got a parse error from the document that defines the format.
#[test]
fn every_spec_toml_block_parses() {
    let mut total = 0;
    for doc in [
        "docs/unitarrow-spec.md",
        "docs/provenance-companion-spec.md",
    ] {
        let src = std::fs::read_to_string(root().join(doc)).unwrap();
        let mut rest = src.as_str();
        while let Some(start) = rest.find("```toml\n") {
            let after = &rest[start + 8..];
            let end = after.find("```").expect("unterminated toml fence");
            let block = &after[..end];
            total += 1;
            toml_lite::parse(block)
                .unwrap_or_else(|e| panic!("{doc}: a TOML example does not parse: {e}\n{block}"));
            rest = &after[end..];
        }
    }
    assert!(
        total >= 4,
        "expected the spec to carry TOML examples; found {total}"
    );
    println!("{total} spec TOML blocks parse");
}

/// One spec version, stated in one shape, everywhere it appears.
///
/// Across three version bumps this drifted between the spec header, the crate
/// constant, and the fixture envelopes — each caught by hand, none by a test.
#[test]
fn the_spec_version_is_consistent_everywhere() {
    let spec = std::fs::read_to_string(root().join("docs/unitarrow-spec.md")).unwrap();
    let declared = spec
        .lines()
        .find_map(|l| l.strip_prefix("**Version:** "))
        .expect("the spec states a version")
        .trim()
        .to_string();

    assert_eq!(
        unitarrow_core::SPEC_VERSION,
        declared,
        "SPEC_VERSION does not match the specification header"
    );

    for path in fixture_files() {
        let doc = json_lite::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let v = doc
            .get("spec")
            .and_then(|s| s.get("version"))
            .and_then(Json::as_str)
            .unwrap();
        assert_eq!(
            v,
            declared,
            "{} pins spec {v}, but the specification is {declared}",
            path.file_name().unwrap().to_string_lossy()
        );
    }
}

/// The registry the fixtures resolve against must load, and must stay the one
/// they name.
#[test]
fn the_conformance_registry_loads_and_is_the_one_fixtures_name() {
    let src =
        std::fs::read_to_string(root().join("conformance/registry/conformance-core.toml")).unwrap();
    let r = unitarrow_core::Registry::from_toml(&src).expect("the fixture registry must load");
    for path in fixture_files() {
        let doc = json_lite::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let Some(named) = doc.get("registry").and_then(Json::as_str) else {
            continue;
        };
        let (name, version) = named.split_once('@').expect("registry is <name>@<version>");
        assert_eq!(
            name,
            r.name,
            "{} names a different registry",
            path.display()
        );
        assert_eq!(
            version,
            r.version,
            "{} pins a different registry version",
            path.display()
        );
    }
}

/// The shipping registries must load, and the factors a deployment depends on
/// must be the ones intended.
///
/// A registry is data, so nothing else type-checks it: a wrong factor is a
/// silently wrong deployment. These assertions are the review, expressed as a
/// test.
#[test]
fn the_power_systems_registry_loads_and_converts_correctly() {
    use unitarrow_core::{canonicalize, commensurable, conversion, Registry};

    let src = std::fs::read_to_string(root().join("registries/power-systems.toml")).unwrap();
    let r = Registry::from_toml(&src).expect("the power-systems registry must load");

    // Prefixes are derived, not enumerated (AMB-048). Expressed as a ratio
    // rather than a count: a bound on the authored set goes stale every time a
    // unit is legitimately added, whereas the amplification is the actual
    // invariant — it collapses toward 1 the moment someone starts listing
    // prefixed forms by hand.
    let amplification = r.unit_count() as f64 / r.authored_count() as f64;
    assert!(
        amplification > 4.0,
        "prefixed forms look enumerated rather than derived: {} authored resolve \
         only {} symbols ({amplification:.1}x)",
        r.authored_count(),
        r.unit_count()
    );

    // Making twelve bases prefixable claims a spelling per prefix. None may
    // contest an authored symbol.
    assert!(
        r.collision_risks().is_empty(),
        "unresolved prefix collisions: {:?}",
        r.collision_risks()
            .iter()
            .map(|c| &c.symbol)
            .collect::<Vec<_>>()
    );

    // The factors a power-systems deployment actually depends on.
    for (from, to, expect) in [
        ("MW", "kW", 1_000.0),
        ("kV", "V", 1_000.0),
        ("GWh", "MWh", 1_000.0),
        ("MWh", "J", 3.6e9),
        ("h", "s", 3_600.0),
    ] {
        let c = conversion(from, to, &r).unwrap();
        assert_eq!(c.apply(1.0), expect, "1 {from} should be {expect} {to}");
    }

    // 180 degrees is pi radians exactly, not a 13-digit approximation of it.
    let deg = conversion("deg", "rad", &r).unwrap();
    assert_eq!(deg.apply(180.0), std::f64::consts::PI);

    // Real, apparent and reactive power share a dimension deliberately: no
    // quantity kinds are declared, so the checker does not separate them.
    let u = |s: &str| canonicalize(s, &r, 1).unwrap();
    assert!(commensurable(&u("MW"), &u("MVAr")));
    assert!(commensurable(&u("MW"), &u("MVA")));

    // Angular frequency must NOT be commensurable with frequency: omega = 2*pi*f,
    // and a silent factor-of-1 coercion there is a 6.28x error (AMB-066).
    assert!(!commensurable(&u("rad*s^-1"), &u("Hz")));
    assert!(conversion("rad*s^-1", "Hz", &r).is_err());

    // `MWh` and `MW*h` are the same quantity and different canonical forms
    // (§1 goal 4). Worth an assertion so nobody "fixes" it later.
    assert_eq!(
        conversion("MWh", "J", &r).unwrap().apply(1.0),
        conversion("MW*h", "J", &r).unwrap().apply(1.0)
    );
    assert_ne!(u("MWh").canonical, u("MW*h").canonical);

    // The compound quantities this registry exists to express.
    for q in [
        "USD/MWh", "Btu/kWh", "t/MWh", "kg/MWh", "ohm/km", "MW/min", "MWh/yr", "MVA", "MVAr",
        "MVAR", "Mvar", "kvar", "MMBtu", "uF", "mH", "mS",
    ] {
        assert!(canonicalize(q, &r, 1).is_ok(), "{q} should resolve");
    }

    // `MBtu` MUST NOT resolve. Under SI prefixes it is 10^6 Btu; the US gas
    // industry reads M as the Roman thousand and means 10^3 — a 1000x gap with
    // nothing in the string to say which. `Btu` is therefore not prefixable,
    // and refusing the symbol is the correct answer to an ambiguous one.
    assert!(
        canonicalize("MBtu", &r, 1).is_err(),
        "MBtu must not resolve — it is ambiguous by 1000x between SI and US gas convention"
    );
    assert_eq!(conversion("MMBtu", "Btu", &r).unwrap().apply(1.0), 1e6);

    // Exactly one currency. A second at factor 1 would make USD -> EUR convert
    // at y = x, inventing an exchange rate (§2 forbids rates as registry data).
    let currencies = r
        .units()
        .filter(|un| un.dimension_name == "currency")
        .count();
    assert_eq!(
        currencies, 1,
        "more than one currency invents an exchange rate"
    );

    // Mass is authored as `g` because the SI base unit already carries a
    // prefix; `kg` must derive to exactly 1.
    assert_eq!(conversion("kg", "g", &r).unwrap().apply(1.0), 1000.0);
    assert_eq!(conversion("t", "kg", &r).unwrap().apply(1.0), 1000.0);
}
