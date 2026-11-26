#!/usr/bin/env python3
"""Import the Java mods' assets into a local, git-ignored content-java/ layer.

The Java mods' assets (textures, models, sounds, lang files) are All Rights
Reserved, so they must never be committed to this public repository.  This
script reads the four season-1 jars (or their extracted folders), copies only
the resources our ids need into content-java/, and writes our own item and
block definitions pointing at them.  `tools/pack.py` overlays the layer when it
builds the .hmod; `--no-java` builds without it.

Usage:
  python tools/import_java_assets.py [MODS-OR-JAVA-REF]
  python tools/import_java_assets.py C:/Users/yello/horizon-work/java-ref

The default source is the PrismLauncher season-1 mods folder.  The script is
idempotent: it regenerates content-java/ and tools/java_assets_map.md from
scratch and verifies every JSON and every non-vanilla texture reference.

Gems are deliberately left alone: another workstream replaces our 12 gems with
the Java gems.  Add their entries to JAVA_ALIASES when that happens.
"""
from __future__ import annotations

import argparse
import copy
import json
import os
import shutil
import sys
import zipfile
from pathlib import Path
from typing import Iterable

ROOT = Path(__file__).resolve().parent.parent
CONTENT = ROOT / "content"
OUT = ROOT / "content-java"
REPORT = ROOT / "tools" / "java_assets_map.md"

OUR_NS = "apotheosis"
JAVA_NAMESPACES = (
    "apotheosis",
    "apothic_enchanting",
    "apothic_attributes",
    "apothic_spawners",
)
LOCALES = ("en_us", "fr_fr")

# Our ids that map to a differently named Java id.
JAVA_ALIASES: dict[str, tuple[str, str]] = {
    "enchantment_library": ("apothic_enchanting", "library"),
    # Apothic Enchanting's other tables and shelves: same path, explicit so a
    # future rename in the jars is a one-line change here.
    "apothic_enchanting_table": ("apothic_enchanting", "apothic_enchanting_table"),
    "ender_library": ("apothic_enchanting", "ender_library"),
    "filtering_shelf": ("apothic_enchanting", "filtering_shelf"),
    "geode_shelf": ("apothic_enchanting", "geode_shelf"),
    "treasure_shelf": ("apothic_enchanting", "treasure_shelf"),
}

# The real Apotheosis gems: one item (`apotheosis:gem`) whose model selector
# names `item/gem_<name>`, so each of these Java textures is imported under our
# own texture id.  They live at `textures/items/gems/<name>.png` in the jar.
# The tables' screens (`menus.rs` draws `apotheosis:gui/<name>`, with a panel of
# its own when the texture is missing, as in the `--no-java` pack).
GUI_TEXTURES = ("reforge", "salvage", "gem_cutting", "augmenting")

GEM_TEXTURES = (
    "ballast",
    "blood_lord",
    "brawlers",
    "breach",
    "combatant",
    "earth",
    "endersurge",
    "guardian",
    "inferno",
    "lightning",
    "lunar",
    "mageslayer",
    "molten_breach",
    "royalty",
    "samurai",
    "slipstream",
    "solar",
    "splendor",
    "tyrannical",
    "verdant_ruin",
    "warlord",
)

# One item per gem and purity (`tools/gen_java_gems.py`), mapped to the Java
# texture its `item/gem_<name>` model draws.
PORT_GEM_ITEMS = {
    f"gem_{gem}_{purity}": f"apotheosis:item/gems/{gem}"
    for gem in GEM_TEXTURES
    for purity in ("cracked", "chipped", "flawed", "normal", "flawless", "perfect")
}

