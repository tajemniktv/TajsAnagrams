"""Paired, randomized native generation measurements with exact output checks."""

import argparse
import hashlib
import json
from pathlib import Path
import random
import statistics

try:
    from tools import run_quality_baseline as baseline
    from tools.tajsbench import load_cases
except ModuleNotFoundError:
    import run_quality_baseline as baseline
    from tajsbench import load_cases


def output_identity(result):
    # Version labels may change independently; bags, order and stop must not.
    semantic = {name: value for name, value in result.items() if name != "engine_version"}
    return hashlib.sha256(json.dumps(semantic, sort_keys=True).encode()).hexdigest()


def summarize(records):
    output = []
    for case_id, strategy in sorted({(r["case_id"], r["strategy"]) for r in records}):
        group = [r for r in records if r["case_id"] == case_id and r["strategy"] == strategy]
        row = {"case_id": case_id, "strategy": strategy}
        for variant in ("before", "after"):
            runs = [r for r in group if r["variant"] == variant]
            row[variant] = {"median_attempt_ms": statistics.median(r["wall_ms"] for r in runs) if runs else None,
                            "completed": sum("error" not in r for r in runs), "attempted": len(runs)}
        paired = []
        for trial in sorted({r["trial"] for r in group}):
            pair = {r["variant"]: r for r in group if r["trial"] == trial}
            if len(pair) != 2 or any("error" in r for r in pair.values()):
                continue
            paired.append(pair)
        row["compared_completed_pairs"] = len(paired)
        row["completed_output_equal"] = all(
            output_identity(pair["before"]["result"]) == output_identity(pair["after"]["result"])
            for pair in paired) if paired else None
        row["median_paired_speedup"] = statistics.median(
            pair["before"]["wall_ms"] / pair["after"]["wall_ms"] for pair in paired
            if pair["after"]["wall_ms"] > 0) if paired and all(pair["after"]["wall_ms"] > 0 for pair in paired) else None
        output.append(row)
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    baseline.add_corpus_arguments(parser)
    parser.add_argument("--before", type=Path, required=True, help="Preserved baseline executable")
    parser.add_argument("--output", type=Path, default=".codex/temp/generation-paired.jsonl")
    parser.add_argument("--case-ids", nargs="+", help="Default: all dataset cases")
    parser.add_argument("--candidate-budget", type=int, default=500)
    parser.add_argument("--strategies", nargs="+", choices=("prefix", "diverse"), default=["prefix", "diverse"])
    parser.add_argument("--timeout-ms", type=int, default=10000)
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--seed", type=int, default=13)
    args = parser.parse_args()
    if min(args.candidate_budget, args.timeout_ms, args.repeats) <= 0:
        parser.error("budget, timeout and repeats must be positive")
    cases = load_cases(args.cases)
    selected = args.case_ids or list(cases)
    if len(set(selected)) != len(selected) or any(case_id not in cases for case_id in selected):
        parser.error("case IDs must be known and unique")
    if len(set(args.strategies)) != len(args.strategies):
        parser.error("duplicate strategies")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    metadata = baseline.manifest(args, {case_id: cases[case_id] for case_id in selected})
    metadata["inputs"]["before_cli"] = {"sha256": baseline.file_hash(args.before)}
    metadata.update(seed=args.seed, repeats=args.repeats, candidate_budget=args.candidate_budget,
                    timeout_ms=args.timeout_ms, strategies=args.strategies, profile="desktop generation",
                    cache_state="uncached", timing="Attempt wall time; timeouts are not completion latency")
    args.output.with_suffix(".manifest.json").write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")
    schedule = [(case_id, strategy, trial) for case_id in selected for strategy in args.strategies
                for trial in range(args.repeats)]
    rng = random.Random(args.seed)
    rng.shuffle(schedule)
    records = []
    with args.output.open("w", encoding="utf-8") as out:
        for case_id, strategy, trial in schedule:
            req = baseline.desktop_request(cases[case_id], args.candidate_budget)["generation"]
            req["strategy"] = strategy
            order = ["before", "after"]
            rng.shuffle(order)
            for variant in order:
                command = baseline.cli_command(args, "generate")
                command[0] = str((args.before if variant == "before" else args.cli).resolve())
                run = {"case_id": case_id, "strategy": strategy, "trial": trial, "variant": variant,
                       "cache_state": "uncached", "request": req,
                       **baseline.capture_run(command, req, args.timeout_ms)}
                records.append(run)
                out.write(json.dumps(run) + "\n")
                out.flush()
                print(f"{case_id}/{strategy}/{trial}/{variant}: {run['wall_ms']:.0f} ms "
                      f"{run.get('error', {}).get('code', 'completed')}", flush=True)
    summary = summarize(records)
    args.output.with_suffix(".summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary), flush=True)


if __name__ == "__main__":
    main()
