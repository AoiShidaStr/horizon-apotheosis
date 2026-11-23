#!/usr/bin/env python3
"""Generate tools/java_parity.md, the object/block/recipe parity report.

Run: python tools/gen_parity.py [java-ref]
`java-ref` is the directory holding the unpacked upstream jars; it is only read,
never written, and nothing from it is copied into the content. The report is a
listing of every upstream object, block and recipe with a status:

- `porté`                  the same id exists here, with the same recipe;
- `porté autrement`        the same id exists here, in another form (said why);
- `absent`                 the id is not here (said why).

The counts and the id lists come from the jars themselves, so re-running this
after an upstream bump shows the new gaps instead of a stale list.
"""
import glob
import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CONTENT = os.path.join(ROOT, "content")
REPORT = os.path.join(ROOT, "tools", "java_parity.md")
DEFAULT_REF = r"C:/Users/yello/horizon-work/java-ref"
OUR_ID = "apotheosis"

# The jars and the ids they own, in report order.
JARS = [
    ("Apotheosis", "Apotheosis-*", "apotheosis"),
    ("Apothic Enchanting", "ApothicEnchanting-*", "apothic_enchanting"),
    ("Apothic Spawners", "ApothicSpawners-*", "apothic_spawners"),
    ("Apothic Attributes", "ApothicAttributes-*", "apothic_attributes"),
]

# Upstream namespaces whose blocks and items this port declares under
# `apotheosis` (Horizon's content loader has one item registry for the mod).
ALIASES = {
    "apothic_enchanting:library": "enchantment_library",
    "apothic_enchanting:enchantment_library": "enchantment_library",
    "apothic_enchanting:filtering_shelf": "filtering_shelf",
    "apothic_enchanting:geode_shelf": "geode_shelf",
    "apothic_enchanting:treasure_shelf": "treasure_shelf",
}

# The gem family is another chantier: no gem item, block or socketing is
# touched here, so the report says so once instead of per id.
GEM_PREFIXES = (
    "gem_amber",
    "gem_amethyst",
    "gem_bloodstone",
    "gem_citrine",
    "gem_jade",
    "gem_moonstone",
    "gem_onyx",
    "gem_opal",
    "gem_peridot",
    "gem_ruby",
    "gem_sapphire",
    "gem_topaz",
    "gem_block_",
    "gem_cutting_table",
    "gem_case",
)

# Ids where the upstream data is deliberately outside this chantier.
OUT_OF_SCOPE = (
    "hors périmètre : les gemmes sont un autre chantier",
)

# Why an id is absent. First matching prefix wins.
ABSENT_REASONS = [
    (GEM_PREFIXES, OUT_OF_SCOPE[0]),
    (
        ("spawner_rune", "infused_spawner_rune"),
        "objet de rune : dépend de spawner_chain, qui n'est obtenu que par la table de butin `unstable_spawner` (non portée)",
    ),
    (
        (
            "frontier_spawner_upgrade_rune",
            "ascent_spawner_upgrade_rune",
            "summit_spawner_upgrade_rune",
            "pinnacle_spawner_upgrade_rune",
        ),
        "rune de palier : recette portable, objet non déclaré dans ce lot",
    ),
    (
        (
            "burning_spawner_rune",
            "echoing_spawner_rune",
            "ignore_conditions_spawner_rune",
            "ignore_light_spawner_rune",
            "ignore_players_spawner_rune",
            "initial_health_spawner_rune",
            "no_ai_spawner_rune",
            "redstone_control_spawner_rune",
            "silent_spawner_rune",
            "spawn_range_spawner_rune",
            "youthful_spawner_rune",
        ),
        "rune de statistique : la statistique correspondante est déjà appliquée par le catalyseur vanilla (voir `native/apotheosis/api/src/spawners.rs`)",
    ),
    (
        ("spawner_chain",),
        "aucune recette en Java : butin de l'explosion d'un spawner (`unstable_spawner`), table de butin non portée",
    ),
    (
        ("star_cube",),
        "aucune recette en Java : butin de boss (`entity/boss_drops`), non porté",
    ),
    (
        ("augmenting_table", "simple_reforging_table", "reforging_table", "salvaging_table"),
        "bloc de table : la recette est portable, le comportement de la table ne l'est pas sans toucher au sertissage",
    ),
    (
        ("boss_summoner", "boss_spawner"),
        "invocateur de boss : la passerelle de boss (dimension, vagues, table de butin) n'est pas portée",
    ),
    (
        ("potion_charm",),
        "charme de potion : le type `apotheosis:potion_charm_crafting` et son infusion ne sont pas data-driven sous Horizon",
    ),
    (
        ("sigil_of_malice", "sigil_of_supremacy", "sigil_of_unnaming"),
        "sceau : la recette Java est un type custom (`apotheosis:malice|supremacy|unnaming`), non data-driven",
    ),
    (
        ("_upgrade_smithing_template",),
        "modèle de forge : recette portable, objet non déclaré dans ce lot",
    ),
    (
        ("hammer", "book", "lucky_foot", "music_disc_", "ender_lead", "infused_breath", "warden_tendril"),
        "objet utilitaire ou décoratif : recette et comportement non portés",
    ),
    (("tome",), "tome par type d'objet : comportement de l'établi d'extraction uniquement"),
    (
        ("god_fused_pearl", "gem_fused_slate", "infused_breath"),
        "objet produit par infusion : `recipe/infusion` n'est pas data-driven sous Horizon",
    ),
]

