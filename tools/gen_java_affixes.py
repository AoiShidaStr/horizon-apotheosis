#!/usr/bin/env python3
"""Generate the Java affix catalogue for the native Apotheosis port.

Run: python tools/gen_java_affixes.py [path-to-extracted-Apotheosis-jar]

The source is the *data* half of Apotheosis 8.9.0 (ids, categories, rarities,
value ranges, operations). No asset and no Java code is copied: the generated
file holds game values, and the language entries below are short fallbacks
written for this port. The Java translation keys are used verbatim so a
client that has the Java language file sees the Java names.

The script is idempotent: it rewrites `native/apotheosis/api/src/java_affixes.rs`
only when its content changes, and merges its language keys into the shared
tables without removing the keys of other generators.

The reference tree defaults to the season-1 extraction. Set
`HORIZON_JAVA_REF` or pass a path to use another extraction.
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
DEFAULT_REF = os.path.join("C:", os.sep, "Users", "yello", "horizon-work",
                           "java-ref", "Apotheosis-1.21.1-8.9.0")
REF = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("HORIZON_JAVA_REF", DEFAULT_REF)
ID = "apotheosis"
OUT = os.path.join(ROOT, "native", "apotheosis", "api", "src", "java_affixes.rs")
LANG = os.path.join(ROOT, "content", "assets", ID, "lang")

RARITIES = ["common", "uncommon", "rare", "epic", "mythic"]

# Attributes whose behaviour this port implements, and the `Attr` variant that
# stores them. Anything else stays in the catalogue with `supported = false`.
APOTHIC_ATTRS = {
    "crit_chance": "CritChance",
    "crit_damage": "CritDamage",
    "life_steal": "LifeSteal",
    "overheal": "Overheal",
    "armor_pierce": "ArmorPierce",
    "armor_shred": "ArmorShred",
    "fire_damage": "FireDamage",
    "cold_damage": "ColdDamage",
    "experience_gained": "ExperienceGained",
    "healing_received": "HealingReceived",
    "arrow_damage": "ArrowDamage",
    "arrow_velocity": "ArrowVelocity",
    "draw_speed": "DrawSpeed",
    "mining_speed": "MiningSpeed",
    "current_hp_damage": "CurrentHpDamage",
    "prot_pierce": "ProtPierce",
    "prot_shred": "ProtShred",
}

# Effect types whose payout a hook of this port can reach. `radial`,
# `stoneforming`, `attribute_toggle`, `omnetic`, `enlightened`, `catalyzing`,
# `psychic` and the ability affixes with no data stay unsupported.
EFFECT_TYPES = {
    "damage_reduction",
    "executing",
    "cleaving",
    "festive",
    "thunderstruck",
    "retreating",
    "telepathic",
}

# Java targets a damage hook can pay: the attacker's weapon or the victim's
# worn or held gear. `arrow_target`/`arrow_self` need the projectile hit, which
# this API gives no host for, so only a trident (melee) can pay them.
REACHABLE_TARGETS = {
    "attack_target",
    "attack_self",
    "hurt_attacker",
    "hurt_self",
    "block_attacker",
    "block_self",
}
TRIDENT_TARGETS = {"arrow_target", "arrow_self"}

# The effect names and damage kinds whose tooltip this port writes itself.
EFFECT_TEXT = {
    "minecraft:blindness": ("Blindness", "Cécité"),
    "minecraft:poison": ("Poison", "Vénin"),
    "minecraft:weakness": ("Weakness", "Faiblesse"),
    "minecraft:slowness": ("Slowness", "Lenteur"),
    "minecraft:wither": ("Wither", "Flétrissement"),
    "minecraft:speed": ("Speed", "Vitesse"),
    "minecraft:resistance": ("Resistance", "Résistance"),
    "minecraft:instant_health": ("Instant Health", "Soin instantané"),
    "minecraft:levitation": ("Levitation", "Lévitation"),
    "minecraft:haste": ("Haste", "Célérité"),
}
# The categories a folder implies when the affix does not spell them out.
CATEGORY_BY_FOLDER = {
    "armor": ["apotheosis:armor"],
    "boots": ["apotheosis:boots"],
    "breaker": ["apotheosis:breaker"],
    "generic": [
        "apotheosis:armor",
        "apotheosis:melee_weapon",
        "apotheosis:trident",
        "apotheosis:bow",
        "apotheosis:breaker",
        "apotheosis:shield",
        "apotheosis:shears",
    ],
    "melee": ["apotheosis:melee_weapon", "apotheosis:trident"],
    "ranged": ["apotheosis:bow"],
    "shears": ["apotheosis:shears"],
    "shield": ["apotheosis:shield"],
    "weapon": ["apotheosis:weapon"],
}

DAMAGE_TEXT = {
    "explosion": ("Explosion", "Explosion"),
    "physical": ("Physical", "Physique"),
    "projectile": ("Projectile", "Projectile"),
    "fall": ("Fall", "Chute"),
    "lightning": ("Lightning", "Foudre"),
    "magic": ("Magic", "Magie"),
}

# `minecraft:generic.foo` is the 1.21.1 spelling; Horizon's flattened
# attribute ids are `minecraft:foo`. These are real attributes, so the game
# applies their modifiers itself once the id and operation are right.
REAL_ATTRIBUTES = {
    "armor", "armor_toughness", "attack_damage", "attack_knockback",
    "attack_speed", "block_break_speed", "block_interaction_range",
    "burning_time", "camera_distance", "entity_interaction_range",
    "explosion_knockback_resistance", "fall_damage_multiplier", "flying_speed",
    "follow_range", "gravity", "jump_strength", "knockback_resistance",
    "luck", "max_health", "mining_efficiency", "movement_efficiency",
    "movement_speed", "oxygen_bonus", "safe_fall_distance", "scale",
    "sneaking_speed", "step_height", "submerged_mining_speed",
    "sweeping_damage_ratio", "water_movement_efficiency", "waypoint_transmit_range",
}

# Hand-written short fallbacks. The key is the affix path inside the Java data
# tree. English and French are our own wording; the Java text is not copied.
NAMES = {
    "armor/attribute/adamantine": ("Adamantine", "of the Adamant", "Adamantin", "de l'Adamant"),
    "armor/attribute/aquatic": ("Aquatic", "of the Tide", "Aquatique", "de la Marée"),
    "armor/attribute/blessed": ("Blessed", "of Vigor", "Béni", "de la Vigueur"),
    "armor/attribute/elastic": ("Elastic", "of the Climb", "Élastique", "de l'Escalade"),
    "armor/attribute/fireproof": ("Fireproof", "of Cooling", "Ignifugé", "du Refuge"),
    "armor/attribute/fortunate": ("Fortunate", "of Clover", "Fortuné", "du Trèfle"),
    "armor/attribute/gravitational": ("Gravitational", "of Gravity", "Gravitationnel", "de la Gravité"),
    "armor/attribute/ironforged": ("Ironforged", "of Iron", "Forgé de fer", "du Fer"),
    "armor/attribute/oxygenated": ("Oxygenated", "of the Diver", "Oxygéné", "du Plongeur"),
    "armor/attribute/spiritual": ("Spiritual", "of the Healer", "Spirituel", "du Guérisseur"),
    "armor/attribute/stalwart": ("Stalwart", "of Endurance", "Inébranlable", "de l'Endurance"),
    "armor/attribute/steel_touched": ("Steel Touched", "of the Bulwark", "Trempé d'acier", "du Rempart"),
    "armor/attribute/timekeeping": ("Timekeeping", "of Chronos", "Chronomètre", "de Chronos"),
    "armor/attribute/unbound": ("Unbound", "of Icarus", "Affranchi", "d'Icare"),
    "armor/attribute/windswept": ("Windswept", "of the Gale", "Venteux", "de la Rafale"),
    "armor/attribute/winged": ("Winged", "of the Sky", "Ailé", "du Ciel"),
    "breaker/attribute/destructive": ("Destructive", "of Quarrying", "Destructeur", "de la Carrière"),
    "breaker/attribute/experienced": ("Experienced", "of the Scholar", "Expérimenté", "de l'Érudit"),
    "breaker/attribute/lengthy": ("Lengthy", "of Reach", "Long", "de la Portée"),
    "breaker/attribute/submerged": ("Submerged", "of the Deep", "Submergé", "des Profondeurs"),
    "generic/attribute/lucky": ("Lucky", "of Fortune", "Chanceux", "de la Fortune"),
    "melee/attribute/berserking": ("Berserking", "of Rage", "Berserk", "de la Rage"),
    "melee/attribute/forceful": ("Forceful", "of Impact", "Puissant", "de l'Impact"),
    "melee/attribute/giant_slaying": ("Giant Slaying", "of Colossus", "Tueur de géants", "du Colosse"),
    "melee/attribute/glacial": ("Glacial", "of the Frost", "Glacial", "du Givre"),
    "melee/attribute/graceful": ("Graceful", "of Poise", "Gracieux", "de l'Aplomb"),
    "melee/attribute/infernal": ("Infernal", "of Embers", "Infernal", "des Braises"),
    "melee/attribute/intricate": ("Intricate", "of Precision", "Intriqué", "de la Précision"),
    "melee/attribute/lacerating": ("Lacerating", "of Deep Wounds", "Déchirant", "des Plaies"),
    "melee/attribute/lengthy": ("Lengthy", "of Reach", "Long", "de la Portée"),
    "melee/attribute/murderous": ("Murderous", "of the Beast", "Meurtrier", "de la Bête"),
    "melee/attribute/piercing": ("Piercing", "of the Lance", "Perçant", "de la Lance"),
    "melee/attribute/vampiric": ("Vampiric", "of the Leech", "Vampirique", "de la Sangsue"),
    "melee/attribute/violent": ("Violent", "of Fury", "Violent", "de la Fureur"),
    "ranged/attribute/agile": ("Agile", "of the Bow", "Agile", "de l'Arc"),
    "ranged/attribute/elven": ("Elven", "of the Archer", "Elfique", "de l'Archer"),
    "ranged/attribute/streamlined": ("Streamlined", "of the Sniper", "Profilé", "du Tireur"),
    "ranged/attribute/windswept": ("Windswept", "of the Gale", "Venteux", "de la Rafale"),
    "shield/attribute/ironforged": ("Ironforged", "of Iron", "Forgé de fer", "du Fer"),
    "shield/attribute/stalwart": ("Stalwart", "of Endurance", "Inébranlable", "de l'Endurance"),
    "shield/attribute/steel_touched": ("Steel Touched", "of the Bulwark", "Trempé d'acier", "du Rempart"),
    "weapon/attribute/nullifying": ("Nullifying", "of Unmaking", "Annihilant", "de l'Anéantissement"),
    "weapon/attribute/runebreaking": ("Runebreaking", "of the Runebreaker", "Briserunes", "du Briseur de runes"),
    "weapon/attribute/shredding": ("Shredding", "of the Maelstrom", "Déchiqueteur", "du Maelström"),
    "armor/mob_effect/blinding": ("Blinding", "of Darkness", "Aveuglant", "des Ténèbres"),
    "melee/mob_effect/weakening": ("Weakening", "of Frailty", "Affaiblissant", "de la Faiblesse"),
    "ranged/mob_effect/ensnaring": ("Ensnaring", "of the Predator", "Entravant", "du Prédateur"),
    "shield/mob_effect/venomous": ("Venomous", "of the Serpent", "Venimeux", "du Serpent"),
    "shield/mob_effect/withering": ("Withering", "of Decay", "Flétrissant", "du Déclin"),
    "melee/executing": ("Executing", "of the Executioner", "Exécuteur", "du Bourreau"),
    "melee/cleaving": ("Cleaving", "of the Butcher", "Fendant", "du Boucher"),
    "melee/thunderstruck": ("Thunderstruck", "of the Storm", "Foudroyant", "de la Tempête"),
    "shield/retreating": ("Retreating", "of the Jester", "Retrait", "du Bouffon"),
    "durable": ("Durable", "of Durability", "Durable", "de la Durabilité"),
    "generic/telepathic": ("Telepathic", "of Space and Time", "Télépathique", "de l'Espace-Temps"),
    "ranged/spectral": ("Spectral", "of the Forlorn", "Spectral", "des Abandonnés"),
    "ranged/magical": ("Magical", "of the Arcane", "Magique", "des Arcanes"),
    "melee/festive": ("Festive", "of the Revel", "Festif", "de la Fête"),
}


def read_json(path):
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def write_if_changed(path, text):
    old = None
    if os.path.exists(path):
        with open(path, encoding="utf-8") as handle:
            old = handle.read()
    if old != text:
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8", newline="\n") as handle:
            handle.write(text)
        return True
    return False


def rust_string(value):
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def number(value):
    text = ("%.6f" % float(value)).rstrip("0").rstrip(".")
    if text in ("", "-0"):
        text = "0"
    if "." not in text:
        text += ".0"
    return text


def value_range(value):
    """A Java AffixValue as (min, max, step); a plain number is a fixed one.

    Effect rolls carry their numbers under named fields (`duration`,
    `amplifier`, ...), so they are read by `effect_values` instead.
    """
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        return (float(value), float(value), 0.0)
    if not isinstance(value, dict):
        return None
    if "min" in value or "max" in value:
        low = value.get("min", value.get("max", 0.0))
        high = value.get("max", low)
        return (float(low), float(high), float(value.get("step", 0.0)))
    return None


def effect_values(raw):
    """One rarity's effect roll: its ranges, cooldown and roll count.

    `None` when the rarity carries no effect roll, which is also how Java's
    plain-value affixes (executing, thunderstruck, damage reduction) stay in
    `values`.
    """
    if not isinstance(raw, dict):
        return None
    found = {"duration": None, "amplifier": None, "chance": None,
             "targets": None, "cooldown": 0, "rolls": 0}
    hit = False
    for key in ("duration", "amplifier", "chance", "targets"):
        if key in raw:
            found[key] = value_range(raw[key])
            hit = True
    if "cooldown" in raw:
        found["cooldown"] = int(raw["cooldown"])
        hit = True
    if "rolls" in raw:
        found["rolls"] = int(raw["rolls"])
        hit = True
    return found if hit else None


def effect_support(kind, data):
    """Whether a non-attribute affix can be paid out by this port's hooks."""
    if kind == "mob_effect":
        effect = data.get("mob_effect") or ""
        if not effect.startswith("minecraft:"):
            return False
        target = data.get("target") or ""
        if target in REACHABLE_TARGETS:
            return True
        categories = data.get("categories") or data.get("types") or []
        return target in TRIDENT_TARGETS and "apotheosis:trident" in categories
    if kind == "enchantment":
        # Only the two vanilla enchantments the game itself applies: Fortune on
        # a tool, Looting on a bow (the loot hook reads the killer's weapon).
        return data.get("enchantment") in ("minecraft:fortune", "minecraft:looting")
    return kind in EFFECT_TYPES


