//! The conformance suite, run against the golden files in `conformance/`.
//!
//! Spec §11: "Every binding runs the full suite. A new binding is conformant
//! when it passes **without modification to the golden files**." So this test
//! reads `conformance/fixtures/parsing/*.json` as data and adapts to it — if a
//! fixture and this implementation disagree, the implementation is wrong.
//!
//! `provisional` cases are reported separately: their expected values encode a
//! recommended resolution to an open register entry (AMBIGUITIES.md) rather than
//! ratified spec text.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use unitarrow_core::json_lite::{self, Json};
use unitarrow_core::{canonicalize, check_unit_token, Dimension, Registry};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn load_registry(name: &str) -> Registry {
    // Fixture envelopes name their registry as `<name>@<version>`.
    let file = name.split('@').next().unwrap();
    let path = repo_root()
        .join("conformance/registry")
        .join(format!("{file}.toml"));
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    Registry::from_toml(&src)
        .unwrap_or_else(|e| panic!("{} is not a valid registry: {e}", path.display()))
}

struct Outcome {
    ok: bool,
    detail: String,
}

/// Run one case exactly as a runner should: metadata-layer escape check first
/// (§5.2), then the unit grammar and canonical form (§6).
fn run_case(case: &Json, registry: &Registry) -> Outcome {
    let input = case.get("input").expect("case has input");
    let grammar = case.get("grammar").and_then(Json::as_i64).unwrap_or(1) as u32;
    let expect = case.get("expect").expect("case has expect");
    let want_outcome = expect.get("outcome").and_then(Json::as_str).unwrap();

    // `unit_json` carries the raw wire token; `unit` carries a decoded value.
    let decoded: Result<String, unitarrow_core::Error> = match input.get("unit_json") {
        Some(tok) => {
            let raw = tok.as_str().expect("unit_json is a string");
            check_unit_token(raw).map(str::to_string)
        }
        None => Ok(input
            .get("unit")
            .and_then(Json::as_str)
            .expect("unit")
            .to_string()),
    };

    let result = decoded.and_then(|s| canonicalize(&s, registry, grammar));

    match (want_outcome, result) {
        ("error", Err(e)) => {
            let want = expect.get("code").and_then(Json::as_str).unwrap();
            if e.code_str() == want {
                Outcome {
                    ok: true,
                    detail: e.code_str().to_string(),
                }
            } else {
                Outcome {
                    ok: false,
                    detail: format!("expected {want}, got {} ({})", e.code_str(), e.message()),
                }
            }
        }
        ("error", Ok(u)) => Outcome {
            ok: false,
            detail: format!(
                "expected error {}, but parsed successfully as {:?}",
                expect.get("code").and_then(Json::as_str).unwrap_or("?"),
                u.canonical
            ),
        },
        ("ok", Err(e)) => Outcome {
            ok: false,
            detail: format!("expected success, got {e}"),
        },
        ("ok", Ok(u)) => {
            let want_canonical = expect.get("canonical").and_then(Json::as_str).unwrap();
            let want_dim: BTreeMap<String, i64> = expect
                .get("dimension")
                .and_then(Json::as_object)
                .map(|o| {
                    o.iter()
                        .filter_map(|(k, v)| v.as_i64().map(|n| (k.clone(), n)))
                        .collect()
                })
                .unwrap_or_default();
            let got_dim: BTreeMap<String, i64> = u
                .dimension
                .sparse()
                .into_iter()
                .map(|(k, v)| (k.to_string(), v as i64))
                .collect();

            let mut problems = Vec::new();
            if u.canonical != want_canonical {
                problems.push(format!(
                    "canonical: expected {want_canonical:?}, got {:?}",
                    u.canonical
                ));
            }
            if got_dim != want_dim {
                problems.push(format!("dimension: expected {want_dim:?}, got {got_dim:?}"));
            }
            if problems.is_empty() {
                Outcome {
                    ok: true,
                    detail: u.canonical.clone(),
                }
            } else {
                Outcome {
                    ok: false,
                    detail: problems.join("; "),
                }
            }
        }
        (other, _) => Outcome {
            ok: false,
            detail: format!("unknown expected outcome {other:?}"),
        },
    }
}

struct Tally {
    normative_pass: usize,
    normative_fail: Vec<String>,
    provisional_pass: usize,
    provisional_fail: Vec<String>,
}

