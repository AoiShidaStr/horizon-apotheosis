#!/bin/bash
# Build the Docker context for a dedicated Horizon server with Apotheosis compiled in.
#
# Usage: bash tools/docker/stage-context.sh HORIZON_SOURCE OUT_DIR
#   HORIZON_SOURCE  a checkout of horizon-rust/horizon at the commit this mod pins
#                   (native/apotheosis/horizon-mixin.toml, [compatibility] horizon_revision),
#                   e.g. the launcher's cache: %APPDATA%/horizon-launcher/native/source/<commit>
#   OUT_DIR         the folder to write (deleted first); then: docker build -f OUT_DIR/Dockerfile OUT_DIR
#
# It reproduces what the launcher's native_mods::api::stage does (native-api/<id> and a generated
# crates/native-game whose main registers the mod), the same as tools/vps/stage.sh.
set -euo pipefail
ID=apotheosis
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
SRC=${1:?HORIZON_SOURCE}
OUT=${2:?OUT_DIR}

rm -rf "$OUT" && mkdir -p "$OUT"
(cd "$SRC" && tar --exclude=./target --exclude=./.git -cf - .) | (cd "$OUT" && tar xf -)
cd "$OUT"

API="$ROOT/native/$ID/api"
mkdir -p "native-api/$ID"
cp -r "$API/src" "native-api/$ID/src"
sed "s|^horizon-client = .*|horizon-client = { path = \"../../crates/horizon-client\", default-features = false }|" \
    "$API/Cargo.toml" > "native-api/$ID/Cargo.toml"

PYTHON=$(command -v python3 >/dev/null 2>&1 && python3 -c 'import tomllib' 2>/dev/null && echo python3 || echo python)
BUILD_DEPS=$("$PYTHON" - <<'PY'
import tomllib
t = tomllib.load(open("crates/horizon-client/Cargo.toml", "rb"))
out = []
for k, v in t.get("build-dependencies", {}).items():
    if isinstance(v, str):
        out.append(f'{k} = "{v}"')
        continue
    parts = []
    for kk, vv in v.items():
        if isinstance(vv, bool):
            parts.append(f'{kk} = {"true" if vv else "false"}')
        elif isinstance(vv, list):
            parts.append(f'{kk} = [' + ", ".join(f'"{x}"' for x in vv) + ']')
        else:
            parts.append(f'{kk} = "{vv}"')
    out.append(f'{k} = {{ ' + ", ".join(parts) + ' }')
print("\n".join(out))
PY
)

mkdir -p crates/native-game/src
cat > crates/native-game/Cargo.toml <<TOML
[package]
name = "horizon-native-game"
version = "0.1.0"
edition = "2024"
publish = false
build = "../horizon-client/build.rs"

[[bin]]
name = "horizon-client"
path = "src/main.rs"

[dependencies]
horizon-client = { path = "../horizon-client", default-features = false }
horizon-threads = { path = "../horizon-threads" }
horizon-mods = { path = "../horizon-mods" }
horizon-official = { path = "../horizon-official", optional = true }
mod_0 = { path = "../../native-api/$ID", package = "mod-$ID" }

[build-dependencies]
$BUILD_DEPS

[target."cfg(windows)".build-dependencies]
winresource = "0.1"

[features]
default = ["horizon-client/default", "official"]
official = ["dep:horizon-official"]
TOML

cat > crates/native-game/src/main.rs <<'RS'
//! Generated as the launcher does (tools/docker/stage-context.sh).
#![forbid(unsafe_code)]

#[global_allocator]
static ALLOCATOR: horizon_threads::Allocator = horizon_threads::ALLOCATOR;

static MOD_API_LEVEL: [u8; horizon_mods::api_level::MARK.len()] = horizon_mods::api_level::MARK;

fn main() {
    std::hint::black_box(&MOD_API_LEVEL);
    let mods: horizon_client::mod_api::Mods = vec![
        mod_0::hooks(),
    ].into();
    horizon_client::mod_api::name_mods(&["apotheosis"]);
    #[cfg(feature = "official")]
    horizon_official::register();
    horizon_client::entry::main_with(mods);
}
RS

"$PYTHON" - <<'PY'
s = open("Cargo.toml", encoding="utf-8").read()
for m in ['"native-api/apotheosis"', '"crates/native-game"']:
    if m not in s:
        s = s.replace("members = [\n", "members = [\n    " + m + ",\n", 1)
open("Cargo.toml", "w", encoding="utf-8").write(s)
PY

cp "$HERE/Dockerfile" "$OUT/Dockerfile"
cp "$HERE/.dockerignore" "$OUT/.dockerignore" 2>/dev/null || true
echo "staged $OUT"
