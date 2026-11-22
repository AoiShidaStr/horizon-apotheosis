# Apotheosis for Horizon

A clean-room Horizon port of [Apotheosis](https://github.com/Shadows-of-Fire/Apotheosis)
by Shadows_of_Fire (MIT) and its 1.21 companions **Apothic Enchanting**,
**Apothic Attributes** and **Apothic Spawners**, as the Saison 1 server ran them
on NeoForge 1.21.1. Version 0.5.0 targets Horizon a85af177 (native hook API 415).

| Part | Section |
| --- | --- |
| Gems, rarities, affixes from loot, `/apoth` | *Done* / *Missing* below |
| Sockets, gem purity, sigils, reforging, salvaging, gem cutting | *Gemmes, sockets, reforge et recyclage* |
| Apothic attributes, 40 affixes, affix effects, Apothic Invader bosses | *Apothic Attributes et affixes* |
| Enchanting shelves and Eterna, tomes, Prismatic Web, Enchantment Library | *Apothic Enchanting* |
| Silk-touch spawners and Apothic spawner modifiers | *Apothic Spawners* |

In-game test plan (French): [`TEST-EN-JEU.md`](TEST-EN-JEU.md). Nothing in this
port has been played in game yet: every part is type-checked against Horizon
a85af177 and covered by unit tests only.

> **Clean room.** Nothing here is copied from Apotheosis: neither code nor
> art. The behaviour (tiers, gems, affixes, boss-only high tiers) comes from the
> mod's public description; the code is this port's own, written against Horizon's
> native hook API, and the textures are original generated 16x16 pixel art. Apotheosis' MIT
> licence would allow reuse; this slice did not need it.

## Done

- **Content mod `.hmod`** (`side = "both"`, id `apotheosis`) with creative
  entries for twelve gems, gem dust, twelve decorative gem blocks, twenty-three
  enchanting shelves and five rarity salvage materials. All of them are
  craftable or salvageable in survival (see *Recipes* below).

- **Content mod `.hmod`** (`side = "both"`, id `apotheosis`) with **65 creative
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
  pickaxe tag. A Ruby Block is storage, light and decoration; the working
  tables are blocks of their own.
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
    shred and then by pierce). It also rolls the victim's dodge **once** and
    cancels the hit there when it succeeds, so the 95 % of hits that land are
    not read as dodged; `living_incoming_damage` reuses that decision and only
    rolls for damage routes v2 does not see. Dodge is the only case that
    cancels the hit (flow `Cancel`), so a dodged arrow is not a free hit.
  - *Life steal and overheal*: the damage actually dealt heals the attacker by
    the life-steal share, above max health through absorption for overheal;
    a dodged hit pays neither.
  - *Bow*: `arrow_loose` queues the shot's arrow-damage multiplier, and
    `projectile_launch` binds it to the arrow it becomes; only that arrow's own
    `arrow` damage spends it, never a sword hit in the next five seconds. Draw
    speed scales the charge needed to loose, so a faster bow fires sooner.
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

- **Affixes from chest loot tables** — chest tables can supply gems, but their
  gear remains un-affixed; `loot_table_load` cannot roll the item components
  the later loot generation creates.
- **The enchanting library** and its tomes (Scrap, Extraction) are not
  implemented. Enchanting stats themselves are ported (see *Apothic Enchanting*
  below), with the limits listed there; configurable spawners are ported (see
  *Apothic Spawners*).

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
- **The shelves also feed the player's own table.** Each shelf with positive
  Eterna is in `minecraft:enchantment_power_provider`, so in single player,
  where Horizon does not dispatch API 291, it counts like a bookshelf for
  vanilla power and shows the table's particles. The negative-Eterna shelves
  (bee, melon and stone) are deliberately left out: the game would otherwise
  treat them as bookshelves and raise the offers they are meant to lower.
  Vanilla still caps that path at 15 providers (level 30).
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
- **The Enchantment Library** (`block_entity` / `block_entity_ticks` /
  `block_entity_tick` API 15–17, `right_click_block` API 21). The block stores
  enchantments as points — one point per level, capped at level 30 per
  enchantment — in the saved compound of its own block entity, so the points
  survive a restart. Right-clicking with an enchanted book absorbs it (its
  levels are added, capped); right-clicking empty-handed cycles through the
  offers it can pay for, from level 1 up; right-clicking with a plain
  `minecraft:book` takes the shown offer, spends its points and gives the
  enchanted book. It is a real block with an original 16x16 texture, en_us and
  fr_fr names, a creative-tab entry, a self-drop loot table, the axe tag and a
  vanilla recipe (eight books around a bookshelf). No menu and no item slot is
  involved — every click is applied at once on the server thread, so rapid
  clicks, two players and a broken block cannot duplicate a book.

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
- **Infusion and rectifiers.** No infusion altar or rectifier is added; the
  recipes stop at vanilla crafting. The tomes are used by a right click rather
  than in an anvil, since API 415 has no anvil hook.
- **The Enchantment Library has no screen and drops no contents.** API 415's
  menu hooks only reach a LAN host's guests and vanilla-menu clicks cannot be
  intercepted, so the library is driven by right clicks and shows offers in
  chat instead of a slot grid; a broken library keeps its block item but its
  stored points are lost (the block-break hook cannot read a block entity's
  saved compound at this API).

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
  **All unit tests pass** (see each section), and `cargo check -p horizon-native-game

  mod-apotheosis`, no warnings).
  **34 unit tests pass**, and `cargo check -p horizon-native-game

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
