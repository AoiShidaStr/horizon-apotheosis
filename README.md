# Apotheosis (gems) for Horizon

A Horizon slice of [Apotheosis](https://github.com/Shadows-of-Fire/Apotheosis)
by Shadows_of_Fire (MIT). Saison 2 runs it for **gem rarity tiers**: gems whose
tier goes above vanilla's epic, and whose mythic and ancient tiers drop only
from bosses.

> **Clean room.** Nothing here is copied from Apotheosis: neither code nor
> art. The behaviour (tiers, gems, boss-only high tiers) comes from the mod's
> public wiki; the code is this port's own, written against Horizon's native
> hook API, and the textures are generated placeholders. Apotheosis' MIT
> licence would allow reuse; this slice did not need it.

## Done

- **Content mod `.hmod`** (`side = "both"`, id `apotheosis`): six gems —
  `gem_ruby`, `gem_topaz`, `gem_sapphire`, `gem_jade`, `gem_onyx`,
  `gem_bloodstone` — as data-driven items on `minecraft:amethyst_shard`, with
  their own models, generated 16x16 textures, an **Apotheosis** creative tab
  and en_us/fr_fr names. Vanilla rarity carries common → epic.
- **Native trait mod `.hnmod`** (`native/apotheosis/api`), two hooks:
  - `tooltip`: a gem's tooltip gets a `Tier: Mythic` line in the tier's colour,
    so the two tiers vanilla cannot name (mythic, ancient) show.
  - `loot_drops` (server): a *wither*, *ender dragon*, *warden* or *elder
    guardian* kill drops one mythic or ancient gem, alternating. Ordinary mobs
    never drop those tiers.

## Missing

- **Affixes** (prefixes/suffixes rolled onto gear, with attribute modifiers),
  **sockets/gems-in-items**, **reforging**, and Apotheosis' **advanced
  enchanting** and **configurable spawners** are not implemented. They need
  per-stack data and attribute changes on an item, which this slice does not
  attempt; the gems here are inert items whose only behaviour is their tier
  line and their drop source.
- Gem application to gear, gem dust, salvaging, and the Apotheosis GUI.
- Rarity of ordinary loot (chance for a dropped sword to be rare/epic) — that
  would go through `loot_drops` plus item components; not done.
- Natural ore generation for gems: gems come from bosses only.
- No client-side drawing beyond what the data-driven items already do.

## Install

Both halves use the id `apotheosis`.

1. Put `apotheosis.hmod` in `horizon-mods/` and `apotheosis.hnmod` in
   `horizon-native-mods/`, and turn them on in `horizon-mods.txt`
   (`mod.apotheosis:true`).
2. Build the native half into the game (launcher **Mods** page, or
   `horizon-launcher native-build <folder>`). A dedicated server needs a
   server engine built with the mod: `horizon-launcher native-build <folder>
   --server-output <dir>`.
3. Clients and the dedicated server need the same `.hmod` (a `side = "both"`
   mod: a client with another version is refused).

## Build

- Content: `python tools/gen_content.py` regenerates the item data, models,
  names and textures; `python tools/pack.py` writes `target/apotheosis.hmod`
  and `target/apotheosis.hnmod`.
- Native: the crate is checked against the pinned Horizon commit on the build
  VPS with `tools/vps/sync.sh full` (see `tools/vps/check.sh`); it builds as a
  trait mod with `horizon-launcher native-build`.

### Build state (2026-11-13)

- `cargo test -p mod-apotheosis` on the VPS: **3 tests pass** (tier lines for
  the two high tiers, no line for vanilla items, boss tiers alternate), and the
  crate type-checks against the real `horizon-client` of the pinned commit.
- The dedicated-server engine check (`cargo check -p horizon-native-game
  --no-default-features`) and the client-flavour check were started in the same
  run but had not finished when the session ended (the shared build lock is
  serialized across mods; a first build of the client is ~15 min). They are the
  first thing to re-run: `bash tools/vps/sync.sh full`.

## Licence

Apotheosis is MIT (Shadows_of_Fire). This port's own code and assets are MIT
as well; see `NOTICE.md`.
