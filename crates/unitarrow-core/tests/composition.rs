//! Runs the `conformance/fixtures/composition/*.json` golden files (spec §7.5,
//! §5.6).
//!
//! Same contract as the parsing suite: the fixtures are **data**, this file
//! adapts to them, and a fixture is never edited to make the implementation
//! pass. Provisional cases encode a recommended resolution to an open register
//! entry and are expected to fail until it is ruled — that failure is the
//! point, so it is reported rather than hidden.

use std::path::PathBuf;

use unitarrow_core::boundary::{check_boundary, unverifiable, Boundary};
use unitarrow_core::compose::{compose, verify_pin, Embedding, Resolution, Source};
use unitarrow_core::json_lite::{self, Json};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn sources_of(input: &Json, key: &str) -> Vec<Source> {
    let Some(items) = input.get(key).and_then(Json::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .map(|s| {
            Source::new(
                s.get("name").and_then(Json::as_str).expect("source.name"),
                s.get("version").and_then(Json::as_str).expect("source.version"),
                s.get("toml").and_then(Json::as_str).expect("source.toml"),
            )
        })
        .collect()
}

struct Outcome {
    ok: bool,
    detail: String,
}

fn pass() -> Outcome {
    Outcome { ok: true, detail: "ok".into() }
}
fn fail(d: impl Into<String>) -> Outcome {
    Outcome { ok: false, detail: d.into() }
}

fn run_case(case: &Json) -> Outcome {
    let input = case.get("input").expect("input");
    let expect = case.get("expect").expect("expect");
    let sources = sources_of(input, "sources");
    let rulings: Vec<Resolution> = input
        .get("rulings")
        .and_then(Json::as_array)
        .map(|rs| {
            rs.iter()
                .map(|r| {
                    Resolution::new(
                        r.get("symbol").and_then(Json::as_str).unwrap_or(""),
                        r.get("from").and_then(Json::as_str).unwrap_or(""),
                        r.get("reason").and_then(Json::as_str).unwrap_or(""),
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    // AMB-059 boundary cases: a table tagged against one registry read against
    // another. Modelled on `check_boundary`, which is what §8.3 now requires.
    if let Some(local) = input.get("local") {
        let load = |v: &Json| {
            unitarrow_core::Registry::from_toml(v.get("toml").and_then(Json::as_str).unwrap())
                .expect("side loads")
        };
        let lpin = input.get("local_pin").and_then(Json::as_str).unwrap_or("");
        let fpin = input.get("foreign_pin").and_then(Json::as_str).unwrap_or("");
        let syms: Vec<String> = input
            .get("symbols")
            .and_then(Json::as_array)
            .map(|a| a.iter().filter_map(Json::as_str).map(str::to_string).collect())
            .unwrap_or_default();
        let refs: Vec<&str> = syms.iter().map(String::as_str).collect();

        let verdict = if input.get("foreign_unavailable").is_some() {
            unverifiable(lpin, fpin)
        } else {
            let foreign = load(input.get("foreign").expect("foreign side"));
            check_boundary(&load(local), lpin, &foreign, fpin, &refs)
        };

        let wants_error = expect.get("outcome").and_then(Json::as_str) == Some("error");
        if wants_error {
            let Some(code) = verdict.strict_code() else {
                return fail(format!("expected an error, boundary was {}", verdict.message()));
            };
            if code.as_str() != expect.get("code").and_then(Json::as_str).unwrap_or("") {
                return fail(format!(
                    "expected {}, got {}",
                    expect.get("code").and_then(Json::as_str).unwrap_or("?"),
                    code.as_str()
                ));
            }
            if let Some(names) = expect.get("names").and_then(Json::as_array) {
                let m = verdict.message();
                for n in names.iter().filter_map(Json::as_str) {
                    if !m.contains(n) {
                        return fail(format!("diagnostic never mentions {n:?}: {m}"));
                    }
                }
            }
            return pass();
        }
        if !verdict.is_safe() {
            return fail(format!("expected safe, got {}", verdict.message()));
        }
        if let Some(want) = expect.get("boundary").and_then(Json::as_str) {
            let got = match &verdict {
                Boundary::SamePin => "same_pin",
                Boundary::Compatible { .. } => "compatible",
                _ => "other",
            };
            if got != want {
                return fail(format!("boundary {got} != expected {want}"));
            }
        }
        if let Some(n) = expect.get("checked").and_then(Json::as_i64) {
            if !matches!(&verdict, Boundary::Compatible { checked } if *checked as i64 == n) {
                return fail(format!("checked count mismatch: {verdict:?}"));
            }
        }
        return pass();
    }

    let composed = compose("merged", "1", &sources, &rulings);

    // `verify` cases assert the §5.6 rule that a fetched registry not matching
    // the pin MUST be refused rather than used.
    if let Some(kind) = input.get("verify").and_then(Json::as_str) {
        let Ok(c) = composed else {
            return fail("verify case needs a composable input");
        };
        let fetched = match kind {
            "tampered" => c.toml.replace("oil barrel", "oil barrel "),
            "exact" => c.toml.clone(),
            other => return fail(format!("unknown verify kind {other:?}")),
        };
        let accepted = verify_pin(&fetched, &c.pin);
        let wants_error = expect.get("outcome").and_then(Json::as_str) == Some("error");
        return if wants_error != accepted {
            pass()
        } else {
            fail(format!("verify {kind:?}: accepted={accepted}, expected_error={wants_error}"))
        };
    }
    let want = expect.get("outcome").and_then(Json::as_str).unwrap_or("");

    match (want, composed) {
        ("error", Ok(_)) => fail("expected an error, composition succeeded"),
        ("error", Err(e)) => {
            if e.code_str() != expect.get("code").and_then(Json::as_str).unwrap_or("") {
                return fail(format!(
                    "expected {}, got {}",
                    expect.get("code").and_then(Json::as_str).unwrap_or("?"),
                    e.code_str()
                ));
            }
            // A content requirement on the diagnostic, never a wording one:
            // AMB-058's whole complaint was an error that failed to name the
            // contested symbol.
            if let Some(names) = expect.get("names").and_then(Json::as_array) {
                let m = e.message();
                for n in names.iter().filter_map(Json::as_str) {
                    if !m.contains(n) {
                        return fail(format!("diagnostic never mentions {n:?}: {m}"));
                    }
                }
            }
            pass()
        }
        ("ok", Err(e)) => fail(format!("expected success, got {}: {}", e.code_str(), e.message())),
        ("ok", Ok(c)) => {
            if let Some(p) = expect.get("pin").and_then(Json::as_str) {
                if c.pin != p {
                    return fail(format!("pin {} != expected {p}", c.pin));
                }
            }
            for (key, actual) in [
                ("applied", c.applied.iter().map(|a| a.symbol.clone()).collect::<Vec<_>>()),
                ("agreed", c.agreed.clone()),
                ("composed_from", c.inputs.iter().map(|(n, _, _)| n.clone()).collect()),
            ] {
                if let Some(want) = expect.get(key).and_then(Json::as_array) {
                    let want: Vec<String> =
                        want.iter().filter_map(Json::as_str).map(str::to_string).collect();
                    if want != actual {
                        return fail(format!("{key}: expected {want:?}, got {actual:?}"));
                    }
                }
            }
            if let Some(resolves) = expect.get("resolves").and_then(Json::as_object) {
                let r = match c.registry() {
                    Ok(r) => r,
                    Err(e) => return fail(format!("effective registry does not load: {}", e.message())),
                };
                for (symbol, want) in resolves {
                    let got = r.resolve(symbol);
                    if let Some(Json::Bool(b)) = want.get("resolvable") {
                        if got.is_some() != *b {
                            return fail(format!(
                                "{symbol}: resolvable expected {b}, got {}",
                                got.is_some()
                            ));
                        }
                        if !*b {
                            continue;
                        }
                    }
                    let Some(u) = got else {
                        return fail(format!("{symbol} does not resolve"));
                    };
                    if let Some(d) = want.get("dimension_name").and_then(Json::as_str) {
                        if u.dimension_name != d {
                            return fail(format!("{symbol}: dimension {} != {d}", u.dimension_name));
                        }
                    }
                    if let Some(f) = want.get("factor").and_then(Json::as_array) {
                        let (n, d) = (
                            f[0].as_i64().unwrap() as i128,
                            f[1].as_i64().unwrap() as i128,
                        );
                        if (u.factor.ratio().numerator(), u.factor.ratio().denominator()) != (n, d) {
                            return fail(format!("{symbol}: factor {} != {n}/{d}", u.factor));
                        }
                    }
                    if let Some(desc) = want.get("describes").and_then(Json::as_str) {
                        if r.describe(symbol).as_deref() != Some(desc) {
                            return fail(format!(
                                "{symbol}: describes {:?} != {desc:?}",
                                r.describe(symbol)
                            ));
                        }
                    }
                }
            }
            pass()
        }
        (other, _) => fail(format!("unknown expected outcome {other:?}")),
    }
}

fn run_all() -> (usize, Vec<String>, usize, Vec<String>) {
    let dir = repo_root().join("conformance/fixtures/composition");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no fixtures in {}", dir.display());

    let (mut np, mut pp) = (0usize, 0usize);
    let (mut nf, mut pf) = (Vec::new(), Vec::new());
    for path in files {
        let src = std::fs::read_to_string(&path).unwrap();
        let doc = json_lite::parse(&src).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        for case in doc.get("cases").and_then(Json::as_array).expect("cases") {
            let id = case.get("id").and_then(Json::as_str).unwrap();
            let status = case.get("status").and_then(Json::as_str).unwrap();
            let o = run_case(case);
            let line = format!("{id}: {}", o.detail);
            match (status, o.ok) {
                ("normative", true) => np += 1,
                ("normative", false) => nf.push(line),
                (_, true) => pp += 1,
                (_, false) => pf.push(line),
            }
        }
    }
    (np, nf, pp, pf)
}

#[test]
fn normative_composition_cases_all_pass() {
    let (np, nf, _, _) = run_all();
    assert!(nf.is_empty(), "{}/{} failed:\n  {}", nf.len(), np + nf.len(), nf.join("\n  "));
    assert!(np >= 15, "expected real coverage, got {np}");
    println!("{np} normative composition cases pass");
}

/// Every provisional case must still be blocked on a genuinely open entry.
///
/// AMB-059 was resolved and its cases promoted to normative, so this suite has
/// none left. The assertion is inverted from the usual shape on purpose: if a
/// provisional case reappears it should be deliberate, and if one starts
/// passing it should be promoted rather than left looking open.
#[test]
fn provisional_cases_are_accounted_for() {
    let (np, _, pp, pf) = run_all();
    println!("{np} normative, {pp} provisional passing, {} provisional failing", pf.len());
    assert_eq!(pp + pf.len(), 0, "unexpected provisional cases: {pf:?}");
}

/// Every fixture pin must be reachable, and every `blocked_on` must name a real
/// register entry. A dangling reference silently turns a tracked question into
/// an untracked one.
#[test]
fn fixture_references_resolve() {
    let register =
        std::fs::read_to_string(repo_root().join("conformance/AMBIGUITIES.md")).unwrap();
    let dir = repo_root().join("conformance/fixtures/composition");
    for path in std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok().map(|e| e.path())) {
        if path.extension().is_none_or(|x| x != "json") {
            continue;
        }
        let doc = json_lite::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
        for case in doc.get("cases").and_then(Json::as_array).unwrap() {
            let id = case.get("id").and_then(Json::as_str).unwrap();
            if let Some(bs) = case.get("blocked_on").and_then(Json::as_array) {
                for b in bs.iter().filter_map(Json::as_str) {
                    assert!(
                        register.contains(&format!("### {b}")),
                        "{id} is blocked on {b}, which is not in the register"
                    );
                }
            }
        }
    }
}

/// The §5.6 embedding modes, asserted directly rather than through a fixture:
/// they are about the provenance block's shape, not composition's output.
#[test]
fn embedding_modes_trade_availability_not_integrity() {
    let toml = "[registry]\nschema_version = 1\nname = \"oil\"\nversion = \"1.4\"\n\
                [unit.bbl]\ndimension = \"length\"\nfactor = [9938205933, 62500000000]\n\
                display = { long = \"oil barrel\" }\n";
    let c = compose("merged", "1", &[Source::new("oil", "1.4", toml)], &[]).unwrap();

    for mode in [Embedding::Full, Embedding::Pin] {
        assert!(c.provenance_json(mode).contains(&c.pin), "{mode:?} carries the hash");
    }
    assert!(c.provenance_json(Embedding::Full).contains("\\\"oil barrel\\\""));
    assert!(!c.provenance_json(Embedding::Pin).contains("oil barrel"));
    assert!(verify_pin(&c.toml, &c.pin));
    assert!(!verify_pin(&c.toml.replace("oil barrel", "oil barrel "), &c.pin));
}
