import tempfile
import unittest
from pathlib import Path

from tools import research_ngram


ARPA = r"""\data\
ngram 1=5
ngram 2=3

\1-grams:
-1 <s> -0.1
-1 </s>
-3 <unk>
-1 good -0.2
-1 bad -0.2

\2-grams:
-0.1 <s> good -0.1
-1 <s> bad -0.1
-0.2 good </s>

\end\
"""


class NgramTests(unittest.TestCase):
    def test_direct_hit_and_backoff(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "synthetic.arpa"
            path.write_text(ARPA, encoding="utf-8")
            model = research_ngram.load_arpa(path, research_ngram.contexts(["good", "bad"]))
            good = research_ngram.score(model, "good")
            bad = research_ngram.score(model, "bad")
            self.assertGreater(good["total_log10"], bad["total_log10"])
            self.assertEqual(good["direct_hits"]["2"], 2)
            self.assertGreater(bad["backoff_steps"], 0)


if __name__ == "__main__":
    unittest.main()
