#!/usr/bin/env python3
"""Write the salvage materials of each rarity: item declarations, models, own
16x16 textures, English and French names, recipes, and their place in the
creative tab.

Run: python tools/gen_sockets.py

The ids and the rarities they belong to are Apotheosis' (its `rarities/*.json`
name one `material` each); the art is this port's own, drawn here.
"""
import json
import os
import struct
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
ID = "apotheosis"
CONTENT = os.path.join(ROOT, "content")

# A material: id, the tier whose salvage yields it, a vanilla template, the
# English and French names, and where a survivor finds it.
MATERIALS = [
    ("mysterious_scrap_metal", "common", "minecraft:iron_nugget",
     "Mysterious Scrap Metal", "Ferraille mystérieuse",
     "Salvaged from common affixed gear."),
    ("timeworn_fabric", "uncommon", "minecraft:paper",
     "Timeworn Fabric", "Étoffe usée",
     "Salvaged from uncommon affixed gear."),
    ("luminous_crystal_shard", "rare", "minecraft:amethyst_shard",
     "Luminous Crystal Shard", "Éclat de cristal lumineux",
     "Salvaged from rare affixed gear."),
    ("arcane_sands", "epic", "minecraft:glowstone_dust",
     "Arcane Sands", "Sables arcaniques",
     "Salvaged from epic affixed gear."),
    ("godforged_pearl", "mythic", "minecraft:prismarine_crystals",
     "Godforged Pearl", "Perle divine",
     "Salvaged from mythic and ancient affixed gear."),
]

# English and French explanations, shown in the tooltip-free item's name only
# for now; the salvage message names the material it gave.
FRENCH_NOTES = {
    "mysterious_scrap_metal": "Récupérée sur un objet commun affixé.",
    "timeworn_fabric": "Récupérée sur un objet peu commun affixé.",
    "luminous_crystal_shard": "Récupérée sur un objet rare affixé.",
    "arcane_sands": "Récupérée sur un objet épique affixé.",
    "godforged_pearl": "Récupérée sur un objet mythique ou ancien affixé.",
}

# The native module's own messages and purity names, so a French client reads
# them in French: key -> (English, French). `%1$s` takes the first argument.
MESSAGES = {
    "apotheosis.purity.cracked": ("Cracked", "Fêlée"),
    "apotheosis.purity.chipped": ("Chipped", "Ébréchée"),
    "apotheosis.purity.flawed": ("Flawed", "Imparfaite"),
    "apotheosis.purity.normal": ("Normal", "Normale"),
    "apotheosis.purity.flawless": ("Flawless", "Sans défaut"),
    "apotheosis.purity.perfect": ("Perfect", "Parfaite"),
    "apotheosis.purity.line": ("Purity: %s", "Pureté : %s"),
    "apotheosis.socket.total": ("Sockets: %s/%s", "Sockets : %s/%s"),
    "apotheosis.socket.line": (
        "%2$s %1$s: %3$s %4$s",
        "%2$s %1$s : %3$s %4$s",
    ),
    "apotheosis.socket.done": ("Socketed %s (%s).", "Gemme sertie : %s (%s)."),
    "apotheosis.socket.full": (
        "No free socket there, or that gem is already in it.",
        "Pas de socket libre, ou cette gemme y est déjà.",
    ),
    "apotheosis.socket.hint": (
        "Right-click with this gem in hand and the gear in your off hand to socket it.",
        "Clic droit avec cette gemme en main et l'équipement en main secondaire pour la sertir.",
    ),
    "apotheosis.polish.done": ("Purity raised to %s.", "Pureté améliorée : %s."),
    "apotheosis.polish.hint": (
        "Right-click with gem dust in your off hand to raise its purity.",
        "Clic droit avec de la poussière de gemme en main secondaire pour augmenter sa pureté.",
    ),
    "apotheosis.reforge.already": (
        "That piece is neither plain gear nor an affixed piece of this mod.",
        "Cette pièce n'est ni de l'équipement simple, ni une pièce affixée de ce mod.",
    ),
    "apotheosis.reforge.rerolled": (
        "Reforged: the piece in your hand rolls again.",
        "Reforgé : la pièce tenue relance ses affixes.",
    ),
    "apotheosis.reforge.not_affixed": (
        "Hold an affixed piece to reforge it.",
        "Tenez un objet affixé pour le reforger.",
    ),
    "apotheosis.salvage.not_affixed": (
        "Hold an affixed piece to salvage it.",
        "Tenez un objet affixé pour le recycler.",
    ),
    "apotheosis.salvage.done": ("Salvaged into %s.", "Recyclé en %s."),
    "apotheosis.salvage.no_room": (
        "Some of it did not fit in your inventory.",
        "Une partie n'a pas tenu dans l'inventaire.",
    ),
    "apotheosis.command.help": (
        "Apotheosis: /apoth reforge — roll affixes onto the gear you hold (3 levels); "
        "/apoth salvage — break an affixed piece into rarity materials.",
        "Apothéose : /apoth reforge — relance les affixes de l'équipement tenu "
        "(3 niveaux) ; /apoth salvage — recycle un objet affixé en matériaux de rareté.",
    ),
}

