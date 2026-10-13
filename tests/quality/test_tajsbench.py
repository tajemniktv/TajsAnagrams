import unittest
from pathlib import Path

from tools import tajsbench


ROOT = Path(__file__).resolve().parents[2]


class TajsBenchTests(unittest.TestCase):
    def setUp(self):
        self.cases = tajsbench.load_cases(ROOT / "tests/quality/cases.v1.json")

    def test_curated_cases_are_exact_and_split_by_bag(self):
        self.assertEqual(len(self.cases), 7)

    def test_ranking_distinguishes_orders_with_same_letters(self):
        case = self.cases["testing_anagram_solver"]
        bad = {"phrase": "Solver Testing Anagram"}
        good = {"phrase": "Testing Anagram Solver"}
        metrics = tajsbench.case_metrics(case, [bad, good])
        self.assertEqual(metrics["recall_at_1"], 0)
        self.assertEqual(metrics["mrr"], .5)
        self.assertGreater(metrics["ndcg_at_5"], 0)
        with self.assertRaises(ValueError):
            tajsbench.case_metrics(case, [{"phrase": "Testing Solver"}])

    def test_budget_uses_only_observed_snapshot(self):
        runs = [
            {"variant": "baseline", "case_id": "testing_anagram_solver", "warm": False,
             "wall_ms": 40, "rows": [{"phrase": "Solver Testing Anagram"}]},
            {"variant": "baseline", "case_id": "testing_anagram_solver", "warm": False,
             "wall_ms": 150, "rows": [{"phrase": "Testing Anagram Solver"}]},
        ]
        budgets = tajsbench.quality_at_budgets(runs, self.cases)
        self.assertEqual(budgets[0]["recall_at_1"], 0)
        self.assertEqual(budgets[1]["recall_at_1"], 1)

    def test_native_order_matches_desktop_not_global_score(self):
        good = {"display_phrase": "Testing Anagram Solver", "final": 1}
        bad = {"display_phrase": "A German Signs To Travel", "final": 100}
        run = {"result": {"buckets": {"5": [bad], "3": [good]}}}
        self.assertEqual(tajsbench.extract_rows(run), [good, bad])
        self.assertEqual(tajsbench.extract_rows(run, "final_score"), [bad, good])
        run["result"]["buckets"]["3"] = [good, {**good, "final": 200}]
        self.assertEqual(tajsbench.extract_rows(run)[0], good)

    def test_failures_are_in_primary_denominator(self):
        runs = [
            {"variant": "v", "case_id": "testing_anagram_solver", "warm": False,
             "wall_ms": 100, "rows": [{"phrase": "Testing Anagram Solver"}]},
            {"variant": "v", "case_id": "knowledge_power", "warm": False,
             "wall_ms": 10000, "error": {"code": "timed_out"}},
        ]
        group = next(g for g in tajsbench.summarize(runs, self.cases) if g["dimension"] == "all")
        self.assertEqual((group["attempts"], group["completed"], group["failed"]), (2, 1, 1))
        self.assertEqual(group["recall_at_1"], .5)
        self.assertEqual(group["successful_metrics"]["recall_at_1"], 1)
        self.assertEqual(group["coverage"], .5)
        self.assertEqual(group["completed_p50_ms"], 100)
        self.assertGreater(group["attempt_p95_ms"], 9000)

    def test_cache_states_and_trials_do_not_double_case_coverage(self):
        runs = [{"variant": "v", "case_id": "testing_anagram_solver",
                 "cache_state": state, "trial": trial, "wall_ms": ms,
                 "rows": [{"phrase": "Testing Anagram Solver"}]}
                for state, ms in (("disk_cache_empty", 100), ("disk_cache_reuse", 40))
                for trial in (0, 1)]
        budgets = [b for b in tajsbench.quality_at_budgets(runs, self.cases) if b["budget_ms"] == 50]
        cold, warm = budgets
        self.assertEqual(cold["cache_state"], "disk_cache_empty")
        self.assertIsNone(cold["recall_at_1"])
        self.assertEqual(cold["delivery_metrics"]["recall_at_1"], 0)
        self.assertEqual(cold["unobserved_attempts"], 2)
        self.assertEqual((warm["covered_cases"], warm["covered_attempts"]), (1, 2))

    def test_snapshots_not_independent_attempts_and_partial_result_survives_deadline(self):
        runs = [
            {"variant": "v", "case_id": "testing_anagram_solver", "warm": False,
             "wall_ms": 40, "rows": [{"phrase": "Testing Anagram Solver"}]},
            {"variant": "v", "case_id": "testing_anagram_solver", "warm": False,
             "wall_ms": 150, "error": {"code": "timed_out"}},
        ]
        group = next(g for g in tajsbench.summarize(runs, self.cases) if g["dimension"] == "all")
        self.assertEqual((group["attempts"], group["failed"], group["recall_at_1"]), (1, 1, 0))
        budget = tajsbench.quality_at_budgets(runs, self.cases)[1]
        self.assertEqual((budget["covered_attempts"], budget["failed_attempts"]), (1, 1))
        self.assertEqual(budget["delivery_metrics"]["recall_at_1"], 1)

    def test_blank_shallow_rows_keep_positions_without_inventing_order(self):
        case = self.cases["testing_anagram_solver"]
        blank = {"display_phrase": "", "best_order": [], "deep": False,
                 "words": ["anagram", "solver", "testing"]}
        good = {"phrase": "Testing Anagram Solver"}
        self.assertEqual(tajsbench.case_metrics(case, [blank, blank, good])["mrr"], 1 / 3)
        with self.assertRaises(ValueError):
            tajsbench.case_metrics(case, [{**blank, "words": ["testing"]}])
        with self.assertRaises(ValueError):
            tajsbench.case_metrics(case, [{**blank, "deep": True}])

    def test_invalid_times_and_trials_rejected(self):
        run = {"variant": "v", "case_id": "testing_anagram_solver", "warm": False,
               "wall_ms": 1, "rows": []}
        for ms in (float("nan"), float("inf"), -1, True):
            with self.subTest(ms=ms), self.assertRaises(ValueError):
                tajsbench.summarize([{**run, "wall_ms": ms}], self.cases)
        with self.assertRaises(ValueError):
            tajsbench.quality_at_budgets([{**run, "trial": -1}], self.cases)

    def test_pairwise_evaluation_does_not_choose_fastest_cache_state(self):
        good, bad = "Testing Anagram Solver", "Solver Testing Anagram"
        runs = [{"variant": "v", "case_id": "testing_anagram_solver", "warm": warm,
                 "wall_ms": ms, "rows": [{"phrase": p} for p in phrases]}
                for warm, ms, phrases in ((False, 100, [good, bad]), (True, 10, [bad, good]))]
        judgments = [{"case_id": "testing_anagram_solver", "left": good, "right": bad, "preferred": "left"}]
        cold, warm = tajsbench.preference_accuracy(runs, judgments)
        self.assertEqual((cold["accuracy"], warm["accuracy"]), (1, 0))

    def test_actual_usefulness_labels_are_separate_from_exact_target_recovery(self):
        judgments = tajsbench.load_usefulness(ROOT / "tests/quality/usefulness.v1.json", self.cases)
        run = {"variant": "v", "case_id": "testing_anagram_solver", "cache_state": "uncached",
               "wall_ms": 100, "rows": [{"phrase": "Several Migrants Tango"},
                                         {"phrase": "Solver Testing Anagram"}]}
        self.assertEqual(tajsbench.case_metrics(self.cases[run["case_id"]], run["rows"])["recall_at_1"], 0)
        top_one, top_five, _ = tajsbench.standalone_usefulness([run], judgments)
        self.assertEqual((top_one["useful_rows"], top_one["not_useful_rows"]), (1, 0))
        self.assertEqual(top_five["unreviewed_rows"], 1)
        self.assertEqual(top_five["not_useful_rows"], 0)
        self.assertEqual(top_one["unique_phrase_judgments"], 2)


if __name__ == "__main__":
    unittest.main()
