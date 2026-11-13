# Apotheosis (gems + affixes) for Horizon

A Horizon slice of [Apotheosis](https://github.com/Shadows-of-Fire/Apotheosis)
by Shadows_of_Fire (MIT). Saison 2 runs it for **item rarities with affixes**
and **gems whose high tiers drop only from bosses**.

> **Clean room.** Nothing here is copied from Apotheosis: neither code nor
> art. The behaviour (tiers, gems, affixes, boss-only high tiers) comes from the
> mod's public wiki; the code is this port's own, written against Horizon's
> native hook API, and the textures are generated placeholders. Apotheosis' MIT
> licence would allow reuse; this slice did not need it.

## Done

- **Content mod `.hmod`** (`side = "both"`, id `apotheosis`): six gems —
  `gem_ruby`, `gem_topaz`, `gem_sapphire`, `gem_jade`, `gem_onyx`,
  `gem_bloodstone` — as data-driven items on `minecraft:amethyst_shard`, with
  their own models, generated 16x16 textures, an **Apotheosis** creative tab,
  and en_us/fr_fr names, keys and tooltip text. Each gem's vanilla rarity
  follows its tier (common → epic), so a ruby, a topaz and a sapphire already
  differ in name colour.
- **Six tiers**, `Common`, `Uncommon`, `Rare`, `Epic`, `Mythic`, `Ancient`
  (`Tier` in `native/apotheosis/api/src/lib.rs`): the tier decides how many
  affixes an item rolls (0, 1, 2, 3, 4, 5) and how strong they are (×1 to ×4).
  Vanilla's `minecraft:rarity` component only names four of them; mythic and
  ancient are shown in the tooltip and in an affixed item's lore.
- **Gems do something** (`modify_default_components`, API 79): each gem inserts
  a real `minecraft:attribute_modifiers` entry in the **off-hand** slot, so the
  game applies it through its own equipment code:

  | gem | tier | off-hand bonus |
  | --- | --- | --- |
  | ruby | common | +1 attack damage |
  | topaz | uncommon | +0.01 movement speed (+10%) |
  | sapphire | rare | +2 max health |
  | jade | epic | +2 armor |
  | onyx | mythic | +3 attack damage |
  | bloodstone | ancient | +4 max health |

  Its tooltip (`tooltip`, API 12) shows the tier in the tier's colour and the
  effect line, both translated (en_us/fr_fr).
- **Affixes on looted gear** (`loot_drops`, API 75): gear a mob drops (a
  zombie's sword, a skeleton's bow…) is rolled into a rarity and carries real
  `minecraft:attribute_modifiers`, a lore that names each affix, and the
  `minecraft:rarity` that colours its name. Eight affixes: Keen, Brutal, Swift,
  Stalwart, Bulwarked, Hasty, Steadfast, Tough — over attack damage, speed,
  max health, armor, armor toughness, mining efficiency and knockback
  resistance. Distinct affixes per item, scaled by tier.
- **High tiers stay boss-only**: an ordinary mob killed by a player rolls
  uncommon → epic only (55/30/15); a *wither*, *ender dragon*, *warden* or
  *elder guardian* rolls rare → ancient (25/40/25/10) and always drops one
  mythic or ancient gem, alternating.
- **Extra drops**: a boss always drops an affixed diamond piece (or a bow); an
  ordinary mob killed by a player has a 4% chance to drop an affixed iron piece.
- **`/apoth reforge`** (`register_commands`/`perform_command`, API 8): rolls
  tier and affixes onto the plain piece of gear the player holds, for 3
  experience levels; `/apoth` alone explains it. Answers are translated.

## Missing

- **Sockets on gear.** Gems give their bonus from the off hand; there is no
  socket slot in a piece of gear, no gem application screen and no way (in the
  current hook API) to rewrite an arbitrary stack in a player's inventory, so a
  gem cannot be fused into a sword. Gem dust, salvaging and the Apotheosis GUI
  are missing with it.
- **Reforging an already-affixed item.** `/apoth reforge` takes *plain* gear:
  the API's item helpers only take stacks with no components, so an affixed
  piece cannot be taken, rerolled and handed back. It rolls once onto a plain
  item instead.
- **Affixes replace an item's base attribute modifiers.** A stack's
  `minecraft:attribute_modifiers` component replaces the item's own defaults
  (vanilla semantics), so an affixed diamond sword loses its innate +3 attack
  damage and only keeps the rolled affixes. Reading an item's default
  modifiers is not possible from the hooks this mod uses, so merging them is
  not done; the affixes are an alternative to the base stats, not a bonus on
  top of them.
- **Affix pools are filtered by slot group only** (armour vs hand). A bow can
  still roll `attack_damage`, which only matters for a melee hit with it.
- **Affixes from chest loot tables** — only mob drops and the reforge command
  roll them. `loot_table_load` could affix dungeon loot; not done.
- **Advanced enchanting** (Apotheosis' enchanting stats and library) and
  **configurable spawners** (silk-touched spawners with a chosen mob) are not
  implemented.
- **Natural ore generation** for gems: gems come from bosses and reforge only.
- Affix names, lore and command answers are translated, but an affixed item
  keeps its vanilla name: the tier shows as a lore line, not as a prefix.
- No client-side drawing beyond the data-driven items and the tooltip lines.

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
  VPS with `tools/vps/sync.sh full` (see `tools/vps/check.sh` and
  `tools/vps/verify.sh`); it builds as a trait mod with
  `horizon-launcher native-build`.

### Build state (2026-11-14)

- v0.2.0 now compiles against the pinned commit `a85af177`: the first queued
  build caught two hook-signature errors (`perform_command` returned `()` and
  set a nonexistent `event.ran`); both are fixed (it returns `Flow`). The
  crate check, 11 unit tests, the dedicated-server engine without default
  features and the client flavour are reported in the session's verification
  note.
- What did run locally: all 21 content JSON files parse, and
  `python tools/pack.py` rebuilt `target/apotheosis.hmod` and
  `target/apotheosis.hnmod`.

## Licence

Apotheosis is MIT (Shadows_of_Fire). This port's own code and assets are MIT
as well; see `NOTICE.md`.
