#!/usr/bin/env python3
"""Write the gem and gem-dust content: declarations, models, names, recipes,
tags and the creative tab.

Run: python tools/gen_content.py

The gems themselves are the Java ones: one `apotheosis:gem` item whose type
lives in its custom data, with a `minecraft:select` model that picks the
imported Java texture of that type (see `tools/gen_java_gems.py`).  The dust
texture is this port's own; Apotheosis' art is All Rights Reserved and is not
used.

The script is idempotent: it rewrites its own files, drops the files of the
gems this port used to invent, and leaves the other generators' entries in the
creative tab and in the pickaxe block tag alone.
"""
import json
import os
import re
import struct
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
ID = "apotheosis"
CONTENT = os.path.join(ROOT, "content")
RUST = os.path.join(ROOT, "native", "apotheosis", "api", "src", "java_gems.rs")

# The gems this port invented before the Java catalogue was imported; their
# files are removed so the pack only ships the Java gems.
OLD_SPECIES = (
    "ruby", "amber", "topaz", "amethyst", "sapphire", "peridot",
    "jade", "citrine", "onyx", "moonstone", "bloodstone", "opal",
)

# The dust heap, 16x16: '+' picks up the gem colours, '#' is pale dust.
DUST_SHAPE = [
    "................",
    "................",
    "................",
    "................",
    "................",
    ".......##.......",
    "......#..#......",
    ".....##+###.....",
    "....#+#.##.#....",
    "...##.###+##....",
    "...#++#####+....",
    "..##.#####.##...",
    "..#+########+...",
    "..############..",
    "................",
    "................",
]


def blank():
    return [[(0, 0, 0, 0)] * 16 for _ in range(16)]


def dust_pixels():
    """Pale gem dust with a few coloured flecks."""
    pixels = blank()
    pale = (0xE8, 0xE1, 0xD0, 255)
    pale_light = (0xFF, 0xFB, 0xEE, 255)
    flecks = [(0xD9, 0x3A, 0x3A, 255), (0x3A, 0x6F, 0xD9, 255), (0x3F, 0xAF, 0x6A, 255)]
    n = 0
    for y, line in enumerate(DUST_SHAPE):
        for x, ch in enumerate(line):
            if ch == "#":
                pixels[y][x] = pale_light if (x + y) % 5 == 0 else pale
            elif ch == "+":
                pixels[y][x] = flecks[n % len(flecks)]
                n += 1
    return pixels


def png(path, pixels):
    rows = []
    for line in pixels:
        row = bytearray([0])
        for (r, g, b, a) in line:
            row += bytes((r, g, b, a))
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


def read_json(path, fallback):
    if not os.path.exists(path):
        return fallback
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def decl_dir(kind):
    return os.path.join(CONTENT, "data", ID, "horizon", kind)


def gate(name, value):
    write_json(os.path.join(CONTENT, "data", ID, "recipe", name + ".json"), value)


def java_gem_ids():
    """The gem type ids of the generated catalogue, in its own order."""
    text = open(RUST, encoding="utf-8").read()
    return re.findall(r'id: "apotheosis:([^"]+)"', text)


def remove_old_gems():
    """Delete the files of the gems this port invented."""
    patterns = []
    for species in OLD_SPECIES:
        patterns += [
            os.path.join(decl_dir("item"), "gem_%s.json" % species),
            os.path.join(decl_dir("block"), "gem_block_%s.json" % species),
            os.path.join(CONTENT, "data", ID, "loot_table", "blocks",
                         "gem_block_%s.json" % species),
            os.path.join(CONTENT, "assets", ID, "items", "gem_%s.json" % species),
            os.path.join(CONTENT, "assets", ID, "items", "gem_block_%s.json" % species),
            os.path.join(CONTENT, "assets", ID, "blockstates",
                         "gem_block_%s.json" % species),
            os.path.join(CONTENT, "assets", ID, "models", "item",
                         "gem_%s.json" % species),
            os.path.join(CONTENT, "assets", ID, "models", "block",
                         "gem_block_%s.json" % species),
            os.path.join(CONTENT, "assets", ID, "textures", "item",
                         "gem_%s.png" % species),
            os.path.join(CONTENT, "assets", ID, "textures", "block",
                         "gem_block_%s.png" % species),
            os.path.join(CONTENT, "data", ID, "recipe", "gem_%s.json" % species),
            os.path.join(CONTENT, "data", ID, "recipe",
                         "gem_%s_from_block.json" % species),
            os.path.join(CONTENT, "data", ID, "recipe",
                         "gem_block_%s.json" % species),
        ]
    for path in patterns:
        if os.path.exists(path):
            os.remove(path)


def french_gem_name(gem_id):
    """`Gem of the Molten Breach` -> `Gemme de brèche en fusion`, roughly."""
    word = gem_id.split("/")[-1].replace("_", " ")
    return "Gemme de %s" % word


def english_gem_name(gem_id):
    word = gem_id.split("/")[-1].replace("_", " ")
    return "%s%s gem" % (word[:1].upper(), word[1:])


