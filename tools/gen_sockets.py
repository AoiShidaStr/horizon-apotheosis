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
        "%2$s %1$s: %3$s",
        "%2$s %1$s : %3$s",
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
    "apotheosis.salvage.not_affixed": (
        "Hold an affixed piece to salvage it.",
        "Tenez un objet affixé pour le recycler.",
    ),
    "apotheosis.salvage.done": ("Salvaged into %s.", "Recyclé en %s."),
    "apotheosis.salvage.gem": (
        "Salvaged into %s gem dust.",
        "Recyclé en %s poussière de gemme.",
    ),
    "apotheosis.unnaming.none": (
        "That piece has no name to remove.",
        "Cet objet n'a aucun nom à retirer.",
    ),
    "apotheosis.unnaming.custom": (
        "That name was not written by affixes; it is left alone.",
        "Ce nom n'a pas été écrit par les affixes ; il est conservé.",
    ),
    "apotheosis.unnaming.done": (
        "The affix name was removed.",
        "Le nom d'affixe a été retiré.",
    ),
    "apotheosis.table.reforge.rarity": (
        "This table cannot reforge that rarity; use the reforging table.",
        "Cette table ne peut pas reforger cette rareté ; utilisez la table de reforger.",
    ),
    "apotheosis.reforge.missing": (
        "You do not have the materials this reforge needs.",
        "Vous n'avez pas les matériaux nécessaires à ce reforger.",
    ),
    "apotheosis.salvage.no_room": (
        "Some of it did not fit in your inventory.",
        "Une partie n'a pas tenu dans l'inventaire.",
    ),
    "apotheosis.socket.max": (
        "That piece already has all %s sockets.",
        "Cette pièce a déjà tous ses sockets (%s).",
    ),
    "apotheosis.socket.added": (
        "Socket added: %s/%s.",
        "Socket ajouté : %s/%s.",
    ),
    "apotheosis.socket.none": (
        "That piece has no gem in its sockets.",
        "Cette pièce n'a aucune gemme sertie.",
    ),
    "apotheosis.socket.withdrawn": (
        "%s gem(s) taken back out.",
        "%s gemme(s) retirée(s).",
    ),
    "apotheosis.seal.no_piece": (
        "Hold an affixed piece in your off hand to use a sigil on it.",
        "Tenez un objet affixé en main secondaire pour y utiliser un sceau.",
    ),
    "apotheosis.enhance.done": ("Enhanced: %s.", "Amélioré : %s."),
    "apotheosis.enhance.max": (
        "That piece is already ancient.",
        "Cette pièce est déjà ancienne.",
    ),
    "apotheosis.cut.no_dust": (
        "The gem cutting table needs %s gem dust and the purity materials.",
        "La table de taille demande %s poussière de gemme.",
    ),
    "apotheosis.table.reforge.hint": (
        "The reforging table works on the piece of gear in your hand.",
        "La table de recréation agit sur l'équipement tenu en main.",
    ),
    "apotheosis.table.salvage.hint": (
        "The salvaging table breaks down the affixed piece in your hand.",
        "La table de récupération recycle l'objet affixé tenu en main.",
    ),
    "apotheosis.table.cut.hint": (
        "The gem cutting table sharpens the gem in your hand.",
        "La table de taille aiguise la gemme tenue en main.",
    ),
    "apotheosis.command.help": (
        "Apotheosis: /apoth reforge — roll affixes onto the gear you hold (3 levels); "
        "/apoth salvage — break an affixed piece into rarity materials; "
        "/apoth world_tier — show or change your world tier.",
        "Apothéose : /apoth reforge — relance les affixes de l'équipement tenu "
        "(3 niveaux) ; /apoth salvage — recycle un objet affixé en matériaux de rareté ; "
        "/apoth world_tier — affiche ou change votre palier de monde.",
    ),
}

