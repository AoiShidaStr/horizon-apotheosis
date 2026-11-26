#!/usr/bin/env python3
"""Generate this port's gem catalogue from Apotheosis 8.9.0's gem data.

The Java mod's gem data (`data/apotheosis/gems/**`) is the reference: 23 gems,
21 of them outside the Twilight Forest (which this port does not ship).  This
script reads that data read-only and writes:

  native/apotheosis/api/src/java_gems.rs   the catalogue the native mod uses
  content/assets/apotheosis/models/item/gem_<name>.json
                                            one model per gem, on the imported
                                            Java texture (git-ignored layer)
  content/data/apotheosis/horizon/item/gem_<name>_<purity>.json
  content/assets/apotheosis/items/gem_<name>_<purity>.json
                                            one item per gem and purity at or
                                            above its floor, as Apotheosis lists
                                            them in its creative tab; its default
                                            components carry the type, purity
                                            and name (the native tooltip hook
                                            writes the rest)
  the creative tab and the `apotheosis:gems` tag, with those items

The generated Rust is a data table, not code copied from Apotheosis: only the
ids, names, weights, purity floors, dimension constraints and per-purity bonus
values are carried over, and every bonus whose type or attribute this port
cannot pay out is recorded instead of being replaced by an invented one.

Usage:
  python tools/gen_java_gems.py [JAVA-REF]
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_REF = Path("C:/Users/yello/horizon-work/java-ref/Apotheosis-1.21.1-8.9.0")
RUST = ROOT / "native" / "apotheosis" / "api" / "src" / "java_gems.rs"
MODELS = ROOT / "content" / "assets" / "apotheosis" / "models" / "item"

PURITIES = ("cracked", "chipped", "flawed", "normal", "flawless", "perfect")
# The rarity `gems::stack` gave the generic gem.
PURITY_RARITIES = ("common", "uncommon", "uncommon", "rare", "epic", "epic")
ITEM_DECLS = ROOT / "content" / "data" / "apotheosis" / "horizon" / "item"
ITEM_MODELS = ROOT / "content" / "assets" / "apotheosis" / "items"
TAB = ROOT / "content" / "data" / "apotheosis" / "horizon" / "creative_tab" / "apotheosis.json"
GEMS_TAG = ROOT / "content" / "data" / "apotheosis" / "tags" / "item" / "gems.json"

# Gem classes, as Apotheosis names them, mapped to this port's item ids.  A gem
# bonus applies to a piece whose id matches one of the class's suffixes.
CLASSES = {
    "melee_weapon": ("MeleeWeapon", ("_sword", "_axe", "_mace")),
    "trident": ("Trident", ("trident",)),
    "bow": ("Bow", ("_bow", "_crossbow")),
    "breaker": ("Breaker", ("_pickaxe", "_shovel", "_hoe")),
    "shears": ("Shears", ("shears",)),
    "helmet": ("Helmet", ("_helmet",)),
    "chestplate": ("Chestplate", ("_chestplate",)),
    "leggings": ("Leggings", ("_leggings",)),
    "boots": ("Boots", ("_boots",)),
    "shield": ("Shield", ("shield",)),
    "anything": ("Anything", ()),
}

# Attributes this port pays out: vanilla ones the game applies from the item's
# own `minecraft:attribute_modifiers`, and the Apothic Attributes this crate
# implements in `attributes.rs`.  Any other attribute is recorded as
# unsupported: no substitute is invented for it.
ATTRIBUTES = {
    "minecraft:generic.attack_damage": "minecraft:attack_damage",
    "minecraft:generic.attack_speed": "minecraft:attack_speed",
    "minecraft:generic.max_health": "minecraft:max_health",
    "minecraft:generic.armor": "minecraft:armor",
    "minecraft:generic.armor_toughness": "minecraft:armor_toughness",
    "minecraft:generic.knockback_resistance": "minecraft:knockback_resistance",
    "minecraft:generic.luck": "minecraft:luck",
    "minecraft:generic.attack_knockback": "minecraft:attack_knockback",
    "minecraft:generic.movement_speed": "minecraft:movement_speed",
    "apothic_attributes:armor_pierce": "apothic_attributes:armor_pierce",
    "apothic_attributes:armor_shred": "apothic_attributes:armor_shred",
    "apothic_attributes:arrow_damage": "apothic_attributes:arrow_damage",
    "apothic_attributes:arrow_velocity": "apothic_attributes:arrow_velocity",
    "apothic_attributes:cold_damage": "apothic_attributes:cold_damage",
    "apothic_attributes:crit_chance": "apothic_attributes:crit_chance",
    "apothic_attributes:crit_damage": "apothic_attributes:crit_damage",
    "apothic_attributes:dodge_chance": "apothic_attributes:dodge_chance",
    "apothic_attributes:experience_gained": "apothic_attributes:experience_gained",
    "apothic_attributes:fire_damage": "apothic_attributes:fire_damage",
    "apothic_attributes:healing_received": "apothic_attributes:healing_received",
    "apothic_attributes:life_steal": "apothic_attributes:life_steal",
    "apothic_attributes:mining_speed": "apothic_attributes:mining_speed",
    "apothic_attributes:draw_speed": "apothic_attributes:draw_speed",
    "apothic_attributes:overheal": "apothic_attributes:overheal",
}

# French names for the classes Java's own fr_fr file leaves out, plus the
# generic words used to join several classes.  These are our own short
# formulations, not copies of the Java assets.
CLASS_NAMES_EN = {
    "shears": "Shears",
}

CLASS_NAMES_FR = {
    "bow": "Arcs",
    "trident": "Tridents",
    "shears": "Cisailles",
    "anything": "Tout objet",
}

# English and French names for the vanilla and Apothic attributes the gem
# catalogue uses.  The English side matches `attributes::Attr::name` and the
# vanilla names `gems::attribute_name` prints today.
ATTR_NAMES = {
    "minecraft:attack_damage": ("Attack Damage", "Dégâts d'attaque"),
    "minecraft:attack_speed": ("Attack Speed", "Vitesse d'attaque"),
    "minecraft:max_health": ("Max Health", "Santé maximale"),
    "minecraft:armor": ("Armor", "Armure"),
    "minecraft:armor_toughness": ("Armor Toughness", "Solidité d'armure"),
    "minecraft:knockback_resistance": ("Knockback Resistance", "Résistance au recul"),
    "minecraft:luck": ("Luck", "Chance"),
    "minecraft:attack_knockback": ("Attack Knockback", "Recul d'attaque"),
    "minecraft:movement_speed": ("Movement Speed", "Vitesse de déplacement"),
    "apothic_attributes:armor_pierce": ("Armor Pierce", "Perce-armure"),
    "apothic_attributes:armor_shred": ("Armor Shred", "Brise-armure"),
    "apothic_attributes:arrow_damage": ("Arrow Damage", "Dégâts des flèches"),
    "apothic_attributes:arrow_velocity": ("Arrow Velocity", "Vitesse des flèches"),
    "apothic_attributes:cold_damage": ("Cold Damage", "Dégâts de froid"),
    "apothic_attributes:crit_chance": ("Crit Chance", "Chances de coup critique"),
    "apothic_attributes:crit_damage": ("Crit Damage", "Dégâts critiques"),
    "apothic_attributes:dodge_chance": ("Dodge Chance", "Chances d'esquive"),
    "apothic_attributes:experience_gained": ("Experience Gained", "Expérience gagnée"),
    "apothic_attributes:fire_damage": ("Fire Damage", "Dégâts de feu"),
    "apothic_attributes:healing_received": ("Healing Received", "Soins reçus"),
    "apothic_attributes:life_steal": ("Life Steal", "Vol de vie"),
    "apothic_attributes:mining_speed": ("Mining Speed", "Vitesse de minage"),
    "apothic_attributes:draw_speed": ("Draw Speed", "Vitesse de tension"),
    "apothic_attributes:overheal": ("Overheal", "Soins excédentaires"),
}

ENCHANT_NAMES = {
    "minecraft:sharpness": ("Sharpness", "Tranchant"),
    "minecraft:protection": ("Protection", "Protection"),
    "minecraft:fortune": ("Fortune", "Fortune"),
    "minecraft:efficiency": ("Efficiency", "Efficacité"),
}

# The port's own wording for a gem's tooltip, which the native tooltip hook
# builds (`gems::tooltip_lines`): the gear categories (`gem_class.<key>`, one
# per Java `gem_class` key, their English text also written into the Rust as
# `CLASS_TEXT`), the headings and the purity words.  Written here, not taken
# from the Java lang files.
GEM_CLASS_TEXT = {
    "anything": ("Any gear", "Tout équipement"),
    "armor": ("Armor", "Armures"),
    "boots": ("Boots", "Bottes"),
    "bow": ("Bows and crossbows", "Arcs et arbalètes"),
    "breaker": ("Digging tools", "Outils de minage"),
    "chestplate": ("Chestplates", "Plastrons"),
    "core_armor": ("Chestplates and leggings", "Plastrons et jambières"),
    "helmet": ("Helmets", "Casques"),
    "leggings": ("Leggings", "Jambières"),
    "light_weapon": ("Melee weapons and tridents", "Armes de mêlée et tridents"),
    "lower_armor": ("Leggings and boots", "Jambières et bottes"),
    "melee_weapon": ("Melee weapons", "Armes de mêlée"),
    "ranged_weapon": ("Ranged weapons", "Armes à distance"),
    "shield": ("Shields", "Boucliers"),
    "tools": ("Tools", "Outils"),
    "trident": ("Tridents", "Tridents"),
    "weapon_or_tool": ("Weapons and tools", "Armes et outils"),
    "weapons": ("Weapons", "Armes"),
}
LORE_TEXT = {
    "text.apotheosis.socketable_into": ("Can be set into:", "Se sertit dans :"),
    "text.apotheosis.when_socketed_in": ("Once set into:", "Une fois sertie dans :"),
    "text.apotheosis.dot_prefix": ("• %s", "• %s"),
    "text.apotheosis.gem_bonus": ("%s: %s", "%s : %s"),
}
PURITY_TEXT = {
    "cracked": ("Cracked", "Fêlée"),
    "chipped": ("Chipped", "Ébréchée"),
    "flawed": ("Flawed", "Imparfaite"),
    "normal": ("Normal", "Normale"),
    "flawless": ("Flawless", "Sans défaut"),
    "perfect": ("Perfect", "Parfaite"),
}
# The purity word a gem's item name carries: none at Normal, as in Java.
PURITY_PREFIX_EN = ("Cracked ", "Chipped ", "Flawed ", "", "Flawless ", "Perfect ")
PURITY_SUFFIX_FR = ("fêlée", "ébréchée", "imparfaite", "", "sans défaut", "parfaite")
# Bonus types this port implements.  Every other type is listed in the gem's
# `unsupported` field, and the README says what it would have done.
SUPPORTED_TYPES = ("attribute", "multi_attribute", "enchantment")


def classes_of(gem_class: object) -> list[str]:
    """The port class names a `gem_class` names (a string or key/types object).

    A `types` value that is a tag (`{"type": "neoforge:any"}`) means any item.
    """
    if isinstance(gem_class, str):
        return [gem_class.removeprefix("apotheosis:")]
    if not isinstance(gem_class, dict):
        raise SystemExit(f"unreadable gem_class: {gem_class!r}")
    types = gem_class.get("types")
    if isinstance(types, dict):  # a tag such as neoforge:any
        return ["anything"]
    if isinstance(types, str):
        types = [types]
    if not isinstance(types, list):
        raise SystemExit(f"unreadable gem_class types: {gem_class!r}")
    out = []
    for entry in types:
        if isinstance(entry, dict):
            return ["anything"]
        out.append(str(entry).removeprefix("apotheosis:"))
    return out


def class_key(gem_class: object) -> str:
    """The `gem_class.<key>` a bonus is shown under: the object's `key`, or the
    class a plain string names."""
    if isinstance(gem_class, str):
        return gem_class.removeprefix("apotheosis:")
    if isinstance(gem_class, dict) and isinstance(gem_class.get("key"), str):
        return gem_class["key"]
    raise SystemExit(f"gem_class without a key: {gem_class!r}")


def values_of(raw: object) -> list[float | None]:
    """A per-purity value table, in this port's purity order."""
    if not isinstance(raw, dict):
        return [None] * len(PURITIES)
    return [None if purity not in raw else raw[purity] for purity in PURITIES]


