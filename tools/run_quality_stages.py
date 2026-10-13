"""Offline stage isolation and fixed retained-order ablations, not a quality gate."""

import argparse
import copy
import json
import math
from pathlib import Path
import random

try:
    from tools import run_quality_baseline as baseline
    from tools.tajsbench import extract_rows, load_cases, phrase_key, row_phrase, signature
except ModuleNotFoundError:
    import run_quality_baseline as baseline
    from tajsbench import extract_rows, load_cases, phrase_key, row_phrase, signature


def bag_key(words):
    return tuple(sorted(words))


def stage_metrics(case, result, display_limit):
    """Full native output exposes selection; project only the final display limit."""
    rows = extract_rows({"result": result})
    for row in rows:
        if signature(" ".join(row["words"])) != signature(case["source"]):
            raise ValueError("non-anagram candidate bag")
        if row_phrase(row) and signature(row_phrase(row)) != signature(case["source"]):
            raise ValueError("non-anagram displayed phrase")
    complete_pool = len(rows) == result["generated"]
    displayed = [row for _, bucket in sorted(result["buckets"].items(), key=lambda item: int(item[0]))
                 for row in bucket[:display_limit]]
    metrics = []
    for gold in case["relevance"]:
        words = phrase_key(gold).split()
        matches = [row for row in rows if bag_key(row["words"]) == bag_key(words)]
        row = matches[0] if matches else None
        chosen = row_phrase(row) if row else None
        bucket = result["buckets"].get(str(len(words)), [])
        metrics.append({"gold": gold, "gold_bag_in_pool": bool(matches) if complete_pool else (True if matches else None),
                        "deep_selected": row["deep"] if row else None,
                        "pre_score": row["pre_score"] if row else None,
                        "chosen_order": chosen,
                        "chosen_order_matches_gold": phrase_key(chosen) == phrase_key(gold) if chosen else None,
                        "bag_bucket_rank": next((i + 1 for i, r in enumerate(bucket) if bag_key(r["words"]) == bag_key(words)), None),
                        "displayed_gold_rank": next((i + 1 for i, r in enumerate(displayed) if phrase_key(row_phrase(r)) == phrase_key(gold)), None)})
    return {"all_rows_available": complete_pool, "generated": result["generated"],
            "deep_analyzed": result["deep_analyzed"], "stop": result["generation_stop"],
            "blank_display_rows": sum(not row_phrase(row) for row in rows),
            "display_limit_projection": display_limit, "gold": metrics}


ORDER_WEIGHTS = {"grammar_norm": .38, "structure_norm": .44, "valency_norm": .12, "syntax_coverage": .06}


def order_ablations(case, result):
    """Remove one objective component AND its tie-break influence on a frozen pool."""
    orders = result["orders"]
    for candidate in orders:
        if signature(" ".join(candidate["order"])) != signature(case["source"]):
            raise ValueError("non-anagram retained order")
    rows = []
    ties = ("structure_norm", "grammar_norm", "valency_norm", "syntax_coverage")
    for omitted in (None, *ORDER_WEIGHTS):
        def sort_key(candidate):
            # Use the exact native objective for the control to preserve its rounding.
            score = candidate["objective"] if omitted is None else sum(
                weight * candidate[name] for name, weight in ORDER_WEIGHTS.items() if name != omitted)
            return (-score, *(-candidate[name] for name in ties if name != omitted), candidate["order"])
        ranked = sorted(orders, key=sort_key)
        rows.append({"variant": "native_objective" if omitted is None else f"without_{omitted}",
                     "evaluated_orders": result["evaluated"], "retained_orders": len(orders),
                     "complete_order_pool": len(orders) == result["evaluated"],
                     "top": " ".join(ranked[0]["order"]) if ranked else None,
                     "gold_ranks": {gold: next((i + 1 for i, candidate in enumerate(ranked)
                                                if phrase_key(" ".join(candidate["order"])) == phrase_key(gold)), None)
                                    for gold in case["relevance"]}})
    return rows


