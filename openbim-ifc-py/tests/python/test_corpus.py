"""Corpus round trip: every committed fixture parses, writes, and re-parses
to the same model through Python."""

import pathlib
import unittest

from openbim_ifc import IfcModel

ROOT = pathlib.Path(__file__).resolve().parents[3]
FIXTURES = sorted((ROOT / "test" / "fixtures").rglob("*.ifc"))


class Corpus(unittest.TestCase):
    def test_the_corpus_is_not_empty(self):
        self.assertGreaterEqual(len(FIXTURES), 40, ROOT)

    def test_every_fixture_round_trips(self):
        for path in FIXTURES:
            with self.subTest(fixture=path.relative_to(ROOT).as_posix()):
                model = IfcModel.parse(path.read_bytes())
                again = IfcModel.parse(model.write())
                self.assertEqual(again.ids(), model.ids())
                for id in model.ids():
                    self.assertEqual(again.type_of(id), model.type_of(id))
                    self.assertEqual(again.attributes(id), model.attributes(id))


if __name__ == "__main__":
    unittest.main()
