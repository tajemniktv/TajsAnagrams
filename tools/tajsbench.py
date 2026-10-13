"""Grouped anagram-quality evaluation for recorded solver runs.

Input runs are JSONL. Each line contains case_id, variant, warm, wall_ms,
peak_rss_bytes, and either a native solve `result` or explicit ranked `rows`.
Rows may have phrase/display_phrase. A run is
one snapshot at its wall-time budget; the harness never invents intermediate
results from a later snapshot.
"""

import argparse
from collections import defaultdict
import json
import math
from pathlib import Path
import statistics
import random


def key(text):
    import unicodedata

    return "".join(c for c in unicodedata.normalize("NFKD", text).lower() if "a" <= c <= "z")


def signature(text):
    return "".join(sorted(key(text)))


def phrase_key(text):
    return " ".join(filter(None, (key(word) for word in text.split())))


def load_cases(path):
    payload = json.loads(Path(path).read_text(encoding="utf-8"))
    if payload.get("schema_version") != 1:
        raise ValueError("unsupported quality case schema")
    cases, bags = {}, {}
    for case in payload["cases"]:
        source = signature(case["source"])
        if not source or case["id"] in cases:
            raise ValueError("empty bag or duplicate case ID")
        if source in bags:
            raise ValueError(f"duplicate/near-leaking letter bag: {case['id']} and {bags[source]}")
        bags[source] = case["id"]
        if case["split"] not in ("train", "validation", "test") or not case.get("provenance"):
            raise ValueError(f"missing split/provenance: {case['id']}")
        if not case["relevance"]:
            raise ValueError(f"missing relevance: {case['id']}")
        for phrase, grade in case["relevance"].items():
            if signature(phrase) != source or not isinstance(grade, int) or not 1 <= grade <= 3:
                raise ValueError(f"invalid relevance phrase/grade: {case['id']} / {phrase}")
        for phrase in case.get("negatives", []):
            if signature(phrase) != source:
                raise ValueError(f"invalid negative: {case['id']} / {phrase}")
        cases[case["id"]] = case
    return cases


def percentile(values, percentage):
    if not values:
        return None
    ordered = sorted(values)
    index = (len(ordered) - 1) * percentage
    lo = math.floor(index)
    hi = math.ceil(index)
    return ordered[lo] + (ordered[hi] - ordered[lo]) * (index - lo)


def extract_rows(run, ranking="desktop"):
    """Explicit rows already have an order; native buckets mirror Results.svelte."""
    if ranking not in ("desktop", "final_score"):
        raise ValueError("unsupported ranking")
    if "rows" in run:
        return run["rows"]
    result = run["result"]
    rows = [row for _, bucket in sorted(result["buckets"].items(), key=lambda item: int(item[0]))
            for row in bucket]
    if ranking == "final_score":
        return sorted(rows, key=lambda r: (-r["final"], r.get("display_phrase", "")))
    return rows


def row_phrase(row):
    return row.get("phrase") or row.get("display_phrase") or " ".join(row.get("best_order", []))


def grade_rows(case, rows):
    grades = {phrase_key(phrase): grade for phrase, grade in case["relevance"].items()}
    seen = set()
    found = []
    for row in rows:
        phrase = row_phrase(row)
        # Native shallow rows occupy a visible position but have no chosen order.
        # Validate the bag, then score the blank display as zero; never invent an order.
        if not phrase and row.get("deep") is False and row.get("words"):
            if signature(" ".join(row["words"])) != signature(case["source"]):
                raise ValueError(f"non-anagram shallow bag: {case['id']}")
            found.append(0)
            continue
        identity = phrase_key(phrase)
        if signature(phrase) != signature(case["source"]):
            raise ValueError(f"non-anagram solver result: {case['id']} / {phrase}")
        if identity not in seen:
            seen.add(identity)
            found.append(grades.get(identity, 0))
    return found


def case_metrics(case, rows):
    grades = grade_rows(case, rows)
    hits = [i + 1 for i, grade in enumerate(grades) if grade > 0]
    relevant = list(case["relevance"].values())
    def dcg(values, limit):
        return sum((2**grade - 1) / math.log2(i + 2) for i, grade in enumerate(values[:limit]))
    return {
        "recall_at_1": sum(rank <= 1 for rank in hits) / len(relevant),
        "recall_at_5": sum(rank <= 5 for rank in hits) / len(relevant),
        "recall_at_10": sum(rank <= 10 for rank in hits) / len(relevant),
        "mrr": 1 / hits[0] if hits else 0.0,
        "ndcg_at_5": dcg(grades, 5) / dcg(sorted(relevant, reverse=True), 5),
        "ndcg_at_10": dcg(grades, 10) / dcg(sorted(relevant, reverse=True), 10),
    }


