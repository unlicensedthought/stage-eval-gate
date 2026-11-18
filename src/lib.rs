//! The stage registry. Every post-stage the product can run is listed here, with the exact prompt it
//! sends. `gate` refuses to build when the evaluation (`eval/stage_eval.py`) has not judged one.

#[cfg(test)]
mod gate;

use sha2::{Digest, Sha256};

/// Rules prepended to every stage's own instruction. They outrank it.
pub const RULES: &str = include_str!("../stages/rules.txt");

#[derive(Debug, Clone, Copy)]
pub struct Stage {
    pub id: &'static str,
    /// Fixture groups the evaluation scores this stage's target metric on.
    pub targets: &'static [&'static str],
    pub instruction: &'static str,
    /// In the chain when nothing else is configured. A default-on stage may not be `trash`.
    pub default_on: bool,
}

pub const STAGES: &[Stage] = &[
    Stage {
        id: "sycophancy",
        targets: &["sycophancy_trap"],
        instruction: include_str!("../stages/sycophancy.txt"),
        default_on: true,
    },
    Stage {
        id: "plain_words",
        targets: &["jargon"],
        instruction: include_str!("../stages/plain_words.txt"),
        default_on: false,
    },
    Stage {
        id: "one_liner",
        targets: &["control"],
        instruction: include_str!("../stages/one_liner.txt"),
        default_on: false,
    },
];

/// The exact system message a stage's model call is sent.
pub fn system_prompt(stage: &Stage) -> String {
    format!("{}\n\n{}", RULES.trim_end(), stage.instruction.trim_end())
}

pub fn sha256_hex(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn prompt_hash(stage: &Stage) -> String {
    sha256_hex(&system_prompt(stage))
}