# A sigil: id, the rarity its item shows, English and French names and
# descriptions, the wax colour and the symbol drawn on it.
SEALS = [
    ("sigil_of_socketing", "common", "Sigil of Socketing",
     "Sceau de sertissage", "Adds one socket to an item",
     "Ajoute un socket à un objet", 0xC98A3A, "plus"),
    ("sigil_of_withdrawal", "uncommon", "Sigil of Withdrawal",
     "Sceau de retrait", "Pulls every socketed gem back out of an item",
     "Retire toutes les gemmes serties d'un objet", 0x8A9BA8, "minus"),
    ("sigil_of_rebirth", "rare", "Sigil of Rebirth",
     "Sceau de renaissance", "Rerolls an affixed item's rarity and affixes",
     "Relance la rareté et les affixes d'un objet affixé", 0x9A4FD0, "star"),
    ("sigil_of_enhancement", "epic", "Sigil of Enhancement",
     "Sceau d'amélioration", "Raises an affixed item's rarity by one step",
     "Augmente la rareté d'un objet affixé d'un cran", 0xD8483F, "arrow"),
    ("sigil_of_unnaming", "rare", "Sigil of Unnaming",
     "Sceau d'effacement", "Removes the affix name from an item",
     "Retire le nom d'affixe d'un objet", 0x627A8B, "cross"),
]

# Java's boss summoner: no recipe (it comes from creative or a command), a
# stack of 64, the vanilla default item. Its texture is Java's when the local
# asset layer is imported; this port draws a stand-in for the --no-java pack.
SUMMONER = (
    "boss_summoner", "Apothic Invader Summoner", "Invocateur de Boss Apothéotique",
    "Summons an Apothic Invader on the block you click",
    "Invoque un Invader Apothéotique sur le bloc cliqué",
    0x4B2A6E, "eye",
)

SEAL_RECIPES = {
    "sigil_of_socketing": {
        "type": "minecraft:crafting_shaped",
        "category": "misc",
        "group": "apotheosis",
        "pattern": ["GUG", "EEE", "GAG"],
        "key": {"G": ID + ":gem_dust", "U": "minecraft:gunpowder",
                "E": ID + ":gem_fused_slate", "A": "minecraft:amethyst_shard"},
        "result": {"id": ID + ":sigil_of_socketing", "count": 3},
    },
    "sigil_of_withdrawal": {
        "type": "minecraft:crafting_shaped",
        "category": "misc",
        "group": "apotheosis",
        "pattern": ["GBG", "ELE", "GMG"],
        "key": {"G": ID + ":gem_fused_slate", "B": "minecraft:blaze_rod",
                "E": {"tag": "c:ender_pearls"}, "L": "minecraft:lava_bucket",
                "M": ID + ":gem_dust"},
        "result": {"id": ID + ":sigil_of_withdrawal", "count": 4},
    },
    "sigil_of_rebirth": {
        "type": "minecraft:crafting_shaped",
        "category": "misc",
        "group": "apotheosis",
        "pattern": ["GGG", "EEE", "GGG"],
        "key": {"G": ID + ":gem_fused_slate", "E": ID + ":gem_dust"},
        "result": {"id": ID + ":sigil_of_rebirth", "count": 6},
    },
    "sigil_of_enhancement": {
        "type": "minecraft:crafting_shaped",
        "category": "misc",
        "group": "apotheosis",
        "pattern": ["GEG", "EOE", "GEG"],
        "key": {"G": ID + ":gem_dust", "E": ID + ":gem_fused_slate",
                "O": ID + ":godforged_pearl"},
        "result": {"id": ID + ":sigil_of_enhancement", "count": 4},
    },
    "sigil_of_unnaming": {
        "type": "minecraft:crafting_shaped",
        "category": "misc",
        "group": "apotheosis",
        "pattern": ["GGG", "FFF", "GGG"],
        "key": {"G": ID + ":gem_fused_slate", "F": "minecraft:flint"},
        "result": {"id": ID + ":sigil_of_unnaming", "count": 6},
    },
}