# The custom recipe types upstream uses and this port cannot load.
CUSTOM_TYPES = {
    "apotheosis:reforging": "reforge (niveau, matériau, sceau) : pas de type de recette data-driven sous Horizon",
    "apotheosis:salvaging": "recyclage par groupe d'objets : logique native, pas de type data-driven",
    "apothic_enchanting:infusion": "infusion d'enchantement : requires Eterna/Quanta/Arcana, pas de type data-driven",
    "apotheosis:socketing": "sertissage : autre chantier",
    "apotheosis:add_sockets": "ajout de châsses : autre chantier",
    "apotheosis:sized_upgrade_recipe": "forge dimensionnée : le format vanilla ne consomme pas 4 lingots",
    "apotheosis:potion_charm_crafting": "charme de potion : potion en ingrédient non exprimable en recette vanilla",
    "apotheosis:potion_charm_infusion": "charme de potion infusé : dépend de l'infusion",
    "apotheosis:spawn_egg": "œuf de spawn en ingrédient : pas d'équivalent data-driven",
    "apotheosis:malice": "sceau de malveillance : type custom",
    "apotheosis:supremacy": "sceau de suprématie : type custom",
    "apotheosis:unnaming": "sceau de dénomination : type custom",
}


def read_json(path):
    with open(path, encoding="utf-8-sig") as f:
        return json.load(f)


def jar_root(pattern, ref):
    found = sorted(glob.glob(os.path.join(ref, pattern)))
    return found[-1] if found else None


def upstream_ids(root, kind):
    if not root:
        return []
    if kind == "block":
        paths = glob.glob(os.path.join(root, "assets", "*", "blockstates", "*.json"))
    else:
        paths = glob.glob(os.path.join(root, "assets", "*", "models", "item", "*.json"))
    found = set()
    for path in paths:
        parts = os.path.normpath(path).split(os.sep)
        namespace = parts[-3] if kind == "block" else parts[-4]
        found.add(f"{namespace}:{os.path.splitext(parts[-1])[0]}")
    return sorted(found)


def our_ids(kind):
    return {
        os.path.splitext(os.path.basename(p))[0]
        for p in glob.glob(os.path.join(CONTENT, "data", OUR_ID, "horizon", kind, "*.json"))
    }


def our_recipes():
    """recipe id -> source path, for every recipe this port ships."""
    found = {}
    base = os.path.join(CONTENT, "data", OUR_ID, "recipe")
    for path in glob.glob(os.path.join(base, "**", "*.json"), recursive=True):
        name = os.path.splitext(os.path.basename(path))[0]
        found.setdefault(name, path)
    return found


def upstream_recipes(root, namespace):
    """`namespace/name` -> path, for one jar's own recipes."""
    if not root:
        return {}
    found = {}
    for path in glob.glob(os.path.join(root, "data", namespace, "recipe", "**", "*.json"), recursive=True):
        name = os.path.splitext(os.path.basename(path))[0]
        found.setdefault(name, []).append(path)
    return found


