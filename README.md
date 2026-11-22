# Apotheosis (gems + affixes) for Horizon

A Horizon slice of [Apotheosis](https://github.com/Shadows-of-Fire/Apotheosis)
by Shadows_of_Fire (MIT). Saison 2 runs it for **item rarities with affixes**
and **gems whose high tiers drop only from bosses**.

> **Clean room.** Nothing here is copied from Apotheosis: neither code nor
> art. The behaviour (tiers, gems, affixes, boss-only high tiers) comes from the
> mod's public description; the code is this port's own, written against Horizon's
> native hook API, and the textures are original generated 16x16 pixel art. Apotheosis' MIT
> licence would allow reuse; this slice did not need it.

## Done

- **Content mod `.hmod`** (`side = "both"`, id `apotheosis`) with **25 creative
  entries**: twelve gems, gem dust, and twelve decorative gem blocks. All of
  them are craftable in survival (see *Recipes* below).
- **Twelve gems, two per tier** — `gem_ruby`, `gem_amber`, `gem_topaz`,
  `gem_amethyst`, `gem_sapphire`, `gem_peridot`, `gem_jade`, `gem_citrine`,
  `gem_onyx`, `gem_moonstone`, `gem_bloodstone`, `gem_opal` — as data-driven
  items on `minecraft:amethyst_shard` that stack to 16, each with its own model,
  generated 16x16 texture, en_us/fr_fr name and tooltip. Each gem's vanilla
  rarity follows its tier (common → epic), so a ruby, a topaz and a sapphire
  already differ in name colour; the four mythic and ancient gems carry
  `minecraft:enchantment_glint_override`, so they shimmer in the inventory
  before their tooltip is read.
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
  | amber | common | +15% mining speed |
  | topaz | uncommon | +0.01 movement speed (+10%) |
  | amethyst | uncommon | +0.4 attack speed (+10%) |
  | sapphire | rare | +2 max health |
  | peridot | rare | +1 luck |
  | jade | epic | +2 armor |
  | citrine | epic | +0.5 attack knockback |
  | onyx | mythic | +3 attack damage |
  | moonstone | mythic | +3 max health |
  | bloodstone | ancient | +4 max health |
  | opal | ancient | +3 armor toughness |

  Its tooltip (`tooltip`, API 12) shows the tier in the tier's colour and the
  effect line, both translated (en_us/fr_fr).
- **Recipes for everything** (vanilla 26.3 data-pack format under
  `data/apotheosis/recipe/`):
  - **Gem dust** (`gem_dust`, a stack of 64) from one amethyst shard → 2 dust,
    and 1 gem (via the `apotheosis:gems` item tag) → 3 dust.
  - **Each gem**: four gem dust around a themed vanilla catalyst (redstone for
    ruby, honeycomb for amber, glowstone dust for topaz, amethyst shard for
    amethyst, lapis for sapphire, emerald for peridot, prismarine shard for
    jade, gold ingot for citrine, obsidian for onyx, quartz for moonstone,
    magma cream for bloodstone, prismarine crystals for opal).
  - **Each gem block**: 9 gems → 1 block, and 1 block → its 9 gems back.
- **Twelve gem blocks** (`gem_block_<gem>`): decorative `minecraft:amethyst_block`
  templates with generated faceted textures, hardness and blast resistance
  rising with the tier, a **light level from 3 (common) to 8 (ancient)**, own
  blockstate/model/item model, a self-drop loot table, and an entry in the
  pickaxe tag. A Ruby Block is storage, light and decoration; it is not a
  socketing table.
- **Affixes on looted gear** (`loot_drops`, API 75): gear a mob drops (a
  zombie's sword, a skeleton's bow…) is rolled into a rarity and carries real
  `minecraft:attribute_modifiers`, a lore that names each affix, and the
  `minecraft:rarity` that colours its name. Eight affixes: Keen, Brutal, Swift,
  Stalwart, Bulwarked, Hasty, Steadfast, Tough — over attack damage, speed,
  max health, armor, armor toughness, mining efficiency and knockback
  resistance. Distinct affixes per item, scaled by tier.
- **Gems from mobs**: an ordinary mob killed by a player has a 12% chance to
  drop one of the four low gems (ruby, amber, topaz, amethyst). **High tiers
  stay boss-only**: an ordinary mob's gear rolls uncommon → epic only
  (55/30/15); a *wither*, *ender dragon*, *warden* or *elder guardian* rolls
  rare → ancient (25/40/25/10) and always drops one high gem, cycling onyx,
  moonstone, bloodstone, opal.
- **Gems from vanilla chests** (`loot_table_load`, API 75): every
  `minecraft:chests/*` table has an 8% chance to roll one weighted common or
  uncommon gem. Mythic and ancient gems remain boss-only.
- **Extra drops**: a boss always drops an affixed diamond piece (or a bow); an
  ordinary mob killed by a player has a 4% chance to drop an affixed iron piece.
- **`/apoth reforge`** (`register_commands`/`perform_command`, API 8): rolls
  tier and affixes onto the plain piece of gear the player holds, for 3
  experience levels; `/apoth` alone explains it. Answers are translated.

## Missing

- **Sockets on gear.** Gems give their bonus from the off hand; there is no
  socket slot in a piece of gear, no gem application screen and no way (in the
  current hook API) to rewrite an arbitrary stack in a player's inventory, so a
  gem cannot be fused into a sword. Gem dust, the gem-cutting and salvaging
  tables, and the Apotheosis GUI are missing with it; dust is a plain crafting
  ingredient here.
- **Gem purity.** The original rolls each gem at a purity (Cracked → Perfect)
  that scales its bonus; this slice's tier is fixed per gem type and there are
  no purity rolls or upgrade recipes.
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
- **Affixes from chest loot tables** — chest tables can supply gems, but their
  gear remains un-affixed; `loot_table_load` cannot roll the item components
  the later loot generation creates.
- **Advanced enchanting** (Apotheosis' enchanting stats and library) is not
  implemented.
- **Natural ore generation** for gems: gems come from mobs, bosses, recipes
  and reforge only.
- Affix names, lore and command answers are translated, but an affixed item
  keeps its vanilla name: the tier shows as a lore line, not as a prefix.
- No client-side drawing beyond the data-driven items and the tooltip lines.

## Apothic Spawners

### Done

- **Silk-touched spawners keep their mob and their settings**
  (`left_click_block` API 30, `loot_drops` API 75, `place_block` API 14):
  starting to dig a spawner reads its block entity; breaking it with a
  silk-touch tool drops a `minecraft:spawner` whose
  `minecraft:block_entity_data` is that data; placing it writes the data back
  into the block, so both the mob and the stats survive, even for a player
  who may not set a spawner's data themselves.
- **The sixteen modifiers of Apothic Spawners** (`use_item_on` API 13), with
  their real items and values: sugar and a clock for the delays, a fermented
  spider eye for the wave size, a ghast tear for the mobs nearby, prismarine
  crystals for the player range, a piston for the spawn range, pointed
  dripstone for the initial health, an echo shard for the echo (0 to 3), and
  the flags: nether star (ignores players), soul lantern (ignores light),
  conduit (ignores conditions), chorus fruit (no AI), wool (silent),
  comparator (redstone controlled), turtle egg (young mobs) and campfire
  (burning mobs). Every step is clamped to Apothic Spawners' bounds, and the
  item is not used up by the click.
- **The inverter** (`minecraft:quartz`) takes the same steps back, and turns
  nothing off: a flag is only ever turned on, as in Apothic Spawners.
- **The stats are shown** (`action_bar` API 21) as one line above the hotbar
  on every click on a spawner: the mob, the two delays, the mobs per wave, the
  player range, the mobs allowed nearby, the spawn range, then the flags that
  are on. Both languages ship the line (`tools/gen_spawners.py`).
- **A carried spawner says what it holds** (`tooltip` API 12), on the client
  and with no command at all: the item's own `minecraft:block_entity_data`
  gives one line per fact — the mob, the delays, the mobs per wave, the three
  ranges, the mob health and the echo while a modifier moved them, and the
  flags that are on. Both languages ship these lines too.
- **A read is a one-tick probe, gated on the block.** Each read names its
  probe, so two reads never answer for each other and an old probe is never
  read as the new one; every command is written `execute in <dimension> if
  block <pos> minecraft:spawner run …`, so a spawner broken between the click
  and the tick, a chunk that is not loaded, or another block that took the
  position fails quietly instead of writing into the log or into that block;
  a probe that a killed command left behind is swept; a failed read also drops
  what an older read had cached for that position, so a new spawner never
  inherits the old one's data.

### Missing

- **No block entity NBT hook at the pinned API.** `block_entity_tag`,
  `set_block_entity`, `block_entity_nbt` and `run_command_captured` are all
  still pending fragments at hooks 415, so this slice reads a spawner by
  copying its data into a one-tick probe item entity with `/data modify` and
  reading that item back (`item_entities`, API 42), and writes it with
  `/data merge block` (`run_command`, API 21). The read therefore happens on
  the game tick after the click, never on a client, and it needs the player
  to be in a server world. Replacing `read` and `write` is all it takes when
  the hook lands.
- **The off-hand inverter is best effort.** Reading another hand needs
  `player_equipment`, which is pending at 415, so the quartz has to be the
  item actually used on the spawner; a quartz held only in the off hand is
  not seen.
- **No spawner screen, no mob capture item, no "spawner module"**: Apothic
  Spawners' GUI and its extra mob sources (a captured mob's equipment and
  loot) are not ported. The stats are shown as an action bar line and as the
  carried spawner's tooltip, not in a screen.
- **The Apotheosis bosses** are another chantier's, not this one's.

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

- Content: `python tools/gen_content.py` regenerates the item and block data,
  models, blockstates, loot tables, recipes, tags, names and textures;
  `python tools/pack.py` writes `target/apotheosis.hmod` and
  `target/apotheosis.hnmod`.
- Native: the crate is checked against the pinned Horizon commit on the build
  VPS with `tools/vps/sync.sh full` (see `tools/vps/check.sh` and
  `tools/vps/verify.sh`); it builds as a trait mod with
  `horizon-launcher native-build`.

### Build state (2026-11-14)

- v0.4.0 compiles against the pinned commit `a85af177` (`cargo check -p
  mod-apotheosis`, one warning: private `Rng` behind the public test helper).
  **14 unit tests pass**, and `cargo check -p horizon-native-game
  --no-default-features` (dedicated-server engine with this mod staged)
  passes, as does the client flavour (`cargo check -p horizon-native-game`,
  default features).
- `cargo horizon-mod check` passes on both packed files
  (`apotheosis.hmod`: kind `mod`, `side both`; `apotheosis.hnmod`: kind
  `native-mod`, api `native-1/a85af177…/5a64af93…`).
- The content mod has twelve gems (two per tier), gem dust, twelve gem blocks
  with textures, models, loot tables and a pickaxe tag, and 38 recipes. Mob
  drops and all vanilla chest loot tables now provide survival gem routes;
  chest gems are restricted to common and uncommon tiers. The four
  mythic/ancient gems shimmer. All content JSON files parse, and
  `python tools/pack.py` rebuilds `target/apotheosis.hmod` and
  `target/apotheosis.hnmod`.
- The registry check caught `horizon-mixin.toml` still naming version `0.2.0`
  after the bump ("native manifest and mixin identity/revision differ"); the
  three version fields now move together, and both packed files check clean.

## Licence

Apotheosis is MIT (Shadows_of_Fire). This port's own code and assets are MIT
as well; see `NOTICE.md`.
