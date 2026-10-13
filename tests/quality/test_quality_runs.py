import json
from pathlib import Path
import re
import subprocess
import unittest
from unittest.mock import patch

from tools import run_quality_baseline as baseline
from tools.tajsbench import signature


ROOT = Path(__file__).resolve().parents[2]


class QualityRunTests(unittest.TestCase):
    def test_optional_phrase_model_paths_are_native_inputs_not_python_fallbacks(self):
        from types import SimpleNamespace
        args = SimpleNamespace(cli=ROOT/'build/release/anagram-cli.exe', dictionary=ROOT/'dictionary',
            unigrams=ROOT/'one',bigrams=ROOT/'two',wordnet=ROOT/'wordnet',timeout_ms=10,
            phrase_db=ROOT/'phrases.db', model=ROOT/'model.json')
        command = baseline.cli_command(args)
        self.assertEqual(command[6:8], [str(args.phrase_db.resolve()),str(args.model.resolve())])
        args.phrase_db = None
        self.assertEqual(baseline.cli_command(args)[6], '-')
        generated = baseline.cli_command(args, 'generate')
        self.assertNotIn(str(args.model.resolve()), generated)

    def test_desktop_profile_matches_native_defaults(self):
        source = (ROOT / "apps/desktop/src-tauri/src/settings.rs").read_text(encoding="utf-8")
        match = re.search(r"request: json!\((.*?)\),\s*\n", source, re.DOTALL)
        self.assertIsNotNone(match, "update the fixture comparison if native default construction changes")
        defaults = json.loads(match.group(1))
        self.assertEqual(baseline.desktop_request({"source": ""}), defaults)

    def test_legacy_profile_retained(self):
        req = baseline.request({"source": "test"}, 123)
        self.assertEqual(req["generation"]["strategy"], "diverse")
        self.assertEqual(req["generation"]["min_zipf"], 2.5)
        self.assertTrue(req["generation"]["allow_repeat"])
        self.assertEqual(req["generation"]["candidate_budget"], 123)

    def test_shuffled_input_control_preserves_inventory_and_is_repeatable(self):
        case = {"id": "sanity", "source": "testinganagramsolver", "relevance": {"Testing Anagram Solver": 3}}
        shuffled = baseline.shuffled_case(case, 13)
        self.assertEqual(signature(shuffled["source"]), signature(case["source"]))
        self.assertEqual(shuffled, baseline.shuffled_case(case, 13))
        self.assertNotEqual(shuffled["source"], case["source"])
        self.assertEqual(case["source"], "testinganagramsolver")

    def test_process_failures_are_retained(self):
        for process, expected in [
            (subprocess.CompletedProcess([], 1, "not json", "broken"), "invalid_response"),
            (subprocess.CompletedProcess([], 0, "[]", ""), "invalid_response"),
            (subprocess.CompletedProcess([], 1, '{}', ""), "process_failed"),
            (subprocess.CompletedProcess([], 1, '{"error":{"code":"timed_out"}}', ""), "timed_out"),
        ]:
            with self.subTest(expected=expected), patch.object(baseline.subprocess, "run", return_value=process):
                record = baseline.capture_run(["unused"], {}, 100)
                self.assertEqual(record["error"]["code"], expected)
                self.assertNotIn("result", record)
                self.assertGreaterEqual(record["wall_ms"], 0)

    def test_subprocess_timeout_and_launch_error_are_retained(self):
        for error, expected in [(subprocess.TimeoutExpired("unused", 30), "process_timeout"),
                                (OSError("not found"), "process_error")]:
            with self.subTest(expected=expected), patch.object(baseline.subprocess, "run", side_effect=error):
                self.assertEqual(baseline.capture_run(["unused"], {}, 100)["error"]["code"], expected)

    def test_success_keeps_response_and_correct_timeout(self):
        process = subprocess.CompletedProcess([], 0, '{"generated":1}', "")
        with patch.object(baseline.subprocess, "run", return_value=process) as mock:
            record = baseline.capture_run(["unused"], {"a": 1}, 5000)
        self.assertEqual(record["result"], {"generated": 1})
        self.assertEqual(mock.call_args.kwargs["timeout"], 35)
        self.assertEqual(json.loads(mock.call_args.kwargs["input"]), {"a": 1})


if __name__ == "__main__":
    unittest.main()
