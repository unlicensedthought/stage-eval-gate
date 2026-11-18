//! The gate: a stage the evaluation has not judged cannot ship.
//!
//! `eval/stage_eval.py` runs every stage over the fixtures and writes `advice.json`. This file reads
//! that advice when the test binary is built and fails when:
//!
//! * a registered stage has no entry in the advice,
//! * an entry's verdict is anything other than `useful`, `cost_little_gain` or `trash` (the
//!   placeholder `unevaluated` included: a stage nobody measured has not passed),
//! * a stage that is ON BY DEFAULT has the verdict `trash`,
//! * the entry carries no prompt hash, or one that is not the hash of the stage's CURRENT prompt. The
//!   evaluator writes `prompt_hash` when it measured the verdict against that prompt. An entry with
//!   only `prompt_hash_unverified` passes while the hash matches, and says so on stderr.
//!
//! So adding a stage to the registry turns `cargo test` red until the evaluation has been run with it.
//!
//! The decision is one pure function, [`gate_failures`], so it can be shown to fail on JSON written
//! here. The real-file test is the one that guards; the others prove it can fail.

use serde_json::Value;

use crate::{prompt_hash, Stage, STAGES};

/// The advice, read when the test binary is built: a stale or missing evaluation is a build-time
/// fact, not something a deploy discovers.
const ADVICE: &str = include_str!("../advice.json");

const VERDICTS: &[&str] = &["useful", "cost_little_gain", "trash"];

/// One registered stage as the gate sees it.
struct Reg {
    id: &'static str,
    on_by_default: bool,
    prompt_hash: String,
}

fn registry() -> Vec<Reg> {
    STAGES
        .iter()
        .map(|s| Reg {
            id: s.id,
            on_by_default: s.default_on,
            prompt_hash: prompt_hash(s),
        })
        .collect()
}

/// What the gate found: `failures` refuse the build, `warnings` are printed and let it pass.
#[derive(Debug, Default, PartialEq)]
struct Report {
    failures: Vec<String>,
    warnings: Vec<String>,
}

/// Everything wrong with `advice` for this registry. No failures means the gate passes.
///
/// An empty registry is itself a failure: a gate walking nothing would pass for the wrong reason.
fn gate_failures(advice: &str, registry: &[Reg]) -> Report {
    let mut out = Report::default();
    if registry.is_empty() {
        out.failures
            .push("the stage registry is empty, so the gate is checking nothing".into());
        return out;
    }
    let doc: Value = match serde_json::from_str(advice) {
        Ok(v) => v,
        Err(e) => {
            out.failures
                .push(format!("advice.json is not valid JSON: {e}"));
            return out;
        }
    };
    let Some(stages) = doc.get("stages").and_then(Value::as_object) else {
        out.failures
            .push("advice.json has no `stages` object".into());
        return out;
    };
    for Reg {
        id,
        on_by_default,
        prompt_hash,
    } in registry
    {
        let Some(entry) = stages.get(*id) else {
            out.failures.push(format!(
                "stage `{id}` is registered but has no entry in advice.json; run \
                 `python3 eval/stage_eval.py`"
            ));
            continue;
        };
        match entry.get("verdict").and_then(Value::as_str) {
            Some(v) if VERDICTS.contains(&v) => {
                if *on_by_default && v == "trash" {
                    out.failures.push(format!(
                        "stage `{id}` is on by default but the evaluation calls it `trash`: turn it \
                         off by default or fix it ({})",
                        entry.get("reason").and_then(Value::as_str).unwrap_or("no reason given")
                    ));
                }
            }
            other => out.failures.push(format!(
                "stage `{id}` has verdict {other:?}, not one of {VERDICTS:?}; run \
                 `python3 eval/stage_eval.py`"
            )),
        }
        let measured = entry.get("prompt_hash").and_then(Value::as_str);
        let unverified = entry.get("prompt_hash_unverified").and_then(Value::as_str);
        match (measured, unverified) {
            (Some(h), _) if h == prompt_hash => {}
            (Some(_), _) => out.failures.push(format!(
                "stage `{id}`: its prompt changed since the verdict was measured; re-run \
                 `python3 eval/stage_eval.py`"
            )),
            (None, Some(h)) if h == prompt_hash => out.warnings.push(format!(
                "stage `{id}`: the verdict was NOT measured against this prompt (prompt_hash_unverified \
                 only); re-run `python3 eval/stage_eval.py`"
            )),
            (None, Some(_)) => out.failures.push(format!(
                "stage `{id}`: its prompt changed since the hash on record; re-run \
                 `python3 eval/stage_eval.py`"
            )),
            (None, None) => out.failures.push(format!(
                "stage `{id}` has no prompt_hash in advice.json; run `python3 eval/stage_eval.py`"
            )),
        }
    }
    out
}

/// THE GATE: the real registry against the real advice file.
#[test]
fn every_registered_stage_has_an_evaluated_verdict_and_no_default_stage_is_trash() {
    let report = gate_failures(ADVICE, &registry());
    for w in &report.warnings {
        eprintln!("gate warning: {w}");
    }
    assert!(
        report.failures.is_empty(),
        "the gate refused advice.json:\n  - {}",
        report.failures.join("\n  - ")
    );
}

// ---- negative proof: the same function, on JSON written here ----