def evaluate_capture(case, phase, run, display_limit=None):
    """Keep unexpected response/letter failures with their raw evidence."""
    if "error" in run:
        return {"error": run["error"]}
    try:
        result = run["result"]
        if phase in ("admission", "generation"):
            if any(signature(" ".join(bag)) != signature(case["source"]) for bag in result["bags"]):
                raise ValueError("non-anagram generated bag")
            return {"generated": result["generated"], "stop": result["stop"],
                    "gold_positions": {gold: next((i + 1 for i, bag in enumerate(result["bags"])
                                                   if bag_key(bag) == bag_key(phrase_key(gold).split())), None)
                                       for gold in case["relevance"]}}
        if phase == "ordering":
            return {"ablations": order_ablations(case, result)}
        return stage_metrics(case, result, display_limit)
    except (KeyError, TypeError, ValueError) as error:
        run["error"] = {"code": "invalid_diagnostic_result", "message": str(error)}
        return {"error": run["error"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    baseline.add_corpus_arguments(parser)
    parser.add_argument("--output", default=".codex/temp/quality-stages.jsonl", type=Path)
    parser.add_argument("--profile", choices=("desktop", "legacy"), default="desktop")
    parser.add_argument("--strategies", nargs="+", choices=("prefix", "diverse"), default=["prefix", "diverse"])
    parser.add_argument("--candidate-budgets", nargs="+", type=int, default=[500, 5000])
    parser.add_argument("--timeout-ms", type=int, default=10000)
    parser.add_argument("--repeats", type=int, default=1)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--deep-all", action="store_true", help="Also analyze every generated row; may reach the deadline")
    parser.add_argument("--ordering-probe", type=Path, help="Optional existing Rust ordering_probe executable")
    args = parser.parse_args()
    if min(*args.candidate_budgets, args.timeout_ms, args.repeats) <= 0:
        parser.error("budgets, timeout and repeats must be positive")
    if len(set(args.strategies)) != len(args.strategies) or len(set(args.candidate_budgets)) != len(args.candidate_budgets):
        parser.error("duplicate strategies or candidate budgets")
    cases = load_cases(args.cases)
    make_request = baseline.desktop_request if args.profile == "desktop" else baseline.request
    metadata = baseline.manifest(args, cases)
    metadata.update(profile=args.profile, strategies=args.strategies, candidate_budgets=args.candidate_budgets,
                    timeout_ms=args.timeout_ms, repeats=args.repeats, seed=args.seed, cache_state="uncached",
                    interpretation="Oracle and display-limit interventions are diagnostic, not production-quality wins")
    if args.ordering_probe:
        metadata["inputs"]["ordering_probe"] = {"sha256": baseline.file_hash(args.ordering_probe)}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.with_suffix(".manifest.json").write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")
    summaries = []
    with args.output.open("w", encoding="utf-8") as output:
        def record(case, variant, phase, req, command=None, trial=0, display_limit=None):
            mode = "generate" if phase in ("admission", "generation") else "solve"
            run = {"case_id": case["id"], "variant": variant, "phase": phase, "trial": trial,
                   "cache_state": "uncached", "request": req,
                   **baseline.capture_run(command or baseline.cli_command(args, mode), req, args.timeout_ms)}
            summary = {name: run[name] for name in ("case_id", "variant", "phase", "trial", "wall_ms")}
            summary.update(evaluate_capture(case, phase, run, display_limit))
            run["summary"] = summary
            output.write(json.dumps(run) + "\n")
            output.flush()
            summaries.append(summary)
            print(json.dumps(summary), flush=True)
            return run

        # Single-word admission is independent of search-cap/strategy interventions.
        words = sorted({word for case in cases.values() for phrase in case["relevance"] for word in phrase_key(phrase).split()})
        for word in words:
            case = {"id": f"word:{word}", "source": word, "relevance": {word: 3}}
            req = make_request(case, 500)["generation"]
            req.update(min_words=1, max_words=1, strategy="prefix")
            admitted = record(case, args.profile, "admission", req)
            if "error" not in admitted and admitted["summary"]["gold_positions"][word] is None:
                zero = copy.deepcopy(req)
                zero["min_zipf"] = 0.0
                record(case, args.profile + "_zipf_zero", "admission", zero)

        schedule = [(case_id, trial, strategy, budget) for trial in range(args.repeats)
                    for case_id in cases for strategy in args.strategies for budget in args.candidate_budgets]
        random.Random(args.seed).shuffle(schedule)
        for case_id, trial, strategy, budget in schedule:
            case = cases[case_id]
            req = make_request(case, budget)
            req["generation"]["strategy"] = strategy
            variant = f"{args.profile}_{strategy}_{budget}"
            record(case, variant, "generation", req["generation"], trial=trial)
            display_limit = req["result_limit_per_group"]
            req["result_limit_per_group"] = max(budget, display_limit)
            record(case, variant, "shortlist", req, trial=trial, display_limit=display_limit)
            if args.deep_all:
                req = copy.deepcopy(req)
                req["deep_all"] = True
                record(case, variant, "deep_all", req, trial=trial, display_limit=display_limit)

        for case in cases.values():
            for gold in case["relevance"]:
                req = make_request(case, 500)
                req["generation"]["required"] = phrase_key(gold).split()
                req["order_mode"] = "exact"
                record(case, args.profile, "oracle_gold_bag", req, display_limit=req["result_limit_per_group"])
                if args.ordering_probe:
                    # The existing probe's maximum width allows 512 retained orders.
                    # Six-word fixtures therefore report truncation explicitly.
                    order_req = {"words": phrase_key(gold).split(), "exact": True,
                                 "beam_width": 128, "top_k": min(512, math.factorial(len(gold.split())))}
                    record(case, args.profile, "ordering", order_req,
                           command=[str(args.ordering_probe.resolve()), str(args.wordnet.resolve())])
    args.output.with_suffix(".summary.json").write_text(json.dumps(summaries, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
