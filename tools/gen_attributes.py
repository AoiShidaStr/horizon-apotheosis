#!/usr/bin/env python3
"""Add the Apothic Attributes entries to the shared content files.

Run: python tools/gen_attributes.py

The attributes and affixes live in the native half
(`native/apotheosis/api/src/attributes.rs`); this script only gives them names
in en_us and fr_fr. It merges into the existing tables instead of replacing
them, so the gems, tiers, commands and the creative tab are left alone, and it
is idempotent: a second run changes nothing.

No item or block is added here, so the creative tab and the item tags have
nothing new to hold; both are read and checked, not rewritten.
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
ID = "apotheosis"
CONTENT = os.path.join(ROOT, "content")
LANG = os.path.join(CONTENT, "assets", ID, "lang")

# (key, English, French) — the attribute names shown in a tooltip line.
ATTRIBUTES = [
    ("crit_chance", "Crit Chance", "Chances de coup critique"),
    ("crit_damage", "Crit Damage", "Dégâts critiques"),
    ("life_steal", "Life Steal", "Vol de vie"),
    ("overheal", "Overheal", "Soins excédentaires"),
    ("dodge_chance", "Dodge Chance", "Chances d'esquive"),
    ("armor_pierce", "Armor Pierce", "Perce-armure"),
    ("armor_shred", "Armor Shred", "Brise-armure"),
    ("fire_damage", "Fire Damage", "Dégâts de feu"),
    ("cold_damage", "Cold Damage", "Dégâts de froid"),
    ("experience_gained", "Experience Gained", "Expérience gagnée"),
    ("healing_received", "Healing Received", "Soins reçus"),
    ("arrow_damage", "Arrow Damage", "Dégâts des flèches"),
    ("arrow_velocity", "Arrow Velocity", "Vitesse des flèches"),
    ("draw_speed", "Draw Speed", "Vitesse de tension"),
    ("mining_speed", "Mining Speed", "Vitesse de minage"),
]

# (affix id, English, French) for the affixes added with the attributes; the
# eight older affixes are already named in the language files.
AFFIXES = [
    ("vampiric", "Vampiric", "Vampirique"),
    ("berserking", "Berserking", "Berserk"),
    ("piercing", "Piercing", "Perçant"),
    ("shredding", "Shredding", "Déchirant"),
    ("glacial", "Glacial", "Glacial"),
    ("infernal", "Infernal", "Infernal"),
    ("murderous", "Murderous", "Meurtrier"),
    ("violent", "Violent", "Violent"),
    ("elusive", "Elusive", "Insaisissable"),
    ("blessed", "Blessed", "Béni"),
    ("elven", "Elven", "Elfique"),
    ("agile", "Agile", "Agile"),
    ("streamlined", "Streamlined", "Profilé"),
    ("experienced", "Experienced", "Expérimenté"),
    ("destructive", "Destructive", "Destructeur"),
    ("surefooted", "Surefooted", "Assuré"),
    ("ironforged", "Ironforged", "Forgé de fer"),
    ("elastic", "Elastic", "Élastique"),
    # Effect affixes (epic and above): a payout that is not an attribute.
    ("venomous", "Venomous", "Venimeux"),
    ("withering", "Withering", "Flétrissant"),
    ("weakening", "Weakening", "Affaiblissant"),
    ("ensnaring", "Ensnaring", "Ralentissant"),
    ("blinding", "Blinding", "Aveuglant"),
    ("executing", "Executing", "Exécution"),
    ("cleaving", "Cleaving", "Fendoir"),
    ("thunderstruck", "Thunderstruck", "Foudroyant"),
    ("retreating", "Retreating", "Retraite"),
    ("durable", "Durable", "Durable"),
    ("telepathic", "Telepathic", "Télépathique"),
    ("spectral", "Spectral", "Spectral"),
    ("magical", "Magical", "Magique"),
    ("festive", "Festive", "Festif"),
]

# (affix id, English, French) for the effect affixes' lore line. `%s` is the
# rolled value (seconds, a percentage or a target count).
EFFECT_DESCRIPTIONS = [
    ("venomous", "Poison for %s s", "Poison pendant %s s"),
    ("withering", "Wither for %s s", "Flétrissement pendant %s s"),
    ("weakening", "Weakness for %s s", "Faiblesse pendant %s s"),
    ("ensnaring", "Slowness for %s s", "Lenteur pendant %s s"),
    ("blinding", "Blinds attackers for %s s", "Aveugle les attaquants pendant %s s"),
    ("executing", "Executes foes below %s%% health", "Achève les ennemis sous %s%% de vie"),
    ("cleaving", "Hits %s nearby foes", "Frappe %s ennemis proches"),
    ("thunderstruck", "%s%% chance of lightning", "%s%% de chances de foudre"),
    ("retreating", "Knocks you back when hit", "Vous repousse quand vous êtes touché"),
    ("durable", "%s%% less wear", "%s%% d'usure en moins"),
    ("telepathic", "Drops go to your inventory", "Les butins vont dans votre inventaire"),
    ("spectral", "%s%% chance of a spectral arrow", "%s%% de chances d'une flèche spectrale"),
    ("magical", "Arrows pierce armour as magic", "Les flèches transpercent l'armure comme magie"),
    ("festive", "%s%% chance of extra loot", "%s%% de chances de butin supplémentaire"),
]

# The Apothic Invader's generated name: words and title shapes. The key carries
# the whole component, so the client translates a name it has never seen.
BOSS_WORDS = [
    ("prefix.grim", "Grim", "Sombre"),
    ("prefix.vile", "Vile", "Vil"),
    ("prefix.ancient", "Ancient", "Ancien"),
    ("prefix.cursed", "Cursed", "Maudit"),
    ("prefix.hollow", "Hollow", "Creux"),
    ("prefix.blighted", "Blighted", "Flétri"),
    ("prefix.savage", "Savage", "Sauvage"),
    ("prefix.ethereal", "Ethereal", "Éthéré"),
    ("suffix.devourer", "the Devourer", "le Dévoreur"),
    ("suffix.abyss", "of the Abyss", "des Abysses"),
    ("suffix.unbroken", "the Unbroken", "l'Invaincu"),
    ("suffix.embers", "of Embers", "des Braises"),
    ("suffix.silent", "the Silent", "le Silencieux"),
    ("suffix.deep", "of the Deep", "des Profondeurs"),
    ("suffix.ravager", "the Ravager", "le Ravageur"),
    ("suffix.twilight", "of Twilight", "du Crépuscule"),
    ("title.full", "%s %s %s", "%s %s %s"),
    ("title.prefix", "%s %s", "%s %s"),
    ("title.suffix", "%s %s", "%s %s"),
    ("awakened", "%s has awoken nearby!", "%s s'est éveillé tout près !"),
]

BOSS_BASES = [
    ("zombie", "Zombie", "Zombie"),
    ("skeleton", "Skeleton", "Squelette"),
    ("spider", "Spider", "Araignée"),
    ("cave_spider", "Cave Spider", "Araignée venimeuse"),
    ("husk", "Husk", "Mommie"),
    ("drowned", "Drowned", "Noyé"),
    ("stray", "Stray", "Vagabond"),
    ("bogged", "Bogged", "Bogué"),
    ("wither_skeleton", "Wither Skeleton", "Wither squelette"),
    ("creeper", "Creeper", "Creeper"),
    ("enderman", "Enderman", "Enderman"),
    ("piglin", "Piglin", "Piglin"),
    ("zombified_piglin", "Zombified Piglin", "Piglin zombifié"),
    ("blaze", "Blaze", "Blaze"),
    ("ghast", "Ghast", "Ghast"),
    ("hoglin", "Hoglin", "Hoglin"),
    ("magma_cube", "Magma Cube", "Cube de magma"),
    ("slime", "Slime", "Slime"),
    ("witch", "Witch", "Sorcière"),
    ("pillager", "Pillager", "Pillard"),
    ("vindicator", "Vindicator", "Vindicateur"),
    ("evoker", "Evoker", "Invocateur"),
    ("ravager", "Ravager", "Ravageur"),
    ("zombie_villager", "Zombie Villager", "Villageois zombie"),
]


def read_json(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def write_json(path, value):
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        json.dump(value, f, indent=2, ensure_ascii=False)
        f.write("\n")


def merge(path, entries):
    """Add entries to a translation table, keeping every existing key."""
    table = read_json(path)
    before = dict(table)
    table.update(entries)
    if table != before:
        write_json(path, table)
    return len(table) - len(before)


def main():
    en = {}
    fr = {}
    for key, english, french in ATTRIBUTES:
        en[f"{ID}.attribute.{key}"] = english
        fr[f"{ID}.attribute.{key}"] = french
    for key, english, french in AFFIXES:
        en[f"{ID}.affix.{key}"] = english
        fr[f"{ID}.affix.{key}"] = french
    for key, english, french in EFFECT_DESCRIPTIONS:
        en[f"{ID}.affix.{key}.desc"] = english
        fr[f"{ID}.affix.{key}.desc"] = french
    for key, english, french in BOSS_WORDS:
        en[f"{ID}.boss.{key}"] = english
        fr[f"{ID}.boss.{key}"] = french
    for key, english, french in BOSS_BASES:
        en[f"{ID}.boss.base.{key}"] = english
        fr[f"{ID}.boss.base.{key}"] = french

    for name, entries in (("en_us", en), ("fr_fr", fr)):
        path = os.path.join(LANG, f"{name}.json")
        added = merge(path, entries)
        print(f"{name}: +{added} keys, {len(entries)} wanted")

    # Nothing new is item-shaped: check the shared surfaces still parse and
    # still hold what the gem generator put there.
    tab = read_json(os.path.join(CONTENT, "data", ID, "creative_tab", f"{ID}.json"))
    print(f"creative_tab: {len(tab.get('items', []))} items kept")

    tags = []
    for base, _dirs, files in os.walk(os.path.join(CONTENT, "data")):
        for file in files:
            path = os.path.join(base, file)
            if file.endswith(".json") and f"{os.sep}tags{os.sep}" in path:
                tags.append(path)
    for path in tags:
        read_json(path)
    print(f"tags: {len(tags)} files checked, untouched")


if __name__ == "__main__":
    main()
