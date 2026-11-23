#!/usr/bin/env python3
"""Unit tests for the local Java asset importer and packer overlay."""
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import import_java_assets as java  # noqa: E402
import pack  # noqa: E402


class FakeSource:
    def __init__(self, files):
        self.files = files

    def exists(self, logical):
        return logical in self.files

    def read_json(self, logical):
        return json.loads(self.files[logical])


class ModelTests(unittest.TestCase):
    def test_texture_folders_and_parents_are_normalized(self):
        model = {
            "parent": "block/cube_all",
            "textures": {
                "all": "apotheosis:blocks/foo",
                "particle": "minecraft:items/bar",
                "ref": "#all",
            },
        }
        result = java.normalize_model(model)
        self.assertEqual(result["parent"], "minecraft:block/cube_all")
        self.assertEqual(result["textures"]["all"], "apotheosis:block/foo")
        self.assertEqual(result["textures"]["particle"], "minecraft:item/bar")
        self.assertEqual(result["textures"]["ref"], "#all")
        # The source dict is not mutated.
        self.assertEqual(model["textures"]["all"], "apotheosis:blocks/foo")

    def test_default_variant_picks_unrotated_java_state(self):
        state = {
            "variants": {
                "facing=north": {"model": "apotheosis:block/table", "y": 180},
                "facing=south": {"model": "apotheosis:block/table", "y": 0},
            }
        }
        result, added = java.normalize_blockstate(state)
        self.assertTrue(added)
        self.assertIn("", result["variants"])
        self.assertEqual(result["variants"][""]["y"], 0)
        self.assertEqual(
            result["variants"][""]["model"], "apotheosis:block/table"
        )

    def test_existing_default_variant_is_kept(self):
        state = {"variants": {"": {"model": "apothic_enchanting:block/hellshelf"}}}
        result, added = java.normalize_blockstate(state)
        self.assertFalse(added)
        self.assertNotIn("facing=north", result["variants"])


class LanguageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.old_content = java.CONTENT
        self.addCleanup(setattr, java, "CONTENT", self.old_content)
        root = Path(self.temp.name) / "content"
        lang = root / "assets" / java.OUR_NS / "lang"
        lang.mkdir(parents=True)
        (lang / "en_us.json").write_text(
            json.dumps(
                {
                    "block.apotheosis.reforging_table": "Our Reforging Table",
                    "apotheosis.our_key": "Our value",
                }
            ),
            encoding="utf-8",
        )
        (lang / "fr_fr.json").write_text(
            json.dumps(
                {
                    "block.apotheosis.reforging_table": "Notre table",
                    "apotheosis.our_key": "Notre valeur",
                }
            ),
            encoding="utf-8",
        )
        java.CONTENT = root
        self.source = FakeSource(
            {
                "assets/apotheosis/lang/en_us.json": json.dumps(
                    {
                        "block.apotheosis.reforging_table": "Reforging Table",
                        "item.apotheosis.gem_dust": "Gem Dust",
                        "apotheosis.rarity.common": "Common",
                        "comment_id": "translator note",
                    }
                ),
                "assets/apothic_enchanting/lang/en_us.json": json.dumps(
                    {"block.apothic_enchanting.hellshelf": "Hellshelf"}
                ),
            }
        )
        self.importer = java.Importer(self.source, Path(self.temp.name) / "out")
        self.importer.block_map = {
            "reforging_table": ("apotheosis", "reforging_table")
        }
        self.importer.item_map = {"gem_dust": ("apotheosis", "gem_dust")}

    def test_our_keys_use_java_text_and_java_keys_are_added(self):
        self.importer.import_lang()
        en = json.loads(
            self.importer.files["assets/apotheosis/lang/en_us.json"]
        )
        self.assertEqual(
            en["block.apotheosis.reforging_table"], "Reforging Table"
        )
        self.assertEqual(en["item.apotheosis.gem_dust"], "Gem Dust")
        self.assertEqual(en["block.apothic_enchanting.hellshelf"], "Hellshelf")
        self.assertEqual(en["apotheosis.rarity.common"], "Common")
        self.assertNotIn("comment_id", en)
        # Our original key survives when Java has no equivalent.
        self.assertEqual(en["apotheosis.our_key"], "Our value")

    def test_missing_java_french_falls_back_to_java_english(self):
        self.importer.import_lang()
        fr = json.loads(
            self.importer.files["assets/apotheosis/lang/fr_fr.json"]
        )
        # Java's fr_fr has no entry, exactly as at runtime: use en_us.
        self.assertEqual(
            fr["block.apotheosis.reforging_table"], "Reforging Table"
        )
        self.assertEqual(fr["apotheosis.our_key"], "Notre valeur")


class PackOverlayTests(unittest.TestCase):
    def test_java_layer_overrides_content_and_no_java_skips_it(self):
        with tempfile.TemporaryDirectory() as temp:
            content = Path(temp) / "content"
            layer = Path(temp) / "content-java"
            for root in (content, layer):
                (root / "assets").mkdir(parents=True)
                (root / "data").mkdir(parents=True)
            (content / "assets" / "a.txt").write_text("content")
            (layer / "assets" / "a.txt").write_text("java")
            (layer / "data" / "b.txt").write_text("java data")

            merged = pack.content_files(True, str(content), str(layer))
            self.assertEqual(Path(merged["assets/a.txt"]).read_text(), "java")
            self.assertIn("data/b.txt", merged)

            base = pack.content_files(False, str(content), str(layer))
            self.assertEqual(Path(base["assets/a.txt"]).read_text(), "content")
            self.assertNotIn("data/b.txt", base)


if __name__ == "__main__":
    unittest.main()
