"""Run retained reference pipelines from any cwd with portable UTF-8 output."""
import argparse
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
ORACLE = ROOT / 'tests/oracle'
COMMANDS = ('anagram_solver', 'anagram_generate', 'anagram_rerank', 'anagram_benchmark',
            'train_feature_ranker', 'build_wikimedia_phrase_index', 'benchmark_user_runs',
            'ci_user_runs', 'ci_ordering_gate', 'ci_order_refinement', 'ci_performance_probe', 'ci_phrase_matrix')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=COMMANDS)
    parser.add_argument('arguments', nargs=argparse.REMAINDER, help='Forwarded unchanged to the reference command')
    args = parser.parse_args()
    temp = ROOT / '.codex/temp/oracle-runtime'; temp.mkdir(parents=True, exist_ok=True)
    environment = dict(os.environ, PYTHONIOENCODING='utf-8', TEMP=str(temp), TMP=str(temp))
    forwarded = args.arguments[1:] if args.arguments[:1] == ['--'] else args.arguments
    return subprocess.call([sys.executable, str(ORACLE / (args.command + '.py')), *forwarded],
                           cwd=ORACLE, env=environment)


if __name__ == '__main__': raise SystemExit(main())
