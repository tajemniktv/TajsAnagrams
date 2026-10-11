"""Offline order-5 ARPA backoff scorer for explicit retained-order pools.

This reader accepts a trusted, reviewed plain-text ARPA file. It never
downloads or executes a model. The production solver does not import it.
"""

import argparse
from collections import Counter
import json
from pathlib import Path


def contexts(orders):
    required = set()
    for order in orders:
        tokens = ["<s>", *order.lower().split(), "</s>"]
        for position in range(1, len(tokens)):
            for length in range(1, min(5, position + 1) + 1):
                required.add(tuple(tokens[position - length + 1:position + 1]))
                if length > 1:
                    required.add(tuple(tokens[position - length + 1:position]))
    required.add(("<unk>",))
    return required


def load_arpa(path, required):
    model = {}
    order = 0
    with Path(path).open("rt", encoding="utf-8") as stream:
        for raw in stream:
            line = raw.strip()
            if line.startswith("\\") and line.endswith("-grams:"):
                order = int(line[1:].split("-")[0])
                if not 1 <= order <= 5:
                    raise ValueError("unsupported n-gram order")
                continue
            if line.startswith("\\"):
                order = 0
                continue
            if not order or not line:
                continue
            fields = line.split()
            if len(fields) not in (order + 1, order + 2):
                raise ValueError(f"malformed {order}-gram row")
            phrase = tuple(fields[1:order + 1])
            if phrase in required:
                model[phrase] = (float(fields[0]), float(fields[-1]) if len(fields) == order + 2 else 0.0)
    return model


def conditional(model, sequence):
    """Return log10 probability, direct hit order, number of backoff steps."""
    history = tuple(sequence[-5:])
    backoff = 0.0
    steps = 0
    while len(history) > 1:
        row = model.get(history)
        if row is not None:
            return backoff + row[0], len(history), steps
        backoff += model.get(history[:-1], (0.0, 0.0))[1]
        history = history[1:]
        steps += 1
    row = model.get(history) or model.get(("<unk>",))
    if row is None:
        raise ValueError("ARPA model has neither queried unigram nor <unk>")
    return backoff + row[0], 1, steps


def score(model, order):
    words = order.lower().split()
    tokens = ["<s>", *words, "</s>"]
    probabilities = [conditional(model, tokens[max(0, i - 4):i + 1]) for i in range(1, len(tokens))]
    hits = Counter(hit for _, hit, _ in probabilities)
    total = sum(value for value, _, _ in probabilities)
    return {"phrase": order, "total_log10": total,
            "mean_log10_per_word": total / max(len(words), 1),
            "worst_log10": min(value for value, _, _ in probabilities),
            "direct_hits": {str(n): hits[n] for n in (2, 3, 4, 5)},
            "backoff_steps": sum(steps for _, _, steps in probabilities),
            "bos_log10": probabilities[0][0], "eos_log10": probabilities[-1][0]}


def score_groups(model, groups):
    output = []
    for group in groups:
        ranked = sorted((score(model, phrase) for phrase in group["orders"]),
                        key=lambda row: (-row["total_log10"], row["phrase"]))
        if len(ranked) > 1:
            margin = ranked[0]["total_log10"] - ranked[1]["total_log10"]
            for row in ranked:
                row["top_margin_log10"] = margin
        output.append({"case_id": group["case_id"], "orders": ranked})
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--arpa", required=True, type=Path)
    parser.add_argument("--groups", required=True, type=Path,
                        help="JSON array of {case_id, orders: [phrase, ...]}")
    args = parser.parse_args()
    groups = json.loads(args.groups.read_text(encoding="utf-8"))
    model = load_arpa(args.arpa, contexts(phrase for group in groups for phrase in group["orders"]))
    print(json.dumps(score_groups(model, groups), indent=2))


if __name__ == "__main__":
    main()