# Items that only exist in Apotheosis (the Java mod) and not in Apothic
# Enchanting, or whose Apotheosis copy is the canonical one.
APOTHEOSIS_ITEMS = {
    "arcane_sands",
    "boss_summoner",
    "gem_dust",
    "godforged_pearl",
    "luminous_crystal_shard",
    "mysterious_scrap_metal",
    "timeworn_fabric",
    "gem_fused_slate",
    "iron_upgrade_smithing_template",
    "gold_upgrade_smithing_template",
    "diamond_upgrade_smithing_template",
    "sigil_of_enhancement",
    "sigil_of_malice",
    "sigil_of_rebirth",
    "sigil_of_socketing",
    "sigil_of_supremacy",
    "sigil_of_unnaming",
    "sigil_of_withdrawal",
}
# Items this port declares itself, whose Java namesake must never replace them.
# `items/gem.json` is the port's `minecraft:select` over the gem types; the
# Java base gem model would render the default texture for every gem.
PORT_ITEMS_KEPT = {"gem"}
# The per-purity gem items (`PORT_GEM_ITEMS`) are kept too: Java has no item of
# that name, and its base gem model must never stand in for one.

# Block items whose Later versions live in Apotheosis rather than Apothic
# Enchanting.
APOTHEOSIS_BLOCKS = {
    "reforging_table",
    "salvaging_table",
    "gem_cutting_table",
    "simple_reforging_table",
    "augmenting_table",
}


class MissingAsset(Exception):
    pass


class AssetSource:
    """A read-only index of assets/ and data/ in jars or extracted folders."""

    def __init__(self, paths: Iterable[Path]):
        self.entries: dict[str, Path | tuple[zipfile.ZipFile, str]] = {}
        self._zips: list[zipfile.ZipFile] = []
        for path in paths:
            self._add(path)

    def _add(self, path: Path) -> None:
        if path.is_file() and path.suffix.lower() == ".jar":
            self._add_jar(path)
            return
        if not path.is_dir():
            return
        if (path / "assets").is_dir() or (path / "data").is_dir():
            self._add_folder(path)
            return
        for child in sorted(path.iterdir()):
            if child.is_dir() and (
                (child / "assets").is_dir() or (child / "data").is_dir()
            ):
                self._add_folder(child)
            elif child.is_file() and child.suffix.lower() == ".jar":
                self._add_jar(child)

    def _add_folder(self, root: Path) -> None:
        for path in sorted(root.rglob("*")):
            if not path.is_file():
                continue
            rel = path.relative_to(root).as_posix()
            if rel.startswith(("assets/", "data/")):
                self.entries.setdefault(rel, path)

    def _add_jar(self, path: Path) -> None:
        zf = zipfile.ZipFile(path)
        self._zips.append(zf)
        for name in sorted(zf.namelist()):
            if name.endswith("/"):
                continue
            if name.startswith(("assets/", "data/")):
                self.entries.setdefault(name, (zf, name))

    def exists(self, logical: str) -> bool:
        return logical in self.entries

    def read(self, logical: str) -> bytes:
        entry = self.entries.get(logical)
        if entry is None:
            raise MissingAsset(logical)
        if isinstance(entry, Path):
            return entry.read_bytes()
        zf, name = entry
        return zf.read(name)

    def read_json(self, logical: str) -> dict:
        try:
            return json.loads(self.read(logical).decode("utf-8-sig"))
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            raise MissingAsset(f"{logical}: invalid JSON ({error})") from error


def split_id(name: str) -> tuple[str, str]:
    if ":" in name:
        namespace, path = name.split(":", 1)
    else:
        namespace, path = "minecraft", name
    return namespace, path.lstrip("/")


def qualify(name: str) -> str:
    return name if ":" in name else f"minecraft:{name}"


def texture_id(name: str) -> str:
    """A texture reference with the 26.3 folder names (block/, item/)."""
    namespace, path = split_id(qualify(name))
    if path.endswith(".png"):
        path = path[:-4]
    if path.startswith("blocks/"):
        path = "block/" + path[len("blocks/") :]
    elif path.startswith("items/"):
        path = "item/" + path[len("items/") :]
    return f"{namespace}:{path}"