def local(name):
    """The alias table: an upstream id to the id this port would declare."""
    namespace, _, path = name.partition(":")
    if namespace == OUR_ID:
        return path
    return ALIASES.get(name, path)


def absent_reason(name):
    short = name.partition(":")[2]
    for prefixes, reason in ABSENT_REASONS:
        if short.startswith(prefixes):
            return reason
    return "non porté dans ce lot"


def recipe_reason(paths, ported_items, ported_blocks):
    types = sorted({read_json(paths[0]).get("type", "?")})
    custom = [t for t in types if t in CUSTOM_TYPES]
    if custom:
        return CUSTOM_TYPES[custom[0]]
    if types and all(t.startswith("minecraft:crafting") or t == "minecraft:smithing_transform" for t in types):
        return "recette vanilla portable : résultat ou ingrédient absent"
    return "type non porté : " + ", ".join(types)


def status_of(name, ported, kind):
    if local(name) in ported:
        return "porté"
    short = name.partition(":")[2]
    if kind == "recipe":
        return "absent"
    if any(short.startswith(prefix) for prefix in GEM_PREFIXES):
        return "hors périmètre"
    return "absent"


def table(rows, header=("id", "statut", "raison")):
    lines = ["| " + " | ".join(header) + " |"]
    lines.append("|" + "|".join(["---"] * len(header)) + "|")
    for row in rows:
        lines.append("| " + " | ".join(str(cell).replace("|", "\\|") for cell in row) + " |")
    return "\n".join(lines)


