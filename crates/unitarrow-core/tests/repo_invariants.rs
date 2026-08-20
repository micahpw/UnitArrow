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

/// The domain registry must compose with core, and the result must resolve
/// everything the domain actually writes.
///
/// power-systems is no longer standalone — it adds what core lacks and nothing
/// else. The assertions that matter are therefore about the *composition*: that
/// it needs no rulings, and that the effective registry covers the vocabulary.
#[test]
fn the_power_systems_extension_composes_with_core() {
    use unitarrow_core::{canonicalize, commensurable, compose, contested, conversion, Source};

    let core = Source::new(
        "core",
        "2026-08-07",
        &std::fs::read_to_string(root().join("registries/core.toml")).unwrap(),
    );
    let ps = Source::new(
        "power-systems",
        "0.2.0",
        &std::fs::read_to_string(root().join("registries/power-systems.toml")).unwrap(),
    );

    // A domain registry that restates core would force a ruling on every shared
    // symbol. Adding only what is missing keeps composition free.
    assert!(
        contested(&[core.clone(), ps.clone()]).unwrap().is_empty(),
        "the domain registry should restate nothing core already defines"
    );

    let composed = compose("effective", "test", &[core, ps], &[]).expect("composes");
    let r = composed.registry().unwrap();

    for q in [
        "MW", "GW", "MVA", "MVAr", "MVAR", "Mvar", "kV", "kA", "MWh", "GWh", "MVArh", "Hz", "ohm",
        "pu", "degC", "deg", "rad", "USD/MWh", "Btu/kWh", "MMBtu", "t/MWh", "ohm/km", "MW/min",
        "S", "F", "H", "therm", "mi",
    ] {
        assert!(
            canonicalize(q, &r, 1).is_ok(),
            "{q} should resolve in the effective registry"
        );
    }

    // The therm comparison caught a real error: the hand-written value here was
    // the EC therm labelled as the US one. Core's is correct, and this pins it.
    assert_eq!(
        conversion("therm", "J", &r).unwrap().apply(1.0),
        105_480_400.0
    );

    // AMB-066 must survive composition too.
    let u = |s: &str| canonicalize(s, &r, 1).unwrap();
    assert!(!commensurable(&u("rad*s^-1"), &u("Hz")));
    assert_eq!(
        conversion("deg", "rad", &r).unwrap().apply(180.0),
        std::f64::consts::PI
    );

    // Real, apparent and reactive power share a dimension deliberately.
    assert!(commensurable(&u("MW"), &u("MVAr")));
    assert!(commensurable(&u("MW"), &u("MVA")));
}

/// The generated core registry must load, and its factors must be right.
///
/// It is generated, so a regeneration after an upstream change could alter any
/// factor silently. These assertions are the review of the generated artifact,
/// expressed as a test — a wrong factor is a silently wrong deployment.
#[test]
fn the_core_registry_loads_and_converts_correctly() {
    use unitarrow_core::{canonicalize, commensurable, conversion, Registry};

    let src = std::fs::read_to_string(root().join("registries/core.toml")).unwrap();
    let r = Registry::from_toml(&src).expect("the generated registry must load");

    assert!(r.authored_count() > 200, "the import should be substantial");
    assert!(
        r.collision_risks().is_empty(),
        "unresolved prefix collisions: {:?}",
        r.collision_risks()
            .iter()
            .map(|c| &c.symbol)
            .collect::<Vec<_>>()
    );

    for (from, to, expect) in [
        ("Btu", "J", 1_055.055_852_62),
        ("mi", "m", 1609.344),
        ("bar", "Pa", 100_000.0),
        ("lb", "kg", 0.453_592_37),
        ("h", "s", 3600.0),
        ("L", "m^3", 0.001),
        // `kg` is not authored: it derives from the prefixable `g`, because the
        // SI base unit of mass already carries a prefix.
        ("kg", "g", 1000.0),
        ("km", "m", 1000.0),
        ("MW", "W", 1_000_000.0),
    ] {
        let c =
            conversion(from, to, &r).unwrap_or_else(|e| panic!("{from} -> {to}: {}", e.message()));
        assert!(
            (c.apply(1.0) - expect).abs() < expect.abs() * 1e-12,
            "1 {from} should be {expect} {to}, got {}",
            c.apply(1.0)
        );
    }

    // Affine units survive the import with their intercepts.
    assert!((conversion("degF", "K", &r).unwrap().apply(0.0) - 255.372_222_222_222).abs() < 1e-9);
    assert!((conversion("degree_Celsius", "K", &r).unwrap().apply(100.0) - 373.15).abs() < 1e-9);

    // The two deliberate departures from upstream (AMB-066). UDUNITS makes the
    // radian dimensionless; if a regeneration ever inherits that, `rad/s` and
    // `Hz` become commensurable and a 2*pi error stops being detectable.
    let u = |s: &str| canonicalize(s, &r, 1).unwrap();
    assert_eq!(u("rad").dimension.sparse(), vec![("angle", 1)]);
    assert!(!commensurable(&u("rad*s^-1"), &u("Hz")));

    // And pi is carried as an exponent, not as the truncated decimal upstream
    // stores: 180 degrees must be pi exactly.
    let deg = conversion("arc_degree", "rad", &r).unwrap();
    assert_eq!(deg.apply(180.0), std::f64::consts::PI);
}