def fmt_number(value: float) -> str:
    if float(value).is_integer():
        return f"{value:.1f}"
    return repr(float(value))


def fmt_option(value: float | None) -> str:
    return "None" if value is None else f"Some({fmt_number(value)})"


def fmt_values(values: list[float | None]) -> str:
    return "[" + ", ".join(fmt_option(v) for v in values) + "]"


def rust_gem(
    path: Path, lang: dict[str, str], fr_lang: dict[str, str], gem_id: str
) -> tuple[str, dict]:
    data = json.loads(path.read_text(encoding="utf-8-sig"))
    weights = data.get("weights", {})
    if "weight" in weights:
        weight = int(weights["weight"])
    else:  # one entry per world tier; this port has no tiers, so keep the max
        weight = max(int(entry["weight"]) for entry in weights.values())
    dimensions = data.get("constraints", {}).get("dimensions", [])
    min_purity = data.get("min_purity", "cracked")

    bonuses: list[str] = []
    bonus_lang: list[tuple[str, str, str]] = []
    lore_bonuses: list[dict] = []
    unsupported: list[str] = []
    unsupported_attributes: list[str] = []

    def add_bonus(kind: str, classes: list[str], body: str) -> int:
        class_list = ", ".join(f"GemClass::{CLASSES[c][0]}" for c in classes)
        bonuses.append(
            "        Bonus {\n"
            f"            classes: &[{class_list}],\n"
            f'            key: "{key}",\n'
            f"            kind: {body},\n"
            "        },"
        )
        return len(bonuses) - 1

    def class_text(classes: list[str], fr: bool) -> str:
        names = []
        for class_name in classes:
            key = f"gem_class.{class_name}"
            if fr:
                names.append(fr_lang.get(key) or CLASS_NAMES_FR.get(class_name, class_name))
            else:
                names.append(lang.get(key) or CLASS_NAMES_EN.get(class_name, class_name))
        return (" ou " if fr else " or ").join(names)

    def translated_line(number: int, classes: list[str], en: str, fr: str) -> None:
        bonus_lang.append(
            (
                f"apotheosis.gem.bonus.{gem_id}.{number}",
                f"{class_text(classes, False)}: %s {en}",
                f"{class_text(classes, True)} : %s {fr}",
            )
        )

    for bonus in data.get("bonuses", []):
        kind = str(bonus.get("type", "")).removeprefix("apotheosis:")
        if kind not in SUPPORTED_TYPES:
            unsupported.append(kind)
            continue
        classes = classes_of(bonus.get("gem_class"))
        key = class_key(bonus.get("gem_class"))
        if kind == "multi_attribute":
            for modifier in bonus.get("modifiers", []):
                attribute = modifier["attribute"]
                if attribute not in ATTRIBUTES:
                    unsupported_attributes.append(attribute)
                    continue
                number = add_bonus(
                    kind,
                    classes,
                    "Kind::Attribute {\n"
                    f'                attribute: "{ATTRIBUTES[attribute]}",\n'
                    f'                operation: "{modifier["operation"]}",\n'
                    f"                values: {fmt_values(values_of(modifier.get('values')))},\n"
                    "            }",
                )
                en_name, fr_name = ATTR_NAMES[ATTRIBUTES[attribute]]
                translated_line(number, classes, en_name, fr_name)
                lore_bonuses.append({
                    "class": key,
                    "attribute": ATTRIBUTES[attribute],
                    "values": values_of(modifier.get("values")),
                })
        elif kind == "attribute":
            attribute = bonus["attribute"]
            if attribute not in ATTRIBUTES:
                unsupported_attributes.append(attribute)
                continue
            number = add_bonus(
                kind,
                classes,
                "Kind::Attribute {\n"
                f'                attribute: "{ATTRIBUTES[attribute]}",\n'
                f'                operation: "{bonus["operation"]}",\n'
                f"                values: {fmt_values(values_of(bonus.get('values')))},\n"
                "            }",
            )
            en_name, fr_name = ATTR_NAMES[ATTRIBUTES[attribute]]
            translated_line(number, classes, en_name, fr_name)
            lore_bonuses.append({
                "class": key,
                "attribute": ATTRIBUTES[attribute],
                "values": values_of(bonus.get("values")),
            })
        else:  # enchantment
            enchantment = bonus["enchantment"]
            if not enchantment.startswith("minecraft:"):
                # A modded enchantment this port does not have: no placeholder.
                unsupported.append(f"enchantment:{enchantment}")
                continue
            levels = [None if v is None else int(v) for v in values_of(bonus.get("values"))]
            number = add_bonus(
                kind,
                classes,
                "Kind::Enchantment {\n"
                f'                id: "{enchantment}",\n'
                f"                existing_only: {str(bonus.get('mode', 'add') != 'global').lower()},\n"
                f"                levels: [{', '.join('None' if v is None else f'Some({v})' for v in levels)}],\n"
                "            }",
            )
            en_name, fr_name = ENCHANT_NAMES.get(
                enchantment, (enchantment.rsplit(':', 1)[-1].title(), enchantment.rsplit(':', 1)[-1])
            )
            translated_line(number, classes, en_name, fr_name)
            lore_bonuses.append({
                "class": key,
                "enchantment": enchantment,
                "values": levels,
            })

    name = lang.get(f"item.apotheosis.gem.{gem_id}")
    if name is None:
        raise SystemExit(f"no English name for {gem_id}")
    dims = ", ".join(f'"{d}"' for d in dimensions)
    rust = [
        "    Gem {",
        f'        id: "{gem_id}",',
        f'        name: "{name}",',
        f"        weight: {weight},",
        f"        min_purity: Purity::{min_purity.capitalize()},",
        f"        dimensions: &[{dims}],",
        "        bonuses: &[",
        *bonuses,
        "        ],",
        f"        unsupported: &[{', '.join(chr(34) + u + chr(34) for u in sorted(set(unsupported)))}],",
        "        unsupported_attributes: &["
        + ", ".join(f'"{a}"' for a in sorted(set(unsupported_attributes)))
        + "],",
        "    },",
    ]
    texture = path.stem
    return "\n".join(rust), {
        "texture": texture,
        "gem_id": gem_id,
        "name": name,
        "min_purity": min_purity,
        "bonus_lang": bonus_lang,
        "lore_bonuses": lore_bonuses,
    }


