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

- **Content mod `.hmod`** (`side = "both"`, id `apotheosis`) with **30 creative
  entries**: twelve gems, gem dust, twelve decorative gem blocks and five
  rarity salvage materials. All of them are craftable or salvageable in
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
  pickaxe tag. A Ruby Block is storage, light and decoration; the working
  tables are blocks of their own.
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
  experience levels; on an already-affixed piece it rerolls that piece instead.
  `/apoth salvage` breaks an affixed piece into rarity materials; `/apoth`
  alone explains both. Answers are translated.

## Gemmes, sockets, reforge et recyclage

### Done

- **Sockets (0 to 3, by rarity)** stored in the item's `minecraft:custom_data`
  (`apotheosis:sockets`, `apotheosis:socketed`): Common 0, Uncommon 0–1, Rare
  0–2, Epic 1–3, Mythic 2–3, Ancient 3. The tooltip (`tooltip`, API 12) shows
  `Sockets: n/total` and one line per socketed gem: purity, bonus and
  attribute, coloured by the gem's purity.
- **Socketing by right click** (`use_item`, API 13): gem in the main hand, a
  piece with free sockets in the off hand, right click. The gem is consumed
  (`inventory_tick`, API 39, through `Host::carried_stack`) and its bonus
  becomes a real `minecraft:attribute_modifiers` entry on the piece
  (`apotheosis:socket_<n>_<gem>`, `add_value`, `hand` or `armor` slot), beside
  the affixes' entries, so the game applies it when the piece is worn. An
  unsocketed gem keeps the off-hand bonus the content mod gives it.
