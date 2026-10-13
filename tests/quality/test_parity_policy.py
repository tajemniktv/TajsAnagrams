import importlib
from pathlib import Path
import sys
import unittest
import tempfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tests/oracle'))
sys.path.insert(0, str(ROOT / 'tests/parity'))
from compatibility import native_rows
from scoring import close


class ParityPolicyTests(unittest.TestCase):
    def test_derived_metadata_requires_exact_recomputed_policy(self):
        import json
        from tools.reference_fixtures import verify_derived_policy
        import anagram_generate as generator
        import anagram_user_lexicon as lexical
        temp_root = ROOT / '.codex/temp'
        temp_root.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=temp_root) as directory:
            base = Path(directory); dictionary = base / 'dictionary'; counts = base / 'count_1w.txt'; policy = base / 'policy.json'
            dictionary.write_text('hi\ngo\n', encoding='utf-8'); counts.write_text('hi\t1000\ngo\t1000\n', encoding='utf-8')
            payload = dict(schema=2,policy_source=lexical._policy_source_token(),
                source_token=lexical._cache_source_token(generator.DEFAULT_DICT_URL, base),
                dictionary_stamp=list(lexical._source_stamp(dictionary)), unigram_stamp=list(lexical._source_stamp(counts)),
                extra_short_words=list(lexical.select_corpus_short_words(dictionary,generator.load_unigram_model(counts))))
            policy.write_text(json.dumps(payload),encoding='utf-8'); verify_derived_policy(policy,dictionary,counts)
            for bad in [dict(payload,extra_short_words=['zz']),dict(payload,source_token='invented'),dict(payload,unknown=True)]:
                policy.write_text(json.dumps(bad),encoding='utf-8')
                with self.assertRaises(RuntimeError): verify_derived_policy(policy,dictionary,counts)

    def test_presentation_is_verified_not_discarded(self):
        row = {'word_count': 2, 'best_order': ['dont', 'run'], 'final': 1.0}
        expected = native_rows(row, display=True)
        self.assertEqual(expected['display_phrase'], "don't run")
        close(expected, expected)
        for changed in [dict(expected, display_phrase='wrong'), dict(expected, unknown_field=True), dict(expected, final=2.0)]:
            with self.assertRaises(AssertionError): close(changed, expected)
        self.assertEqual(native_rows(row)['display_phrase'], '')

    def test_active_oracle_and_case_paths_do_not_require_archive(self):
        for name in ['anagram_paths', 'anagram_generate', 'anagram_rerank', 'anagram_suite',
                     'ci_phrase_matrix', 'ci_performance_probe', 'train_feature_ranker']:
            module = importlib.import_module(name)
            self.assertTrue(Path(module.__file__).resolve().is_relative_to(ROOT / 'tests/oracle'))
        import anagram_paths
        import anagram_suite
        self.assertEqual(anagram_paths.DATA_DIR, ROOT / '.anagram_data')
        self.assertEqual(anagram_suite.DEFAULT_CASES, ROOT / 'anagram_benchmarks.json')
        for base in [ROOT / 'tools', ROOT / 'tests/parity', ROOT / '.github/workflows']:
            for path in base.rglob('*'):
                if path.suffix not in {'.py', '.yml'}: continue
                source = path.read_text(encoding='utf-8')
                self.assertNotIn('working-directory: archive', source, str(path))
                self.assertNotIn('ROOT / "archive"', source, str(path))
                self.assertNotIn('ROOT/"archive"', source, str(path))
