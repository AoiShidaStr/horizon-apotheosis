#!/usr/bin/env python3
"""Regression tests for the lang files: every key the code reads exists, legacy keys are gone,
and the tracked content/ tree carries no copy of a Java asset."""
import json
import os
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
NATIVE = ROOT / "native"
LANG = ROOT / "content" / "assets" / "apotheosis" / "lang"
JAVA_REF = Path("C:/Users/yello/horizon-work/java-ref/Apotheosis-1.21.1-8.9.0")

TRANSLATE = re.compile(r'translate_with_fallback\(\s*"(apotheosis\.[^"]+)"')
# The table screens' tooltip lines (menus.rs `line(key, ...)`); `container.*` keys are vanilla's.
MENU_LINE = re.compile(r'\bline\(\s*"((?!container\.)[^"]+)"')

# The shelf tooltip lines (enchanting.rs `shelf_tooltip_lines`) keep Java's key names.
SHELF_STAT = re.compile(r'"(info\.apothic_enchanting\.[a-z_.]+)"')

# Legacy keys that no code reads any more (proof: grep over native/ and content/).
LEGACY_FIXED = ["apotheosis.tooltip.tier", "apotheosis.affix.line", "apotheosis.reforge.not_affixed"]
LEGACY_PATTERNS = [
    re.compile(r"^apotheosis\.affix\.(?!stat$|effect\.)[a-z_]+(\.desc)?$"),
    re.compile(r"^apotheosis\.material\.[a-z_]+\.note$"),
]


def load_lang(name):
    with open(LANG / name, encoding="utf-8") as handle:
        return json.load(handle)


def native_keys():
    keys = set()
    for path in NATIVE.rglob("*.rs"):
        keys.update(TRANSLATE.findall(path.read_text(encoding="utf-8")))
    return keys


class LangKeyTests(unittest.TestCase):
    def test_every_translated_key_exists_in_both_lang_files(self):
        missing = []
        for name in ("en_us.json", "fr_fr.json"):
            lang = load_lang(name)
            missing += [f"{name}:{key}" for key in sorted(native_keys()) if key not in lang]
        self.assertEqual(missing, [])

    def test_every_menu_tooltip_key_exists_in_both_lang_files(self):
        menus = (NATIVE / "apotheosis" / "api" / "src" / "menus.rs").read_text(encoding="utf-8")
        keys = set(MENU_LINE.findall(menus))
        self.assertIn("text.apotheosis.reforge_cost", keys)
        for name in ("en_us.json", "fr_fr.json"):
            lang = load_lang(name)
            self.assertEqual(sorted(k for k in keys if k not in lang), [], name)

    def test_every_shelf_stat_key_exists_in_both_lang_files(self):
        enchanting = (NATIVE / "apotheosis" / "api" / "src" / "enchanting.rs").read_text(encoding="utf-8")
        keys = set(SHELF_STAT.findall(enchanting))
        self.assertIn("info.apothic_enchanting.eterna.p", keys)
        for name in ("en_us.json", "fr_fr.json"):
            lang = load_lang(name)
            self.assertEqual(sorted(k for k in keys if k not in lang), [], name)

    def test_gem_items_have_their_item_name_key(self):
        for name in ("en_us.json", "fr_fr.json"):
            lang = load_lang(name)
            purities = ("cracked", "chipped", "flawed", "normal", "flawless", "perfect")
            decls = [d for d in sorted((ROOT / "content" / "data" / "apotheosis" / "horizon" / "item")
                                       .glob("gem_*.json")) if d.stem.rsplit("_", 1)[-1] in purities]
            self.assertEqual(len(decls), 107)
            for decl in decls:
                gem = json.loads(decl.read_text(encoding="utf-8"))["components"]["minecraft:item_name"]["translate"]
                self.assertEqual(lang.get(f"item.apotheosis.{decl.stem}"), lang[gem], f"{name}:{decl.stem}")

    def test_legacy_keys_are_removed_from_both_files(self):
        for name in ("en_us.json", "fr_fr.json"):
            lang = load_lang(name)
            left = [key for key in lang if key in LEGACY_FIXED
                    or any(p.match(key) for p in LEGACY_PATTERNS)]
            self.assertEqual(left, [], name)

    def test_legacy_keys_are_not_read_by_native_code(self):
        used = native_keys()
        self.assertFalse(used & set(LEGACY_FIXED))
        self.assertEqual([k for k in used if any(p.match(k) for p in LEGACY_PATTERNS)], [])

    def test_sigil_of_withdrawal_description_is_original_english(self):
        lang = load_lang("en_us.json")
        self.assertNotEqual(lang["item.apotheosis.sigil_of_withdrawal.desc"],
                            "Removes all socketed gems from an item")


class LicenseTests(unittest.TestCase):
    def test_tracked_blockstates_are_not_java_copies(self):
        if not JAVA_REF.is_dir():
            self.skipTest("Java reference extract absent")
        for name in ("reforging_table.json", "salvaging_table.json"):
            java = JAVA_REF / "assets" / "apotheosis" / "blockstates" / name
            ours = LANG.parent.parent / "blockstates" / name
            if ours.exists():
                self.assertNotEqual(json.loads(ours.read_text(encoding="utf-8")),
                                    json.loads(java.read_text(encoding="utf-8")), name)


if __name__ == "__main__":
    unittest.main()