CACHE_STATES = {"disk_cache_empty", "disk_cache_reuse", "in_process_warm", "uncached", "unspecified"}
METRICS = ("recall_at_1", "recall_at_5", "recall_at_10", "mrr", "ndcg_at_5", "ndcg_at_10")


def cache_state(run):
    state = run.get("cache_state")
    if state is None:
        if "warm" not in run:
            return "unspecified"
        if not isinstance(run["warm"], bool):
            raise ValueError("warm must be boolean")
        return "disk_cache_reuse" if run["warm"] else "disk_cache_empty"
    if state not in CACHE_STATES:
        raise ValueError("unsupported cache state")
    return state


def run_identity(run):
    wall_ms = run["wall_ms"]
    if isinstance(wall_ms, bool) or not isinstance(wall_ms, (int, float)) or not math.isfinite(wall_ms) or wall_ms < 0:
        raise ValueError("invalid wall_ms")
    trial = run.get("trial", 0)
    if isinstance(trial, bool) or not isinstance(trial, int) or trial < 0:
        raise ValueError("trial must be a nonnegative integer")
    return run["variant"], cache_state(run), run["case_id"], trial


def latest_attempts(runs):
    """Several observations of one solve are snapshots, not independent cases."""
    latest = {}
    for run in runs:
        identity = run_identity(run)
        if identity not in latest or run["wall_ms"] >= latest[identity]["wall_ms"]:
            latest[identity] = run
    return list(latest.values())


def average_metrics(values):
    return {name: statistics.mean(value[name] for value in values) if values else None for name in METRICS}


def summarize(runs, cases, ranking="desktop"):
    groups = defaultdict(list)
    for run in latest_attempts(runs):
        case = cases[run["case_id"]]
        metrics = case_metrics(case, [] if "error" in run else extract_rows(run, ranking))
        length = len(key(case["source"]))
        word_count = len(next(iter(case["relevance"])).split())
        dimensions = ("all", f"words:{min(word_count, 5)}{'+' if word_count >= 5 else ''}",
                      f"letters:{(length // 5) * 5}-{(length // 5) * 5 + 4}", case["split"])
        for dimension in dimensions:
            groups[(run["variant"], dimension, cache_state(run))].append((run, metrics))
    report = []
    for (variant, dimension, state), samples in sorted(groups.items()):
        successful = [(run, metrics) for run, metrics in samples if "error" not in run]
        metrics = average_metrics([m for _, m in samples])
        times = [run["wall_ms"] for run, _ in successful]
        attempt_times = [run["wall_ms"] for run, _ in samples]
        counters = {}
        for name in ("generated", "expanded", "deep_analyzed", "orders_evaluated", "peak_rss_bytes", "persistent_bytes"):
            values = [run.get(name, run.get("result", {}).get(name)) for run, _ in successful]
            counters[name] = statistics.mean(values) if values and all(isinstance(v, (int, float)) for v in values) else None
        report.append({"variant": variant, "dimension": dimension, "cache_state": state,
                       "ranking": ranking, "attempts": len(samples),
                       "attempted_cases": len({run["case_id"] for run, _ in samples}),
                       "completed": len(successful), "failed": len(samples) - len(successful),
                       "coverage": len(successful) / len(samples), **metrics,
                       "successful_metrics": average_metrics([m for _, m in successful]),
                       "completed_p50_ms": percentile(times, .5), "completed_p95_ms": percentile(times, .95),
                       "attempt_p50_ms": percentile(attempt_times, .5), "attempt_p95_ms": percentile(attempt_times, .95),
                       **counters})
    return report


def quality_at_budgets(runs, cases, ranking="desktop"):
    output = []
    groups = defaultdict(list)
    for run in runs:
        variant, state, _, _ = run_identity(run)
        groups[(variant, state)].append(run)
    for (variant, state), group in sorted(groups.items()):
        attempts = latest_attempts(group)
        for budget in (50, 200, 1000, 5000):
            observed = latest_attempts([run for run in group if run["wall_ms"] <= budget])
            delivered = latest_attempts([run for run in group if run["wall_ms"] <= budget and "error" not in run])
            values = [case_metrics(cases[run["case_id"]], extract_rows(run, ranking)) for run in delivered]
            # Zero here means no useful result was delivered by the deadline,
            # not that an unobserved intermediate ranking was measured as zero.
            delivery_values = values + [dict.fromkeys(METRICS, 0.0)] * (len(attempts) - len(delivered))
            output.append({"variant": variant, "cache_state": state, "ranking": ranking,
                           "budget_ms": budget, "attempts": len(attempts),
                           "attempted_cases": len({run["case_id"] for run in attempts}),
                           "covered_cases": len({run["case_id"] for run in delivered}),
                           "covered_attempts": len(delivered),
                           "failed_attempts": sum("error" in run for run in observed),
                           "unobserved_attempts": len(attempts) - len(observed),
                           **average_metrics(values), "delivery_metrics": average_metrics(delivery_values)})
    return output