def rarity_list(data):
    found = []
    for key in data.get("values", {}):
        name = key.rsplit(":", 1)[-1]
        if name in RARITIES and name not in found:
            found.append(name)
    for key in data.get("rarities", []):
        name = key.rsplit(":", 1)[-1]
        if name in RARITIES and name not in found:
            found.append(name)
    if not found:
        found = list(RARITIES)
    return sorted(found, key=RARITIES.index)


def attribute_id(raw):
    """Normalise a Java attribute id to the id Horizon registers."""
    if raw.startswith("minecraft:generic."):
        return "minecraft:" + raw[len("minecraft:generic."):]
    return raw


def attribute_support(raw):
    """(game id, Attr variant or None, supported?) for an attribute affix."""
    if raw is None:
        return (None, None, False)
    game = attribute_id(raw)
    namespace, path = game.split(":", 1)
    if namespace == "apothic_attributes":
        variant = APOTHIC_ATTRS.get(path)
        return (game, variant, variant is not None)
    if namespace == "minecraft":
        path = path.rsplit(".", 1)[-1]
        return (game, None, path in REAL_ATTRIBUTES)
    return (game, None, False)


def fallback_name(path):
    leaf = path.rsplit("/", 1)[-1]
    special = {"shulkers": "Shulk-Touched", "blighted": "Blighted"}
    words = special.get(leaf, leaf.replace("_", " "))
    return " ".join(word[:1].upper() + word[1:] for word in words.split())


