import unittest

from tools import run_quality_stages as stages


class QualityStageTests(unittest.TestCase):
    def setUp(self):
        self.case = {"source": "testinganagramsolver", "relevance": {"Testing Anagram Solver": 3}}

    def test_selection_and_display_are_different_stages(self):
        other = {"words": ["given", "games", "translator"], "deep": True,
                 "display_phrase": "Given Games Translator", "pre_score": 90}
        gold = {"words": ["anagram", "solver", "testing"], "deep": False,
                "display_phrase": "", "best_order": [], "pre_score": 50}
        result = {"generated": 2, "deep_analyzed": 1, "generation_stop": "candidate_cap", "buckets": {"3": [other, gold]}}
        metric = stages.stage_metrics(self.case, result, 1)
        row = metric["gold"][0]
        self.assertTrue(metric["all_rows_available"])
        self.assertTrue(row["gold_bag_in_pool"])
        self.assertFalse(row["deep_selected"])
        self.assertIsNone(row["chosen_order_matches_gold"])
        self.assertEqual(row["bag_bucket_rank"], 2)
        self.assertIsNone(row["displayed_gold_rank"])

    def test_missing_row_in_truncated_output_is_unknown(self):
        result = {"generated": 10, "deep_analyzed": 0, "generation_stop": "candidate_cap", "buckets": {}}
        self.assertIsNone(stages.stage_metrics(self.case, result, 10)["gold"][0]["gold_bag_in_pool"])
        result["generated"] = 0
        self.assertFalse(stages.stage_metrics(self.case, result, 10)["gold"][0]["gold_bag_in_pool"])

    def test_invalid_bags_rejected(self):
        result = {"generated": 1, "deep_analyzed": 0, "generation_stop": "exhausted",
                  "buckets": {"3": [{"words": ["testing"], "display_phrase": ""}]}}
        with self.assertRaises(ValueError):
            stages.stage_metrics(self.case, result, 10)

    def test_invalid_diagnostic_preserves_raw_evidence(self):
        run = {"result": {"bags": [["testing"]], "generated": 1, "stop": "exhausted"}}
        raw = run["result"]
        summary = stages.evaluate_capture(self.case, "generation", run)
        self.assertEqual(summary["error"]["code"], "invalid_diagnostic_result")
        self.assertIs(run["result"], raw)
        self.assertEqual(run["error"], summary["error"])
        run = {"result": {}}
        self.assertIn("error", stages.evaluate_capture(self.case, "shortlist", run, 10))

    def test_ablations_retain_pool_and_remove_tiebreak_influence(self):
        candidate = {"order": ["testing", "anagram", "solver"], "grammar_norm": 1,
                     "structure_norm": 0, "valency_norm": 0, "syntax_coverage": 0, "objective": .38}
        other = {**candidate, "order": ["anagram", "solver", "testing"], "grammar_norm": 0, "objective": 0}
        result = {"evaluated": 6, "orders": [candidate, other]}
        rows = stages.order_ablations(self.case, result)
        self.assertEqual(rows[0]["top"], "testing anagram solver")
        self.assertEqual(rows[1]["top"], "anagram solver testing")
        self.assertTrue(all(row["retained_orders"] == 2 and not row["complete_order_pool"] for row in rows))


if __name__ == "__main__":
    unittest.main()