def write_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def english_item_name(info: dict, index: int) -> str:
    """`Cracked Solar Gem`; a Normal gem has no purity word."""
    return PURITY_PREFIX_EN[index] + info["name"]


def french_item_name(base: str, index: int) -> str:
    """`Gemme solaire fêlée`, `Gemme fêlée du bagarreur`: the purity agrees
    with `Gemme` and follows the noun, before a `de`/`du` complement."""
    suffix = PURITY_SUFFIX_FR[index]
    if not suffix:
        return base
    head, _, rest = base.partition(" ")
    if rest.startswith(("de ", "du ", "des ", "d'", "d’")):
        return f"{head} {suffix} {rest}"
    return f"{base} {suffix}"


def gem_items(models: list[dict]) -> list[str]:
    """Writes one item per gem and allowed purity; returns their ids in the
    Java creative tab's order (gems by id, then purity)."""
    wanted: list[str] = []
    for info in models:
        floor = PURITIES.index(info["min_purity"])
        for index, purity in enumerate(PURITIES):
            if index < floor:
                continue
            name = f"gem_{info['texture']}_{purity}"
            wanted.append(name)
            write_json(ITEM_DECLS / f"{name}.json", {
                "template": "minecraft:amethyst_shard",
                "max_stack_size": 64,
                "rarity": PURITY_RARITIES[index],
                "components": {
                    "minecraft:item_name": {
                        "translate": f"item.apotheosis.{name}",
                        "fallback": english_item_name(info, index),
                    },
                },
            })
            write_json(ITEM_MODELS / f"{name}.json", {
                "model": {
                    "type": "minecraft:model",
                    "model": f"apotheosis:item/gem_{info['texture']}",
                }
            })
    # A gem item a previous catalogue had (a floor that rose) is removed.
    every = {
        f"gem_{info['texture']}_{purity}" for info in models for purity in PURITIES
    }
    for folder in (ITEM_DECLS, ITEM_MODELS):
        for path in folder.glob("gem_*.json"):
            if path.stem in every and path.stem not in wanted:
                path.unlink()
    ids = [f"apotheosis:{name}" for name in wanted]
    every_ids = {f"apotheosis:{name}" for name in every}

    # The tab: the gem items right after the generic gem, in Java's order.
    tab = json.loads(TAB.read_text(encoding="utf-8"))
    items = [item for item in tab["items"] if item not in every_ids]
    at = items.index("apotheosis:gem") + 1 if "apotheosis:gem" in items else 0
    tab["items"] = items[:at] + ids + items[at:]
    write_json(TAB, tab)
    write_json(GEMS_TAG, {"values": ["apotheosis:gem", *ids]})
    return ids