def affix_entries():
    base = os.path.join(REF, "data", ID, "affixes")
    entries = []
    for directory, _dirs, files in os.walk(base):
        for name in files:
            if not name.endswith(".json"):
                continue
            source = os.path.join(directory, name)
            data = read_json(source)
            path = os.path.relpath(source, base)[:-5].replace(os.sep, "/")
            kind = data.get("type", "apotheosis:attribute").split(":", 1)[-1]
            raw_attribute = data.get("attribute")
            game_attribute, variant, supported = attribute_support(raw_attribute)
            if kind != "attribute":
                game_attribute, variant = None, None
                supported = effect_support(kind, data)
            rarities = rarity_list(data)
            values = {}
            effects = {}
            for rarity_name, raw in data.get("values", {}).items():
                short = rarity_name.rsplit(":", 1)[-1]
                if short in RARITIES:
                    parsed = value_range(raw)
                    if parsed is not None:
                        values[short] = parsed
                    rolled = effect_values(raw)
                    if rolled is not None:
                        effects[short] = rolled
            definition = data.get("definition", {})
            weights = definition.get("weights", {})
            weight = int(weights.get("weight", 0)) if isinstance(weights, dict) else 0
            if isinstance(weights, dict) and not weight:
                weight = max((int(v.get("weight", 0)) for v in weights.values()
                              if isinstance(v, dict)), default=0)
            # A file with no category list takes its folder's: Java's
            # `shield/retreating` and `generic/telepathic` rely on this.
            categories = (data.get("categories") or data.get("types")
                          or CATEGORY_BY_FOLDER.get(path.split("/", 1)[0], []))
            names = NAMES.get(path)
            if names is None:
                english = fallback_name(path)
                names = (english, "of " + english, english, "de " + english)
            entries.append({
                "path": path,
                "id": "%s:%s" % (ID, path),
                "name": names[0],
                "suffix": names[1],
                "fr_name": names[2],
                "fr_suffix": names[3],
                "kind": kind,
                "affix_type": definition.get("affix_type", "stat"),
                "attribute": game_attribute,
                "operation": data.get("operation"),
                "categories": categories,
                "rarities": rarities,
                "values": values,
                "effects": effects,
                "effect": data.get("mob_effect"),
                "effect_target": data.get("target"),
                "damage_kind": data.get("damage_type"),
                "enchantment": data.get("enchantment"),
                "weight": weight,
                "attr": variant,
                "supported": supported,
            })
    entries.sort(key=lambda entry: entry["path"])
    return entries