def main():
    remove_old_gems()
    ids = java_gem_ids()
    models = os.path.join(CONTENT, "assets", ID, "models", "item")

    # One item, whose model picks the texture of the gem type it carries.  The
    # native module writes the type id into `minecraft:custom_model_data`, which
    # is what the selector reads.
    write_json(os.path.join(decl_dir("item"), "gem.json"), {
        "template": "minecraft:amethyst_shard",
        "max_stack_size": 64,
        "rarity": "common",
    })
    cases = [
        {
            "when": "%s:%s" % (ID, gem_id),
            "model": {
                "type": "minecraft:model",
                "model": "%s:item/gem_%s" % (ID, gem_id.split("/")[-1]),
            },
        }
        for gem_id in ids
    ]
    write_json(os.path.join(CONTENT, "assets", ID, "items", "gem.json"), {
        "model": {
            "type": "minecraft:select",
            "property": "minecraft:custom_model_data",
            "index": 0,
            "cases": cases,
            "fallback": {"type": "minecraft:model", "model": "%s:item/gem_ballast" % ID},
        }
    })

    # Gem dust: from amethyst, and salvaged from any gem.
    write_json(os.path.join(decl_dir("item"), "gem_dust.json"), {
        "template": "minecraft:gunpowder",
        "max_stack_size": 64,
        "rarity": "common",
    })
    write_json(os.path.join(CONTENT, "assets", ID, "items", "gem_dust.json"), {
        "model": {"type": "minecraft:model", "model": "%s:item/gem_dust" % ID}
    })
    write_json(os.path.join(models, "gem_dust.json"), {
        "parent": "minecraft:item/generated",
        "textures": {"layer0": "%s:item/gem_dust" % ID},
    })
    png(os.path.join(CONTENT, "assets", ID, "textures", "item", "gem_dust.png"),
        dust_pixels())

    gate("gem_dust_from_amethyst", {
        "type": "minecraft:crafting_shapeless",
        "category": "misc",
        "ingredients": ["minecraft:amethyst_shard"],
        "result": {"id": "%s:gem_dust" % ID, "count": 2},
    })
    gate("gem_dust_from_gems", {
        "type": "minecraft:crafting_shapeless",
        "category": "misc",
        "ingredients": ["#%s:gems" % ID],
        "result": {"id": "%s:gem_dust" % ID, "count": 3},
    })

    # The per-purity gem items are added by `tools/gen_java_gems.py`; keep them.
    gems_tag_path = os.path.join(CONTENT, "data", ID, "tags", "item", "gems.json")
    gems_tag = read_json(gems_tag_path, {"values": []})
    values = gems_tag.get("values", [])
    if "%s:gem" % ID not in values:
        values = ["%s:gem" % ID] + values
    write_json(gems_tag_path, {"values": values})

    # The blocks this port invented are gone; keep the tables' entries.
    tag_path = os.path.join(CONTENT, "data", "minecraft", "tags", "block",
                            "mineable", "pickaxe.json")
    tag = read_json(tag_path, {"values": []})
    tag["values"] = [value for value in tag.get("values", [])
                     if not value.startswith("%s:gem_block_" % ID)]
    write_json(tag_path, tag)

    # The creative tab keeps the other generators' items and swaps the invented
    # gems for the single Java one.
    tab_path = os.path.join(decl_dir("creative_tab"), ID + ".json")
    tab = read_json(tab_path, {"title": "itemGroup.%s" % ID, "items": []})
    # Drop only the gems and gem blocks this port used to invent, not the other
    # gem_* entries (gem_cutting_table, gem_fused_slate...).
    stale_items = {"%s:gem_%s" % (ID, s) for s in OLD_SPECIES}
    stale_items |= {"%s:gem_block_%s" % (ID, s) for s in OLD_SPECIES}
    items = [item for item in tab.get("items", []) if item not in stale_items]
    for item in ("%s:gem" % ID, "%s:gem_dust" % ID):
        if item not in items:
            items.insert(0, item)
    tab["icon"] = "%s:gem" % ID
    tab["items"] = items
    write_json(tab_path, tab)
    # An older generator also read a tab at `data/apotheosis/creative_tab`; keep
    # it in step instead of leaving the invented gems there.
    write_json(os.path.join(CONTENT, "data", ID, "creative_tab", ID + ".json"), {
        "title": tab.get("title", "itemGroup.%s" % ID),
        "icon": "%s:gem" % ID,
        "items": ["%s:gem" % ID, "%s:gem_dust" % ID],
    })

    en, fr = {}, {}
    for gem_id in ids:
        key = "item.%s.gem.%s:%s" % (ID, ID, gem_id)
        en[key] = english_gem_name(gem_id)
        fr[key] = french_gem_name(gem_id)
    # The base name the item shows: the gem's own name is the key above.
    en["item.%s.gem" % ID] = "Gem"
    fr["item.%s.gem" % ID] = "Gemme"
    en["item.%s.gem_dust" % ID] = "Gem Dust"
    fr["item.%s.gem_dust" % ID] = "Poussière de gemme"

    stale = set()
    for species in OLD_SPECIES:
        stale |= {
            "item.%s.gem_%s" % (ID, species),
            "block.%s.gem_block_%s" % (ID, species),
            "apotheosis.gem.%s.effect" % species,
        }

    for lang, table in (("en_us", en), ("fr_fr", fr)):
        path = os.path.join(CONTENT, "assets", ID, "lang", lang + ".json")
        existing = read_json(path, {})
        for key in stale:
            existing.pop(key, None)
        existing.update(table)
        write_json(path, existing)
    print("wrote the gem item, %d gem names and gem dust" % len(ids))


if __name__ == "__main__":
    main()