fn reg(items: &[(&'static str, bool, &str)]) -> Vec<Reg> {
    items
        .iter()
        .map(|(id, on, h)| Reg {
            id,
            on_by_default: *on,
            prompt_hash: (*h).into(),
        })
        .collect()
}

fn r2() -> Vec<Reg> {
    reg(&[("sycophancy", true, "h1"), ("plain_words", false, "h2")])
}

fn fails(advice: &str) -> Vec<String> {
    gate_failures(advice, &r2()).failures
}

#[test]
fn good_advice_passes() {
    let good = r#"{"stages":{"sycophancy":{"verdict":"useful","prompt_hash":"h1"},"plain_words":{"verdict":"trash","prompt_hash":"h2"},"stale":{"verdict":"x"}}}"#;
    // `plain_words` is trash but OFF by default, which is allowed; an entry for an unregistered id is ignored.
    assert_eq!(gate_failures(good, &r2()), Report::default());
    let little = r#"{"stages":{"sycophancy":{"verdict":"cost_little_gain","prompt_hash":"h1"},"plain_words":{"verdict":"useful","prompt_hash":"h2"}}}"#;
    assert_eq!(fails(little), Vec::<String>::new());
}

#[test]
fn an_unevaluated_stage_fails() {
    let stub = r#"{"stages":{"sycophancy":{"verdict":"unevaluated","prompt_hash":"h1"},"plain_words":{"verdict":"useful","prompt_hash":"h2"}}}"#;
    let f = fails(stub);
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(
        f[0].contains("`sycophancy`") && f[0].contains("unevaluated"),
        "{f:?}"
    );
}

#[test]
fn a_missing_entry_fails() {
    let f = fails(r#"{"stages":{"sycophancy":{"verdict":"useful","prompt_hash":"h1"}}}"#);
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(
        f[0].contains("`plain_words`") && f[0].contains("no entry"),
        "{f:?}"
    );
}

#[test]
fn a_default_on_trash_stage_fails() {
    let f = fails(
        r#"{"stages":{"sycophancy":{"verdict":"trash","reason":"mangles text","prompt_hash":"h1"},"plain_words":{"verdict":"useful","prompt_hash":"h2"}}}"#,
    );
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(
        f[0].contains("on by default") && f[0].contains("mangles text"),
        "{f:?}"
    );
}

#[test]
fn a_missing_verdict_or_a_non_string_verdict_fails() {
    let f = fails(
        r#"{"stages":{"sycophancy":{"reason":"forgot","prompt_hash":"h1"},"plain_words":{"verdict":3,"prompt_hash":"h2"}}}"#,
    );
    assert_eq!(f.len(), 2, "{f:?}");
}

#[test]
fn malformed_or_shapeless_advice_fails() {
    assert_eq!(fails("{not json").len(), 1);
    assert_eq!(fails(r#"{"stages":[]}"#).len(), 1);
    assert_eq!(fails("null").len(), 1);
}

#[test]
fn an_empty_registry_fails() {
    assert_eq!(gate_failures(r#"{"stages":{}}"#, &[]).failures.len(), 1);
}

#[test]
fn a_changed_prompt_after_the_measurement_fails() {
    let f = fails(
        r#"{"stages":{"sycophancy":{"verdict":"useful","prompt_hash":"OLD"},"plain_words":{"verdict":"useful","prompt_hash":"h2"}}}"#,
    );
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(
        f[0].contains("`sycophancy`") && f[0].contains("prompt changed"),
        "{f:?}"
    );
}

#[test]
fn a_verdict_with_no_hash_at_all_fails() {
    let f = fails(
        r#"{"stages":{"sycophancy":{"verdict":"useful"},"plain_words":{"verdict":"useful","prompt_hash":"h2"}}}"#,
    );
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(f[0].contains("no prompt_hash"), "{f:?}");
}

#[test]
fn an_unverified_hash_passes_but_warns_and_a_stale_one_fails() {
    let on_record = r#"{"stages":{"sycophancy":{"verdict":"useful","prompt_hash_unverified":"h1"},"plain_words":{"verdict":"useful","prompt_hash":"h2"}}}"#;
    let r = gate_failures(on_record, &r2());
    assert!(r.failures.is_empty(), "{r:?}");
    assert_eq!(r.warnings.len(), 1, "{r:?}");
    assert!(r.warnings[0].contains("NOT measured"), "{r:?}");
    let stale = r#"{"stages":{"sycophancy":{"verdict":"useful","prompt_hash_unverified":"OLD"},"plain_words":{"verdict":"useful","prompt_hash":"h2"}}}"#;
    let r = gate_failures(stale, &r2());
    assert_eq!(r.failures.len(), 1, "{r:?}");
}

/// A placeholder file with every verdict `unevaluated` must fail, so nothing passes until a real
/// evaluation replaces it. Each stage fails twice: once for the verdict, once for the missing hash.
#[test]
fn the_placeholder_shape_fails_the_gate() {
    let placeholder = r#"{"stages":{"sycophancy":{"verdict":"unevaluated"},
        "plain_words":{"verdict":"unevaluated"},"one_liner":{"verdict":"unevaluated"}}}"#;
    let reg = reg(&[("sycophancy", true, "a"), ("plain_words", false, "b"), ("one_liner", false, "c")]);
    assert_eq!(gate_failures(placeholder, &reg).failures.len(), 6);
}

/// The hash must follow the prompt text the model is sent, or the gate watches nothing.
#[test]
fn the_current_hash_changes_when_the_rules_or_instruction_change() {
    let base = &STAGES[0];
    let h = prompt_hash(base);
    assert_eq!(h.len(), 64);
    let altered = Stage {
        instruction: "something else entirely",
        ..*base
    };
    assert_ne!(prompt_hash(&altered), h);
}