def normalize_model(model: dict) -> dict:
    """Return a model with qualified parents and 26.3 texture folders."""
    out = copy.deepcopy(model)
    if isinstance(out.get("parent"), str):
        out["parent"] = qualify(out["parent"])
    if isinstance(out.get("textures"), dict):
        out["textures"] = {
            key: (value if value.startswith("#") else texture_id(value))
            for key, value in out["textures"].items()
        }
    children = out.get("children")
    if isinstance(children, dict):
        out["children"] = {
            key: normalize_model(value) if isinstance(value, dict) else value
            for key, value in children.items()
        }
    return out


def model_textures(model: dict) -> list[str]:
    found: list[str] = []
    textures = model.get("textures")
    if isinstance(textures, dict):
        for value in textures.values():
            if isinstance(value, str) and not value.startswith("#"):
                found.append(texture_id(value))
    children = model.get("children")
    if isinstance(children, dict):
        for child in children.values():
            if isinstance(child, dict):
                found.extend(model_textures(child))
    return found


def model_parents(model: dict) -> list[str]:
    parents: list[str] = []
    if isinstance(model.get("parent"), str):
        parents.append(qualify(model["parent"]))
    children = model.get("children")
    if isinstance(children, dict):
        for child in children.values():
            if isinstance(child, dict):
                parents.extend(model_parents(child))
    return parents


def blockstate_models(state: dict) -> list[str]:
    found: list[str] = []
    variants = state.get("variants")
    if isinstance(variants, dict):
        for value in variants.values():
            entries = value if isinstance(value, list) else [value]
            for entry in entries:
                if isinstance(entry, dict) and isinstance(entry.get("model"), str):
                    found.append(qualify(entry["model"]))
    multipart = state.get("multipart")
    if isinstance(multipart, list):
        for part in multipart:
            if not isinstance(part, dict):
                continue
            apply = part.get("apply")
            entries = apply if isinstance(apply, list) else [apply]
            for entry in entries:
                if isinstance(entry, dict) and isinstance(entry.get("model"), str):
                    found.append(qualify(entry["model"]))
    return found


def _first_unrotated(entries: list[dict]) -> dict | None:
    for entry in entries:
        if not isinstance(entry, dict):
            continue
        if not entry.get("x") and not entry.get("y"):
            return copy.deepcopy(entry)
    for entry in entries:
        if isinstance(entry, dict):
            return copy.deepcopy(entry)
    return None


def add_default_variant(state: dict) -> bool:
    """Add a `""` variant when the Java blockstate only names properties.

    Our blocks are template blocks without a facing property, so a blockstate
    with only `facing=...` would bind no state and would silently leave the
    old texture in place.  The default model is the least-rotated Java variant.
    A multipart state (a Java chiseled bookshelf, whose slot properties our
    block does not declare) gets the same treatment: its parts are dropped and
    the first unrotated model becomes the one state our block has.
    """
    variants = state.get("variants")
    if not isinstance(variants, dict):
        multipart = state.get("multipart")
        if not isinstance(multipart, list):
            return False
        entries: list[dict] = []
        for part in multipart:
            if not isinstance(part, dict):
                continue
            apply = part.get("apply")
            for entry in apply if isinstance(apply, list) else [apply]:
                if isinstance(entry, dict):
                    entries.append(copy.deepcopy(entry))
        chosen = _first_unrotated(entries)
        if chosen is None:
            return False
        chosen.pop("weight", None)
        state.pop("multipart", None)
        state["variants"] = {"": chosen}
        return True
    if "" in variants:
        return False
    entries: list[dict] = []
    for value in variants.values():
        if isinstance(value, list):
            entries.extend(v for v in value if isinstance(v, dict))
        elif isinstance(value, dict):
            entries.append(value)
    chosen = _first_unrotated(entries)
    if chosen is None:
        return False
    chosen.pop("weight", None)
    variants[""] = chosen
    return True


