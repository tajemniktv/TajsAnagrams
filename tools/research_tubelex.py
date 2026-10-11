"""Compare TUBELEX frequency/diversity signals with the existing unigram data."""

import argparse
import csv
import hashlib
import json
import lzma
import math
from pathlib import Path

if __package__:
    from tools.tajsbench import key, load_cases
else:
    from tajsbench import key, load_cases


def sha256(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def read_tubelex(path):
    csv.field_size_limit(1_000_000)
    with lzma.open(path, "rt", encoding="utf-8") as stream:
        reader = csv.DictReader(stream, delimiter="\t")
        if not {"word", "count", "videos", "channels"} <= set(reader.fieldnames or []):
            raise ValueError("TUBELEX expected word/count/videos/channels columns")
        return {key(row["word"]): {field: int(row[field]) for field in ("count", "videos", "channels")}
                for row in reader if row["word"] != "[TOTAL]" and key(row["word"]) == row["word"]}


def read_unigrams(path):
    result = {}
    for line in Path(path).read_text(encoding="utf-8", errors="ignore").splitlines():
        parts = line.rsplit(maxsplit=1)
        if len(parts) == 2 and parts[1].isdigit() and key(parts[0]) == parts[0]:
            result[parts[0]] = result.get(parts[0], 0) + int(parts[1])
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tubelex", required=True, type=Path)
    parser.add_argument("--unigrams", default=".anagram_data/ngrams/count_1w.txt")
    parser.add_argument("--cases", default="tests/quality/cases.v1.json")
    args = parser.parse_args()
    tubelex = read_tubelex(args.tubelex)
    old = read_unigrams(args.unigrams)
    words = sorted({key(word) for case in load_cases(args.cases).values()
                    for phrase in case["relevance"] for word in phrase.split()})
    result = {"tubelex_sha256": sha256(args.tubelex), "old_sha256": sha256(args.unigrams),
              "tubelex_tokens": len(tubelex), "old_tokens": len(old),
              "gold_words": len(words), "tubelex_covered": sum(word in tubelex for word in words),
              "old_covered": sum(word in old for word in words),
              "features": {word: {"old_count": old.get(word, 0),
                                  "log_count": math.log1p(tubelex.get(word, {}).get("count", 0)),
                                  "log_videos": math.log1p(tubelex.get(word, {}).get("videos", 0)),
                                  "log_channels": math.log1p(tubelex.get(word, {}).get("channels", 0))}
                           for word in words}}
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
