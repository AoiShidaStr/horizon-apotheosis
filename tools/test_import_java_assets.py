#!/usr/bin/env python3
"""Unit tests for the local Java asset importer and packer overlay."""
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

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

    def test_multipart_blockstate_falls_back_to_one_model(self):
        state = {
            "multipart": [
                {
                    "apply": {"model": "apothic_enchanting:block/filtering_shelf/base", "y": 90},
                    "when": {"facing": "east"},
                },
                {
                    "apply": {
                        "model": "apothic_enchanting:block/filtering_shelf/occupied",
                        "y": 0,
                    },
                    "when": {"AND": [{"facing": "north"}, {"slot_2_occupied": "true"}]},
                },
            ]
        }
        result, added = java.normalize_blockstate(state)
        self.assertTrue(added)
        self.assertNotIn("multipart", result)
        self.assertEqual(
            result["variants"][""]["model"],
            "apothic_enchanting:block/filtering_shelf/occupied",
        )


class AliasTests(unittest.TestCase):
    def test_an_explicit_alias_wins_over_a_same_named_java_asset(self):
        source = FakeSource(
            {
                "assets/apotheosis/blockstates/library.json": {},
                "assets/apotheosis/models/item/library.json": {},
            }
        )
        importer = java.Importer(source, Path("."))
        with mock.patch.dict(
            java.JAVA_ALIASES, {"library": ("apotheosis", "library")}
        ):
            self.assertEqual(
                importer.find_block("library"), ("apotheosis", "library")
            )
            self.assertEqual(
                importer.find_item("library"), ("apotheosis", "library")
            )


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
        # Java's fr_fr has no entry: the port's own French wins over Java's
        # English, so a French text is never replaced by an English one.
        self.assertEqual(
            fr["block.apotheosis.reforging_table"], "Notre table"
        )
        self.assertEqual(fr["apotheosis.our_key"], "Notre valeur")

    def test_java_french_beats_the_port_french(self):
        self.source.files["assets/apotheosis/lang/fr_fr.json"] = json.dumps(
            {"block.apotheosis.reforging_table": "Table Java"}
        )
        self.importer.import_lang()
        fr = json.loads(
            self.importer.files["assets/apotheosis/lang/fr_fr.json"]
        )
        self.assertEqual(fr["block.apotheosis.reforging_table"], "Table Java")


    def test_gem_item_names_come_from_java_texts(self):
        items = java.CONTENT / "assets" / java.OUR_NS / "items"
        items.mkdir(parents=True)
        for name in (
            "gem_solar_cracked", "gem_solar_normal", "gem_brawlers_flawed",
            "gem_verdant_ruin_perfect",
        ):
            (items / f"{name}.json").write_text("{}", encoding="utf-8")
        lang = java.CONTENT / "assets" / java.OUR_NS / "lang"
        fr_port = json.loads((lang / "fr_fr.json").read_text(encoding="utf-8"))
        fr_port["item.apotheosis.gem_verdant_ruin_perfect"] = "Notre gemme"
        (lang / "fr_fr.json").write_text(json.dumps(fr_port), encoding="utf-8")
        en = json.loads(self.source.files["assets/apotheosis/lang/en_us.json"])
        en.update({
            "item.apotheosis.gem.apotheosis:core/solar": "Solar Gem",
            "item.apotheosis.gem.apotheosis:core/brawlers": "Brawler's Gem",
            "item.apotheosis.gem.apotheosis:overworld/verdant_ruin":
                "Gem of the Verdant Ruin",
            "item.apotheosis.gem.cracked": "Cracked %s",
            "item.apotheosis.gem.flawed": "Flawed %s",
            "item.apotheosis.gem.normal": "%s",
            "item.apotheosis.gem.perfect": "Perfect %s",
        })
        self.source.files["assets/apotheosis/lang/en_us.json"] = json.dumps(en)
        # Java's fr_fr names the gems but has no per-purity format of that
        # name (only rarity-keyed ones), and no verdant_ruin gem.
        self.source.files["assets/apotheosis/lang/fr_fr.json"] = json.dumps({
            "item.apotheosis.gem.apotheosis:core/solar": "Gemme Solaire",
            "item.apotheosis.gem.apotheosis:core/brawlers": "Gemme du Bagarreur",
            "item.apotheosis.gem.apotheosis:common": "%s Fêlé",
        })
        self.importer.import_lang()
        files = self.importer.files
        en = json.loads(files["assets/apotheosis/lang/en_us.json"])
        fr = json.loads(files["assets/apotheosis/lang/fr_fr.json"])
        self.assertEqual(en["item.apotheosis.gem_solar_cracked"], "Cracked Solar Gem")
        self.assertEqual(en["item.apotheosis.gem_solar_normal"], "Solar Gem")
        self.assertEqual(
            en["item.apotheosis.gem_brawlers_flawed"], "Flawed Brawler's Gem"
        )
        self.assertEqual(
            en["item.apotheosis.gem_verdant_ruin_perfect"],
            "Perfect Gem of the Verdant Ruin",
        )
        self.assertEqual(fr["item.apotheosis.gem_solar_cracked"], "Gemme Solaire fêlée")
        self.assertEqual(fr["item.apotheosis.gem_solar_normal"], "Gemme Solaire")
        self.assertEqual(
            fr["item.apotheosis.gem_brawlers_flawed"],
            "Gemme imparfaite du Bagarreur",
        )
        # No Java French name: the port's own French stays.
        self.assertEqual(
            fr["item.apotheosis.gem_verdant_ruin_perfect"], "Notre gemme"
        )


class GemOverlayTests(unittest.TestCase):
    def test_overlay_never_ships_the_java_gem_item(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / "content"
            (root / "assets" / java.OUR_NS / "items").mkdir(parents=True)
            (root / "assets" / java.OUR_NS / "items" / "gem.json").write_text("{}")
            old = java.CONTENT
            java.CONTENT = root
            try:
                files = {
                    "assets/apotheosis/models/item/gem.json": json.dumps(
                        {"parent": "minecraft:item/generated",
                         "textures": {"layer0": "apotheosis:item/gems/default"}}
                    ),
                }

                class AnyTexture(FakeSource):
                    def exists(self, logical):
                        return logical in self.files or logical.startswith(
                            "assets/apotheosis/textures/"
                        )

                    def read(self, logical):
                        return b"png"

                importer = java.Importer(AnyTexture(files), Path(temp) / "out")
                importer.build()
            finally:
                java.CONTENT = old
            self.assertNotIn("assets/apotheosis/items/gem.json", importer.files)
            self.assertNotIn("assets/apotheosis/models/item/gem.json", importer.files)


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