# What each material is crafted from, so a player can reach the first one
# without a salvage: (recipe type, ingredients or pattern/key).
RECIPES = {
    "mysterious_scrap_metal": {
        "type": "minecraft:crafting_shapeless",
        "category": "misc",
        "ingredients": [
            "minecraft:iron_nugget",
            "minecraft:iron_nugget",
            "minecraft:iron_nugget",
            "minecraft:flint",
        ],
        "result": {"id": ID + ":mysterious_scrap_metal", "count": 2},
    },
    "timeworn_fabric": {
        "type": "minecraft:crafting_shapeless",
        "category": "misc",
        "ingredients": ["minecraft:string", "minecraft:string", "minecraft:leather"],
        "result": {"id": ID + ":timeworn_fabric", "count": 2},
    },
    "luminous_crystal_shard": {
        "type": "minecraft:crafting_shapeless",
        "category": "misc",
        "ingredients": [
            "minecraft:amethyst_shard",
            "minecraft:glowstone_dust",
            "minecraft:glowstone_dust",
        ],
        "result": {"id": ID + ":luminous_crystal_shard"},
    },
    "arcane_sands": {
        "type": "minecraft:crafting_shapeless",
        "category": "misc",
        "ingredients": [
            "minecraft:soul_sand",
            "minecraft:glowstone_dust",
            "minecraft:lapis_lazuli",
        ],
        "result": {"id": ID + ":arcane_sands", "count": 2},
    },
    "godforged_pearl": {
        "type": "minecraft:crafting_shaped",
        "category": "misc",
        "pattern": ["GGG", "GEG", "GGG"],
        "key": {"G": "minecraft:gold_ingot", "E": "minecraft:ender_pearl"},
        "result": {"id": ID + ":godforged_pearl"},
    },
}

# The scrap plate, 16x16: '#' is the metal, '+' a rivet, '.' nothing.
SCRAP_SHAPE = [
    "................",
    "................",
    "...#########....",
    "..#+++++++++#...",
    "..#+#.....#+#...",
    "..#+.......+#...",
    "..#+#.....#+#...",
    "..#+++++++++#...",
    "..#+#.....#+#...",
    "..#+.......+#...",
    "..#+#.....#+#...",
    "..#+++++++++#...",
    "...#########....",
    "................",
    "................",
    "................",
]

# The woven fabric, 16x16: alternating warp and weft threads.
FABRIC_SHAPE = [
    "................",
    "................",
    "..############..",
    "..#..#..#..#.#..",
    "..#.#..#..#..#..",
    "..#..#..#..#.#..",
    "..#.#..#..#..#..",
    "..#..#..#..#.#..",
    "..#.#..#..#..#..",
    "..#..#..#..#.#..",
    "..#.#..#..#..#..",
    "..############..",
    "................",
    "................",
    "................",
    "................",
]

# The crystal shard: a tall splinter with a bright core.
SHARD_SHAPE = [
    "................",
    ".......##.......",
    "......####......",
    "......#++#......",
    ".....#++++#.....",
    ".....#++++#.....",
    "....#++##++#....",
    "....#++##++#....",
    "...#++#..#++#...",
    "...#++#..#++#...",
    "..#++#....#++#..",
    "..#+#......#+#..",
    "..#+#......#+#..",
    "...#........#...",
    "................",
    "................",
]

# The sands: scattered grains with a few bright ones.
SAND_GRAINS = [
    (3, 6), (5, 9), (7, 4), (8, 8), (10, 6), (11, 10), (12, 5), (4, 12),
    (6, 13), (9, 12), (11, 13), (13, 8), (2, 9), (7, 10), (10, 3), (12, 11),
]

# The pearl: a round body with a highlight.
PEARL_SHAPE = [
    "................",
    "................",
    "......####......",
    "....########....",
    "...###++#####...",
    "..###++++#####..",
    "..###++#######..",
    "..############..",
    "..############..",
    "..############..",
    "...##########...",
    "....########....",
    "......####......",
    "................",
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


def from_shape(shape, base, light, dark, outline):
    pixels = blank()
    inside = {
        (x, y)
        for y, line in enumerate(shape)
        for x, ch in enumerate(line)
        if ch != "."
    }
    for (x, y) in inside:
        ch = shape[y][x]
        edge = any(
            (x + dx, y + dy) not in inside
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))
        )
        if ch == "+":
            pixels[y][x] = light
        elif edge:
            pixels[y][x] = outline
        elif x + y <= 12:
            pixels[y][x] = light
        elif x + y >= 22:
            pixels[y][x] = dark
        else:
            pixels[y][x] = base
    return pixels


