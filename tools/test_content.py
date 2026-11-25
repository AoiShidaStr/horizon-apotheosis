"""Regression tests for the generated content: the creative tab lists every
declared item and block exactly once, and nothing that is not declared.

Run: python tools/test_content.py
"""
import json
import os
import re
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA = os.path.join(ROOT, "content", "data", "apotheosis", "horizon")
PURITIES = ["cracked", "chipped", "flawed", "normal", "flawless", "perfect"]


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

    def test_every_gem_item_carries_its_identity_and_is_a_gem(self):
        folder = os.path.join(DATA, "item")
        names = sorted(name[:-5] for name in os.listdir(folder)
                       if re.fullmatch(r"gem_[a-z_]+_(%s)\.json" % "|".join(PURITIES), name))
        self.assertEqual(len(names), 107)
        with open(os.path.join(ROOT, "content", "data", "apotheosis", "tags", "item",
                               "gems.json"), encoding="utf-8") as handle:
            tag = json.load(handle)["values"]
        tab = tab_items()
        for name in names:
            with open(os.path.join(folder, name + ".json"), encoding="utf-8") as handle:
                components = json.load(handle)["components"]
            custom = components["minecraft:custom_data"]
            purity = name.rsplit("_", 1)[1]
            self.assertEqual(custom["apotheosis:gem_purity"], PURITIES.index(purity), name)
            self.assertTrue(custom["apotheosis:gem"].endswith("/" + name[4:-len(purity) - 1]))
            label = components["minecraft:item_name"]
            self.assertEqual(label["translate"], "item.apotheosis.gem." + custom["apotheosis:gem"])
            self.assertIs(label["italic"], False)
            self.assertIn("apotheosis:" + name, tag)
            self.assertTrue(os.path.exists(os.path.join(
                ROOT, "content", "assets", "apotheosis", "items", name + ".json")), name)
        # Java's order: each gem's purities together, after the generic gem.
        start = tab.index("apotheosis:gem") + 1
        listed = tab[start:start + len(names)]
        self.assertEqual(sorted(listed), ["apotheosis:" + n for n in names])
        for before, after in zip(listed, listed[1:]):
            if before.rsplit("_", 1)[0] == after.rsplit("_", 1)[0]:
                self.assertLess(PURITIES.index(before.rsplit("_", 1)[1]),
                                PURITIES.index(after.rsplit("_", 1)[1]))


if __name__ == "__main__":
    unittest.main()