def optional_string(value):
    return "Some(%s)" % rust_string(value) if value else "None"


def range_literal(value):
    return "Some(Range { min: %s, max: %s, step: %s })" % (
        number(value[0]), number(value[1]), number(value[2]))


def effect_literal(value):
    """One rarity's `Effect`, or `None`."""
    if value is None:
        return "None"
    fields = []
    for key in ("duration", "amplifier", "chance", "targets"):
        fields.append("%s: %s" % (key, "None" if value[key] is None
                                  else range_literal(value[key])))
    fields.append("cooldown: %d" % value["cooldown"])
    fields.append("rolls: %d" % value["rolls"])
    return "Some(Effect { %s })" % ", ".join(fields)


def render(entries):
    lines = [
        "// @generated by tools/gen_java_affixes.py from Apotheosis 8.9.0 data.",
        "//! Java affix catalogue: ids, categories, rarities and value ranges.",
        "//!",
        "//! Only game values are reproduced: no Java code or asset is included.",
        "//! `supported` means this port can apply the affix today; unsupported",
        "//! entries keep their Java id so a later slice can enable them, and are",
        "//! documented in the README.",
        "",
        "/// A Java value range. `step` is 0 for a fixed value; `min` may be greater",
        "/// than `max` for Java's negative ranges (fireproof, gravitational).",
        "#[derive(Clone, Copy, Debug, PartialEq)]",
        "pub struct Range {",
        "    pub min: f64,",
        "    pub max: f64,",
        "    pub step: f64,",
        "}",
        "",
        "/// The extra numbers a `mob_effect`, `cleaving` or `festive` rarity rolls.",
        "#[derive(Clone, Copy, Debug)]",
        "pub struct Effect {",
        "    /// Ticks, for a timed effect.",
        "    pub duration: Option<Range>,",
        "    pub amplifier: Option<Range>,",
        "    pub chance: Option<Range>,",
        "    pub targets: Option<Range>,",
        "    /// Ticks between two payouts of the same affix on one entity.",
        "    pub cooldown: u32,",
        "    pub rolls: u32,",
        "}",
        "",
        "/// One affix from `data/apotheosis/affixes/**`.",
        "#[derive(Clone, Copy, Debug)]",
        "pub struct Affix {",
        "    /// `apotheosis:<folder>/<type>/<name>`, the Java id and modifier id.",
        "    pub id: &'static str,",
        "    /// The path under `affixes/`, without the namespace.",
        "    pub path: &'static str,",
        "    pub name_key: &'static str,",
        "    pub suffix_key: &'static str,",
        "    /// Short fallbacks written for this port, not Java's text.",
        "    pub name: &'static str,",
        "    pub suffix: &'static str,",
        "    /// Java's affix type: `attribute`, `mob_effect`, ...",
        "    pub kind: &'static str,",
        "    /// `stat`, `basic_effect` or `ability`.",
        "    pub affix_type: &'static str,",
        "    /// The normalized attribute id, for an attribute affix.",
        "    pub attribute: Option<&'static str>,",
        "    pub operation: Option<&'static str>,",
        "    pub categories: &'static [&'static str],",
        "    /// Indices into `Tier` (common..mythic). Ancient uses mythic's values.",
        "    pub rarities: &'static [u8],",
        "    /// Value ranges for common, uncommon, rare, epic, mythic.",
        "    pub values: [Option<Range>; 5],",
        "    /// Effect rolls for common, uncommon, rare, epic, mythic.",
        "    pub effects: [Option<Effect>; 5],",
        "    /// The vanilla effect id of a `mob_effect` affix.",
        "    pub effect: Option<&'static str>,",
        "    /// Java's application target: `attack_target`, `hurt_self`, ...",
        "    pub effect_target: Option<&'static str>,",
        "    /// The vanilla damage class of a `damage_reduction` affix.",
        "    pub damage_kind: Option<&'static str>,",
        "    /// The vanilla enchantment of an `enchantment` affix.",
        "    pub enchantment: Option<&'static str>,",
        "    pub weight: u32,",
        "    /// The port's storage slot for an Apothic attribute.",
        "    pub attr: Option<crate::attributes::Attr>,",
        "    pub supported: bool,",
        "}",
        "",
        "pub const AFFIXES: [Affix; %d] = [" % len(entries),
    ]
    for entry in entries:
        values = [entry["values"].get(rarity) for rarity in RARITIES]
        value_text = ", ".join(
            "None" if value is None else
            "Some(Range { min: %s, max: %s, step: %s })"
            % (number(value[0]), number(value[1]), number(value[2]))
            for value in values
        )
        rolled = [entry["effects"].get(rarity) for rarity in RARITIES]
        effect_text = ", ".join(effect_literal(value) for value in rolled)
        rarity_indices = ", ".join(str(RARITIES.index(r)) for r in entry["rarities"])
        categories = ", ".join(rust_string(c) for c in entry["categories"])
        attr = ("Some(crate::attributes::Attr::%s)" % entry["attr"]
                if entry["attr"] else "None")
        lines.extend([
            "    Affix {",
            "        id: %s," % rust_string(entry["id"]),
            "        path: %s," % rust_string(entry["path"]),
            "        name_key: %s," % rust_string("affix." + entry["id"]),
            "        suffix_key: %s," % rust_string("affix." + entry["id"] + ".suffix"),
            "        name: %s," % rust_string(entry["name"]),
            "        suffix: %s," % rust_string(entry["suffix"]),
            "        kind: %s," % rust_string(entry["kind"]),
            "        affix_type: %s," % rust_string(entry["affix_type"]),
            "        attribute: %s," % ("Some(%s)" % rust_string(entry["attribute"])
                                       if entry["attribute"] else "None"),
            "        operation: %s," % ("Some(%s)" % rust_string(entry["operation"])
                                      if entry["operation"] else "None"),
            "        categories: &[%s]," % categories,
            "        rarities: &[%s]," % rarity_indices,
            "        values: [%s]," % value_text,
            "        effects: [%s]," % effect_text,
            "        effect: %s," % optional_string(entry["effect"]),
            "        effect_target: %s," % optional_string(entry["effect_target"]),
            "        damage_kind: %s," % optional_string(entry["damage_kind"]),
            "        enchantment: %s," % optional_string(entry["enchantment"]),
            "        weight: %d," % entry["weight"],
            "        attr: %s," % attr,
            "        supported: %s," % ("true" if entry["supported"] else "false"),
            "    },",
        ])
    lines.append("];")
    lines.append("")
    return "\n".join(lines)


