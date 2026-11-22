#!/usr/bin/env python3
"""Generate the clean-room Apothic Enchanting bookshelf content.

Run: python tools/gen_enchanting.py
The 16x16 shelf textures are drawn here; no upstream art or code is copied.
"""
import json
import os
import struct
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CONTENT = os.path.join(ROOT, "content")
ID = "apotheosis"

# block id, Eterna, max Eterna, Quanta, Arcana, English name, French name,
# recipe catalyst, RGB texture colour. Missing upstream maxEterna defaults to 30.
SHELVES = [
    ("basic_bookshelf", 2, 30, 0, 0, "Basic Bookshelf", "Bibliothèque simple", "minecraft:book", 0x9B633D),
    ("beeshelf", -30, 0, 100, 0, "Bee Shelf", "Bibliothèque des abeilles", "minecraft:honeycomb", 0xE5B532),
    ("blazing_hellshelf", 10, 65, 10, 0, "Blazing Hellshelf", "Bibliothèque infernale ardente", "minecraft:blaze_powder", 0xE24B24),
    ("crystal_seashelf", 5, 60, 3, 5, "Crystal Seashelf", "Bibliothèque marine cristalline", "minecraft:prismarine_crystals", 0x52D4C0),
    ("deepshelf", 5, 70, 5, 5, "Deepshelf", "Bibliothèque des profondeurs", "minecraft:sculk", 0x174B55),
    ("dormant_deepshelf", 2, 30, 0, 0, "Dormant Deepshelf", "Bibliothèque dormante des profondeurs", "minecraft:sculk", 0x29535A),
    ("draconic_endshelf", 20, 100, 0, 0, "Draconic Endshelf", "Bibliothèque draconique de l’End", "minecraft:dragon_head", 0x9A64D6),
    ("echoing_deepshelf", 5, 75, 0, 15, "Echoing Deepshelf", "Bibliothèque résonnante des profondeurs", "minecraft:echo_shard", 0x26899A),
    ("echoing_sculkshelf", 10, 80, 10, 15, "Echoing Sculkshelf", "Bibliothèque résonnante de sculk", "minecraft:echo_shard", 0x27A8A2),
    ("endshelf", 5, 90, 5, 5, "Endshelf", "Bibliothèque de l’End", "minecraft:ender_pearl", 0x8C5CBE),
    ("glowing_hellshelf", 5, 60, 5, 3.3333, "Glowing Hellshelf", "Bibliothèque infernale lumineuse", "minecraft:glowstone", 0xF08B35),
    ("heart_seashelf", 15, 60, 0, 20, "Heartforged Seashelf", "Bibliothèque marine au cœur forgé", "minecraft:heart_of_the_sea", 0x307DBA),
    ("hellshelf", 3, 45, 3, 0, "Hellshelf", "Bibliothèque infernale", "minecraft:blaze_rod", 0xA94431),
    ("infused_hellshelf", 5, 60, 5, 0, "Infused Hellshelf", "Bibliothèque infernale imprégnée", "minecraft:nether_wart", 0xB94E43),
    ("infused_seashelf", 5, 60, 0, 5, "Infused Seashelf", "Bibliothèque marine imprégnée", "minecraft:prismarine_shard", 0x37A5A0),
    ("melonshelf", -2, 0, -10, 0, "Melonshelf", "Bibliothèque de melon", "minecraft:melon_slice", 0x59A844),
    ("pearl_endshelf", 10, 90, 7.5, 7.5, "Pearl Endshelf", "Bibliothèque perlée de l’End", "minecraft:ender_pearl", 0xA68ADD),
    ("seashelf", 3, 45, 0, 3, "Seashelf", "Bibliothèque marine", "minecraft:pufferfish", 0x347E99),
    ("sightshelf", 0, 30, 0, 0, "Sightshelf", "Bibliothèque clairvoyante", "minecraft:spyglass", 0xC2A844),
    ("sightshelf_t2", 0, 30, 0, 0, "Sightshelf II", "Bibliothèque clairvoyante II", "minecraft:emerald", 0x75AA54),
    ("soul_touched_deepshelf", 5, 75, 15, 0, "Soul-Touched Deepshelf", "Bibliothèque des profondeurs touchée par l’âme", "minecraft:soul_lantern", 0x5B5E81),
    ("soul_touched_sculkshelf", 10, 80, 15, 10, "Soul-Touched Sculkshelf", "Bibliothèque de sculk touchée par l’âme", "minecraft:soul_lantern", 0x515C81),
    ("stoneshelf", -3, 0, 0, -7.5, "Stoneshelf", "Bibliothèque de pierre", "minecraft:polished_andesite", 0x777D81),
]


def write_json(path, value):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        json.dump(value, f, indent=2, ensure_ascii=False)
        f.write("\n")


def merge_values(path, additions):
    value = {"values": []}
    if os.path.exists(path):
        with open(path, encoding="utf-8") as f:
            value = json.load(f)
    values = value.setdefault("values", [])
    for item in additions:
        if item not in values:
            values.append(item)
    write_json(path, value)