def gem_item_names(
    models: list[dict], ids: list[str], fr_names: dict[str, str]
) -> dict[str, tuple[str, str]]:
    """`item.apotheosis.gem_<texture>_<purity>` -> its English and French
    name, the purity worded in each language.  The French base is the gem's
    `item.apotheosis.gem.<gem_id>` in the port's fr_fr (gen_content.py)."""
    by_texture = {f"gem_{info['texture']}": info for info in models}
    names = {}
    for item in ids:
        path = item.split(":", 1)[1]
        stem, purity = path.rsplit("_", 1)
        info = by_texture[stem]
        index = PURITIES.index(purity)
        base = fr_names.get(f"item.apotheosis.gem.{info['gem_id']}", info["name"])
        names[f"item.apotheosis.{path}"] = (
            english_item_name(info, index),
            french_item_name(base, index),
        )
    return names


def main() -> None:
    ref = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_REF
    gems_dir = ref / "data" / "apotheosis" / "gems"
    lang = json.loads(
        (ref / "assets" / "apotheosis" / "lang" / "en_us.json").read_text(
            encoding="utf-8-sig"
        )
    )
    fr_lang = json.loads(
        (ref / "assets" / "apotheosis" / "lang" / "fr_fr.json").read_text(
            encoding="utf-8-sig"
        )
    )
    entries: list[str] = []
    models: list[dict] = []
    for path in sorted(gems_dir.rglob("*.json")):
        if path.parent.name == "twilight":
            continue  # Twilight Forest is not part of this port
        gem_id = "apotheosis:" + path.relative_to(gems_dir).as_posix().removesuffix(".json")
        rust, info = rust_gem(path, lang, fr_lang, gem_id)
        entries.append(rust)
        models.append(info)
    used_classes = {e["class"] for info in models for e in info["lore_bonuses"]}
    missing = sorted(used_classes - GEM_CLASS_TEXT.keys())
    if missing:
        raise SystemExit(f"no wording for gem classes {missing}")

    header = '''//! Apotheosis 8.9.0's gems, generated by `tools/gen_java_gems.py`.
//!
//! Do not edit: change the generator, or the Java data it reads.  Each gem
//! carries the ids, name, weight, purity floor, dimension constraint and
//! per-purity bonus values of its Java file.  A bonus whose type or attribute
//! this port cannot pay out is named in `unsupported`/`unsupported_attributes`
//! rather than replaced by an invented one.

use crate::gems::GemClass;
use crate::sockets::Purity;

/// One gem bonus: the classes of gear it applies to, and what it does.
pub struct Bonus {
    pub classes: &'static [GemClass],
    /// The Java `gem_class` key it is shown under: `gem_class.<key>`.
    pub key: &'static str,
    pub kind: Kind,
}

pub enum Kind {
    /// A real `minecraft:attribute_modifiers` entry, at the value the gem's
    /// purity names.  An attribute this port implements through Apothic
    /// Attributes is stored in the item's custom data instead.
    Attribute {
        attribute: &'static str,
        operation: &'static str,
        values: [Option<f64>; 6],
    },
    /// An enchantment level, added to the item (`existing_only` raises an
    /// enchantment the item already carries instead of adding one).
    Enchantment {
        id: &'static str,
        existing_only: bool,
        levels: [Option<u8>; 6],
    },
}

pub struct Gem {
    /// `apotheosis:core/ballast`, the Java gem type id.
    pub id: &'static str,
    /// The English name java's lang file gives it.
    pub name: &'static str,
    /// Its weight among the gems of its dimension.
    pub weight: u64,
    /// The lowest purity it drops at.
    pub min_purity: Purity,
    /// The dimensions it drops in; empty for any.
    pub dimensions: &'static [&'static str],
    pub bonuses: &'static [Bonus],
    /// Java bonus types this port does not implement, by type name.
    pub unsupported: &'static [&'static str],
    /// Attributes of this gem's attribute bonuses this port does not implement.
    pub unsupported_attributes: &'static [&'static str],
}

pub const GEMS: [Gem; %d] = [
%s
];

/// The English text of each `gem_class.<key>` a bonus uses: the fallback of
/// the key the lang files translate.
pub const CLASS_TEXT: [(&str, &str); %d] = [
%s
];
''' % (
        len(entries),
        "\n".join(entries),
        len(used_classes),
        "\n".join(
            f'    ("{key}", "{GEM_CLASS_TEXT[key][0]}"),' for key in sorted(used_classes)
        ),
    )
    RUST.write_text(header, encoding="utf-8", newline="\n")

    for info in models:
        path = MODELS / f"gem_{info['texture']}.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        model = {
            "parent": "minecraft:item/generated",
            "textures": {"layer0": f"apotheosis:item/gems/{info['texture']}"},
        }
        path.write_text(
            json.dumps(model, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
            newline="\n",
        )
    ids = gem_items(models)
    print(f"wrote {len(ids)} gem items, the creative tab and the gems tag")
    # Add every bonus line's translation key to our own lang files.  Existing
    # keys are kept as they are, so a hand-written translation is never
    # overwritten by a later run.
    lang_dir = ROOT / "content" / "assets" / "apotheosis" / "lang"
    fr_names = json.loads((lang_dir / "fr_fr.json").read_text(encoding="utf-8"))
    item_names = gem_item_names(models, ids, fr_names)
    own_text = {
        **LORE_TEXT,
        **{f"gem_class.{key}": GEM_CLASS_TEXT[key] for key in sorted(used_classes)},
        **{f"purity.apotheosis.{p}": PURITY_TEXT[p] for p in PURITIES},
        **item_names,
    }
    for filename, column in (("en_us", 1), ("fr_fr", 2)):
        path = lang_dir / f"{filename}.json"
        data = json.loads(path.read_text(encoding="utf-8")) if path.exists() else {}
        added = 0
        for info in models:
            for entry in info["bonus_lang"]:
                key, text = entry[0], entry[column]
                if key not in data:
                    data[key] = text
                    added += 1
        # Each gem item's own name key (its `minecraft:item_name` translates
        # it, as does /give's message), with its purity worded, plus the
        # tooltip's headings, gear categories and purity words: the port's own
        # text, kept in step with this generator.
        for key, texts in own_text.items():
            if data.get(key) != texts[column - 1]:
                data[key] = texts[column - 1]
                added += 1
        path.write_text(
            json.dumps(data, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
            newline="\n",
        )
        print(f"added {added} gem bonus keys to {path.relative_to(ROOT)}")
    print(f"wrote {len(entries)} gems to {RUST.relative_to(ROOT)} and their models")


if __name__ == "__main__":
    main()