def normalize_blockstate(state: dict) -> tuple[dict, bool]:
    out = copy.deepcopy(state)
    variants = out.get("variants")
    if isinstance(variants, dict):
        for key, value in list(variants.items()):
            entries = value if isinstance(value, list) else [value]
            for entry in entries:
                if isinstance(entry, dict) and isinstance(entry.get("model"), str):
                    entry["model"] = qualify(entry["model"])
    multipart = out.get("multipart")
    if isinstance(multipart, list):
        for part in multipart:
            if not isinstance(part, dict):
                continue
            apply = part.get("apply")
            entries = apply if isinstance(apply, list) else [apply]
            for entry in entries:
                if isinstance(entry, dict) and isinstance(entry.get("model"), str):
                    entry["model"] = qualify(entry["model"])
    return out, add_default_variant(out)


def dump_json(value: dict, *, sort_keys: bool = False) -> str:
    return json.dumps(
        value, ensure_ascii=False, indent=2, sort_keys=sort_keys
    ) + "\n"


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8", newline="\n")


class Importer:
    def __init__(self, source: AssetSource, out: Path):
        self.source = source
        self.out = out
        self.files: dict[str, str | bytes] = {}
        self.models: set[str] = set()
        self.textures: set[str] = set()
        self.blockstates: set[str] = set()
        self.items: set[str] = set()
        self.default_variants: list[str] = []
        self.block_map: dict[str, tuple[str, str]] = {}
        self.item_map: dict[str, tuple[str, str]] = {}

    def put_json(self, logical: str, value: dict, *, sort_keys: bool = False) -> None:
        self.files[logical] = dump_json(value, sort_keys=sort_keys)

    def find_block(self, name: str) -> tuple[str, str] | None:
        # An explicit alias wins over the gem skip, so the gem workstream only
        # has to add entries to JAVA_ALIASES.
        if name in JAVA_ALIASES:
            namespace, java_name = JAVA_ALIASES[name]
            if self.source.exists(
                f"assets/{namespace}/blockstates/{java_name}.json"
            ):
                return namespace, java_name
            return None
        order = (
            ("apotheosis", "apothic_enchanting")
            if name in APOTHEOSIS_BLOCKS
            else ("apothic_enchanting", "apotheosis")
        )
        for namespace in order:
            if self.source.exists(f"assets/{namespace}/blockstates/{name}.json"):
                return namespace, name
        return None

    def find_item(self, name: str) -> tuple[str, str] | None:
        # An explicit alias wins over the gem skip, so the gem workstream only
        # has to add entries to JAVA_ALIASES.
        if name in JAVA_ALIASES:
            namespace, java_name = JAVA_ALIASES[name]
            if self.source.exists(f"assets/{namespace}/models/item/{java_name}.json"):
                return namespace, java_name
            if self.source.exists(f"assets/{namespace}/models/block/{java_name}.json"):
                return namespace, java_name
            return None
        if name in self.block_map:
            namespace, java_name = self.block_map[name]
            if self.source.exists(f"assets/{namespace}/models/item/{java_name}.json"):
                return namespace, java_name
            # Block items can point at the block model directly.
            if self.source.exists(f"assets/{namespace}/models/block/{java_name}.json"):
                return namespace, java_name
            return None
        order = (
            ("apotheosis", "apothic_enchanting")
            if name in APOTHEOSIS_ITEMS
            else ("apothic_enchanting", "apotheosis")
        )
        for namespace in order:
            if self.source.exists(f"assets/{namespace}/models/item/{name}.json"):
                return namespace, name
        return None

    def copy_texture(self, reference: str) -> None:
        namespace, path = split_id(reference)
        if namespace == "minecraft":
            return
        logical = f"assets/{namespace}/textures/{path}.png"
        if self.source.exists(logical):
            source_path = logical
        else:
            # The Java mods use textures/blocks and textures/items.
            fallback = None
            if path.startswith("block/"):
                fallback = f"assets/{namespace}/textures/blocks/{path[6:]}.png"
            elif path.startswith("item/"):
                fallback = f"assets/{namespace}/textures/items/{path[5:]}.png"
            if fallback is None or not self.source.exists(fallback):
                raise MissingAsset(f"texture {reference} ({logical})")
            source_path = fallback
        if logical not in self.files:
            self.files[logical] = self.source.read(source_path)
            self.textures.add(logical)
        # Animated textures keep their .png.mcmeta beside them.
        meta = source_path + ".mcmeta"
        if self.source.exists(meta):
            self.files[logical + ".mcmeta"] = self.source.read(meta)

    def copy_model(self, reference: str) -> None:
        namespace, path = split_id(reference)
        if namespace == "minecraft":
            return
        logical = f"assets/{namespace}/models/{path}.json"
        if logical in self.models:
            return
        if not self.source.exists(logical):
            raise MissingAsset(f"model {reference} ({logical})")
        model = self.source.read_json(logical)
        normalized = normalize_model(model)
        self.put_json(logical, normalized)
        self.models.add(logical)
        for texture in model_textures(normalized):
            self.copy_texture(texture)
        for parent in model_parents(normalized):
            self.copy_model(parent)

    def import_block(self, name: str, source: tuple[str, str]) -> None:
        namespace, java_name = source
        logical = f"assets/{namespace}/blockstates/{java_name}.json"
        state, added = normalize_blockstate(self.source.read_json(logical))
        self.put_json(f"assets/{OUR_NS}/blockstates/{name}.json", state)
        self.blockstates.add(name)
        if added:
            self.default_variants.append(name)
        for model in blockstate_models(state):
            self.copy_model(model)

    def import_item(self, name: str, source: tuple[str, str]) -> None:
        namespace, java_name = source
        item_model = f"assets/{namespace}/models/item/{java_name}.json"
        if self.source.exists(item_model):
            model_reference = f"{namespace}:item/{java_name}"
        else:
            model_reference = f"{namespace}:block/{java_name}"
        self.put_json(
            f"assets/{OUR_NS}/items/{name}.json",
            {"model": {"type": "minecraft:model", "model": model_reference}},
        )
        self.items.add(name)
        self.copy_model(model_reference)

    def import_lang(self) -> dict[str, int]:
        base: dict[str, dict] = {}
        java: dict[str, dict] = {}
        for locale in LOCALES:
            base[locale] = {}
            path = CONTENT / "assets" / OUR_NS / "lang" / f"{locale}.json"
            if path.exists():
                base[locale] = json.loads(path.read_text(encoding="utf-8-sig"))
            merged: dict[str, str] = {}
            for namespace in JAVA_NAMESPACES:
                logical = f"assets/{namespace}/lang/{locale}.json"
                if self.source.exists(logical):
                    merged.update(self.source.read_json(logical))
            java[locale] = merged

        # Java's en_us only fills a key that neither Java's fr_fr nor the port's
        # own fr_fr has.
        for locale in LOCALES:
            table = base[locale]
            for kind, mapping in (("block", self.block_map), ("item", self.item_map)):
                for name, (namespace, java_name) in mapping.items():
                    source_key = f"{kind}.{namespace}.{java_name}"
                    our_key = f"{kind}.{OUR_NS}.{name}"
                    # Java fr_fr, then the port's own text, then Java en_us: a
                    # French text the port has is never replaced by English.
                    value = java[locale].get(source_key)
                    if value is None and our_key not in table:
                        value = java["en_us"].get(source_key)
                    if value is not None:
                        table[our_key] = value
            table.update(
                {
                    key: value
                    for key, value in java[locale].items()
                    if key != "comment_id"
                }
            )
            table.update(self.gem_item_names(java, locale))
            self.put_json(
                f"assets/{OUR_NS}/lang/{locale}.json", table, sort_keys=True
            )
        return {locale: len(base[locale]) for locale in LOCALES}

    def gem_item_names(
        self, java: dict[str, dict], locale: str
    ) -> dict[str, str]:
        """`item.apotheosis.gem_<texture>_<purity>` from Java's own texts, for
        each gem item the port declares.

        English: Java's `item.apotheosis.gem.<purity>` format applied to the
        gem's Java name.  Java's fr_fr has no such format, so the French name
        is Java's French gem name worded with the port's purity rule
        (`gen_java_gems.french_item_name`).  A gem Java does not name in that
        locale keeps the port's own text."""
        import gen_java_gems

        prefix = "item.apotheosis.gem.apotheosis:"
        gem_ids = {
            key.removeprefix("item.apotheosis.gem.").rsplit("/", 1)[-1]:
                key.removeprefix("item.apotheosis.gem.")
            for key in (*java["en_us"], *java[locale])
            if key.startswith(prefix) and "/" in key
        }
        items = CONTENT / "assets" / OUR_NS / "items"
        names: dict[str, str] = {}
        for path in sorted(items.glob("gem_*.json")):
            if path.stem not in PORT_GEM_ITEMS:
                continue
            stem, purity = path.stem.rsplit("_", 1)
            gem_id = gem_ids.get(stem.removeprefix("gem_"))
            if gem_id is None:
                continue
            name = java[locale].get(f"item.apotheosis.gem.{gem_id}")
            if name is None:
                continue
            if locale == "fr_fr":
                value = gen_java_gems.french_item_name(
                    name, gen_java_gems.PURITIES.index(purity)
                )
            else:
                form = java[locale].get(f"item.apotheosis.gem.{purity}")
                if form is None:
                    continue
                value = form.replace("%s", name, 1)
            names[f"item.{OUR_NS}.{path.stem}"] = value
        return names

    def build(self) -> dict[str, int]:
        block_names = sorted(
            path.stem
            for path in (CONTENT / "assets" / OUR_NS / "blockstates").glob("*.json")
        )
        item_names = sorted(
            path.stem
            for path in (CONTENT / "assets" / OUR_NS / "items").glob("*.json")
        )
        for name in block_names:
            source = self.find_block(name)
            if source is not None:
                self.block_map[name] = source
        for name in item_names:
            if name in PORT_ITEMS_KEPT or name in PORT_GEM_ITEMS:
                continue
            source = self.find_item(name)
            if source is not None:
                self.item_map[name] = source

        for name, source in self.block_map.items():
            self.import_block(name, source)
        for name, source in self.item_map.items():
            self.import_item(name, source)
        for gem in GEM_TEXTURES:
            self.copy_texture(f"apotheosis:item/gems/{gem}")
        for gui in GUI_TEXTURES:
            self.copy_texture(f"apotheosis:gui/{gui}")
        lang_counts = self.import_lang()
        return {
            "blocks": len(self.blockstates),
            "items": len(self.items),
            "models": len(self.models),
            "textures": len(self.textures),
            **{f"lang_{locale}": count for locale, count in lang_counts.items()},
        }

    def write_files(self) -> None:
        if self.out.exists():
            shutil.rmtree(self.out)
        for logical, content in sorted(self.files.items()):
            path = self.out / logical
            path.parent.mkdir(parents=True, exist_ok=True)
            if isinstance(content, bytes):
                path.write_bytes(content)
            else:
                write(path, content)


