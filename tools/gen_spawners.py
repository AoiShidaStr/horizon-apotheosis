#!/usr/bin/env python3
"""Add the Apothic Spawners strings to this mod's language files.

The spawner chantier adds no item and no block: its modifiers are vanilla
items, and a spawner comes back with a silk-touch pickaxe. So the only file it
writes is the shared `lang/en_us.json` and `lang/fr_fr.json`, and it *merges*
its keys there: the other chantiers' keys stay as they are.

Run: python tools/gen_spawners.py
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
ID = "apotheosis"
CONTENT = os.path.join(ROOT, "content")

# Every line a player sees from the native half. The stats line is the action
# bar message shown when a spawner is clicked, built with `%s` arguments in
# this order: the mob, the shortest delay, the longest one, the mobs per wave,
# the player range, the most mobs nearby, the spawn range.
EN = {
    "apotheosis.spawner.stats": (
        "Spawner: %s · delay %s–%s ticks · %s per wave · "
        "player range %s · nearby max %s · spawn range %s"
    ),
    "apotheosis.spawner.flag.ignore_players": "ignores players",
    "apotheosis.spawner.flag.ignore_light": "ignores light",
    "apotheosis.spawner.flag.ignore_conditions": "ignores conditions",
    "apotheosis.spawner.flag.no_ai": "mobs have no AI",
    "apotheosis.spawner.flag.silent": "silent",
    "apotheosis.spawner.flag.redstone_control": "redstone controlled",
    "apotheosis.spawner.flag.youthful": "young mobs",
    "apotheosis.spawner.flag.burning": "burning mobs",
}

FR = {
    "apotheosis.spawner.stats": (
        "Spawner : %s · délai %s–%s ticks · %s par vague · "
        "portée joueur %s · proches max %s · portée d'apparition %s"
    ),
    "apotheosis.spawner.flag.ignore_players": "ignore les joueurs",
    "apotheosis.spawner.flag.ignore_light": "ignore la lumière",
    "apotheosis.spawner.flag.ignore_conditions": "ignore les conditions",
    "apotheosis.spawner.flag.no_ai": "mobs sans IA",
    "apotheosis.spawner.flag.silent": "silencieux",
    "apotheosis.spawner.flag.redstone_control": "commandé par redstone",
    "apotheosis.spawner.flag.youthful": "mobs juvéniles",
    "apotheosis.spawner.flag.burning": "mobs en feu",
}


def write_json(path, value, newline="\n"):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    text = json.dumps(value, indent=2, ensure_ascii=False) + "\n"
    if newline != "\n":
        text = text.replace("\n", newline)
    with open(path, "w", encoding="utf-8", newline="") as f:
        f.write(text)


def merge(lang, keys):
    """Adds `keys` to a language file, keeping every key already there."""
    path = os.path.join(CONTENT, "assets", ID, "lang", lang + ".json")
    existing = {}
    newline = "\n"
    if os.path.exists(path):
        with open(path, encoding="utf-8", newline="") as f:
            text = f.read()
        if "\r\n" in text:
            newline = "\r\n"
        existing = json.loads(text)
    before = len(existing)
    existing.update(keys)
    write_json(path, existing, newline)
    return len(existing) - before


def main():
    added_en = merge("en_us", EN)
    added_fr = merge("fr_fr", FR)
    print(
        "apothic spawners: %d strings added in en_us, %d in fr_fr"
        % (added_en, added_fr)
    )


if __name__ == "__main__":
    main()
