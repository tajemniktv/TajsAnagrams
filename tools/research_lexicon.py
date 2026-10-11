"""Offline ESDB lexical-tier prototype; does not change the production solver.

Use an ESDB checkout's `combine.py create-db scowl.db` first. The source commit,
database hash, license review and benchmark comparison remain report metadata.
"""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import sqlite3

if __package__:
    from tools.tajsbench import key, load_cases
else:
    from tajsbench import key, load_cases


def digest(path):
    h = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def ingest(db):
    con = sqlite3.connect(f"file:{Path(db).resolve().as_posix()}?mode=ro", uri=True)
    con.execute("pragma query_only=on")
    lexemes = {}
    sql = """select word, min(size), group_concat(distinct pos),
                    group_concat(distinct pos_class), group_concat(distinct spelling)
             from scowl_v0
             where size <= 80 and variant_level <= 1
               and spelling in ('A', '_') and region in ('', 'US')
             group by word"""
    for display, size, pos, pos_class, spelling in con.execute(sql):
        normalized = key(display)
        if not normalized or " " in display or "-" in display:
            # Open compounds need multi-token accounting and are tracked separately.
            continue
        tier = "core" if size <= 60 else "extended" if size <= 70 else "obscure"
        previous = lexemes.get(normalized)
        row = {"display": display, "key": normalized, "tier": tier, "size": size,
               "pos": pos or "", "pos_class": pos_class or "", "spelling": spelling or "",
               "proper_name": "upper" in (pos_class or "").split(",")}
        if previous is None or size < previous["size"]:
            lexemes[normalized] = row
    con.close()
    return lexemes


def report(lexemes, current, cases, db):
    baseline = {key(line.strip()) for line in Path(current).read_text(encoding="utf-8").splitlines()}
    baseline.discard("")
    tiers = Counter(row["tier"] for row in lexemes.values())
    desired = {key(word) for case in cases.values() for phrase in case["relevance"] for word in phrase.split()}
    return {
        "source_db_sha256": digest(db), "baseline_sha256": digest(current),
        "esdb_keys": len(lexemes), "baseline_keys": len(baseline),
        "tiers": dict(tiers), "shared": len(lexemes.keys() & baseline),
        "esdb_only": len(lexemes.keys() - baseline), "baseline_only": len(baseline - lexemes.keys()),
        "gold_words": len(desired), "gold_covered_esdb": len(desired & lexemes.keys()),
        "gold_covered_baseline": len(desired & baseline),
        "gold_missing_esdb": sorted(desired - lexemes.keys()),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", required=True, type=Path)
    parser.add_argument("--current", default=".anagram_data/dictionary/normal_user_v2.txt")
    parser.add_argument("--cases", default="tests/quality/cases.v1.json")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    lexemes = ingest(args.db)
    result = report(lexemes, args.current, load_cases(args.cases), args.db)
    content = json.dumps(result, indent=2) + "\n"
    if args.output:
        args.output.write_text(content, encoding="utf-8")
    else:
        print(content, end="")


if __name__ == "__main__":
    main()