def referenced_models_in_tree(root: Path) -> list[tuple[Path, str]]:
    found: list[tuple[Path, str]] = []
    for path in sorted(root.rglob("*.json")):
        try:
            value = json.loads(path.read_text(encoding="utf-8-sig"))
        except json.JSONDecodeError as error:
            raise MissingAsset(f"{path}: invalid JSON ({error})") from error
        if "/blockstates/" in path.as_posix():
            for model in blockstate_models(value):
                found.append((path, model))
        elif "/items/" in path.as_posix():
            model = value.get("model")
            if isinstance(model, dict) and isinstance(model.get("model"), str):
                found.append((path, qualify(model["model"])))
    return found


def verify(out: Path, source: AssetSource) -> None:
    """Every generated JSON is valid; every non-vanilla model and texture exists."""
    if not out.is_dir():
        raise MissingAsset(f"{out}: no content-java layer")
    for path in sorted(out.rglob("*.json")):
        try:
            json.loads(path.read_text(encoding="utf-8-sig"))
        except json.JSONDecodeError as error:
            raise MissingAsset(f"{path}: invalid JSON ({error})") from error

    for path in sorted(out.rglob("models/**/*.json")):
        model = json.loads(path.read_text(encoding="utf-8-sig"))
        for texture in model_textures(model):
            namespace, texture_path = split_id(texture)
            if namespace == "minecraft":
                continue
            logical = f"assets/{namespace}/textures/{texture_path}.png"
            if not (out / logical).is_file() and not (CONTENT / logical).is_file():
                raise MissingAsset(f"{path}: missing texture {texture}")

    for path, reference in referenced_models_in_tree(out):
        namespace, model_path = split_id(reference)
        if namespace == "minecraft":
            continue
        logical = f"assets/{namespace}/models/{model_path}.json"
        if not (out / logical).is_file() and not (CONTENT / logical).is_file():
            raise MissingAsset(f"{path}: missing model {reference}")

    leftovers = [
        path
        for path in out.rglob("*")
        if path.is_file()
        and path.suffix.lower()
        in {".class", ".java", ".jar", ".toml", ".rs", ".mcmeta"}
        and not path.name.endswith(".png.mcmeta")
    ]
    if leftovers:
        raise MissingAsset(
            "unexpected files in content-java: "
            + ", ".join(str(path) for path in leftovers[:5])
        )