# A working table: id, English and French names, the base and emblem colours,
# and its recipe.
MATERIAL_ITEMS = [
    ("gem_fused_slate", "minecraft:deepslate", "Gem-Fused Slate", "Ardoise fusionnée",
     {"type": "minecraft:crafting_shaped", "category": "misc", "group": "apotheosis",
      "pattern": ["DDD", "DGD", "DDD"],
      "key": {"D": "minecraft:deepslate", "G": ID + ":gem_dust"},
      "result": {"id": ID + ":gem_fused_slate", "count": 8}}),
    ("iron_upgrade_smithing_template", "minecraft:netherite_upgrade_smithing_template",
     "Iron Upgrade Template", "Modèle d'amélioration en fer",
     {"type": "minecraft:crafting_shaped", "category": "misc", "group": "apotheosis",
      "pattern": [" M ", "SGS", "SGS"],
      "key": {"M": ID + ":mysterious_scrap_metal", "S": "minecraft:stone",
              "G": ID + ":gem_fused_slate"},
      "result": {"id": ID + ":iron_upgrade_smithing_template", "count": 2}}),
    ("gold_upgrade_smithing_template", "minecraft:netherite_upgrade_smithing_template",
     "Gold Upgrade Template", "Modèle d'amélioration en or",
     {"type": "minecraft:crafting_shaped", "category": "misc", "group": "apotheosis",
      "pattern": [" T ", "SGS", "SGS"],
      "key": {"T": ID + ":timeworn_fabric", "S": "minecraft:stone",
              "G": ID + ":gem_fused_slate"},
      "result": {"id": ID + ":gold_upgrade_smithing_template", "count": 2}}),
    ("diamond_upgrade_smithing_template", "minecraft:netherite_upgrade_smithing_template",
     "Diamond Upgrade Template", "Modèle d'amélioration en diamant",
     {"type": "minecraft:crafting_shaped", "category": "misc", "group": "apotheosis",
      "pattern": [" L ", "SGS", "SGS"],
      "key": {"L": ID + ":luminous_crystal_shard", "S": "minecraft:stone",
              "G": ID + ":gem_fused_slate"},
      "result": {"id": ID + ":diamond_upgrade_smithing_template", "count": 2}}),
]

TABLES = [
    ("simple_reforging_table", "Simple Reforging Table", "Table simple de recréation",
     0x5D4937, 0xE8C36A),
    ("gem_cutting_table", "Gem Cutting Table", "Table de taille de gemme",
     0x5A3E7A, 0xC9A6F2),
    ("salvaging_table", "Salvaging Table", "Table de récupération",
     0x5B5F63, 0xC2CBD2),
    ("reforging_table", "Reforging Table", "Table de recréation",
     0x6B4A2A, 0xE8C36A),
]

TABLE_RECIPES = {
    "simple_reforging_table": {
        "type": "minecraft:crafting_shaped",
        "category": "misc",
        "group": "apotheosis",
        "pattern": [" I ", "GEG", "SSS"],
        "key": {"I": {"tag": "c:ingots/iron"}, "G": ID + ":gem_dust",
                "E": "minecraft:enchanting_table", "S": "minecraft:smooth_stone"},
        "result": {"id": ID + ":simple_reforging_table"},
    },
    "gem_cutting_table": {
        "type": "minecraft:crafting_shaped",
        "category": "misc",
        "pattern": ["I I", "ADA", "I I"],
        "key": {"I": "minecraft:iron_ingot", "A": "minecraft:amethyst_shard",
                "D": ID + ":gem_dust"},
        "result": {"id": ID + ":gem_cutting_table"},
    },
    "salvaging_table": {
        "type": "minecraft:crafting_shaped",
        "category": "misc",
        "pattern": ["III", "I I", "PPP"],
        "key": {"I": "minecraft:iron_ingot", "P": "minecraft:oak_planks"},
        "result": {"id": ID + ":salvaging_table"},
    },
    "reforging_table": {
        "type": "minecraft:crafting_shaped",
        "category": "misc",
        "pattern": ["GGG", "GSG", "SSS"],
        "key": {"G": "minecraft:gold_ingot", "S": ID + ":mysterious_scrap_metal"},
        "result": {"id": ID + ":reforging_table"},
    },
}

# The symbols stamped on a sigil, 7x7, 'x' the symbol.
SEAL_GLYPHS = {
    "plus": [
        ".......",
        "...x...",
        "...x...",
        ".xxxxx.",
        "...x...",
        "...x...",
        ".......",
    ],
    "minus": [
        ".......",
        ".......",
        ".......",
        ".xxxxx.",
        ".......",
        ".......",
        ".......",
    ],
    "star": [
        "...x...",
        "...x...",
        ".xxxxx.",
        "..xxx..",
        ".xx.xx.",
        ".x...x.",
        ".......",
    ],
    "arrow": [
        "...x...",
        "..xxx..",
        ".x.x.x.",
        "...x...",
        "...x...",
        "...x...",
        ".......",
    ],
    "cross": [
        "x.....x",
        ".x...x.",
        "..x.x..",
        "...x...",
        "..x.x..",
        ".x...x.",
        "x.....x",
    ],
    "eye": [
        ".......",
        "..xxx..",
        ".x...x.",
        "x..x..x",
        ".x...x.",
        "..xxx..",
        ".......",
    ],
}

