#!/usr/bin/env python3
"""Write the gem and gem-block content: declarations, models, names, textures,
loot tables, recipes, tags and the creative tab.

Run: python tools/gen_content.py

The textures are this port's own (flat 16x16 shapes drawn here); Apotheosis'
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

# A gem: species, tier, colour (0xRRGGBB), off-hand attribute, amount, the
# vanilla item its recipe is built around, English and French names, and the
# effect line in both languages. Two gems per tier, so each tier has a choice.
GEMS = [
    ("ruby", "common", 0xD93A3A, "minecraft:attack_damage", 1.0,
     "minecraft:redstone", "Ruby", "Rubis",
     "In the off hand: +1 Attack Damage",
     "En main secondaire : +1 dégât d'attaque"),
    ("amber", "common", 0xC97B2E, "minecraft:mining_efficiency", 0.15,
     "minecraft:honeycomb", "Amber", "Ambre",
     "In the off hand: +15% Mining Speed",
     "En main secondaire : +15 % de vitesse de minage"),
    ("topaz", "uncommon", 0xE3C34B, "minecraft:movement_speed", 0.01,
     "minecraft:glowstone_dust", "Topaz", "Topaze",
     "In the off hand: +10% Movement Speed",
     "En main secondaire : +10 % de vitesse"),
    ("amethyst", "uncommon", 0x9B59D0, "minecraft:attack_speed", 0.4,
     "minecraft:amethyst_shard", "Amethyst", "Améthyste",
     "In the off hand: +10% Attack Speed",
     "En main secondaire : +10 % de vitesse d'attaque"),
    ("sapphire", "rare", 0x3A6FD9, "minecraft:max_health", 2.0,
     "minecraft:lapis_lazuli", "Sapphire", "Saphir",
     "In the off hand: +2 Max Health",
     "En main secondaire : +2 points de vie"),
    ("peridot", "rare", 0x9CCB3B, "minecraft:luck", 1.0,
     "minecraft:emerald", "Peridot", "Péridot",
     "In the off hand: +1 Luck",
     "En main secondaire : +1 chance"),
    ("jade", "epic", 0x3FAF6A, "minecraft:armor", 2.0,
     "minecraft:prismarine_shard", "Jade", "Jade",
     "In the off hand: +2 Armor",
     "En main secondaire : +2 armure"),
    ("citrine", "epic", 0xEFC44E, "minecraft:attack_knockback", 0.5,
     "minecraft:gold_ingot", "Citrine", "Citrine",
     "In the off hand: +0.5 Attack Knockback",
     "En main secondaire : +0,5 recul d'attaque"),
    ("onyx", "mythic", 0x5A3F8C, "minecraft:attack_damage", 3.0,
     "minecraft:obsidian", "Onyx", "Onyx",
     "In the off hand: +3 Attack Damage",
     "En main secondaire : +3 dégâts d'attaque"),
    ("moonstone", "mythic", 0xC9D6F0, "minecraft:max_health", 3.0,
     "minecraft:quartz", "Moonstone", "Pierre de lune",
     "In the off hand: +3 Max Health",
     "En main secondaire : +3 points de vie"),
    ("bloodstone", "ancient", 0x8C1F2F, "minecraft:max_health", 4.0,
     "minecraft:magma_cream", "Bloodstone", "Héliotrope",
     "In the off hand: +4 Max Health",
     "En main secondaire : +4 points de vie"),
    ("opal", "ancient", 0x9BD9CF, "minecraft:armor_toughness", 3.0,
     "minecraft:prismarine_crystals", "Opal", "Opale",
     "In the off hand: +3 Armor Toughness",
     "En main secondaire : +3 solidité d'armure"),
]

# Vanilla's `minecraft:rarity` names four colours; mythic and ancient take
# epic's, their tier is named in the tooltip.
VANILLA_RARITY = {
    "common": "common",
    "uncommon": "uncommon",
    "rare": "rare",
    "epic": "epic",
    "mythic": "epic",
    "ancient": "epic",
}

# How hard a gem block of each tier is, and how much it resists explosions.
BLOCK_HARDNESS = {
    "common": 1.5,
    "uncommon": 2.0,
    "rare": 2.5,
    "epic": 3.0,
    "mythic": 3.5,
    "ancient": 4.0,
}

# The gem silhouette, 16x16. '.' is transparent, '#' is the stone; the shades
# are filled in from the position within the shape.
GEM_SHAPE = [
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


def shade(rgb, factor):
    r, g, b = (rgb >> 16) & 0xFF, (rgb >> 8) & 0xFF, rgb & 0xFF
    return (
        min(255, max(0, int(r * factor))),
        min(255, max(0, int(g * factor))),
        min(255, max(0, int(b * factor))),
        255,
    )


def blank():
    return [[(0, 0, 0, 0)] * 16 for _ in range(16)]


def gem_pixels(rgb):
    """A gem silhouette shaded along its top-left / bottom-right axis."""
    pixels = blank()
    inside = {
        (x, y)
        for y, line in enumerate(GEM_SHAPE)
        for x, ch in enumerate(line)
        if ch == "#"
    }
    outline = shade(rgb, 0.45)
    base = shade(rgb, 1.0)
    light = shade(rgb, 1.35)
    dark = shade(rgb, 0.7)
    for (x, y) in inside:
        edge = any(
            (x + dx, y + dy) not in inside
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))
        )
        if edge:
            pixels[y][x] = outline
        elif x + y <= 10:
            pixels[y][x] = light
        elif x + y >= 17:
            pixels[y][x] = dark
        else:
            pixels[y][x] = base
    return pixels


def block_pixels(rgb):
    """A faceted 16x16 gem block: a dark border, diagonal light and dark bands,
    and a diamond facet line through the middle."""
    pixels = blank()
    outline = shade(rgb, 0.4)
    base = shade(rgb, 1.0)
    light = shade(rgb, 1.3)
    dark = shade(rgb, 0.72)
    bright = shade(rgb, 1.55)
    for y in range(16):
        for x in range(16):
            if x in (0, 15) or y in (0, 15):
                pixels[y][x] = outline
                continue
            diamond = abs(x - 7.5) + abs(y - 7.5)
            if 4.2 <= diamond <= 5.2:
                pixels[y][x] = bright
                continue
            band = (x + y) % 7
            if band in (0, 1):
                pixels[y][x] = light
            elif band in (4, 5, 6):
                pixels[y][x] = dark
            else:
                pixels[y][x] = base
    return pixels


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


def decl_dir(kind):
    return os.path.join(CONTENT, "data", ID, "horizon", kind)


def gem_item(species):
    return "%s:gem_%s" % (ID, species)


def block_item(species):
    return "%s:gem_block_%s" % (ID, species)


def fr_block(name):
    """`Bloc de rubis`, but `Bloc d'améthyste` before a vowel."""
    lowered = name.lower()
    if lowered[0] in "aeiouyéèêà":
        return "Bloc d'%s" % lowered
    return "Bloc de %s" % lowered


def recipe(name, value):
    write_json(os.path.join(CONTENT, "data", ID, "recipe", name + ".json"), value)


def main():
    en, fr = {}, {}
    tab = []

    for species, tier, colour, attribute, amount, catalyst, name_en, name_fr, effect_en, effect_fr in GEMS:
        item = gem_item(species)
        # Item, model, texture.
        declaration = {
            "template": "minecraft:amethyst_shard",
            "max_stack_size": 16,
            "rarity": VANILLA_RARITY[tier],
        }
        if tier in ("mythic", "ancient"):
            # The two tiers above vanilla's shimmer, so a high gem reads as high
            # in the inventory before its tooltip is opened.
            declaration["components"] = {"minecraft:enchantment_glint_override": True}
        write_json(os.path.join(decl_dir("item"), "gem_%s.json" % species), declaration)
        write_json(os.path.join(CONTENT, "assets", ID, "items", "gem_%s.json" % species), {
            "model": {"type": "minecraft:model", "model": "%s:item/gem_%s" % (ID, species)}
        })
        write_json(os.path.join(CONTENT, "assets", ID, "models", "item", "gem_%s.json" % species), {
            "parent": "minecraft:item/generated",
            "textures": {"layer0": "%s:item/gem_%s" % (ID, species)},
        })
        png(os.path.join(CONTENT, "assets", ID, "textures", "item", "gem_%s.png" % species),
            gem_pixels(colour))

        # A block of it, decorative and storable.
        block = "gem_block_%s" % species
        write_json(os.path.join(decl_dir("block"), block + ".json"), {
            "template": "minecraft:amethyst_block",
            "item": True,
            "properties": {
                "hardness": BLOCK_HARDNESS[tier],
                "explosion_resistance": BLOCK_HARDNESS[tier] * 2,
                "sounds": "minecraft:amethyst_block",
                "requires_tool": True,
            },
        })
        write_json(os.path.join(CONTENT, "assets", ID, "blockstates", block + ".json"), {
            "variants": {"": {"model": "%s:block/%s" % (ID, block)}}
        })
        write_json(os.path.join(CONTENT, "assets", ID, "models", "block", block + ".json"), {
            "parent": "minecraft:block/cube_all",
            "textures": {"all": "%s:block/%s" % (ID, block)},
        })
        write_json(os.path.join(CONTENT, "assets", ID, "items", block + ".json"), {
            "model": {"type": "minecraft:model", "model": "%s:block/%s" % (ID, block)}
        })
        png(os.path.join(CONTENT, "assets", ID, "textures", "block", block + ".png"),
            block_pixels(colour))
        write_json(os.path.join(CONTENT, "data", ID, "loot_table", "blocks", block + ".json"), {
            "type": "minecraft:block",
            "pools": [{
                "rolls": 1,
                "bonus_rolls": 0,
                "entries": [{"type": "minecraft:item", "name": block_item(species)}],
            }],
        })

        # Recipes: four gem dust and a catalyst make the gem; nine gems make
        # the block, and the block gives the nine gems back.
        recipe("gem_%s" % species, {
            "type": "minecraft:crafting_shaped",
            "category": "misc",
            "pattern": [" D ", "DCD", " D "],
            "key": {"D": "%s:gem_dust" % ID, "C": catalyst},
            "result": {"id": item},
        })
        recipe(block, {
            "type": "minecraft:crafting_shaped",
            "category": "building",
            "pattern": ["GGG", "GGG", "GGG"],
            "key": {"G": item},
            "result": {"id": block_item(species)},
        })
        recipe("gem_%s_from_block" % species, {
            "type": "minecraft:crafting_shapeless",
            "category": "misc",
            "ingredients": [block_item(species)],
            "result": {"id": item, "count": 9},
        })

        en["item.%s.gem_%s" % (ID, species)] = name_en
        en["block.%s.%s" % (ID, block)] = "%s Block" % name_en
        en["apotheosis.gem.%s.effect" % species] = effect_en
        fr["item.%s.gem_%s" % (ID, species)] = name_fr
        fr["block.%s.%s" % (ID, block)] = fr_block(name_fr)
        fr["apotheosis.gem.%s.effect" % species] = effect_fr

        tab.append(item)
    tab.append("%s:gem_dust" % ID)
    tab.extend(block_item(g[0]) for g in GEMS)

    # Gem dust: from amethyst, and salvaged from a gem.
    write_json(os.path.join(decl_dir("item"), "gem_dust.json"), {
        "template": "minecraft:gunpowder",
        "max_stack_size": 64,
        "rarity": "common",
    })
    write_json(os.path.join(CONTENT, "assets", ID, "items", "gem_dust.json"), {
        "model": {"type": "minecraft:model", "model": "%s:item/gem_dust" % ID}
    })
    write_json(os.path.join(CONTENT, "assets", ID, "models", "item", "gem_dust.json"), {
        "parent": "minecraft:item/generated",
        "textures": {"layer0": "%s:item/gem_dust" % ID},
    })
    png(os.path.join(CONTENT, "assets", ID, "textures", "item", "gem_dust.png"), dust_pixels())

    recipe("gem_dust_from_amethyst", {
        "type": "minecraft:crafting_shapeless",
        "category": "misc",
        "ingredients": ["minecraft:amethyst_shard"],
        "result": {"id": "%s:gem_dust" % ID, "count": 2},
    })
    recipe("gem_dust_from_gems", {
        "type": "minecraft:crafting_shapeless",
        "category": "misc",
        "ingredients": ["#%s:gems" % ID],
        "result": {"id": "%s:gem_dust" % ID, "count": 3},
    })

    en["item.%s.gem_dust" % ID] = "Gem Dust"
    fr["item.%s.gem_dust" % ID] = "Poussière de gemme"

    # The gem tag the salvaging recipe reads, and the pickaxe tag the blocks
    # need to drop with a pickaxe.
    write_json(os.path.join(CONTENT, "data", ID, "tags", "item", "gems.json"), {
        "values": [gem_item(g[0]) for g in GEMS],
    })
    write_json(os.path.join(CONTENT, "data", "minecraft", "tags", "block", "mineable", "pickaxe.json"), {
        "values": [block_item(g[0]) for g in GEMS],
    })

    write_json(os.path.join(decl_dir("creative_tab"), ID + ".json"), {
        "title": "itemGroup.%s" % ID,
        "icon": "%s:gem_opal" % ID,
        "items": tab,
    })
    en["itemGroup.%s" % ID] = "Apotheosis"
    fr["itemGroup.%s" % ID] = "Apothéose"

    for lang, table in (("en_us", en), ("fr_fr", fr)):
        path = os.path.join(CONTENT, "assets", ID, "lang", lang + ".json")
        existing = {}
        if os.path.exists(path):
            with open(path, encoding="utf-8") as f:
                existing = json.load(f)
        existing.update(table)
        write_json(path, existing)
    print("wrote %d gems, %d gem blocks and gem dust" % (len(GEMS), len(GEMS)))


if __name__ == "__main__":
    main()