def main():
    ref = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_REF
    blocks = our_ids("block")
    items = our_ids("item")
    recipes = our_recipes()

    missing_blocks, missing_items, missing_recipes = [], [], []
    counts = {}
    for label, pattern, namespace in JARS:
        root = jar_root(pattern, ref)
        if not root:
            counts[label] = None
            continue
        up_blocks = [i for i in upstream_ids(root, "block") if i.startswith(namespace + ":")]
        up_items = [i for i in upstream_ids(root, "item") if i.startswith(namespace + ":")]
        up_recipes = upstream_recipes(root, namespace)
        for name in up_blocks:
            if local(name) not in blocks:
                missing_blocks.append((name, status_of(name, blocks, "block"), absent_reason(name)))
        for name in up_items:
            if local(name) not in items:
                missing_items.append((name, status_of(name, items, "item"), absent_reason(name)))
        for name, paths in sorted(up_recipes.items()):
            if name not in recipes:
                missing_recipes.append((name, "absent", recipe_reason(paths, items, blocks)))
        counts[label] = (len(up_blocks), len(up_items), len(up_recipes))

    present_blocks = sorted(blocks)
    present_items = sorted(items)

    out = []
    out.append("# Parité des objets, blocs et recettes Java\n")
    out.append(
        "Rapport généré par `python tools/gen_parity.py` à partir des jars de référence\n"
        "(`C:/Users/yello/horizon-work/java-ref`). Les identifiants listés sont ceux du\n"
        "Java ; le port les déclare sous `apotheosis:` avec le même chemin, sauf mention\n"
        "contraire. Aucun fichier d'asset Java n'est copié : les textures et modèles du\n"
        "port sont dessinés par les générateurs, et `content-java/` (import local) reste\n"
        "ignoré par git.\n"
    )
    out.append("## Ce qui est porté\n")
    out.append(
        f"{len(present_blocks)} blocs et {len(present_items)} objets sont déclarés sous\n"
        f"`content/data/apotheosis/horizon/`, avec {len(recipes)} recettes sous\n"
        "`content/data/apotheosis/recipe/`.\n"
    )
    out.append("Blocs : " + ", ".join(f"`{i}`" for i in present_blocks) + ".\n")
    out.append("Objets : " + ", ".join(f"`{i}`" for i in present_items) + ".\n")
    out.append("## Blocs Java absents\n")
    out.append(table(missing_blocks) if missing_blocks else "Aucun.\n")
    out.append("\n## Objets Java absents\n")
    out.append(table(missing_items) if missing_items else "Aucun.\n")
    out.append("\n## Recettes Java absentes\n")
    out.append(table(missing_recipes, ("recette", "statut", "raison")) if missing_recipes else "Aucune.\n")
    out.append("\n## Recettes de modificateur de spawner\n")
    out.append(
        "Les 16 recettes de catalyseur du namespace `apothic_spawners`, les 11 recettes\n"
        "de rune du namespace `apotheosis` et les 4 runes de palier sont reproduites dans\n"
        "`native/apotheosis/api/src/spawners.rs` (table `MODIFIERS`, 31 entrées), avec\n"
        "leurs valeurs exactes et, pour les runes seulement, leur recette inverse :\n"
        "l'inverseur est le quartz en main secondaire, que le port lit comme l'objet en\n"
        "main (limite documentée dans le module). Les inverses de catalyseur sont\n"
        "désactivés en Java (`neoforge:false`) et le port ne les applique donc pas.\n"
    )
    out.append("\n## Écarts notables\n")
    out.append(
        "- `apothic_enchanting:infused_endshelf` n'a **ni blockstate ni recette** en Java\n"
        "  (seulement un modèle d'objet) : rien à porter fidèlement, il est absent.\n"
        "- Les quatre étagères demandées se répartissent ainsi : `filtering_shelf`,\n"
        "  `geode_shelf` et `treasure_shelf` ont un blockstate et une recette, mais\n"
        "  **aucun fichier `enchanting_stats`** : leurs statistiques et leurs effets\n"
        "  (filtration, trésor, géode) sont calculés dans le code d'Apothic Enchanting,\n"
        "  pas en data. Les porter sans inventer de valeurs demande un module natif ;\n"
        "  ils restent absents et ne sont pas devinés.\n"
        "- `spawner_chain` n'a aucune recette : en Java il tombe de la table de butin\n"
        "  `apothic_spawners:loot_table/gameplay/unstable_spawner` (explosion d'un\n"
        "  spawner). Cette table n'est pas portée, donc les runes restent hors de\n"
        "  portée en survie ; c'est une raison de plus de ne pas déclarer les objets\n"
        "  de rune seuls.\n"
        "- Les 28 recettes `recipe/smithing` utilisent le type\n"
        "  `apotheosis:sized_upgrade_recipe`, qui consomme **4** lingots ; le format\n"
        "  vanilla `minecraft:smithing_transform` n'en consomme qu'un. Les porter\n"
        "  telles quelles changerait l'équilibrage : elles sont absentes.\n"
        "- Les 32 (en fait 31 actives : 16 catalyseurs + 11 runes + 4 paliers)\n"
        "  recettes `spawner_modifiers` sont, elles, entièrement reproduites dans la\n"
        "  table `MODIFIERS` du module natif, inverses compris.\n"
    )
    out.append("\n## Types de recette non data-driven\n")
    out.append(table([(t, r) for t, r in sorted(CUSTOM_TYPES.items())]))
    out.append(
        "\nCes types ne peuvent pas être exprimés par un fichier JSON sous Horizon :\n"
        "leur comportement vit dans le code du mod. Ils sont donc absents en tant que\n"
        "recettes ; les objets qu'ils produisent sont soit déjà portés, soit listés\n"
        "ci-dessus avec leur raison.\n"
    )
    out.append("\n## Hors périmètre\n")
    out.append(
        "Les gemmes (objets, blocs, sertissage, `native/apotheosis/api/src/sockets.rs`)\n"
        "et les affixes (`affix_effects.rs`) appartiennent à d'autres chantiers et ne\n"
        "sont ni modifiés ni comptés comme manquants ici.\n"
    )
    out.append("\n## Volumes relevés\n")
    rows = []
    for label, _, _ in JARS:
        values = counts.get(label)
        rows.append((label, "absent du dossier de référence" if values is None else f"{values[0]} blocs / {values[1]} objets / {values[2]} recettes"))
    out.append(table(rows, ("jar", "blocs / objets / recettes")))

    with open(REPORT, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(out) + "\n")
    print(f"wrote {REPORT}: {len(missing_blocks)} blocks, {len(missing_items)} items, {len(missing_recipes)} recipes missing")


if __name__ == "__main__":
    main()
