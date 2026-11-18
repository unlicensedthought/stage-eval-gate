#!/usr/bin/env bash
# Install/build/test gates. The last stdout line is a single VERIFY JSON verdict.
set -u
cd "$(dirname "$0")/.."
gates=0
fail() { echo "VERIFY {\"verdict\":\"fail\",\"failed\":\"$1\"}"; exit 1; }
step() { # name, command...
  local name=$1; shift
  echo "== $name"
  "$@" >"/tmp/verify.$$.log" 2>&1 || { tail -40 "/tmp/verify.$$.log" >&2; rm -f "/tmp/verify.$$.log"; fail "$name"; }
  rm -f "/tmp/verify.$$.log"
  gates=$((gates + 1))
}

command -v cargo >/dev/null || fail "install: cargo not found"
command -v python3 >/dev/null || fail "install: python3 not found"

step "build" cargo build --locked --bins
step "rust gate tests" cargo test --locked
step "python tests" python3 eval/test_stage_eval.py

# The stub evaluation must regenerate advice.json byte for byte.
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
step "stub evaluation reproduces advice.json" bash -c \
  "python3 eval/stage_eval.py --allow-stage-errors --rate-in 1 --rate-out 3 --out-json '$tmp/advice.json' --out-md '$tmp/report.md' && cmp '$tmp/advice.json' advice.json"

# Negative proofs: the gate must go red on a copy of the project with a stage nobody measured.
copy() { rm -rf "$tmp/neg"; mkdir "$tmp/neg"; tar --exclude=target --exclude=.git -cf - . | tar -xf - -C "$tmp/neg"; }
must_fail() { # name, regex that the failure must contain
  local name=$1 want=$2
  if (cd "$tmp/neg" && CARGO_TARGET_DIR="$PWD/../neg-target" cargo test --locked --lib 2>&1) >"$tmp/neg.log"; then
    cat "$tmp/neg.log" >&2; fail "negative: $name did not fail the gate"
  fi
  grep -Eq "$want" "$tmp/neg.log" || { cat "$tmp/neg.log" >&2; fail "negative: $name failed for the wrong reason"; }
  gates=$((gates + 1)); echo "== negative: $name -> gate red, as required"
}
copy; echo "Reduce the ANSWER to its last sentence." >> "$tmp/neg/stages/one_liner.txt"
must_fail "an edited prompt" "prompt changed since the verdict was measured"
copy; python3 - "$tmp/neg/advice.json" <<'PY'
import json, sys
p = sys.argv[1]; d = json.load(open(p)); del d["stages"]["plain_words"]; json.dump(d, open(p, "w"))
PY
must_fail "a stage with no verdict" "plain_words. is registered but has no entry"

echo "VERIFY {\"verdict\":\"pass\",\"gates\":$gates}"
