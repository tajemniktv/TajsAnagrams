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


if __name__ == "__main__":
    unittest.main()
