#!/usr/bin/env python3
"""Write the gem item declarations, models, names and placeholder textures.

Run: python tools/gen_content.py

The textures are this port's own (flat 16x16 gem shapes drawn here); Apotheosis'
art is All Rights Reserved and is not used.
"""
import json
import os
import struct
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
ID = "apotheosis"
CONTENT = os.path.join(ROOT, "content")

# name, template, vanilla rarity, tooltip tier, colour (0xRRGGBB), shape rows
GEMS = [
    ("gem_ruby", "minecraft:amethyst_shard", "common", "Common", 0xD93A3A),
    ("gem_topaz", "minecraft:amethyst_shard", "uncommon", "Uncommon", 0xE0A038),
    ("gem_sapphire", "minecraft:amethyst_shard", "rare", "Rare", 0x3A6FD9),
    ("gem_jade", "minecraft:amethyst_shard", "epic", "Epic", 0x3FAF6A),
    ("gem_onyx", "minecraft:amethyst_shard", "epic", "Mythic", 0x6B3FA0),
    ("gem_bloodstone", "minecraft:amethyst_shard", "epic", "Ancient", 0x8C1F2F),
]

SHAPE = [
    "................",
    "................",
    ".....####.......",
    "....######......",
    "...########.....",
    "...########.....",
    "...########.....",
    "....######......",
    "....######......",
    ".....####.......",
    ".....####.......",
    "......##........",
    "................",
    "................",
    "................",
    "................",
]


def png(path, rgb):
    r, g, b = (rgb >> 16) & 0xFF, (rgb >> 8) & 0xFF, rgb & 0xFF
    rows = []
    for line in SHAPE:
        row = bytearray([0])
        for ch in line:
            if ch == "#":
                row += bytes((r, g, b, 255))
            else:
                row += b"\x00\x00\x00\x00"
        rows.append(bytes(row))
    raw = b"".join(rows)

    def chunk(tag, data):
        return (struct.pack(">I", len(data)) + tag + data
                + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF))

    data = (b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", struct.pack(">IIBBBBB", 16, 16, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9))
            + chunk(b"IEND", b""))
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(data)


def write_json(path, value):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        json.dump(value, f, indent=2, ensure_ascii=False)
        f.write("\n")


def main():
    names = {}
    for name, template, rarity, tier, colour in GEMS:
        write_json(os.path.join(CONTENT, "data", ID, "horizon", "item", name + ".json"), {
            "template": template,
            "max_stack_size": 1,
            "rarity": rarity,
        })
        write_json(os.path.join(CONTENT, "assets", ID, "items", name + ".json"), {
            "model": {"type": "minecraft:model", "model": "%s:item/%s" % (ID, name)}
        })
        write_json(os.path.join(CONTENT, "assets", ID, "models", "item", name + ".json"), {
            "parent": "minecraft:item/generated",
            "textures": {"layer0": "%s:item/%s" % (ID, name)},
        })
        png(os.path.join(CONTENT, "assets", ID, "textures", "item", name + ".png"), colour)
        names["item.%s.%s" % (ID, name)] = name.replace("_", " ").title()

    write_json(os.path.join(CONTENT, "data", ID, "horizon", "creative_tab", ID + ".json"), {
        "title": "itemGroup.%s" % ID,
        "icon": "%s:%s" % (ID, GEMS[-1][0]),
        "items": ["%s:%s" % (ID, g[0]) for g in GEMS],
    })
    names["itemGroup.%s" % ID] = "Apotheosis"

    fr = {
        "itemGroup.%s" % ID: "Apothéose",
        "item.%s.gem_ruby" % ID: "Rubis",
        "item.%s.gem_topaz" % ID: "Topaze",
        "item.%s.gem_sapphire" % ID: "Saphir",
        "item.%s.gem_jade" % ID: "Jade",
        "item.%s.gem_onyx" % ID: "Onyx",
        "item.%s.gem_bloodstone" % ID: "Héliotrope",
    }
    for lang, table in (("en_us", names), ("fr_fr", fr)):
        path = os.path.join(CONTENT, "assets", ID, "lang", lang + ".json")
        existing = {}
        if os.path.exists(path):
            with open(path, encoding="utf-8") as f:
                existing = json.load(f)
        existing.update(table)
        write_json(path, existing)
    print("wrote %d gems" % len(GEMS))


if __name__ == "__main__":
    main()
