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

    def test_every_gem_item_is_named_and_is_a_gem(self):
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
            # Type and purity come from the item id (the native code parses it).
            purity = name.rsplit("_", 1)[1]
            label = components["minecraft:item_name"]
            # Its own name key, worded with its purity in each lang file.
            self.assertEqual(label["translate"], "item.apotheosis." + name)
            # Its colour is the purity's, set by the native tooltip hook: the client ignores one
            # declared here.
            self.assertNotIn("color", label)
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


    def test_gem_items_declare_only_their_name(self):
        # Horizon's client shows neither a declaration's lore nor its name colour: the native
        # tooltip hook writes Java's gem tooltip (gems::tooltip_lines). A lore here would show
        # those lines twice if it ever did; custom_data fails to load.
        folder = os.path.join(DATA, "item")
        names = [name for name in os.listdir(folder)
                 if re.fullmatch(r"gem_[a-z_]+_(%s)\.json" % "|".join(PURITIES), name)]
        self.assertEqual(len(names), 107)
        for name in names:
            with open(os.path.join(folder, name), encoding="utf-8") as handle:
                components = json.load(handle)["components"]
            self.assertNotIn("minecraft:lore", components, name)
            self.assertNotIn("minecraft:custom_data", components, name)
            self.assertEqual(sorted(components), ["minecraft:item_name"], name)
            self.assertEqual(components["minecraft:item_name"]["translate"],
                             "item.apotheosis." + name[:-5], name)

    def test_every_gem_class_key_is_translated(self):
        # gems::tooltip_lines translates gem_class.<key> for each bonus's Java key.
        with open(os.path.join(ROOT, "native", "apotheosis", "api", "src", "java_gems.rs"),
                  encoding="utf-8") as handle:
            keys = set(re.findall(r'^\s+key: "([a-z_]+)",$', handle.read(), re.M))
        self.assertIn("light_weapon", keys)
        for lang in ("en_us", "fr_fr"):
            with open(os.path.join(ROOT, "content", "assets", "apotheosis", "lang",
                                   lang + ".json"), encoding="utf-8") as handle:
                texts = json.load(handle)
            for key in keys:
                self.assertIn("gem_class." + key, texts, lang)

    def test_declarations_only_use_loadable_components(self):
        # Horizon refuses a whole mod's content when a declaration's default components contain
        # minecraft:custom_data ("not a known component type"), seen on a dedicated server.
        folder = os.path.join(ROOT, "content", "data", "apotheosis", "horizon", "item")
        for file in os.listdir(folder):
            with open(os.path.join(folder, file), encoding="utf-8") as f:
                components = json.load(f).get("components", {})
            self.assertNotIn("minecraft:custom_data", components, file)

    def test_menu_tables_use_a_template_that_opens_a_menu(self):
        # Horizon opens a mod menu only on a block whose template opens one.
        for table in ("reforging_table", "simple_reforging_table", "salvaging_table",
                      "gem_cutting_table"):
            with open(os.path.join(ROOT, "content", "data", "apotheosis", "horizon", "block",
                                   table + ".json"), encoding="utf-8") as handle:
                self.assertEqual(json.load(handle)["template"], "minecraft:crafting_table", table)

    def test_rarity_materials_and_dust_have_no_crafting_recipe(self):
        # Java has none: they come from salvaging. A recipe would bypass the
        # salvaging table (and ProjectE would make them free).
        folder = os.path.join(ROOT, "content", "data", "apotheosis", "recipe")
        for name in ("mysterious_scrap_metal", "timeworn_fabric", "luminous_crystal_shard",
                     "arcane_sands", "godforged_pearl"):
            for stem in (name, name + "_to_gem_dust"):
                self.assertFalse(os.path.exists(os.path.join(folder, stem + ".json")), stem)
        for stem in ("gem_dust_from_amethyst", "gem_dust_from_gems"):
            self.assertFalse(os.path.exists(os.path.join(folder, stem + ".json")), stem)

if __name__ == "__main__":
    unittest.main()
