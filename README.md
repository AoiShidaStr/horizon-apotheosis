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
  `minecraft:rarity` that colours its name. 26 affixes (see *Apothic
  Attributes et affixes* below), each rolled only onto gear it can affect.
  Distinct affixes per item, scaled by tier; an affixed copy keeps the item's
  own base modifiers in front of the rolled ones.
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

## Apothic Attributes et affixes

### Done

- **Fifteen attributes** (`native/apotheosis/api/src/attributes.rs`), stored on
  an affixed stack in `minecraft:custom_data` under `apotheosis:attributes`
  and read back by the hooks: crit chance and crit damage, life steal,
  overheal, dodge chance, armor pierce, armor shred, fire and cold damage,
  experience gained, healing received, arrow damage and velocity, bow draw
  speed, and mining speed.
- **They do something in play**:
  - *Damage*: `living_incoming_damage_v2` scales an attacker's hit by crit,
    fire, cold, pierce and shred (the victim's armor absorption is reduced by
    shred and then by pierce); `living_incoming_damage` applies the victim's
    dodge and rolls the crit chance. Dodge is the only case that cancels the
    hit (flow `Cancel`), so a dodged arrow is not a free hit.
  - *Life steal and overheal*: the damage actually dealt heals the attacker by
    the life-steal share, above max health through absorption for overheal.
  - *Bow*: `arrow_loose` remembers the shooter's arrow-damage multiplier for
    that shot (five seconds), `living_incoming_damage` spends it; draw speed
    scales the charge needed to loose, so a faster bow fires sooner.
  - *Healing*: `living_heal` scales incoming healing by *healing received*
    and refunds the **overheal** share, up to the missing health.
  - *Mining*: `break_speed` scales the break speed of the held tool.
  - *Experience*: `experience_orb_pickup` adds the experience-gained share to
    the orb's value.
- **Tooltips** name each attribute and its value in blue under the affix
  lore, translated in en_us and fr_fr.
- **Affixes are filtered by item class**: sword and axe (melee), bow and
  crossbow (ranged), helmet, chestplate, leggings and boots separately, tool,
  shield. A bow can no longer roll `attack_damage`; a chestplate cannot roll a
  bow affix; an item with no matching affix rolls none rather than falling
  back to the whole pool.
- **Base modifiers are merged**: `modify_default_components` records each
  item's own `minecraft:attribute_modifiers` before anything edits them, and
  `apply_affixes` writes them ahead of the rolled ones, so an affixed diamond
  sword keeps its innate attack damage.
- **Names** for all 15 attributes and 18 new affixes are added to the shared
  files by `python tools/gen_attributes.py` (merges, never replaces; the
  creative tab and tags are checked and left untouched, no item is added).

### Effets d'affixes et boss Apothic Invader

#### Done

- **Fourteen effect affixes** (`native/apotheosis/api/src/affix_effects.rs`,
  `EFFECT_AFFIXES`) on top of the attribute ones, with upstream ids, item
  classes and epic+ rarities: Venomous, Withering, Weakening, Ensnaring,
  Blinding (helmet), Executing, Cleaving, Thunderstruck, Retreating (shield),
  Durable, Telepathic, Spectral and Magical (ranged), Festive.
- **Their payouts run in the real hooks**: poison/wither/weakness/slowness on
  the victim, blindness on the attacker, execution under the rolled health
  share, a cleave to nearby foes, real lightning, a knockback away from the
  attacker, less durability wear, drops straight to the inventory, a free
  spectral arrow, extra magic damage, and doubled kill loot.
- They are **extra affixes**, never replacements: `apply_affixes` still rolls
  the same number of attribute modifiers and lore lines, then adds at most one
  effect affix from epic up. The roll is stored in
  `minecraft:custom_data` under `apotheosis:effects` and read back per hit.
  Tooltips show the effect line in green, translated in en_us and fr_fr.
- **Apothic Invader boss**: `entity_join_level` queues fresh joins and
  `mob_spawned` consumes the one matching a natural spawn. An overworld spawn
  must be at night and on the surface (column heightmap); Nether and End
  spawns only need to be natural. A 2 % roll converts it.
- The invader gets a **generated name** (30 % prefix, 80 % suffix, translated
  word by word so FR/EN both read naturally), more health (20/40/70/110 by
  rarity), affixed diamond gear in five slots from the boss tier table, the
  persistent `apotheosis:boss` tag, and a broadcast announcement.
- Its death is recognised by the tag: the existing `loot_drops` boss path
  drops a high gem and rolls boss-tier affixes on its gear.
- Names and descriptions for the 14 effect affixes, the boss words, titles
  and 24 base mob names are merged into `en_us.json` and `fr_fr.json` by
  `python tools/gen_attributes.py` (idempotent, shared files untouched).

#### Missing

- Effects only use vanilla effect ids; Apothic's own bleeding, sundering,
  grievous and knowledge effects have no equivalent here.
- A mob's `active_effects` are merged into its NBT; a mod-added effect or a
  client-side effect icon beyond vanilla's is not shown.
- Retreating fires on any hit with a shield in the off hand, not only on an
  actual shield block: the hook API does not expose the block event.
- Magical adds its share as damage instead of adding vanilla's magic damage
  tags to the arrow's damage source, so armour is not literally bypassed.
- Telepathy and Festive act on the killer's main hand; a projectile kill reads
  the bow still in hand, not the arrow's own affixes.
- A boss roll is per natural spawn, not per dimension tier: the frontier /
  ascent / summit / pinnacle chances are collapsed to 2 % for this slice.
- Invader names are generated server-side from translated components; there
  is no name-preserving save/load beyond the `CustomName` NBT.

### Missing

- The attributes only exist on affixed gear: there is no attribute screen, no
  per-player attribute page and no way to put them on an arbitrary item.
- Overheal is modelled as `minecraft:absorption`; the original keeps a
  separate overheal pool with its own decay and display.
- Armor pierce and shred lower the damage reduction computed by this mod, not
  vanilla's armor points, so the armour bar does not move.
- Dodge is rolled server-side per hit; there is no client animation or sound.
- The mining-speed attribute is applied through `break_speed` only, so it does
  not show on the held tool's vanilla `mining_efficiency` line.

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
  `python tools/pack.py` writes `target/apotheosis.hmod` and
  `target/apotheosis.hnmod`.
- Native: the crate is checked against the pinned Horizon commit on the build
  VPS with `tools/vps/sync.sh full` (see `tools/vps/check.sh` and
  `tools/vps/verify.sh`); it builds as a trait mod with
  `horizon-launcher native-build`.

### Build state (2026-11-22)

- v0.4.0 compiles against the pinned commit `a85af177` (`cargo check -p
  mod-apotheosis`, no warnings).
  **34 unit tests pass**, and `cargo check -p horizon-native-game
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
