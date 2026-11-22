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

- **Content mod `.hmod`** (`side = "both"`, id `apotheosis`) with **52 creative
  entries**: twelve gems, gem dust, twelve decorative gem blocks, twenty-three
  enchanting shelves and four tome/cobweb items. All of them are craftable in
  survival (see *Recipes* below).
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
- **The enchanting library** and **configurable spawners** (silk-touched
  spawners with a chosen mob) are not implemented. Enchanting stats and the
  tomes are ported (see *Apothic Enchanting* below), with the limits listed
  there.
- **Natural ore generation** for gems: gems come from mobs, bosses, recipes
  and reforge only.
- Affix names, lore and command answers are translated, but an affixed item
  keeps its vanilla name: the tier shows as a lore line, not as a prefix.
- No client-side drawing beyond the data-driven items and the tooltip lines.

## Apothic Enchanting

### Done

- **Twenty-three shelf blocks** on `minecraft:bookshelf` templates, with original
  generated 16x16 textures, own blockstate/model, `block.apotheosis.<shelf>`
  names in en_us and fr_fr, an entry in the creative tab, a self-drop loot
  table and the axe mining tag. Their stats are public Apothic Enchanting
  values (missing `maxEterna` fields default to 30):

  | shelf | Eterna | max Eterna | Quanta | Arcana | Clues |
  | --- | --- | --- | --- | --- | --- |
  | basic_bookshelf | 2 | 30 | 0 | 0 | 0 |
  | hellshelf | 3 | 45 | 3 | 0 | 0 |
  | blazing_hellshelf | 10 | 65 | 10 | 0 | -1 |
  | glowing_hellshelf | 5 | 60 | 5 | 3.3333 | 0 |
  | infused_hellshelf | 5 | 60 | 5 | 0 | 0 |
  | seashelf | 3 | 45 | 0 | 3 | 0 |
  | crystal_seashelf | 5 | 60 | 3 | 5 | 0 |
  | heart_seashelf | 15 | 60 | 0 | 20 | 0 |
  | infused_seashelf | 5 | 60 | 0 | 5 | 0 |
  | deepshelf | 5 | 70 | 5 | 5 | 0 |
  | dormant_deepshelf | 2 | 30 | 0 | 0 | 0 |
  | echoing_deepshelf | 5 | 75 | 0 | 15 | 0 |
  | soul_touched_deepshelf | 5 | 75 | 15 | 0 | 0 |
  | echoing_sculkshelf | 10 | 80 | 10 | 15 | 1 |
  | soul_touched_sculkshelf | 10 | 80 | 15 | 10 | 1 |
  | endshelf | 5 | 90 | 5 | 5 | 0 |
  | pearl_endshelf | 10 | 90 | 7.5 | 7.5 | 0 |
  | draconic_endshelf | 20 | 100 | 0 | 0 | 0 |
  | sightshelf | 0 | 30 | 0 | 0 | 1 |
  | sightshelf_t2 | 0 | 30 | 0 | 0 | 2 |
  | beeshelf | -30 | 0 | 100 | 0 | 0 |
  | melonshelf | -2 | 0 | -10 | 0 | 0 |
  | stoneshelf | -3 | 0 | 0 | -7.5 | 0 |

  Vanilla `minecraft:amethyst_cluster`, `minecraft:skeleton_skull` /
  `minecraft:wither_skeleton_skull` and `minecraft:wither_skull` are recognised
  too (+1 Eterna/40 max, +5 Quanta, +10 Quanta respectively), as in the
  original.
- **Eterna at the enchanting table** (`prepare_item_enchant`, API 291). The
  native module (`native/apotheosis/api/src/enchanting.rs`) walks vanilla's 32
  perimeter positions two blocks out, ignores a shelf behind a non-air block,
  and aggregates the stats: base 15 Quanta and one clue, Eterna contributions
  applied in ascending cap order (`min(cap, current + eterna)`), uncapped for
  the negative-cap shelves, then everything clamped to 0..100. The three offer
  costs are then scaled from Eterna — the third offer equals Eterna, the lower
  two take the midpoint of 20–40% and 60–80% of it — so a well-stocked table
  offers level 45, 70 or 100 enchantments instead of vanilla's 30-level cap.
  Each offer keeps at least its vanilla eligibility (1, 2, 3), costs that many
  experience levels, and is never lowered below what vanilla had already
  computed for the table.
