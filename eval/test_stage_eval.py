import contextlib
import io
import json
import os
import shlex
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import stage_eval as se  # noqa: E402

ROOT = se.ROOT
FAKE_REGISTRY = [{"id": "a", "targets": ["control"], "default_on": True, "system_prompt": "sys a", "prompt_hash": "ha"}]


def run_main(*extra, registry=None):
    cmd = ["--registry-cmd", f"python3 -c {shlex.quote('import json,sys;print(json.dumps(' + repr(registry or FAKE_REGISTRY) + '))')}"]
    err = io.StringIO()
    with contextlib.redirect_stderr(err), contextlib.redirect_stdout(io.StringIO()):
        try:
            code = se.main(cmd + list(extra))
        except SystemExit as e:
            code = e.code
    return code, err.getvalue()


class VerdictRules(unittest.TestCase):
    def test_useful_at_the_threshold(self):
        self.assertEqual(se.verdict(0.5, 0.0, 0.0)[0], "useful")

    def test_just_under_the_threshold_is_little_gain(self):
        self.assertEqual(se.verdict(0.49, 0.0, 0.0)[0], "cost_little_gain")

    def test_faithfulness_drop_over_limit_is_trash_even_with_big_gain(self):
        self.assertEqual(se.verdict(2.0, -0.31, 0.0)[0], "trash")

    def test_drop_exactly_at_limit_is_not_trash(self):
        self.assertEqual(se.verdict(2.0, -0.3, -0.3)[0], "useful")

    def test_harm_drop_over_limit_is_trash(self):
        self.assertEqual(se.verdict(0.0, 0.0, -0.31)[0], "trash")

    def test_no_scored_fixture_is_unevaluated(self):
        self.assertEqual(se.verdict(None, 0.0, 0.0)[0], "unevaluated")


class ReplyGuard(unittest.TestCase):
    def test_empty_reply_is_rejected(self):
        self.assertEqual(se.check_reply("keep me", "  ")[0], "keep me")
        self.assertIsNotNone(se.check_reply("keep me", "")[1])

    def test_added_citation_is_rejected(self):
        out, err = se.check_reply("A claim [1].", "A claim [1]. Another [2].")
        self.assertEqual(out, "A claim [1].")
        self.assertIn("[2]", err)

    def test_dropping_a_citation_is_allowed(self):
        self.assertEqual(se.check_reply("A [1]. B [2].", "A [1].")[1], None)

    def test_a_rewrite_that_keeps_under_a_fifth_of_a_long_answer_is_lost_content(self):
        prev = "x" * 300
        out, err = se.check_reply(prev, "x" * 59)
        self.assertEqual(out, prev)
        self.assertIn("lost content", err)

    def test_short_answers_are_not_length_checked(self):
        self.assertIsNone(se.check_reply("x" * 200, "y")[1])


class Harness(unittest.TestCase):
    def test_same_model_judge_is_refused(self):
        os.environ["STAGE_EVAL_API_KEY"] = "dummy"
        code, err = run_main("--backend", "http", "--llm-url", "http://localhost:9/x",
                             "--stage-model", "m", "--judge-model", "m")
        self.assertEqual(code, 2)
        self.assertIn("judge model is the stage model", err)

    def test_a_stage_targeting_a_group_with_no_fixture_is_refused(self):
        reg = [dict(FAKE_REGISTRY[0], targets=["nonexistent"])]
        code, err = run_main(registry=reg)
        self.assertEqual(code, 2)
        self.assertIn("no fixture", err)

    def test_stub_run_is_deterministic_and_reproduces_the_committed_advice(self):
        with tempfile.TemporaryDirectory() as d:
            outs = []
            for i in range(2):
                p = os.path.join(d, f"a{i}.json")
                with contextlib.redirect_stdout(io.StringIO()):
                    se.main(["--allow-stage-errors", "--rate-in", "1", "--rate-out", "3",
                             "--out-json", p, "--out-md", os.path.join(d, f"r{i}.md")])
                outs.append(open(p).read())
        self.assertEqual(outs[0], outs[1])
        self.assertEqual(outs[0], open(os.path.join(ROOT, "advice.json")).read())

    def test_stub_covers_all_three_verdicts_and_hashes_match_the_registry(self):
        advice = json.load(open(os.path.join(ROOT, "advice.json")))
        self.assertEqual({s["verdict"] for s in advice["stages"].values()}, {"useful", "cost_little_gain", "trash"})
        self.assertTrue(all(len(s["prompt_hash"]) == 64 for s in advice["stages"].values()))


if __name__ == "__main__":
    unittest.main()
