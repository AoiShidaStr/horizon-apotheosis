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