fn run_all() -> Tally {
    let dir = repo_root().join("conformance/fixtures/parsing");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no fixtures found in {}", dir.display());

    let mut tally = Tally {
        normative_pass: 0,
        normative_fail: Vec::new(),
        provisional_pass: 0,
        provisional_fail: Vec::new(),
    };

    for path in files {
        let src = std::fs::read_to_string(&path).unwrap();
        let doc = json_lite::parse(&src).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let registry_name = doc
            .get("registry")
            .and_then(Json::as_str)
            .expect("envelope names a registry");
        let registry = load_registry(registry_name);

        for case in doc.get("cases").and_then(Json::as_array).expect("cases") {
            let id = case.get("id").and_then(Json::as_str).unwrap();
            let status = case.get("status").and_then(Json::as_str).unwrap();
            let outcome = run_case(case, &registry);
            let line = format!("{id}: {}", outcome.detail);
            match (status, outcome.ok) {
                ("normative", true) => tally.normative_pass += 1,
                ("normative", false) => tally.normative_fail.push(line),
                (_, true) => tally.provisional_pass += 1,
                (_, false) => tally.provisional_fail.push(line),
            }
        }
    }
    tally
}

/// The load-bearing test: every `normative` case must pass, because those
/// encode ratified spec text.
#[test]
fn normative_cases_all_pass() {
    let t = run_all();
    let total = t.normative_pass + t.normative_fail.len();
    if !t.normative_fail.is_empty() {
        panic!(
            "{}/{} normative cases failed:\n  {}",
            t.normative_fail.len(),
            total,
            t.normative_fail.join("\n  ")
        );
    }
    assert!(
        t.normative_pass >= 60,
        "expected the full normative set, got {}",
        t.normative_pass
    );
    println!("{}/{} normative cases pass", t.normative_pass, total);
}

/// Provisional cases encode recommended resolutions to open register entries.
/// They are expected to pass too — this implementation follows those
/// recommendations — but a failure here is a spec question, not a bug, so it is
/// reported separately.
#[test]
fn provisional_cases_match_the_recommended_resolutions() {
    let t = run_all();
    let total = t.provisional_pass + t.provisional_fail.len();
    if !t.provisional_fail.is_empty() {
        panic!(
            "{}/{} provisional cases diverge from their recommended resolution:\n  {}",
            t.provisional_fail.len(),
            total,
            t.provisional_fail.join("\n  ")
        );
    }
    println!("{}/{} provisional cases match", t.provisional_pass, total);
}

/// Canonical form is the equality primitive, so `canon(canon(x)) == canon(x)`
/// must hold for every successful case in the suite — checked here rather than
/// assumed, because a non-idempotent serializer breaks cross-language hashing in
/// a way individual cases would not reveal.
#[test]
fn canonical_form_is_idempotent_across_the_suite() {
    let dir = repo_root().join("conformance/fixtures/parsing");
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|x| x != "json") {
            continue;
        }
        let src = std::fs::read_to_string(&path).unwrap();
        let doc = json_lite::parse(&src).unwrap();
        let registry = load_registry(doc.get("registry").and_then(Json::as_str).unwrap());
        for case in doc.get("cases").and_then(Json::as_array).unwrap() {
            let expect = case.get("expect").unwrap();
            if expect.get("outcome").and_then(Json::as_str) != Some("ok") {
                continue;
            }
            let canonical = expect.get("canonical").and_then(Json::as_str).unwrap();
            let again = canonicalize(canonical, &registry, 1)
                .unwrap_or_else(|e| panic!("canonical form {canonical:?} does not re-parse: {e}"));
            assert_eq!(
                again.canonical, canonical,
                "canon({canonical:?}) = {:?} — not idempotent",
                again.canonical
            );
            checked += 1;
        }
    }
    assert!(
        checked > 40,
        "expected to check most of the suite, got {checked}"
    );
    println!("{checked} canonical strings are idempotent");
}

/// Every dimension name a fixture asserts must be one of the nine frozen base
/// dimensions (§6.2) — a guard against a fixture drifting to a derived name.
#[test]
fn fixtures_only_assert_base_dimensions() {
    let dir = repo_root().join("conformance/fixtures/parsing");
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|x| x != "json") {
            continue;
        }
        let doc = json_lite::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
        for case in doc.get("cases").and_then(Json::as_array).unwrap() {
            if let Some(d) = case
                .get("expect")
                .and_then(|e| e.get("dimension"))
                .and_then(Json::as_object)
            {
                for key in d.keys() {
                    assert!(
                        Dimension::index_of(key).is_some(),
                        "{}: {key:?} is not a base dimension",
                        case.get("id").and_then(Json::as_str).unwrap()
                    );
                }
            }
        }
    }
}