def report(
    path: Path,
    importer: Importer,
    counts: dict[str, int],
    source_label: str,
) -> None:
    block_names = sorted(
        p.stem for p in (CONTENT / "assets" / OUR_NS / "blockstates").glob("*.json")
    )
    item_names = sorted(
        p.stem for p in (CONTENT / "assets" / OUR_NS / "items").glob("*.json")
    )
    lines = [
        "# Carte des assets Java — Apotheosis pour Horizon",
        "",
        "Rapport généré par `tools/import_java_assets.py`. **Aucun asset Java n'est "
        "dans ce dépôt** : les textures, modèles et textes importés vivent dans "
        "`content-java/`, ignoré par Git, et sont ajoutés au `.hmod` par "
        "`tools/pack.py` (sauf avec `--no-java`).",
        "",
        f"Source inspectée : `{source_label}`",
        "",
        "## Résumé",
        "",
        f"- blocs avec blockstate Java : {counts['blocks']}",
        f"- objets avec modèle Java : {counts['items']}",
        f"- modèles copiés : {counts['models']}",
        f"- textures copiées : {counts['textures']}",
        f"- clés lang en_us : {counts['lang_en_us']}",
        f"- clés lang fr_fr : {counts['lang_fr_fr']}",
        "",
        "Les ids sans équivalent Java gardent nos définitions et nos textures "
        "originales de `content/`. Les gemmes viennent de Java : une seule gemme "
        "(`apotheosis:gem`) dont le modèle choisit la texture du type qu'elle porte, "
        "ci-dessous en liste à part.",
        "",
        "## Blocs",
        "",
        "| Id du port | Source Java | Fichiers importés |",
        "| --- | --- | --- |",
    ]
    for name in block_names:
        source = importer.block_map.get(name)
        if source is None:
            reason = "original — pas d'équivalent Java"
            lines.append(f"| `{name}` | {reason} | — |")
            continue
        namespace, java_name = source
        details = [f"`blockstates/{name}.json` → `{namespace}:block/{java_name}`"]
        if name in importer.default_variants:
            details.append("variant `\"\"` ajouté (notre bloc n'a pas la propriété Java)")
        lines.append(
            f"| `{name}` | `{namespace}:{java_name}` | {', '.join(details)} |"
        )
    lines += [
        "",
        "## Objets",
        "",
        "| Id du port | Source Java | Fichiers importés |",
        "| --- | --- | --- |",
    ]
    for name in item_names:
        source = importer.item_map.get(name)
        if name in PORT_GEM_ITEMS:
            texture = PORT_GEM_ITEMS[name]
            lines.append(
                f"| `{name}` | gemme du port, texture Java `{texture}` | `{texture}` |"
            )
            continue
        if source is None:
            reason = "original — pas d'équivalent Java"
            lines.append(f"| `{name}` | {reason} | — |")
            continue
        namespace, java_name = source
        if (importer.out / f"assets/{namespace}/models/item/{java_name}.json").exists():
            model = f"{namespace}:item/{java_name}"
        else:
            model = f"{namespace}:block/{java_name}"
        lines.append(
            f"| `{name}` | `{namespace}:{java_name}` | `items/{name}.json` → `{model}` |"
        )
    lines += [
        "",
        "## Textures des gemmes",
        "",
        "`apotheosis:gem` porte son type dans ses données ; son modèle "
        "(`minecraft:select`) choisit `apotheosis:item/gem_<nom>`, dont la couche "
        "est la texture Java `items/gems/<nom>.png`. Textures importées : "
        + ", ".join(f"`{name}`" for name in GEM_TEXTURES) + ".",
        "",
        "## Correspondances explicites",
        "",
        "| Id du port | Id Java |",
        "| --- | --- |",
    ]
    if JAVA_ALIASES:
        for name, (namespace, java_name) in sorted(JAVA_ALIASES.items()):
            lines.append(f"| `{name}` | `{namespace}:{java_name}` |")
    else:
        lines.append("| — | — |")
    lines += [
        "",
        "## Contrôles",
        "",
        "```bash",
        "python tools/import_java_assets.py C:/Users/yello/horizon-work/java-ref",
        "python tools/pack.py",
        "C:/Users/yello/horizon-work/target/debug/cargo-horizon-mod.exe check target/apotheosis.hmod",
        "git status --porcelain   # ne doit rien montrer sous content-java/",
        "```",
        "",
    ]
    write(path, "\n".join(lines))


