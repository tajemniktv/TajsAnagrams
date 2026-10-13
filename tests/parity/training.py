"""Native pairwise training weights and pooled held-out metrics versus Python."""
from dataclasses import asdict
import itertools
import json
from pathlib import Path
import random
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tests/oracle'))
from anagram_feature_ranker import RankGroup, RankItem, train_pairwise_ranker, cross_validate_pairwise_ranker
from scoring import close


def main():
    rng = random.Random(1310)
    groups = []
    for index in range(12):
        words = ['a'+chr(97+index), 'b'+chr(97+index), 'c'+chr(97+index)]
        orders = list(itertools.permutations(words)); rng.shuffle(orders)
        items = tuple(RankItem(' '.join(order), tuple(rng.randint(-20,20)/10 for _ in range(18)),
                               number < 2, rng.randint(-10,10)/10) for number,order in enumerate(orders))
        groups.append(RankGroup(' '.join(sorted(words)), items))
    rng.shuffle(groups)
    cases, expected = [], []
    for folds,epochs,l2 in itertools.product([2,3,5], [1,7], [0.0,0.002]):
        options = dict(folds=folds,epochs=epochs,learning_rate=0.08,l2=l2)
        baseline,held_out = cross_validate_pairwise_ranker(groups, **options)
        model = train_pairwise_ranker(groups, epochs=epochs,learning_rate=0.08,l2=l2)
        cases.append(dict(groups=[asdict(group) for group in groups],options=options))
        expected.append(dict(baseline=asdict(baseline),held_out=asdict(held_out),model=model.to_dict()))
    subprocess.run(['cargo','build','--locked','--example','training_probe'],cwd=ROOT,check=True)
    executable = ROOT / 'build/debug/examples' / ('training_probe.exe' if sys.platform == 'win32' else 'training_probe')
    output = subprocess.run([str(executable)],input='\n'.join(map(json.dumps,cases)),text=True,capture_output=True,check=True,timeout=60)
    actual = [json.loads(line) for line in output.stdout.splitlines()]
    close(actual,expected)
    print(f'Training parity passed: {len(cases)} optimizer/fold cases with exact held-out counts and 1e-12 weight tolerance')


if __name__ == '__main__': main()