def merge_lang():
    tables = {"en_us": {}, "fr_fr": {}}
    for entry in affix_entries():
        tables["en_us"]["affix." + entry["id"]] = entry["name"]
        tables["en_us"]["affix." + entry["id"] + ".suffix"] = entry["suffix"]
        tables["fr_fr"]["affix." + entry["id"]] = entry["fr_name"]
        tables["fr_fr"]["affix." + entry["id"] + ".suffix"] = entry["fr_suffix"]
    # Java's name-composition keys, and this port's tooltip line shape.
    for table, (two, three, four, line) in (
        ("en_us", ("%s %s", "%s %s %s", "%s %s %s %s", "%s %s")),
        ("fr_fr", ("%s %s", "%s %s %s", "%s %s %s %s", "%s %s")),
    ):
        tables[table]["misc.apotheosis.affix_name.two"] = two
        tables[table]["misc.apotheosis.affix_name.three"] = three
        tables[table]["misc.apotheosis.affix_name.four"] = four
        tables[table]["apotheosis.affix.stat"] = line
    for effect, (english, french) in EFFECT_TEXT.items():
        key = "apotheosis.effect." + effect.rsplit(":", 1)[-1]
        tables["en_us"][key] = english
        tables["fr_fr"][key] = french
    for damage, (english, french) in DAMAGE_TEXT.items():
        key = "apotheosis.damage." + damage
        tables["en_us"][key] = english
        tables["fr_fr"][key] = french
    # The effect lines' own shapes, one per kind rather than per affix.
    texts = {
        "effect": ("%s for %s s", "%s pendant %s s"),
        "reduction": ("-%s%% %s damage", "-%s%% de dégâts %s"),
        "executing": ("Executes below %s%% health", "Exécute sous %s%% de vie"),
        "cleaving": ("%s%% chance to hit %s foes", "%s%% de chances de toucher %s ennemis"),
        "thunderstruck": ("Lightning for %s damage", "Foudre pour %s dégâts"),
        "festive": ("%s%% chance of extra loot", "%s%% de chances de butin en plus"),
        "durable": ("%s%% less wear", "%s%% d'usure en moins"),
        "enchantment": ("+%s %s", "+%s %s"),
        "retreating": ("Knocks attackers back on block", "Repousse les attaquants au blocage"),
        "telepathic": ("Drops go to your inventory", "Le butin va dans l'inventaire"),
    }
    for kind, (english, french) in texts.items():
        key = "apotheosis.affix.effect." + kind
        tables["en_us"][key] = english
        tables["fr_fr"][key] = french
    for language, additions in tables.items():
        path = os.path.join(LANG, language + ".json")
        table = read_json(path)
        before = dict(table)
        table.update(additions)
        if table != before:
            write_if_changed(path, json.dumps(table, indent=2, ensure_ascii=False) + "\n")


def main():
    entries = affix_entries()
    if not entries:
        raise SystemExit("no affix found under %s" % REF)
    changed = write_if_changed(OUT, render(entries))
    merge_lang()
    supported = sum(1 for e in entries if e["supported"])
    print("%d Java affixes, %d supported: %s"
          % (len(entries), supported, "updated" if changed else "unchanged"))
    for entry in entries:
        if not entry["supported"]:
            print("  not rolled: %s (%s)" % (entry["id"], entry["kind"]))


if __name__ == "__main__":
    main()
