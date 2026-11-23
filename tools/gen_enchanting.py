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


BOOK_SCRAP = (0x7A, 0x4B, 0x2E)
BOOK_IMPROVED = (0x8E, 0x2F, 0x3C)
BOOK_EXTRACTION = (0x2E, 0x6E, 0x7E)


def book_pixels(rgb, emblem):
    """Draw a closed 16x16 tome: coloured cover, pale pages and an emblem."""
    cover = rgb + (255,)
    dark = tuple(int(c * .55) for c in rgb) + (255,)
    light = tuple(min(255, int(c * 1.3)) for c in rgb) + (255,)
    page = (236, 226, 198, 255)
    edge = (198, 186, 160, 255)
    pixels = [[(0, 0, 0, 0)] * 16 for _ in range(16)]
    for y in range(1, 15):
        pixels[y][1] = dark
        for x in range(2, 12):
            pixels[y][x] = cover
        pixels[y][12] = page
        pixels[y][13] = page
        pixels[y][14] = edge
    for x in range(1, 16):
        pixels[0][x] = dark
        pixels[15][x] = dark
    for y in range(1, 15):
        pixels[y][2] = light
        pixels[y][3] = dark
    for x, y in ((6, 4), (7, 4), (8, 4), (9, 4),
                 (5, 5), (6, 5), (7, 5), (8, 5), (9, 5), (10, 5),
                 (5, 6), (6, 6), (7, 6), (8, 6), (9, 6), (10, 6),
                 (6, 7), (7, 7), (8, 7), (9, 7)):
        pixels[y][x] = emblem + (255,)
    for x in range(5, 11):
        pixels[11][x] = (214, 178, 74, 255)
    return pixels


def web_pixels():
    """Draw a prismarine cobweb: eight radial strands and three rings."""
    clear = (0, 0, 0, 0)
    teal = (0x3A, 0xC6, 0xB4, 255)
    light = (0xA8, 0xF0, 0xE4, 255)
    dark = (0x1E, 0x7D, 0x74, 255)
    pixels = [[clear] * 16 for _ in range(16)]
    for dx, dy in ((0, -1), (1, -1), (1, 0), (1, 1),
                   (0, 1), (-1, 1), (-1, 0), (-1, -1)):
        for step in range(8):
            x, y = 8 + dx * step, 8 + dy * step
            if 0 <= x < 16 and 0 <= y < 16 and pixels[y][x] == clear:
                pixels[y][x] = light if step < 3 else teal
    for radius in (2, 4, 6):
        for dx in range(-radius, radius + 1):
            dy = radius - abs(dx)
            for sign in (1, -1):
                for x, y in ((8 + dx, 8 + dy * sign),
                             (8 + dy * sign, 8 + dx)):
                    if 0 <= x < 16 and 0 <= y < 16:
                        pixels[y][x] = dark if radius == 2 else teal
    for x, y in ((7, 8), (8, 7), (8, 8), (9, 8), (8, 9)):
        pixels[y][x] = light
    return pixels


def library_pixels(glow_rgb=(0xC9, 0x8B, 0xF0)):
    """Draw the enchanting library: a dark shelf with a glowing open tome."""
    wood = (0x4A, 0x2F, 0x1F, 255)
    grain = (0x5E, 0x3D, 0x28, 255)
    book = (0x6E, 0x3F, 0x9E, 255)
    glow = glow_rgb + (255,)
    page = (0xEC, 0xE6, 0xF5, 255)
    gold = (0xD8, 0xB0, 0x4A, 255)
    clear = (0, 0, 0, 0)
    pixels = [[clear] * 16 for _ in range(16)]
    for y in range(16):
        for x in range(16):
            pixels[y][x] = grain if (x + y * 3) % 7 == 0 else wood
    # Shelf boards and two rows of books, with the open tome in the middle.
    for y in (4, 5, 10, 11):
        for x in range(16):
            pixels[y][x] = gold if x in (0, 15) else wood
    for row, top in ((0, 0), (1, 6)):
        for index in range(5):
            x0 = 1 + index * 3
            colour = (0x8C, 0x3B, 0x3B, 255) if (row + index) % 2 else (0x2F, 0x5C, 0x8C, 255)
            for x in (x0, x0 + 1):
                for y in range(top, top + 4):
                    pixels[y][x] = colour
            pixels[top][x0] = page
    for x in (6, 7, 8, 9):
        for y in (7, 8, 9):
            pixels[y][x] = page if x in (6, 9) else glow
    for y in (7, 9):
        pixels[y][7] = book
        pixels[y][8] = book
    pixels[8][7] = glow
    pixels[8][8] = glow
    return pixels