def preference_accuracy(runs, comparisons, ranking="desktop"):
    by_case = defaultdict(list)
    for run in latest_attempts(runs):
        if "error" not in run:
            by_case[(run["variant"], cache_state(run), run["case_id"])].append(run)
    output = []
    for variant, state in sorted({(run["variant"], cache_state(run)) for run in runs}):
        correct = total = 0
        for comparison in comparisons:
            for run in by_case.get((variant, state, comparison["case_id"]), []):
                ranks = {}
                for rank, row in enumerate(extract_rows(run, ranking)):
                    if row_phrase(row):
                        ranks.setdefault(phrase_key(row_phrase(row)), rank)
                left = ranks.get(phrase_key(comparison["left"]), float("inf"))
                right = ranks.get(phrase_key(comparison["right"]), float("inf"))
                if left == right:
                    continue
                total += 1
                correct += (left < right) == (comparison["preferred"] == "left")
        output.append({"variant": variant, "cache_state": state, "ranking": ranking,
                       "judgments": len(comparisons), "evaluated": total,
                       "accuracy": correct / total if total else None})
    return output


def load_preferences(path, cases):
    payload = json.loads(Path(path).read_text(encoding="utf-8"))
    if payload.get("schema_version") != 1:
        raise ValueError("unsupported preference schema")
    for comparison in payload["comparisons"]:
        case = cases[comparison["case_id"]]
        if comparison["preferred"] not in ("left", "right"):
            raise ValueError("preference must be left or right")
        if any(signature(comparison[side]) != signature(case["source"]) for side in ("left", "right")):
            raise ValueError("preference phrases must be exact anagrams")
        if comparison["left"] == comparison["right"] or not comparison.get("presentation_seed"):
            raise ValueError("preference needs distinct choices and recorded presentation seed")
    return payload["comparisons"]


def make_pair(case, a, b, seed):
    """Construct a randomized presentation; caller records the seed with the judgment."""
    if signature(a) != signature(case["source"]) or signature(b) != signature(case["source"]):
        raise ValueError("pair phrases must be exact anagrams")
    return [a, b] if random.Random(seed).randrange(2) == 0 else [b, a]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cases", default="tests/quality/cases.v1.json")
    parser.add_argument("--runs", required=True, type=Path)
    parser.add_argument("--preferences", default="tests/quality/preferences.v1.json")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--ranking", choices=("desktop", "final_score", "both"), default="desktop")
    args = parser.parse_args()
    cases = load_cases(args.cases)
    runs = [json.loads(line) for line in args.runs.read_text(encoding="utf-8").splitlines() if line.strip()]
    failures = [{key: run[key] for key in ("case_id", "variant", "cache_state", "warm", "trial", "wall_ms", "error") if key in run}
                for run in runs if "error" in run]
    comparisons = load_preferences(args.preferences, cases)
    rankings = ("desktop", "final_score") if args.ranking == "both" else (args.ranking,)
    attempts = latest_attempts(runs)
    report = {"schema_version": 2, "cases": len(cases), "snapshots": len(runs),
              "attempts": len(attempts), "failures": failures,
              "unattempted_cases": [
                  {"variant": variant, "cache_state": state,
                   "case_ids": sorted(set(cases) - {r["case_id"] for r in attempts
                                                  if r["variant"] == variant and cache_state(r) == state})}
                  for variant, state in sorted({(r["variant"], cache_state(r)) for r in attempts})],
              "groups": [row for ranking in rankings for row in summarize(runs, cases, ranking)],
              "quality_at_budgets": [row for ranking in rankings for row in quality_at_budgets(runs, cases, ranking)],
              "pairwise_preference": [row for ranking in rankings for row in preference_accuracy(runs, comparisons, ranking)]}
    content = json.dumps(report, indent=2) + "\n"
    if args.output:
        args.output.write_text(content, encoding="utf-8")
    else:
        print(content, end="")


if __name__ == "__main__":
    main()