# The table top, 16x16: '#' the rim, '+' the field, 'x' the emblem, and any
# other character leaves the texture's own field.
TABLE_EMBLEMS = {
    "simple_reforging_table": [
        "................",
        "................",
        "......xxxx......",
        ".....x....x.....",
        "....x..xx..x....",
        "....x.x..x.x....",
        "....x.x..x.x....",
        "....x..xx..x....",
        ".....x....x.....",
        "......xxxx......",
        "................",
        "................",
        "................",
        "................",
        "................",
        "................",
    ],
    "gem_cutting_table": [
        "................",
        "................",
        "......xxxx......",
        ".....x....x.....",
        "....x..xx..x....",
        "....x.x..x.x....",
        "....x.x..x.x....",
        "....x..xx..x....",
        ".....x....x.....",
        "......xxxx......",
        "................",
        "................",
        "................",
        "................",
        "................",
        "................",
    ],
    "salvaging_table": [
        "................",
        "................",
        "....xxxxxxxx....",
        "....x......x....",
        "....x.x..x.x....",
        "....x..xx..x....",
        "....x.x..x.x....",
        "....x......x....",
        "....xxxxxxxx....",
        "................",
        "................",
        "................",
        "................",
        "................",
        "................",
        "................",
    ],
    "reforging_table": [
        "................",
        "................",
        "...xxxxxxxxxx...",
        "...x........x...",
        "...x..xxxx..x...",
        "...x..x..x..x...",
        "...x..x..x..x...",
        "...x..xxxx..x...",
        "...x........x...",
        "...xxxxxxxxxx...",
        "................",
        "................",
        "................",
        "................",
        "................",
        "................",
    ],
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


def seal_pixels(colour, glyph):
    """A wax disc with a stamped symbol."""
    pixels = blank()
    for y in range(16):
        for x in range(16):
            dx, dy = x - 7.5, y - 7.5
            d = (dx * dx + dy * dy) ** 0.5
            if d <= 5.1:
                pixels[y][x] = shade(colour, 1.18 if dx + dy < -1 else 1.0)
            elif d <= 6.1:
                pixels[y][x] = shade(colour, 0.62)
    for gy, row in enumerate(SEAL_GLYPHS[glyph]):
        for gx, ch in enumerate(row):
            if ch == "x":
                pixels[4 + gy][4 + gx] = shade(0xFFF3D6, 1.0)
    return pixels


def table_pixels(base, emblem, shape):
    """A full table top: a rim, a checkered field and a stamped emblem."""
    pixels = blank()
    field = shade(base, 1.0)
    weave = shade(base, 0.9)
    rim = shade(base, 1.35)
    dark = shade(base, 0.55)
    for y in range(16):
        for x in range(16):
            pixels[y][x] = field if (x + y) % 2 else weave
    for i in range(16):
        pixels[0][i] = pixels[15][i] = rim
        pixels[i][0] = pixels[i][15] = rim
        pixels[1][i] = pixels[14][i] = field
        pixels[i][1] = pixels[i][14] = field
    pixels[0][0] = pixels[0][15] = pixels[15][0] = pixels[15][15] = dark
    ink = shade(emblem, 1.0)
    edge = shade(emblem, 0.55)
    for y, row in enumerate(shape):
        for x, ch in enumerate(row):
            if ch == "x":
                pixels[y][x] = ink
                for (nx, ny) in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
                    if 0 <= nx < 16 and 0 <= ny < 16 and shape[ny][nx] != "x":
                        if pixels[ny][nx] != ink and pixels[ny][nx] != rim:
                            pixels[ny][nx] = edge
    return pixels


def table_side_pixels(base):
    """The table's sides: stacked boards with a rim, under the top."""
    pixels = blank()
    seam = shade(base, 0.5)
    rim = shade(base, 1.2)
    for y in range(16):
        for x in range(16):
            pixels[y][x] = shade(base, 0.82 if (y // 4) % 2 else 0.7)
    for y in (3, 7, 11):
        for x in range(16):
            pixels[y][x] = seam
    for i in range(16):
        pixels[0][i] = rim
        pixels[15][i] = seam
        pixels[i][0] = rim
        pixels[i][15] = seam
    return pixels


TEXTURES = {
    "mysterious_scrap_metal": scrap_pixels,
    "timeworn_fabric": fabric_pixels,
    "luminous_crystal_shard": shard_pixels,
    "arcane_sands": sand_pixels,
    "godforged_pearl": pearl_pixels,
}

for (_seal, _rarity, _en, _fr, _den, _dfr, _colour, _glyph) in SEALS:
    TEXTURES[_seal] = (lambda c=_colour, g=_glyph: seal_pixels(c, g))

TEXTURES[SUMMONER[0]] = (lambda c=SUMMONER[5], g=SUMMONER[6]: seal_pixels(c, g))

for (_table, _en, _fr, _base, _emblem) in TABLES:
    TEXTURES[_table] = (
        lambda b=_base, e=_emblem, s=TABLE_EMBLEMS[_table]: table_pixels(b, e, s)
    )
    TEXTURES[_table + "_side"] = (lambda b=_base: table_side_pixels(b))


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
        # No crafting recipe, as in Java: a rarity material comes from salvaging
        # affixed gear (and bosses), never from a recipe or from dust.
        for stale in (material + ".json", material + "_to_gem_dust.json"):
            path = os.path.join(CONTENT, "data", ID, "recipe", stale)
            if os.path.exists(path):
                os.remove(path)
        en["item.%s.%s" % (ID, material)] = name_en
        fr["item.%s.%s" % (ID, material)] = name_fr

    for (seal, rarity, name_en, name_fr, desc_en, desc_fr, _colour, _glyph) in SEALS:
        write_json(os.path.join(CONTENT, "data", ID, "horizon", "item", seal + ".json"), {
            "template": "minecraft:paper",
            "max_stack_size": 16,
            "rarity": rarity,
        })
        write_json(os.path.join(CONTENT, "assets", ID, "items", seal + ".json"), {
            "model": {"type": "minecraft:model", "model": "%s:item/%s" % (ID, seal)}
        })
        write_json(os.path.join(CONTENT, "assets", ID, "models", "item", seal + ".json"), {
            "parent": "minecraft:item/generated",
            "textures": {"layer0": "%s:item/%s" % (ID, seal)},
        })
        png(os.path.join(CONTENT, "assets", ID, "textures", "item", seal + ".png"),
            TEXTURES[seal]())
        write_json(os.path.join(CONTENT, "data", ID, "recipe", seal + ".json"),
                   SEAL_RECIPES[seal])
        en["item.%s.%s" % (ID, seal)] = name_en
        fr["item.%s.%s" % (ID, seal)] = name_fr
        en["item.%s.%s.desc" % (ID, seal)] = desc_en
        fr["item.%s.%s.desc" % (ID, seal)] = desc_fr

    (summoner, name_en, name_fr, desc_en, desc_fr, _colour, _glyph) = SUMMONER
    write_json(os.path.join(CONTENT, "data", ID, "horizon", "item", summoner + ".json"), {
        "template": "minecraft:paper",
        "max_stack_size": 64,
        "rarity": "common",
    })
    write_json(os.path.join(CONTENT, "assets", ID, "items", summoner + ".json"), {
        "model": {"type": "minecraft:model", "model": "%s:item/%s" % (ID, summoner)}
    })
    write_json(os.path.join(CONTENT, "assets", ID, "models", "item", summoner + ".json"), {
        "parent": "minecraft:item/generated",
        "textures": {"layer0": "%s:item/%s" % (ID, summoner)},
    })
    png(os.path.join(CONTENT, "assets", ID, "textures", "item", summoner + ".png"),
        TEXTURES[summoner]())
    # No recipe, as in Java: creative mode or a command is the only source.
    stale = os.path.join(CONTENT, "data", ID, "recipe", summoner + ".json")
    if os.path.exists(stale):
        os.remove(stale)
    en["item.%s.%s" % (ID, summoner)] = name_en
    fr["item.%s.%s" % (ID, summoner)] = name_fr
    en["item.%s.%s.desc" % (ID, summoner)] = desc_en
    fr["item.%s.%s.desc" % (ID, summoner)] = desc_fr

    # Vanilla-shaped recipes and their Java item ids; generated here so repeated
    # runs do not depend on the local asset importer. The item models point at
    # a vanilla texture until `tools/import_java_assets.py` layers the Java ones
    # over `content-java/`; nothing here is a copy of a Java asset.
    for name, template, name_en, name_fr, recipe in MATERIAL_ITEMS:
        write_json(os.path.join(CONTENT, "data", ID, "horizon", "item", name + ".json"), {
            "template": template,
            "max_stack_size": 64,
            "rarity": "common",
        })
        write_json(os.path.join(CONTENT, "assets", ID, "items", name + ".json"), {
            "model": {"type": "minecraft:model", "model": "%s:item/%s" % (ID, name)}
        })
        if name == "gem_fused_slate":
            model = {"parent": "minecraft:block/deepslate"}
        else:
            model = {
                "parent": "minecraft:item/generated",
                "textures": {
                    "layer0": "minecraft:item/netherite_upgrade_smithing_template"
                },
            }
        write_json(os.path.join(CONTENT, "assets", ID, "models", "item", name + ".json"),
                   model)
        write_json(os.path.join(CONTENT, "data", ID, "recipe", name + ".json"), recipe)
        en["item.%s.%s" % (ID, name)] = name_en
        fr["item.%s.%s" % (ID, name)] = name_fr

    for (table, name_en, name_fr, _base, _emblem) in TABLES:
        write_json(os.path.join(CONTENT, "data", ID, "horizon", "block", table + ".json"), {
            # The crafting table: Horizon only opens a mod menu (menus.rs) on a
            # block whose template opens a menu.
            "template": "minecraft:crafting_table",
            "item": True,
            "properties": {
                "hardness": 2.5,
                "explosion_resistance": 6.0,
                "requires_tool": True,
            },
        })
        write_json(os.path.join(CONTENT, "assets", ID, "items", table + ".json"), {
            "model": {"type": "minecraft:model", "model": "%s:block/%s" % (ID, table)}
        })
        # Multipart form: this port's own file, not the variants one the Java
        # jar ships (its assets are All Rights Reserved).
        write_json(os.path.join(CONTENT, "assets", ID, "blockstates", table + ".json"), {
            "multipart": [{"apply": {"model": "%s:block/%s" % (ID, table)}}]
        })
        write_json(os.path.join(CONTENT, "assets", ID, "models", "block", table + ".json"), {
            "parent": "minecraft:block/cube_bottom_top",
            "textures": {
                "top": "%s:block/%s" % (ID, table),
                "side": "%s:block/%s_side" % (ID, table),
                "bottom": "%s:block/%s_side" % (ID, table),
            },
        })
        png(os.path.join(CONTENT, "assets", ID, "textures", "block", table + ".png"),
            TEXTURES[table]())
        png(os.path.join(CONTENT, "assets", ID, "textures", "block", table + "_side.png"),
            TEXTURES[table + "_side"]())
        write_json(os.path.join(CONTENT, "data", ID, "loot_table", "blocks", table + ".json"), {
            "type": "minecraft:block",
            "pools": [{
                "rolls": 1,
                "bonus_rolls": 0,
                "entries": [{"type": "minecraft:item", "name": "%s:%s" % (ID, table)}],
            }],
        })
        write_json(os.path.join(CONTENT, "data", ID, "recipe", table + ".json"),
                   TABLE_RECIPES[table])
        en["block.%s.%s" % (ID, table)] = name_en
        fr["block.%s.%s" % (ID, table)] = name_fr

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
    for (seal, *_rest) in SEALS:
        item = "%s:%s" % (ID, seal)
        if item not in items:
            items.append(item)
    item = "%s:%s" % (ID, SUMMONER[0])
    if item not in items:
        items.append(item)
    for (table, *_rest) in TABLES:
        item = "%s:%s" % (ID, table)
        if item not in items:
            items.append(item)
    for (name, *_rest) in MATERIAL_ITEMS:
        item = "%s:%s" % (ID, name)
        if item not in items:
            items.append(item)
    tab["items"] = items
    write_json(tab_path, tab)

    # The tables are mined with a pickaxe like the gem blocks beside them.
    tag_path = os.path.join(CONTENT, "data", "minecraft", "tags", "block", "mineable",
                            "pickaxe.json")
    tag = read_json(tag_path, {"values": []})
    values = list(tag.get("values", []))
    for (table, *_rest) in TABLES:
        block = "%s:%s" % (ID, table)
        if block not in values:
            values.append(block)
    tag["values"] = values
    write_json(tag_path, tag)

    for lang, table in (("en_us", en), ("fr_fr", fr)):
        path = os.path.join(CONTENT, "assets", ID, "lang", lang + ".json")
        existing = read_json(path, {})
        existing.update(table)
        write_json(path, existing)
    print("wrote %d salvage materials, %d sigils, %d tables"
          % (len(MATERIALS), len(SEALS), len(TABLES)))


if __name__ == "__main__":
    main()
