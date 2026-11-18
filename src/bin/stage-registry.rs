//! Prints the registry as JSON for the evaluation harness, so the harness runs the same prompts the
//! gate hashes.
use serde_json::json;
use stage_eval_gate::{prompt_hash, system_prompt, STAGES};

fn main() {
    let out: Vec<_> = STAGES
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "targets": s.targets,
                "default_on": s.default_on,
                "system_prompt": system_prompt(s),
                "prompt_hash": prompt_hash(s),
            })
        })
        .collect();
    println!("{}", serde_json::to_string(&out).expect("registry is serialisable"));
}