- **The shelves also feed the player's own table.** Each one is in
  `minecraft:enchantment_power_provider`, so in single player, where Horizon
  does not dispatch API 291, they count like bookshelves for vanilla power and
  show the table's particles. Vanilla still caps that path at 15 providers
  (level 30).
- **Survival recipes** (vanilla `minecraft:crafting_shaped`): the basic
  bookshelf is three planks, three books, three planks; every other shelf is
  one `minecraft:bookshelf` surrounded by four of its theme catalyst — honeycomb,
  blaze rod or powder, glowstone, nether wart, pufferfish, prismarine shard or
  crystals, heart of the sea, sculk, echo shard, ender pearl, dragon head,
  spyglass, emerald, melon slice or polished andesite. The custom
  `apothic_enchanting:infusion` station does not exist in Horizon, so no recipe
  depends on it.
- **Three tomes and the Prismatic Cobweb** (`use_item`, API 13), used on a
  right click: hold the tome in one hand and the enchanted item in the other.
  API 415 has no anvil hook, so this replaces the original's anvil step. Each
  one consumes the tome and charges experience levels (nothing in creative):
  - scrap tome, 6 levels per enchantment kept: keeps `ceil(n/2)` of them with
    a seeded draw (same player and item give the same result) and puts them in
    an enchanted book;
  - superior scrap tome, 10 levels per enchantment: keeps all of them in a
    book;
  - extraction tome, 16 levels per enchantment: moves all of them into a book
    and leaves the item in hand stripped;
  - Prismatic Cobweb, 30 levels: removes `minecraft:binding_curse` and
    `minecraft:vanishing_curse` and keeps the rest on the item.

  A scrapped item is only taken once the book fits in the inventory, and an
  extraction restores the item if the book cannot be given. All four have
  original 16x16 textures, en_us/fr_fr names, creative-tab entries and vanilla
  shaped recipes: eight books around an anvil give eight scrap tomes, diamonds
  or netherite scraps upgrade one to the superior or extraction tome, and four
  prismarine shards around a cobweb give the Prismatic Cobweb.

### Missing

- **Quanta and Arcana are computed but not applied.** API 291 only hands over
  the three costs; the finished-enchantment hook (API 209) is read-only and
  carries neither the seed, the offers nor a mutable result, so this slice
  cannot widen the power variance, raise the enchantment count or bias rarity.
  Their values live in the native registry for later use and are covered by
  tests.
- **Enchantment levels above vanilla's maximum.** The hooks expose the offers'
  costs, not their enchantments, so Efficiency VI or Sharpness VI cannot be
  written onto the result. A high-Eterna table can *charge* up to 100 levels,
  but the enchantments it grants are still vanilla's.
- **Custom Apothic enchantments** (Life Mending, Scavenger, Knowledge of the
  Ages, Bane of Illagers, Berserker's Fury, Crescendo of Bolts, Earth's Boon,
  Endless Quiver, Icy Thorns, Miner's Fervor, Natural Blessing, Rebounding,
  Reflective Defenses, Shield Bash, Stable Footing, Tempting, Chromatic,
  Exploitation). A `.hmod` cannot declare a `data/<ns>/enchantment/` entry: the
  content loader has no enchantment content type at API 415, and a world
  datapack cannot add registries in Horizon. `native_enchant_effect` (API 77)
  can give a new effect *kind* to an existing `minecraft:post_attack` entry,
  not register the enchantment itself.
- **Eterna above 30 in single player.** API 291 is dispatched only for a LAN
  or dedicated host's guests (`lan_stations.rs`); the integrated single-player
  table computes its offers locally without calling the hook. Local play gets
  the tag-based power above, capped at vanilla's 15 providers.
- **The enchanting library, infusion and rectifiers.** No library block or
  infusion altar is added; the recipes stop at vanilla crafting. The tomes are
  used by a right click rather than in an anvil, since API 415 has no anvil
  hook.

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