def default_source() -> Path:
    appdata = os.environ.get("APPDATA")
    if not appdata:
        return Path(".")
    return (
        Path(appdata)
        / "PrismLauncher"
        / "instances"
        / "Saison 1"
        / "minecraft"
        / "mods"
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "source",
        nargs="?",
        type=Path,
        default=default_source(),
        help="mods folder or extracted jar folder (default: PrismLauncher Saison 1)",
    )
    parser.add_argument("--out", type=Path, default=OUT)
    parser.add_argument("--report", type=Path, default=REPORT)
    args = parser.parse_args(argv)

    source_path = args.source
    if not source_path.exists():
        print(
            f"error: {source_path} does not exist; pass the mods folder or "
            "C:/Users/yello/horizon-work/java-ref",
            file=sys.stderr,
        )
        return 2
    try:
        source = AssetSource([source_path])
        if not source.entries:
            raise MissingAsset(f"{source_path}: no assets/ or data/ found")
        importer = Importer(source, args.out)
        counts = importer.build()
        importer.write_files()
        verify(args.out, source)
        report(args.report, importer, counts, source_path.name)
    except MissingAsset as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(
        f"content-java: {counts['blocks']} blockstates, {counts['items']} items, "
        f"{counts['models']} models, {counts['textures']} textures; "
        f"lang {counts['lang_en_us']} en_us / {counts['lang_fr_fr']} fr_fr"
    )
    print(f"wrote {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
