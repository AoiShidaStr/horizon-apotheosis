#!/usr/bin/env python3
"""Generate this port's gem catalogue from Apotheosis 8.9.0's gem data.

The Java mod's gem data (`data/apotheosis/gems/**`) is the reference: 23 gems,
21 of them outside the Twilight Forest (which this port does not ship).  This
script reads that data read-only and writes:

  native/apotheosis/api/src/java_gems.rs   the catalogue the native mod uses
  content/assets/apotheosis/models/item/gem_<name>.json
                                            one model per gem, on the imported
                                            Java texture (git-ignored layer)

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


def rust_gem(path: Path, lang: dict[str, str], gem_id: str) -> tuple[str, dict]:
    data = json.loads(path.read_text(encoding="utf-8-sig"))
    weights = data.get("weights", {})
    if "weight" in weights:
        weight = int(weights["weight"])
    else:  # one entry per world tier; this port has no tiers, so keep the max
        weight = max(int(entry["weight"]) for entry in weights.values())
    dimensions = data.get("constraints", {}).get("dimensions", [])
    min_purity = data.get("min_purity", "cracked")

    bonuses: list[str] = []
    unsupported: list[str] = []
    unsupported_attributes: list[str] = []

    def add_bonus(kind: str, classes: list[str], body: str) -> None:
        class_list = ", ".join(f"GemClass::{CLASSES[c][0]}" for c in classes)
        bonuses.append(
            "        Bonus {\n"
            f"            classes: &[{class_list}],\n"
            f"            kind: {body},\n"
            "        },"
        )

    for bonus in data.get("bonuses", []):
        kind = str(bonus.get("type", "")).removeprefix("apotheosis:")
        if kind not in SUPPORTED_TYPES:
            unsupported.append(kind)
            continue
        classes = classes_of(bonus.get("gem_class"))
        if kind == "multi_attribute":
            for modifier in bonus.get("modifiers", []):
                attribute = modifier["attribute"]
                if attribute not in ATTRIBUTES:
                    unsupported_attributes.append(attribute)
                    continue
                add_bonus(
                    kind,
                    classes,
                    "Kind::Attribute {\n"
                    f'                attribute: "{ATTRIBUTES[attribute]}",\n'
                    f'                operation: "{modifier["operation"]}",\n'
                    f"                values: {fmt_values(values_of(modifier.get('values')))},\n"
                    "            }",
                )
        elif kind == "attribute":
            attribute = bonus["attribute"]
            if attribute not in ATTRIBUTES:
                unsupported_attributes.append(attribute)
                continue
            add_bonus(
                kind,
                classes,
                "Kind::Attribute {\n"
                f'                attribute: "{ATTRIBUTES[attribute]}",\n'
                f'                operation: "{bonus["operation"]}",\n'
                f"                values: {fmt_values(values_of(bonus.get('values')))},\n"
                "            }",
            )
        else:  # enchantment
            enchantment = bonus["enchantment"]
            if not enchantment.startswith("minecraft:"):
                # A modded enchantment this port does not have: no placeholder.
                unsupported.append(f"enchantment:{enchantment}")
                continue
            levels = [None if v is None else int(v) for v in values_of(bonus.get("values"))]
            add_bonus(
                kind,
                classes,
                "Kind::Enchantment {\n"
                f'                id: "{enchantment}",\n'
                f"                existing_only: {str(bonus.get('mode', 'add') != 'global').lower()},\n"
                f"                levels: [{', '.join('None' if v is None else f'Some({v})' for v in levels)}],\n"
                "            }",
            )

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
    return "\n".join(rust), {"texture": texture, "gem_id": gem_id}


def main() -> None:
    ref = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_REF
    gems_dir = ref / "data" / "apotheosis" / "gems"
    lang = json.loads(
        (ref / "assets" / "apotheosis" / "lang" / "en_us.json").read_text(
            encoding="utf-8-sig"
        )
    )
    entries: list[str] = []
    models: list[dict] = []
    for path in sorted(gems_dir.rglob("*.json")):
        if path.parent.name == "twilight":
            continue  # Twilight Forest is not part of this port
        gem_id = "apotheosis:" + path.relative_to(gems_dir).as_posix().removesuffix(".json")
        rust, info = rust_gem(path, lang, gem_id)
        entries.append(rust)
        models.append(info)

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
''' % (len(entries), "\n".join(entries))
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
    print(f"wrote {len(entries)} gems to {RUST.relative_to(ROOT)} and their models")


if __name__ == "__main__":
    main()
