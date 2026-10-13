"""Record a bounded native-solve TajsBench baseline (not a quality gate)."""

import argparse
import hashlib
import json
from pathlib import Path
import random
import subprocess
import time
import uuid

try:
    from tools.tajsbench import key, load_cases
except ModuleNotFoundError:
    from tajsbench import key, load_cases


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


def desktop_request(case, candidate_budget=10000):
    """Mirror Settings::default; the regression test detects source-default drift."""
    req = request(case, candidate_budget)
    req["generation"].update(required=[], hints=[], excluded=[], exclude_regex=[],
                             forbid_chars="", short_policy="common", extra_short_words=["hi"],
                             min_zipf=1.8, allow_repeat=False, strategy="prefix")
    req.update(phrase_bonus_max=6.0, result_limit_per_group=10)
    return req


def shuffled_case(case, seed):
    letters = list(key(case["source"]))
    random.Random(f"{seed}:{case['id']}").shuffle(letters)
    return {**case, "source": "".join(letters)}


def capture_run(command, req, timeout_ms):
    """Preserve adapter/process failures and continue the remaining scheduled cases."""
    started = time.perf_counter()
    record = {}
    try:
        process = subprocess.run(command, input=json.dumps(req), text=True, encoding="utf-8",
                                 capture_output=True, timeout=timeout_ms / 1000 + 30, check=False)
        record["returncode"] = process.returncode
        try:
            response = json.loads(process.stdout)
        except json.JSONDecodeError:
            response = None
        if not isinstance(response, dict):
            record["error"] = {"code": "invalid_response", "message": "CLI did not return a JSON object"}
            record["stdout"] = process.stdout[:2048]
        elif "error" in response:
            record["error"] = response["error"]
        elif process.returncode:
            record["error"] = {"code": "process_failed", "message": f"CLI exited with {process.returncode}"}
        else:
            record["result"] = response
        if process.stderr:
            record["stderr"] = process.stderr[:2048]
    except subprocess.TimeoutExpired:
        record["error"] = {"code": "process_timeout", "message": "CLI exceeded subprocess guard"}
    except OSError as error:
        record["error"] = {"code": "process_error", "message": str(error)}
    record["wall_ms"] = (time.perf_counter() - started) * 1000
    return record


def file_hash(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def manifest(args, cases):
    root = Path(__file__).resolve().parents[1]
    inputs = {"cli": args.cli, "dictionary": args.dictionary,
              "unigrams": args.unigrams, "bigrams": args.bigrams,
              "cases": Path(args.cases), "desktop_defaults_source": root / "apps/desktop/src-tauri/src/settings.rs"}
    inputs.update({f"wordnet/{path.name}": path for path in sorted(args.wordnet.iterdir()) if path.is_file()})
    metadata = {"schema_version": 1,
                "inputs": {name: {"sha256": file_hash(path), "bytes": path.stat().st_size}
                           for name, path in inputs.items()},
                "case_ids": list(cases), "os_filesystem_cache": "uncontrolled",
                "runtime": "CLI adapter, not installed desktop; no phrase DB or learned model",
                "binary_source_correspondence": "Hash recorded; build correspondence requires separate build evidence"}
    head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=root, capture_output=True, text=True, check=False)
    metadata["source_head"] = head.stdout.strip() if head.returncode == 0 else None
    dirty = subprocess.run(["git", "status", "--porcelain"], cwd=root, capture_output=True, text=True, check=False)
    metadata["source_dirty"] = bool(dirty.stdout.strip()) if dirty.returncode == 0 else None
    return metadata


def add_corpus_arguments(parser):
    parser.add_argument("--cli", default="build/release/anagram-cli.exe", type=Path)
    parser.add_argument("--dictionary", default=".anagram_data/dictionary/normal_user_v2.txt", type=Path)
    parser.add_argument("--unigrams", default=".anagram_data/ngrams/count_1w.txt", type=Path)
    parser.add_argument("--bigrams", default=".anagram_data/ngrams/count_2w.txt", type=Path)
    parser.add_argument("--wordnet", default=".anagram_data/wordnet31/dict", type=Path)
    parser.add_argument("--cases", default="tests/quality/cases.v1.json")


def cli_command(args, mode="solve"):
    command = [str(args.cli.resolve()), mode, str(args.dictionary.resolve()), str(args.unigrams.resolve())]
    if mode == "solve":
        command += [str(args.bigrams.resolve()), str(args.wordnet.resolve()), "-"]
    return [*command, "--timeout-ms", str(args.timeout_ms)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    add_corpus_arguments(parser)
    parser.add_argument("--output", default=".codex/temp/quality-baseline.jsonl", type=Path)
    parser.add_argument("--profile", choices=("desktop", "legacy"), default="desktop")
    parser.add_argument("--strategy", choices=("prefix", "diverse"))
    parser.add_argument("--candidate-budget", type=int)
    parser.add_argument("--timeout-ms", type=int)
    parser.add_argument("--repeats", default=1, type=int)
    parser.add_argument("--seed", default=0, type=int)
    parser.add_argument("--shuffle-input", action="store_true", help="Inventory-preserving control for source-order leakage")
    args = parser.parse_args()
    if args.candidate_budget is None:
        args.candidate_budget = 10000 if args.profile == "desktop" else 500
    if args.timeout_ms is None:
        args.timeout_ms = 120000 if args.profile == "desktop" else 5000
    if args.candidate_budget <= 0 or args.timeout_ms <= 0 or args.repeats <= 0:
        parser.error("candidate budget, timeout and repeats must be positive")
    cases = load_cases(args.cases)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    make_request = desktop_request if args.profile == "desktop" else request
    requests = {case_id: make_request(shuffled_case(case, args.seed) if args.shuffle_input else case, args.candidate_budget)
                for case_id, case in cases.items()}
    if args.strategy:
        for req in requests.values():
            req["generation"]["strategy"] = args.strategy
    strategy = next(iter(requests.values()))["generation"]["strategy"]
    variant = f"native_{args.profile}_{strategy}_{args.candidate_budget}_{args.timeout_ms}ms"
    if args.shuffle_input:
        variant += f"_shuffled_seed{args.seed}"
    metadata = manifest(args, cases)
    metadata.update(variant=variant, timeout_ms=args.timeout_ms, repeats=args.repeats,
                    seed=args.seed, shuffle_input=args.shuffle_input, requests=requests)
    args.output.with_suffix(".manifest.json").write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")
    command = cli_command(args)
    schedule = [(case_id, trial) for trial in range(args.repeats) for case_id in cases]
    random.Random(args.seed).shuffle(schedule)
    with args.output.open("w", encoding="utf-8") as output:
        for case_id, trial in schedule:
            cache = args.output.parent / f"quality-cache-{uuid.uuid4().hex}.sqlite"
            for warm in (False, True):
                record = {"case_id": case_id, "variant": variant, "trial": trial,
                          "warm": warm, "cache_state": "disk_cache_reuse" if warm else "disk_cache_empty",
                          "request": requests[case_id],
                          **capture_run([*command, "--cache", str(cache.resolve())], requests[case_id], args.timeout_ms)}
                output.write(json.dumps(record) + "\n")
                output.flush()
                outcome = record.get("error", "completed")
                print(f"{case_id}: trial {trial} {record['cache_state']} {record['wall_ms']:.0f} ms {outcome}",
                      flush=True)


if __name__ == "__main__":
    main()