# id, English name, French name, stack size, rarity, texture, recipe
ITEMS = [
    ("scrap_tome", "Tome of Scrapping", "Livre de Démantèlement", 16, "common",
     book_pixels(BOOK_SCRAP, (216, 216, 208)), {
         "type": "minecraft:crafting_shaped", "category": "misc",
         "pattern": ["BBB", "BAB", "BBB"],
         "key": {"B": "minecraft:book", "A": "minecraft:anvil"},
         "result": {"id": "apotheosis:scrap_tome", "count": 8},
     }),
    ("improved_scrap_tome", "Tome of Superior Scrapping", "Livre de Démantèlement Amélioré",
     16, "rare", book_pixels(BOOK_IMPROVED, (214, 178, 74)), {
         "type": "minecraft:crafting_shaped", "category": "misc",
         "pattern": [" D ", "DTD", " D "],
         "key": {"D": "minecraft:diamond", "T": "apotheosis:scrap_tome"},
         "result": {"id": "apotheosis:improved_scrap_tome"},
     }),
    ("extraction_tome", "Tome of Extraction", "Livre d'Extraction",
     16, "epic", book_pixels(BOOK_EXTRACTION, (238, 238, 246)), {
         "type": "minecraft:crafting_shaped", "category": "misc",
         "pattern": [" N ", "NTN", " N "],
         "key": {"N": "minecraft:netherite_scrap", "T": "apotheosis:scrap_tome"},
         "result": {"id": "apotheosis:extraction_tome"},
     }),
    ("prismatic_web", "Prismatic Cobweb", "Toile Prismatique",
     64, "rare", web_pixels(), {
         "type": "minecraft:crafting_shaped", "category": "misc",
         "pattern": [" S ", "SCS", " S "],
         "key": {"S": "minecraft:prismarine_shard", "C": "minecraft:cobweb"},
         "result": {"id": "apotheosis:prismatic_web"},
     }),
]

MESSAGES = [
    ("apotheosis.tome.no_target", "Offhand must hold an enchanted item or a cursed item.",
     "La main secondaire doit tenir un objet enchanté ou maudit."),
    ("apotheosis.tome.not_enchanted", "That item has no enchantments.",
     "Cet objet n'a aucun enchantement."),
    ("apotheosis.tome.no_curse", "That item carries no curse.",
     "Cet objet ne porte aucune malédiction."),
    ("apotheosis.tome.levels", "You need %s experience levels.",
     "Il vous faut %s niveaux d'expérience."),
    ("apotheosis.tome.no_room", "No room for the book; nothing was changed.",
     "Pas de place pour le livre ; rien n'a été modifié."),
    ("apotheosis.tome.scrapped", "Scrapped %s enchantments.",
     "%s enchantements démantelés."),
    ("apotheosis.tome.extracted", "Extracted %s enchantments.",
     "%s enchantements extraits."),
    ("apotheosis.tome.cleansed", "Curses removed.",
     "Malédictions retirées."),
    ("apotheosis.library.not_enchanted", "That book carries no enchantment.",
     "Ce livre ne porte aucun enchantement."),
    ("apotheosis.library.absorbed", "Stored %s levels; the library holds %s points.",
     "%s niveaux absorbés ; la bibliothèque totalise %s points."),
    ("apotheosis.library.empty", "The library holds no points yet: store enchanted books first.",
     "La bibliothèque n'a aucun point : déposez d'abord des livres enchantés."),
    ("apotheosis.library.offer", "Offer: %s %s (%s points stored). Right-click with a book to take it.",
     "Proposition : %s %s (%s points stockés). Clic droit avec un livre pour le prendre."),
    ("apotheosis.library.withdrawn", "Took %s %s for %s points.",
     "%s %s récupéré pour %s points."),
    ("apotheosis.library.no_room", "No room for the book; nothing was taken.",
     "Pas de place pour le livre ; rien n'a été pris."),
    ("apotheosis.library.usage", "Hold an enchanted book to store it, or a book to take an enchantment out.",
     "Tenez un livre enchanté pour le déposer, ou un livre vierge pour retirer un enchantement."),
    ("apotheosis.infusion.no_match", "Nothing in your hand can be infused here.",
     "Rien dans votre main ne peut être infusé ici."),
    ("apotheosis.infusion.single", "The table infuses one item at a time.",
     "La table n'infuse qu'un objet à la fois."),
    ("apotheosis.infusion.requirements", "Needs Eterna %s, Quanta %s and Arcana %s; the shelves give %s, %s and %s.",
     "Requiert %s d'Eterna, %s de Quanta et %s d'Arcana ; les étagères fournissent %s, %s et %s."),
    ("apotheosis.infusion.levels", "You need %s experience levels.",
     "Il vous faut %s niveaux d'expérience."),
    ("apotheosis.infusion.done", "Infused %s.",
     "%s infusé."),
]


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


