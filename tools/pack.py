#!/usr/bin/env python3
"""Pack this mod's two files.

  apotheosis.hmod   horizon-mod.toml + assets/ + data/
  apotheosis.hnmod  horizon-native.json + mixins/apotheosis/... (the Rust source)

Run: python tools/pack.py
See docs/api/mod-files.md in Horizon.
"""
import json
import os
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
ID = "apotheosis"
HORIZON = "a85af177a82504f357ff10d5d40d0b7230e2ac32"
# launcher/src/native_mods.rs MIXINS_REV at that Horizon commit.
MIXINS = "5a64af938ec86b00d53d16a408aabc0038fb3e57"
GAME = "26.3"
# NATIVE_HOOKS_API of the pinned Horizon commit (crates/horizon-client/src/game/native_hooks.rs).
HOOKS = 415

BUILD = os.path.join(ROOT, "target")


def add_tree(zf, base, prefix=""):
    for dirpath, dirnames, filenames in os.walk(base):
        dirnames[:] = sorted(d for d in dirnames if not d.startswith("."))
        for name in sorted(filenames):
            if name.startswith("."):
                continue
            full = os.path.join(dirpath, name)
            rel = os.path.relpath(full, base).replace("\\", "/")
            zf.write(full, prefix + rel if prefix else rel)


def version():
    with open(os.path.join(ROOT, "content", "horizon-mod.toml"), encoding="utf-8") as f:
        for line in f:
            if line.strip().startswith("version"):
                return line.split("=", 1)[1].strip().strip('"')
    raise SystemExit("no version in content/horizon-mod.toml")


def build_content(ver):
    os.makedirs(BUILD, exist_ok=True)
    out = os.path.join(BUILD, ID + ".hmod")
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as zf:
        zf.write(os.path.join(ROOT, "content", "horizon-mod.toml"), "horizon-mod.toml")
        add_tree(zf, os.path.join(ROOT, "content", "assets"), "assets/")
        add_tree(zf, os.path.join(ROOT, "content", "data"), "data/")
    return out


def build_native(ver):
    os.makedirs(BUILD, exist_ok=True)
    out = os.path.join(BUILD, ID + ".hnmod")
    meta = {
        "schema": 1,
        "id": ID,
        "version": ver,
        "game": GAME,
        "horizon": HORIZON,
        "mixins": MIXINS,
        "hooks": HOOKS,
    }
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as zf:
        zf.writestr("horizon-native.json", json.dumps(meta, indent=2) + "\n")
        add_tree(zf, os.path.join(ROOT, "native", ID), "mixins/%s/" % ID)
        # A .hnmod holds only horizon-native.json and mixins/<id>/; the licence
        # and the notices belong under the bundle, where README*/NOTICE* are
        # the only allowed extra names.
        for name in ("README.md", "NOTICE.md"):
            path = os.path.join(ROOT, name)
            if os.path.exists(path):
                zf.write(path, "mixins/%s/%s" % (ID, name))
    return out


def main():
    ver = version()
    for path in (build_content(ver), build_native(ver)):
        print("%s  %d bytes" % (path, os.path.getsize(path)))


if __name__ == "__main__":
    main()