def shelf_pixels(rgb):
    """Draw a 16x16 front view: warm wooden frame and five distinct books."""
    def shade(factor):
        return tuple(min(255, int(((rgb >> shift) & 255) * factor)) for shift in (16, 8, 0)) + (255,)
    wood, dark, light = shade(.72), shade(.42), shade(1.18)
    palette = [(0xB4, 0x36, 0x32, 255), (0x35, 0x68, 0xA5, 255),
               (0xD1, 0xA5, 0x43, 255), (0x4A, 0x8A, 0x54, 255),
               (0x8C, 0x53, 0xA6, 255)]
    pixels = [[wood for _ in range(16)] for _ in range(16)]
    for y in (0, 7, 14, 15):
        for x in range(16):
            pixels[y][x] = dark if y in (0, 15) else light
    spans = [(1, 3), (4, 6), (7, 9), (10, 12), (13, 14)]
    for i, (x0, x1) in enumerate(spans):
        top = 2 + (i % 2)
        for y in range(top, 7):
            for x in range(x0, x1 + 1):
                pixels[y][x] = palette[i]
        pixels[top][x0] = dark
        pixels[top][x1] = light
        for x in range(x0, x1 + 1):
            pixels[6][x] = dark if i % 2 else light
    for y in range(8, 14):
        for x in range(1, 15):
            if x not in (3, 6, 9, 12):
                pixels[y][x] = palette[(x + y) % len(palette)]
    for x in range(16):
        pixels[14][x] = dark
    return pixels


def write_png(path, pixels):
    rows = []
    for line in pixels:
        row = bytearray([0])
        for rgba in line:
            row.extend(rgba)
        rows.append(bytes(row))

    def chunk(kind, data):
        return (struct.pack(">I", len(data)) + kind + data
                + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF))

    raw = b"".join(rows)
    png = (b"\x89PNG\r\n\x1a\n"
           + chunk(b"IHDR", struct.pack(">IIBBBBB", 16, 16, 8, 6, 0, 0, 0))
           + chunk(b"IDAT", zlib.compress(raw, 9))
           + chunk(b"IEND", b""))
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(png)


def main():
    en, fr = {}, {}
    shelf_items = []
    axes = []
    for shelf, eterna, maximum, quanta, arcana, name_en, name_fr, catalyst, colour in SHELVES:
        block_id = f"{ID}:{shelf}"
        shelf_items.append(block_id)
        axes.append(block_id)
        write_json(os.path.join(CONTENT, "data", ID, "horizon", "block", shelf + ".json"), {
            "template": "minecraft:bookshelf",
            "item": True,
            "properties": {"hardness": 1.5, "explosion_resistance": 1.5,
                           "sounds": "minecraft:chiseled_bookshelf", "requires_tool": False},
        })
        write_json(os.path.join(CONTENT, "assets", ID, "blockstates", shelf + ".json"), {
            "variants": {"": {"model": f"{ID}:block/{shelf}"}},
        })
        write_json(os.path.join(CONTENT, "assets", ID, "models", "block", shelf + ".json"), {
            "parent": "minecraft:block/cube_all", "textures": {"all": f"{ID}:block/{shelf}"},
        })
        write_json(os.path.join(CONTENT, "assets", ID, "items", shelf + ".json"), {
            "model": {"type": "minecraft:model", "model": f"{ID}:block/{shelf}"},
        })
        write_png(os.path.join(CONTENT, "assets", ID, "textures", "block", shelf + ".png"), shelf_pixels(colour))
        write_json(os.path.join(CONTENT, "data", ID, "loot_table", "blocks", shelf + ".json"), {
            "type": "minecraft:block", "pools": [{"rolls": 1, "bonus_rolls": 0,
                "entries": [{"type": "minecraft:item", "name": block_id}]}],
        })
        # A vanilla bookshelf plus the shelf's theme keeps every block craftable
        # with vanilla recipes and avoids the unavailable custom infusion station.
        if shelf == "basic_bookshelf":
            recipe = {
                "type": "minecraft:crafting_shaped", "category": "building",
                "pattern": ["PPP", "BBB", "PPP"],
                "key": {"P": "minecraft:oak_planks", "B": "minecraft:book"},
                "result": {"id": block_id},
            }
        else:
            recipe = {
                "type": "minecraft:crafting_shaped", "category": "building",
                "pattern": [" C ", "CBC", " C "],
                "key": {"B": "minecraft:bookshelf", "C": catalyst},
                "result": {"id": block_id},
            }
        write_json(os.path.join(CONTENT, "data", ID, "recipe", shelf + ".json"), recipe)
        en[f"block.{ID}.{shelf}"] = name_en
        fr[f"block.{ID}.{shelf}"] = name_fr

    # Merge rather than replace shared artifacts from the other concurrent work.
    # The power-provider tag makes the shelves count at the player's own table;
    # the native hook adds Apotheosis Eterna for LAN guests, the only path
    # Horizon API 291 dispatches today. Bookshelves are axe-mined in vanilla.
    merge_values(os.path.join(CONTENT, "data", "minecraft", "tags", "block", "enchantment_power_provider.json"), shelf_items)
    merge_values(os.path.join(CONTENT, "data", "minecraft", "tags", "block", "mineable", "axe.json"), axes)
    tab_path = os.path.join(CONTENT, "data", ID, "horizon", "creative_tab", ID + ".json")
    if os.path.exists(tab_path):
        with open(tab_path, encoding="utf-8") as f:
            tab = json.load(f)
    else:
        tab = {"title": f"itemGroup.{ID}", "icon": f"{ID}:gem_opal", "items": []}
    entries = tab.setdefault("items", [])
    for item in shelf_items:
        if item not in entries:
            entries.append(item)
    write_json(tab_path, tab)

    for locale, additions in (("en_us", en), ("fr_fr", fr)):
        path = os.path.join(CONTENT, "assets", ID, "lang", locale + ".json")
        existing = {}
        if os.path.exists(path):
            with open(path, encoding="utf-8") as f:
                existing = json.load(f)
        existing.update(additions)
        write_json(path, existing)
    print(f"wrote {len(SHELVES)} enchanting shelves with original 16x16 textures")


if __name__ == "__main__":
    main()
