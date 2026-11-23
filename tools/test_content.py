"""Regression tests for the generated content: the creative tab lists every
declared item and block exactly once, and nothing that is not declared.

Run: python tools/test_content.py
"""
import json
import os
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA = os.path.join(ROOT, "content", "data", "apotheosis", "horizon")


def declared():
    ids = set()
    for kind in ("item", "block"):
        folder = os.path.join(DATA, kind)
        ids |= {"apotheosis:" + name[:-5] for name in os.listdir(folder) if name.endswith(".json")}
    return ids


def tab_items():
    with open(os.path.join(DATA, "creative_tab", "apotheosis.json"), encoding="utf-8") as handle:
        return json.load(handle)["items"]


class ContentTests(unittest.TestCase):
    def test_creative_tab_has_no_duplicates(self):
        items = tab_items()
        self.assertEqual(sorted({i for i in items if items.count(i) > 1}), [])

    def test_creative_tab_matches_declarations(self):
        items, ids = set(tab_items()), declared()
        self.assertEqual(sorted(ids - items), [], "declared but missing from the tab")
        self.assertEqual(sorted(items - ids), [], "in the tab but not declared")


if __name__ == "__main__":
    unittest.main()
