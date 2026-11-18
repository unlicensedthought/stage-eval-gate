# stage-eval-gate

A test that fails the build for any LLM post-stage nobody measured.

## The problem

An LLM pipeline grows stages: a pass that strips flattery, a pass that rewrites jargon, a pass that
trims. Each is one more model call, with a cost and a way to damage the answer. Teams add them on a
feeling. Nobody checks that a stage earns its tokens, and nobody notices when someone edits its prompt
after the last check.

This project makes that a build failure. A harness runs every stage over a fixture set and writes a
verdict file. A Rust test reads the file when the test binary is built and refuses:

- a registered stage with no entry in the file,
- a verdict that is not `useful`, `cost_little_gain` or `trash` (so `unevaluated` fails),
- a default-on stage judged `trash`,
- a stage whose prompt hash is missing, or differs from the hash of its current prompt.

Add a stage, or edit a prompt, and `cargo test` goes red until you re-run the evaluation.

## Quick start

You need Rust (stable) and Python 3.

```sh
scripts/verify.sh          # build, tests, evaluation reproduction, two negative proofs
python3 eval/stage_eval.py # re-run the evaluation (offline stub) and rewrite advice.json + report.md
cargo test                 # the gate
```

To see the gate fail, append a line to `stages/sycophancy.txt` and run `cargo test`. The failure says
the prompt changed since the verdict was measured. Re-run the evaluation and it passes again.

## How it works

```
stages/*.txt ──► src/lib.rs registry ──► bin stage-registry (JSON: id, prompt, sha256)
                                              │
fixtures ──► eval/stage_eval.py ──► every ordered chain ──► judge ──► advice.json
                                                                         │
                                       cargo test: src/gate.rs reads advice.json at build time
```

- **Registry.** `src/lib.rs` lists the stages. The system prompt a stage sends is the shared rules
  plus its instruction. Its sha256 is the stage's identity.
- **Harness.** `eval/stage_eval.py` takes the prompts from the registry binary, so it runs the same text
  the gate hashes. It runs each fixture through: no stages, each stage alone, every ordered pair, and
  every order of the full chain. A judge scores each output 1-5 on faithfulness, target and harm.
- **Reply guard.** A stage reply is rejected, and the previous answer kept, if it is empty, adds a
  citation marker the answer did not have, or keeps under a fifth of a long answer. Rejections are
  counted as stage errors and fail the run unless you pass `--allow-stage-errors`.
- **Verdicts**, from deltas against the no-stage run on the same fixtures:
  `useful` = target gain of 0.5 or more with no faithfulness or harm drop over 0.3;
  `cost_little_gain` = target gain under 0.5; `trash` = faithfulness or harm worse by over 0.3.
- **Ledger.** Per stage: tokens and dollars per turn. Per run: apply calls, judge tokens, stage tokens.
  Set the rates with `--rate-in` and `--rate-out` (USD per million tokens).
- **Order notes.** For each pair, which order scores higher on the composite, and chains that score
  below their better single stage while costing more.
- **Gate.** `src/gate.rs` has one pure function, `gate_failures`, and 13 tests: one against the real
  file, twelve that feed it bad JSON and check it fails. An empty registry also fails.

## Running against a real model

```sh
export STAGE_EVAL_API_KEY=...        # read from the environment, never printed or written
python3 eval/stage_eval.py --backend http \
  --llm-url https://example.invalid/v1/chat/completions \
  --stage-model <stage model> --judge-model <a different judge model> \
  --rate-in <usd per M> --rate-out <usd per M>
```

The harness refuses to run when the judge and stage model are the same model, because a model tends to
score its own style higher. `--allow-same-judge` overrides that and the report prints a warning.
Replace `eval/fixtures.json` with fixtures from your own domain before trusting a verdict.

## Limits

- **The committed `advice.json` and `report.md` come from the offline stub**, a deterministic string-rule
  stand-in for the model and the judge. They exercise the harness. They say nothing about any real model,
  and the stub is built so that all three verdicts occur.
- Eleven synthetic fixtures, one run each. Real verdicts are single-run means over a small sample; there
  is no confidence interval, no repeated run, and no significance test.
- One judge. The `http` backend does not calibrate it against human scores.
- Stage replies are scored on the fixture's seeded draft. The harness does not run a search or retrieval
  step, so it measures stages, not the whole pipeline.
- The gate checks that a measurement exists and was made against the current prompt. It does not check
  that the measurement was good, or that the model behind the stage has not changed since.
- There is no personal-information check and no deterministic scoring here, only the judge.
- Stages run in the order given; the harness does not search chain orders beyond three stages (it
  samples pairs above five).

## Tests

```sh
cargo test            # 13 gate tests, including the one that guards the real advice.json
python3 eval/test_stage_eval.py   # 15 tests: verdict thresholds, reply guard, determinism, refusals
scripts/verify.sh     # everything, plus the negative proofs; last line is VERIFY {...}
```

## License

Apache-2.0. Copyright 2026 Adam Scheurer.
