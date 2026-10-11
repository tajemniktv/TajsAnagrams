"""Grouped anagram-quality evaluation for recorded solver runs.

Input runs are JSONL. Each line contains case_id, variant, warm, wall_ms,
peak_rss_bytes, and either a native solve `result` or explicit ranked `rows`.
Rows may have phrase/display_phrase and optional relevance overrides. A run is
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


def extract_rows(run):
    if "rows" in run:
        return run["rows"]
    result = run["result"]
    rows = []
    for bucket in result["buckets"].values():
        rows.extend(bucket)
    return sorted(rows, key=lambda r: (-r["final"], r.get("display_phrase", "")))


def grade_rows(case, rows):
    grades = {phrase_key(phrase): grade for phrase, grade in case["relevance"].items()}
    seen = set()
    found = []
    for row in rows:
        phrase = row.get("phrase") or row.get("display_phrase") or " ".join(row.get("best_order", []))
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


def summarize(runs, cases):
    groups = defaultdict(list)
    for run in runs:
        case = cases[run["case_id"]]
        if not isinstance(run["wall_ms"], (int, float)) or run["wall_ms"] < 0:
            raise ValueError("invalid wall_ms")
        metrics = case_metrics(case, extract_rows(run))
        length = len(key(case["source"]))
        word_count = len(next(iter(case["relevance"])).split())
        dimensions = ("all", f"words:{min(word_count, 5)}{'+' if word_count >= 5 else ''}",
                      f"letters:{(length // 5) * 5}-{(length // 5) * 5 + 4}", case["split"])
        for dimension in dimensions:
            groups[(run["variant"], dimension, bool(run["warm"]))].append((run, metrics))
    report = []
    for (variant, dimension, warm), samples in sorted(groups.items()):
        metrics = {name: statistics.mean(m[name] for _, m in samples) for name in samples[0][1]}
        times = [run["wall_ms"] for run, _ in samples]
        counters = {}
        for name in ("generated", "expanded", "deep_analyzed", "orders_evaluated", "peak_rss_bytes", "persistent_bytes"):
            values = [run.get(name, run.get("result", {}).get(name)) for run, _ in samples]
            counters[name] = statistics.mean(values) if all(isinstance(v, (int, float)) for v in values) else None
        report.append({"variant": variant, "dimension": dimension, "warm": warm,
                       "samples": len(samples), **metrics, "p50_ms": percentile(times, .5),
                       "p95_ms": percentile(times, .95), **counters})
    return report


def quality_at_budgets(runs, cases):
    output = []
    variants = sorted({run["variant"] for run in runs})
    for variant in variants:
        for budget in (50, 200, 1000, 5000):
            latest = {}
            for run in runs:
                if run["variant"] != variant or run["wall_ms"] > budget:
                    continue
                identity = (run["case_id"], bool(run["warm"]))
                if identity not in latest or latest[identity]["wall_ms"] < run["wall_ms"]:
                    latest[identity] = run
            values = [case_metrics(cases[run["case_id"]], extract_rows(run)) for run in latest.values()]
            output.append({"variant": variant, "budget_ms": budget, "covered_cases": len(latest),
                           **({name: statistics.mean(v[name] for v in values) for name in values[0]}
                              if values else {})})
    return output


def preference_accuracy(runs, comparisons):
    by_case = defaultdict(list)
    for run in runs:
        by_case[(run["variant"], run["case_id"])].append(run)
    output = []
    for variant in sorted({run["variant"] for run in runs}):
        correct = total = 0
        for comparison in comparisons:
            candidates = by_case.get((variant, comparison["case_id"]), [])
            if not candidates:
                continue
            run = min(candidates, key=lambda item: item["wall_ms"])
            rows = extract_rows(run)
            ranks = {}
            for rank, row in enumerate(rows):
                phrase = row.get("phrase") or row.get("display_phrase") or " ".join(row.get("best_order", []))
                ranks.setdefault(phrase_key(phrase), rank)
            left = ranks.get(phrase_key(comparison["left"]), float("inf"))
            right = ranks.get(phrase_key(comparison["right"]), float("inf"))
            if left == right:
                continue
            total += 1
            correct += (left < right) == (comparison["preferred"] == "left")
        output.append({"variant": variant, "evaluated": total,
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
    args = parser.parse_args()
    cases = load_cases(args.cases)
    runs = [json.loads(line) for line in args.runs.read_text(encoding="utf-8").splitlines() if line.strip()]
    failures = [{key: run[key] for key in ("case_id", "variant", "warm", "wall_ms", "error")}
                for run in runs if "error" in run]
    successful = [run for run in runs if "error" not in run]
    comparisons = load_preferences(args.preferences, cases)
    report = {"schema_version": 1, "cases": len(cases), "runs": len(successful),
              "failures": failures, "groups": summarize(successful, cases),
              "quality_at_budgets": quality_at_budgets(successful, cases),
              "pairwise_preference": preference_accuracy(successful, comparisons)}
    content = json.dumps(report, indent=2) + "\n"
    if args.output:
        args.output.write_text(content, encoding="utf-8")
    else:
        print(content, end="")


if __name__ == "__main__":
    main()
