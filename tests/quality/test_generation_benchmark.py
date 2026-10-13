import unittest

from tools import benchmark_generation as benchmark


class GenerationBenchmarkTests(unittest.TestCase):
    def run_record(self, variant, **extras):
        return {"case_id": "a", "strategy": "prefix", "trial": 0,
                "variant": variant, "wall_ms": 10, **extras}

    def test_no_completed_pair_is_unknown_not_vacuous_success(self):
        records = [self.run_record(v, error={"code": "timed_out"}) for v in ("before", "after")]
        row = benchmark.summarize(records)[0]
        self.assertEqual(row["compared_completed_pairs"], 0)
        self.assertIsNone(row["completed_output_equal"])
        self.assertIsNone(row["median_paired_speedup"])

    def test_partial_recorded_pair_keeps_missing_variant_unknown(self):
        row = benchmark.summarize([self.run_record("before", result={"bags": [], "stop": "exhausted"})])[0]
        self.assertEqual(row["after"]["attempted"], 0)
        self.assertIsNone(row["after"]["median_attempt_ms"])
        self.assertIsNone(row["completed_output_equal"])

    def test_stop_and_order_are_part_of_output_identity(self):
        result = {"bags": [["a"], ["b"]], "stop": "candidate_cap", "engine_version": "v1"}
        self.assertEqual(benchmark.output_identity(result), benchmark.output_identity({**result, "engine_version": "v2"}))
        self.assertNotEqual(benchmark.output_identity(result), benchmark.output_identity({**result, "stop": "exhausted"}))
        self.assertNotEqual(benchmark.output_identity(result), benchmark.output_identity({**result, "bags": list(reversed(result["bags"]))}))

    def test_failure_not_used_as_a_speedup_sample(self):
        result = {"bags": [["a"]], "stop": "exhausted"}
        records = [self.run_record("before", result=result, wall_ms=20), self.run_record("after", result=result)]
        records += [{**self.run_record(v, error={"code": "timed_out"}), "trial": 1, "wall_ms": 10000}
                    for v in ("before", "after")]
        row = benchmark.summarize(records)[0]
        self.assertEqual(row["compared_completed_pairs"], 1)
        self.assertTrue(row["completed_output_equal"])
        self.assertEqual(row["median_paired_speedup"], 2)
        self.assertEqual(row["before"]["attempted"], 2)


if __name__ == "__main__":
    unittest.main()
