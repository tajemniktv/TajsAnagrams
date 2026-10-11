"""Record a bounded native-solve TajsBench baseline (not a quality gate)."""

import argparse
import json
from pathlib import Path
import subprocess
import time
import uuid

from tajsbench import load_cases


def request(case, candidate_budget):
    return {"generation": {"schema_version": 1, "text": case["source"],
                           "min_words": 1, "max_words": 6,
                           "min_word_length": 1, "max_word_length": 24,
                           "min_zipf": 2.5, "candidate_budget": candidate_budget,
                           "allow_repeat": True, "strategy": "diverse", "hint_mode": "any"},
            "deep_per_group": 100, "deep_all": False, "order_mode": "auto",
            "beam_width": 128, "exact_max_words": 5, "retained_orders": 56,
            "phrase_rescore_top": 300, "phrase_bonus_max": 5,
            "positive_bigrams": True, "result_limit_per_group": 40}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", default="target/release/anagram-cli.exe", type=Path)
    parser.add_argument("--dictionary", default=".anagram_data/dictionary/normal_user_v2.txt", type=Path)
    parser.add_argument("--unigrams", default=".anagram_data/ngrams/count_1w.txt", type=Path)
    parser.add_argument("--bigrams", default=".anagram_data/ngrams/count_2w.txt", type=Path)
    parser.add_argument("--wordnet", default=".anagram_data/wordnet31/dict", type=Path)
    parser.add_argument("--cases", default="tests/quality/cases.v1.json")
    parser.add_argument("--output", default=".codex/temp/quality-baseline.jsonl", type=Path)
    parser.add_argument("--candidate-budget", default=500, type=int)
    parser.add_argument("--timeout-ms", default=5000, type=int)
    args = parser.parse_args()
    if args.candidate_budget <= 0 or args.timeout_ms <= 0:
        parser.error("candidate budget and timeout must be positive")
    cases = load_cases(args.cases)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    command = [str(args.cli.resolve()), "solve", str(args.dictionary.resolve()),
               str(args.unigrams.resolve()), str(args.bigrams.resolve()),
               str(args.wordnet.resolve()), "-", "--timeout-ms", str(args.timeout_ms)]
    with args.output.open("w", encoding="utf-8") as output:
        for case in cases.values():
            cache = args.output.parent / f"quality-cache-{uuid.uuid4().hex}.sqlite"
            for warm in (False, True):
                started = time.perf_counter()
                process = subprocess.run([*command, "--cache", str(cache.resolve())],
                                         input=json.dumps(request(case, args.candidate_budget)),
                                         text=True, capture_output=True, timeout=args.timeout_ms / 1000 + 30,
                                         check=False)
                wall_ms = (time.perf_counter() - started) * 1000
                response = json.loads(process.stdout)
                if process.returncode or "error" in response:
                    record = {"case_id": case["id"], "variant": "native_baseline_diverse_500",
                              "warm": warm, "wall_ms": wall_ms,
                              "error": response.get("error", response)}
                    output.write(json.dumps(record) + "\n")
                    output.flush()
                    print(f"{case['id']}: {'warm' if warm else 'cold'} failed: {record['error']}",
                          flush=True)
                    continue
                record = {"case_id": case["id"], "variant": "native_baseline_diverse_500",
                          "warm": warm, "wall_ms": wall_ms, "result": response}
                output.write(json.dumps(record) + "\n")
                output.flush()
                print(f"{case['id']}: {'warm' if warm else 'cold'} {wall_ms:.0f} ms",
                      flush=True)


if __name__ == "__main__":
    main()