def scrap_pixels():
    rgb = 0x8A8F98
    return from_shape(SCRAP_SHAPE, shade(rgb, 1.0), shade(rgb, 1.45),
                      shade(rgb, 0.7), shade(rgb, 0.42))


def fabric_pixels():
    rgb = 0xB9A47B
    pixels = blank()
    base = shade(rgb, 1.0)
    light = shade(rgb, 1.25)
    dark = shade(rgb, 0.72)
    for y, line in enumerate(FABRIC_SHAPE):
        for x, ch in enumerate(line):
            if ch == "#":
                pixels[y][x] = light if (x + y) % 4 == 0 else (
                    dark if (x + y) % 4 == 2 else base)
    return pixels


def shard_pixels():
    rgb = 0x7FE6E0
    return from_shape(SHARD_SHAPE, shade(rgb, 1.0), shade(rgb, 1.5),
                      shade(rgb, 0.65), shade(rgb, 0.4))


def sand_pixels():
    pixels = blank()
    grain = shade(0x9A6BD9, 1.0)
    bright = shade(0xC9A6F2, 1.0)
    for (x, y) in SAND_GRAINS:
        pixels[y][x] = bright if (x + y) % 3 == 0 else grain
        if x + 1 < 16 and (x * y) % 2 == 0:
            pixels[y][x + 1] = grain
    return pixels


def pearl_pixels():
    rgb = 0xE8C36A
    return from_shape(PEARL_SHAPE, shade(rgb, 1.0), shade(rgb, 1.5),
                      shade(rgb, 0.62), shade(rgb, 0.4))


TEXTURES = {
    "mysterious_scrap_metal": scrap_pixels,
    "timeworn_fabric": fabric_pixels,
    "luminous_crystal_shard": shard_pixels,
    "arcane_sands": sand_pixels,
    "godforged_pearl": pearl_pixels,
}


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


def read_json(path, default):
    if not os.path.exists(path):
        return default
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def write_json(path, value):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        json.dump(value, f, indent=2, ensure_ascii=False)
        f.write("\n")


def main():
    en, fr = {}, {}
    for (material, tier, template, name_en, name_fr, note_en) in MATERIALS:
        write_json(os.path.join(CONTENT, "data", ID, "horizon", "item", material + ".json"), {
            "template": template,
            "max_stack_size": 64,
            "rarity": "common",
        })
        write_json(os.path.join(CONTENT, "assets", ID, "items", material + ".json"), {
            "model": {"type": "minecraft:model", "model": "%s:item/%s" % (ID, material)}
        })
        write_json(os.path.join(CONTENT, "assets", ID, "models", "item", material + ".json"), {
            "parent": "minecraft:item/generated",
            "textures": {"layer0": "%s:item/%s" % (ID, material)},
        })
        png(os.path.join(CONTENT, "assets", ID, "textures", "item", material + ".png"),
            TEXTURES[material]())
        write_json(os.path.join(CONTENT, "data", ID, "recipe", material + ".json"),
                   RECIPES[material])
        # A material grinds back into dust, the currency a purity upgrade takes.
        write_json(os.path.join(CONTENT, "data", ID, "recipe",
                                material + "_to_gem_dust.json"), {
            "type": "minecraft:crafting_shapeless",
            "category": "misc",
            "ingredients": ["%s:%s" % (ID, material)],
            "result": {"id": "%s:gem_dust" % ID, "count": 3},
        })
        en["item.%s.%s" % (ID, material)] = name_en
        fr["item.%s.%s" % (ID, material)] = name_fr
        en["apotheosis.material.%s.note" % material] = note_en
        fr["apotheosis.material.%s.note" % material] = FRENCH_NOTES[material]

    for key, (text_en, text_fr) in MESSAGES.items():
        en[key] = text_en
        fr[key] = text_fr

    # The creative tab is shared with the gem content: add what is missing and
    # leave the rest of the list alone.
    tab_path = os.path.join(CONTENT, "data", ID, "horizon", "creative_tab", ID + ".json")
    tab = read_json(tab_path, {"title": "itemGroup.%s" % ID, "icon": "%s:gem_opal" % ID,
                               "items": []})
    items = list(tab.get("items", []))
    for (material, *_rest) in MATERIALS:
        item = "%s:%s" % (ID, material)
        if item not in items:
            items.append(item)
    tab["items"] = items
    write_json(tab_path, tab)

    for lang, table in (("en_us", en), ("fr_fr", fr)):
        path = os.path.join(CONTENT, "assets", ID, "lang", lang + ".json")
        existing = read_json(path, {})
        existing.update(table)
        write_json(path, existing)
    print("wrote %d salvage materials" % len(MATERIALS))


if __name__ == "__main__":
    main()