def table_pixels():
    """Draw the Apothic enchanting table: obsidian frame, gold inlay, diamond."""
    obsidian = (0x1B, 0x14, 0x2B, 255)
    light = (0x2E, 0x24, 0x45, 255)
    gold = (0xD8, 0xB0, 0x4A, 255)
    ruby = (0xB0, 0x28, 0x3C, 255)
    diamond = (0x6E, 0xE0, 0xE8, 255)
    pixels = [[obsidian] * 16 for _ in range(16)]
    for y in range(16):
        for x in range(16):
            if (x * 3 + y * 5) % 11 == 0:
                pixels[y][x] = light
    for x in range(16):
        pixels[0][x] = gold
        pixels[15][x] = gold
    for y in range(16):
        pixels[y][0] = gold
        pixels[y][15] = gold
    for y in range(4, 12):
        pixels[y][7] = gold
        pixels[y][8] = gold
    for x in range(4, 12):
        pixels[7][x] = gold
        pixels[8][x] = gold
    for x, y in ((6, 5), (7, 5), (9, 5), (10, 5),
                 (5, 6), (6, 6), (7, 6), (8, 6), (9, 6), (10, 6),
                 (6, 7), (9, 7), (6, 8), (9, 8),
                 (5, 9), (6, 9), (7, 9), (8, 9), (9, 9), (10, 9),
                 (6, 10), (7, 10), (9, 10), (10, 10)):
        pixels[y][x] = ruby
    for x, y in ((7, 7), (8, 7), (7, 8), (8, 8)):
        pixels[y][x] = diamond
    return pixels


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
    # A shelf whose Eterna is negative must not count as a vanilla power
    # provider: the game would treat it as a bookshelf and raise the offers,
    # the opposite of what the native hook does with it.
    negative = {f"{ID}:{shelf}" for shelf, eterna, *_ in SHELVES if eterna < 0}
    power_path = os.path.join(CONTENT, "data", "minecraft", "tags", "block", "enchantment_power_provider.json")
    if os.path.exists(power_path):
        with open(power_path, encoding="utf-8") as f:
            power = json.load(f)
    else:
        power = {"values": []}
    values = [entry for entry in power.setdefault("values", []) if entry not in negative]
    for item in shelf_items:
        if item not in negative and item not in values:
            values.append(item)
    power["values"] = values
    write_json(power_path, power)
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

    # The Enchantment Library: a bookshelf that stores enchantments as points.
    library = f"{ID}:enchantment_library"
    write_json(os.path.join(CONTENT, "data", ID, "horizon", "block", "enchantment_library.json"), {
        "template": "minecraft:bookshelf",
        "item": True,
        "properties": {"hardness": 2.0, "explosion_resistance": 2.0,
                       "sounds": "minecraft:chiseled_bookshelf", "requires_tool": False},
    })
    write_json(os.path.join(CONTENT, "assets", ID, "blockstates", "enchantment_library.json"), {
        "variants": {"": {"model": f"{ID}:block/enchantment_library"}},
    })
    write_json(os.path.join(CONTENT, "assets", ID, "models", "block", "enchantment_library.json"), {
        "parent": "minecraft:block/cube_all", "textures": {"all": f"{ID}:block/enchantment_library"},
    })
    write_json(os.path.join(CONTENT, "assets", ID, "items", "enchantment_library.json"), {
        "model": {"type": "minecraft:model", "model": f"{ID}:block/enchantment_library"},
    })
    write_png(os.path.join(CONTENT, "assets", ID, "textures", "block", "enchantment_library.png"),
              library_pixels())
    write_json(os.path.join(CONTENT, "data", ID, "loot_table", "blocks", "enchantment_library.json"), {
        "type": "minecraft:block", "pools": [{"rolls": 1, "bonus_rolls": 0,
            "entries": [{"type": "minecraft:item", "name": library}]}],
    })
    write_json(os.path.join(CONTENT, "data", ID, "recipe", "enchantment_library.json"), {
        "type": "minecraft:crafting_shaped", "category": "building",
        "pattern": ["BBB", "BSB", "BBB"],
        "key": {"B": "minecraft:book", "S": "minecraft:bookshelf"},
        "result": {"id": library},
    })
    en[f"block.{ID}.enchantment_library"] = "Enchantment Library"
    fr[f"block.{ID}.enchantment_library"] = "Bibliothèque d'enchantement"
    merge_values(os.path.join(CONTENT, "data", "minecraft", "tags", "block", "mineable", "axe.json"),
                 [library])
    if library not in entries:
        entries.append(library)

    # Apothic Enchanting's remaining stat blocks and the Ender Library. Their
    # recipes are the Java ones (calcite and budding amethyst around a
    # bookshelf, gold and emerald around a deepshelf, prismarine around an
    # infused seashelf, the Apothic table's own pattern), with the Java ids.
    extra_blocks = [
        ("filtering_shelf",
         {"hardness": 1.75, "explosion_resistance": 1.75,
          "sounds": "minecraft:stone", "requires_tool": True},
         shelf_pixels(0x52D4C0), "Filtering Shelf", "Bibliothèque filtrante",
         {"type": "minecraft:crafting_shaped", "category": "misc",
          "pattern": ["PSP", "LLL", "PSP"],
          "key": {"P": "minecraft:prismarine_bricks",
                  "S": f"{ID}:infused_seashelf",
                  "L": "minecraft:prismarine_brick_slab"},
          "result": {"id": f"{ID}:filtering_shelf"}},
         "pickaxe"),
        ("geode_shelf",
         {"hardness": 1.75, "explosion_resistance": 1.75,
          "sounds": "minecraft:stone", "requires_tool": True},
         shelf_pixels(0xA79A8C), "Geode Shelf", "Bibliothèque de géode",
         {"type": "minecraft:crafting_shaped", "category": "misc",
          "pattern": ["CCC", "CBC", "CUC"],
          "key": {"C": "minecraft:calcite", "B": "minecraft:bookshelf",
                  "U": "minecraft:budding_amethyst"},
          "result": {"id": f"{ID}:geode_shelf"}},
         "pickaxe"),
        ("treasure_shelf",
         {"hardness": 1.75, "explosion_resistance": 1.75,
          "sounds": "minecraft:stone", "requires_tool": True},
         shelf_pixels(0x1E4B3F), "Treasure Shelf", "Bibliothèque au trésor",
         {"type": "minecraft:crafting_shaped", "category": "misc",
          "pattern": ["SDS", "GTG", "SDS"],
          "key": {"S": "minecraft:gold_block", "D": f"{ID}:deepshelf",
                  "G": "minecraft:diamond", "T": "minecraft:emerald_block"},
          "result": {"id": f"{ID}:treasure_shelf"}},
         "pickaxe"),
        ("ender_library",
         {"hardness": 5.0, "explosion_resistance": 1200.0,
          "sounds": "minecraft:chiseled_bookshelf", "requires_tool": False},
         library_pixels((0x9C, 0x4B, 0xE8)), "Ender Library",
         "Bibliothèque de l'Ender",
         # Java obtains it by infusion from the library, never by crafting;
         # this recipe is the port's survival route while the infusion step
         # keeps the Java requirements (see tools/java_parity.md).
         {"type": "minecraft:crafting_shaped", "category": "building",
          "pattern": ["CBC", "BLB", "CBC"],
          "key": {"C": "minecraft:ender_chest",
                  "B": "minecraft:book", "L": f"{ID}:enchantment_library"},
          "result": {"id": f"{ID}:ender_library"}},
         "axe"),
        ("apothic_enchanting_table",
         {"hardness": 5.0, "explosion_resistance": 1200.0,
          "sounds": "minecraft:enchanting_table", "requires_tool": True},
         table_pixels(), "Apothic Enchanting Table",
         "Table d'enchantement apothéotique",
         {"type": "minecraft:crafting_shaped", "category": "misc",
          "pattern": [" T ", "DOD", "OOO"],
          "key": {"T": f"{ID}:scrap_tome", "D": "minecraft:diamond",
                  "O": "minecraft:obsidian"},
          "result": {"id": f"{ID}:apothic_enchanting_table"}},
         "pickaxe"),
    ]
    for block, props, pixels, name_en, name_fr, recipe, tool in extra_blocks:
        block_id = f"{ID}:{block}"
        write_json(os.path.join(CONTENT, "data", ID, "horizon", "block", block + ".json"), {
            "template": "minecraft:bookshelf" if block != "apothic_enchanting_table"
            else "minecraft:obsidian",
            "item": True,
            "properties": props,
        })
        write_json(os.path.join(CONTENT, "assets", ID, "blockstates", block + ".json"), {
            "variants": {"": {"model": f"{ID}:block/{block}"}},
        })
        write_json(os.path.join(CONTENT, "assets", ID, "models", "block", block + ".json"), {
            "parent": "minecraft:block/cube_all", "textures": {"all": f"{ID}:block/{block}"},
        })
        write_json(os.path.join(CONTENT, "assets", ID, "items", block + ".json"), {
            "model": {"type": "minecraft:model", "model": f"{ID}:block/{block}"},
        })
        write_png(os.path.join(CONTENT, "assets", ID, "textures", "block", block + ".png"), pixels)
        write_json(os.path.join(CONTENT, "data", ID, "loot_table", "blocks", block + ".json"), {
            "type": "minecraft:block", "pools": [{"rolls": 1, "bonus_rolls": 0,
                "entries": [{"type": "minecraft:item", "name": block_id}]}],
        })
        write_json(os.path.join(CONTENT, "data", ID, "recipe", block + ".json"), recipe)
        en[f"block.{ID}.{block}"] = name_en
        fr[f"block.{ID}.{block}"] = name_fr
        merge_values(os.path.join(CONTENT, "data", "minecraft", "tags", "block",
                                 "mineable", tool + ".json"), [block_id])
        if block not in entries:
            entries.append(block_id)
    # Geode and treasure shelves provide Eterna in Java, so the vanilla table
    # may count them as power providers too (the filtering shelf gives none
    # without books in it, and the tables are not shelves).
    for block in ("geode_shelf", "treasure_shelf"):
        merge_values(power_path, [f"{ID}:{block}"])

    for item_id, name_en, name_fr, stack, rarity, pixels, recipe in ITEMS:
        item = f"{ID}:{item_id}"
        write_json(os.path.join(CONTENT, "data", ID, "horizon", "item", item_id + ".json"), {
            "template": "minecraft:book" if "tome" in item_id else "minecraft:string",
            "max_stack_size": stack,
            "rarity": rarity,
        })
        write_json(os.path.join(CONTENT, "assets", ID, "models", "item", item_id + ".json"), {
            "parent": "minecraft:item/generated",
            "textures": {"layer0": f"{ID}:item/{item_id}"},
        })
        write_json(os.path.join(CONTENT, "assets", ID, "items", item_id + ".json"), {
            "model": {"type": "minecraft:model", "model": f"{ID}:item/{item_id}"},
        })
        write_png(os.path.join(CONTENT, "assets", ID, "textures", "item", item_id + ".png"), pixels)
        write_json(os.path.join(CONTENT, "data", ID, "recipe", item_id + ".json"), recipe)
        en[f"item.{ID}.{item_id}"] = name_en
        fr[f"item.{ID}.{item_id}"] = name_fr
        if item not in entries:
            entries.append(item)
    write_json(tab_path, tab)
    for key, fallback_en, fallback_fr in MESSAGES:
        en[key] = fallback_en
        fr[key] = fallback_fr

    for locale, additions in (("en_us", en), ("fr_fr", fr)):
        path = os.path.join(CONTENT, "assets", ID, "lang", locale + ".json")
        existing = {}
        if os.path.exists(path):
            with open(path, encoding="utf-8") as f:
                existing = json.load(f)
        existing.update(additions)
        write_json(path, existing)
    print(f"wrote {len(SHELVES)} enchanting shelves and {len(ITEMS)} items with original 16x16 textures")


if __name__ == "__main__":
    main()