- **Purity** Cracked, Chipped, Flawed, Normal, Flawless, Perfect (×0.2, ×0.4,
  ×0.7, ×1, ×1.4, ×2 on the gem's bonus), rolled where the gem drops
  (`loot_drops`): ordinary loot 30/40/20/8/2/1, boss gems 5/20/35/28/10/2.
  Shown in the gem's tooltip. A gem in the main hand with 4 gem dust in the
  off hand, right-clicked, is polished one step up (4 dust per step, Perfect
  is the ceiling).
- **Reforging an affixed piece**: `/apoth reforge` on an affixed piece held in
  the main hand rerolls its rarity and affixes for 3 levels, keeping the piece
  and the gems in its sockets; the work is done on the next inventory tick, so
  the stack the command saw is the stack that is rewritten.
- **Salvaging**: `/apoth salvage` on an affixed piece breaks it into 1–4 of its
  rarity's material, gem dust (1–4 by rarity) and gives back every socketed
  gem at the purity it had.
- **Four sigils** (`sigil_of_socketing`, `sigil_of_withdrawal`,
  `sigil_of_rebirth`, `sigil_of_enhancement`), each a stackable item with its
  own 16×16 wax-seal texture, en_us/fr_fr name and description line, a shaped
  recipe from the rarity materials (iron and gold, fabric and iron, crystal and
  blaze powder, arcane sands and an emerald), and a place in the creative tab.
  A sigil is used with the affixed piece in the **off hand** and the sigil in
  the **main hand**, right click: socketing adds one socket up to the maximum,
  withdrawal takes every socketed gem back out at the purity it had, rebirth
  rerolls the piece's rarity and affixes (gems kept), enhancement moves the
  rarity one step up (Common→…→Ancient) and rerolls at the new tier. The sigil
  is used up on the next inventory tick, so a hand that changed costs nothing.
- **Three working tables as blocks**, with Apotheosis' ids
  (`gem_cutting_table`, `salvaging_table`, `reforging_table`): a real block
  declaration (oak planks template, own top and side textures, `cube_bottom_top`
  model), item, blockstate, block loot table, pickaxe tag, shaped recipe (iron
  and amethyst around gem dust, iron and planks, gold and scrap metal), and the
  creative tab. Right click with the piece in hand (`use_item_on`, API 13): the
  reforging table does what `/apoth reforge` does (3 levels), the salvaging
  table what `/apoth salvage` does, and the gem cutting table spends 4 gem dust
  from the inventory to raise the held gem's purity one step
  (`Host::take_items`, refunded if the dust is not all there). The click is
  always claimed, so a misclick shows the table's hint instead of opening
  something else.
- **Chest gems roll their purity** (`loot_function`, API 133): the gem pool
  `loot_table_load` adds to vanilla chests carries the mod's own
  `apotheosis:gem_purity` function, which draws from the loot roll's seed with
  the ordinary weights and writes `apotheosis:gem_purity` into the stack's
  `minecraft:custom_data` component.
- **Five rarity materials** with Apotheosis' ids — `mysterious_scrap_metal`
  (common), `timeworn_fabric` (uncommon), `luminous_crystal_shard` (rare),
  `arcane_sands` (epic), `godforged_pearl` (mythic and ancient) — each with its
  own item declaration, model, 16×16 texture drawn by `tools/gen_sockets.py`,
  en_us/fr_fr name, a crafting recipe (iron nuggets and flint, string and
  leather, amethyst and glowstone, soul sand and lapis, gold around an ender
  pearl), a 1 → 3 gem dust recipe, and a place in the creative tab.

### Missing

- **No real menu screens.** The tables are blocks that act on the held stack
  on a right click; none opens a GUI, so there is no grid to place a piece and
  a gem in, no preview of the result, and no slot-level picking of which gem to
  pull out. A chest-style server menu (API 10) could show one, at the cost of
  modelling every click by hand.
- **The other 1.21 sigils are not ported**: `sigil_of_unnaming` (this port never
  writes a custom name onto an affixed piece), `sigil_of_malice` and
  `sigil_of_supremacy` (no overcharged-affix state), and the augmenting table
  they feed (no per-affix level).
- **A socket, once added, is permanent**: no sigil takes one away, only the
  gems can be withdrawn.
- **Gear affixed before this slice** carries no `apotheosis:tier`, so it can be
  neither reforged nor salvaged; only pieces rolled (or dropped) after the
  update can.
- **Purity only scales a socketed gem.** An unsocketed gem's off-hand bonus is
  its flat value; there is no purity-scaled off-hand bonus.

## Missing

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
- **Advanced enchanting** (Apotheosis' enchanting stats and library) and
  **configurable spawners** (silk-touched spawners with a chosen mob) are not
  implemented.
- **Natural ore generation** for gems: gems come from mobs, bosses, recipes
  and reforge only.
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

- Content: `python tools/gen_content.py` regenerates the item and block data,
  models, blockstates, loot tables, recipes, tags, names and textures;
  `python tools/gen_sockets.py` adds the rarity salvage materials, their
  textures, recipes, names and creative-tab entries without touching the gem
  entries; `python tools/pack.py` writes `target/apotheosis.hmod` and
  `target/apotheosis.hnmod`.
- Native: the crate is checked against the pinned Horizon commit on the build
  VPS with `tools/vps/sync.sh full` (see `tools/vps/check.sh` and
  `tools/vps/verify.sh`); it builds as a trait mod with
  `horizon-launcher native-build`.

### Build state (2026-11-22)

- v0.4.0 compiles against the pinned commit `a85af177` (`cargo check -p
  mod-apotheosis`, one warning: private `Rng` behind the public test helper).
  **27 unit tests pass** (twelve of them in the `sockets` module: purity steps,
  socket counts, socketing, reforge and salvage yields, the sigil and table
  ids, the rarity ladder, and the chest gem purity draw), and
  `cargo check -p horizon-native-game
  --no-default-features` (dedicated-server engine with this mod staged)
  passes, as does the client flavour (`cargo check -p horizon-native-game`,
  default features).
- `cargo horizon-mod check` passes on both packed files
  (`apotheosis.hmod`: kind `mod`, `side both`; `apotheosis.hnmod`: kind
  `native-mod`, api `native-1/a85af177…/5a64af93…`).
- The content mod has twelve gems (two per tier), gem dust, twelve gem blocks
  with textures, models, loot tables and a pickaxe tag, five rarity salvage
  materials, four sigils, the three working tables, and 55 recipes. Mob drops
  and all vanilla chest loot tables now provide survival gem routes; chest
  gems are restricted to common and uncommon tiers and roll a purity from the
  table's own seed. The four mythic/ancient gems shimmer. All content JSON files parse, and
  `python tools/pack.py` rebuilds `target/apotheosis.hmod` and
  `target/apotheosis.hnmod`.
- The registry check caught `horizon-mixin.toml` still naming version `0.2.0`
  after the bump ("native manifest and mixin identity/revision differ"); the
  three version fields now move together, and both packed files check clean.

## Licence

Apotheosis is MIT (Shadows_of_Fire). This port's own code and assets are MIT
as well; see `NOTICE.md`.
