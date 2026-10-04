"""The package's types hold (#332): ``mypy --strict`` passes on a typed
example program and on test_pythonic.py, whose docs snippets are the
Python guide's. Skipped without mypy, unless OPENBIM_IFC_REQUIRE_EXTRAS is
set, as scripts/check-python.sh does after installing it."""

import os
import subprocess
import sys
import tempfile
import unittest

import typed_example

HERE = os.path.dirname(os.path.abspath(__file__))
PROPERTIES = os.path.join(HERE, "..", "..", "..", "..", "test", "fixtures", "synthetic-properties", "synthetic_properties.ifc")
CHECKED = ("typed_example.py", "test_pythonic.py")


class Typing(unittest.TestCase):
    def test_the_typed_example_runs(self) -> None:
        walls, count, table = typed_example.main(PROPERTIES)
        self.assertEqual(walls, [(31, "Wall B")])
        self.assertEqual(count, 68)
        self.assertEqual(table["Pset_WallCommon.FireRating"], "F30")

    def test_mypy_strict_passes(self) -> None:
        try:
            import mypy  # noqa: F401
        except ImportError:
            if os.environ.get("OPENBIM_IFC_REQUIRE_EXTRAS"):
                raise
            self.skipTest("mypy is not installed")
        with tempfile.TemporaryDirectory() as cache:
            result = subprocess.run(
                [sys.executable, "-m", "mypy", "--strict", "--cache-dir", cache,
                 *(os.path.join(HERE, name) for name in CHECKED)],
                capture_output=True,
                text=True,
            )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
