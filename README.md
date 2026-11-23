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
> art. The code is this port's own, written against Horizon's native hook API.
> The gem catalogue and the other game values come from the mod's data files
> (MIT, values only). The Java textures, models and lang files are All Rights
> Reserved: they never enter Git, `tools/import_java_assets.py` copies them into
> the git-ignored `content-java/` layer at pack time, and the few textures this
> port draws itself (gem dust) are generated 16x16 pixel art.

## Done

- **Content mod `.hmod`** (`side = "both"`, id `apotheosis`) with creative
  entries for the gem item, gem dust, the enchanting shelves and the rarity
  salvage materials. All of them are craftable or salvageable in survival (see
  *Recipes* below).
- **The real Apotheosis gems** — one item, `apotheosis:gem`, carrying its gem
  type in `minecraft:custom_data` (`apotheosis:gem`) and its purity in
  `apotheosis:gem_purity`, exactly as the mod stores them. The catalogue is
  generated from Apotheosis 8.9.0's `data/apotheosis/gems/**`
  (`tools/gen_java_gems.py` → `java_gems.rs`): 21 gems — the Twilight Forest
  ones are left out, that mod is not installed — with their weights, purity
  floors, dimension constraints and per-purity bonus values. A
  `minecraft:select` model on `minecraft:custom_model_data` picks the imported
  Java texture of the type (`apotheosis:item/gem_<name>`), and the name comes
  from the Java translation key `item.apotheosis.gem.<type>`.
- **Six tiers**, `Common`, `Uncommon`, `Rare`, `Epic`, `Mythic`, `Ancient`
  (`Tier` in `native/apotheosis/api/src/lib.rs`): the tier decides how many
  affixes an item rolls (0, 1, 2, 3, 4, 5) and how strong they are (×1 to ×4).
  Vanilla's `minecraft:rarity` component only names four of them; mythic and
  ancient are shown in the tooltip and in an affixed item's lore.
- **A gem does nothing until it is socketed**, as in Apotheosis: no off-hand
  bonus, no default attribute. Its tooltip (`tooltip`, API 12) lists the bonus
  it gives for each gear class at its purity, and a socketed gem's line appears
  on the piece that holds it.
- **Bonus types this port can apply**: `attribute` and `multi_attribute`
  (each modifier with a supported attribute) and vanilla `enchantment`
  (`existing_only` when the Java mode is not `global`). Every other type —
  `durability`, `damage_reduction`, `mob_effect`, `all_stats`, `drop_transform`,
  `radial`, `omnetic`, `mageslayer`, `bloody_arrow`, `leech_block`,
  `anti_gravity_arrow`, and modded enchantments — is **recorded, not
  substituted**, in the generated `unsupported`/`unsupported_attributes` fields
  of `java_gems.rs`. Unsupported attributes are `prot_pierce`,
  `current_hp_damage`, `ghost_health`, `generic.gravity`, `generic.scale`,
  `generic.step_height`, `neoforge:swim_speed` and `player.*_interaction_range`.
  The Apothic `add_multiplied_base`/`add_multiplied_total` operations are
  carried as ordinary modifiers.
- **Recipes for everything** (vanilla 26.3 data-pack format under
  `data/apotheosis/recipe/`):
  - **Gem dust** (`gem_dust`, a stack of 64) from one amethyst shard → 2 dust,
    and 1 gem (via the `apotheosis:gems` item tag) → 3 dust.
  - **Gem cutting** (`gem_cutting`) and the gem blocks do not exist in this
    port: the Java gems are cut at the gem cutting table on Horizon, not
    crafted. The dust recipe reads the `apotheosis:gems` tag, which now holds
    the single `apotheosis:gem` item.
- **Affixes on looted gear** (`loot_drops`, API 75): gear a mob drops (a
  zombie's sword, a skeleton's bow…) is rolled into a rarity and carries real
  `minecraft:attribute_modifiers`, a lore that names each affix, and the
  `minecraft:rarity` that colours its name. 26 affixes (see *Apothic
  Attributes et affixes* below), each rolled only onto gear it can affect.
  Distinct affixes per item, scaled by tier; an affixed copy keeps the item's
  own base modifiers in front of the rolled ones.
- **Gems from mobs**: `gems::roll` picks a gem by dimension and Java weight,
  then clamps the rolled purity to the gem's `min_purity`. Only monsters
  (`apotheosis:is_monster`) can drop one: 4% for a player kill and 0.5% for a
  kill with no player, as Java's two GLMs do. A *wither*, *ender dragon*,
  *warden* or *elder guardian* always drops one, with the better purity table.
- **Gems from chests** (`loot_table_load`, API 75): `minecraft:chests/*` tables
  have Java's 25% chance, other domains' chests and Twilight Forest structures
  20%, to roll the `apotheosis:chest_gems` pool. One generic entry's function
  draws the gem type by Java weight in the table's dimension and rolls its
  purity, so End and Nether chests give their own gems.
- **Extra drops**: a boss always drops an affixed diamond piece (or a bow); an
  ordinary monster killed by a player has a 4% chance to drop an affixed iron
  piece. Passive entities drop neither gems nor affixed gear.
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
  the affixes' entries, so the game applies it when the piece is worn. A gem
  that is not socketed has no effect, as in Apotheosis.
- **Purity** Cracked, Chipped, Flawed, Normal, Flawless, Perfect, each gem's
  bonus value read at the purity's index from its Java data (no invented
  multiplier), rolled where the gem drops (`loot_drops`) from the Java purity
  weights and above the gem's `min_purity`.
  Shown in the gem's tooltip. A gem in the main hand with gem dust in the off
  hand, right-clicked, is polished one step up at the Java
  `apotheosis:purity_upgrade` cost for the purity it reaches: 3 dust and
  scrap/fabric/crystal to chip it, up to 9 dust and crystal/sands/pearl for
  Flawless. Java has no perfect recipe, so a gem stops at Flawless.
- **Reforging an affixed piece**: `/apoth reforge` on an affixed piece held in
  the main hand rerolls its rarity and affixes for 3 levels (a command-only
  port cost), keeping the piece and the gems in its sockets; the work is done
  on the next inventory tick, so the stack the command saw is the stack that is
  rewritten. The tables charge the five Java `recipe/reforging` rows instead:
  levels 2/5/15/30/50, rarity material 1/2/2/2/3 and sigils of rebirth
  0/1/2/4/5 for Common→Mythic, all taken from the inventory before the reroll
  and given back in full if any part is short or the hand changes. Ancient is
  this port's extension and uses the mythic row.
- **Salvaging**: `/apoth salvage` on an affixed piece breaks it into 1–4 of its
  rarity's material (no gem dust: the Java salvaging recipes name the material
  alone) and gives back every socketed gem at the purity it had. The salvaging
  table also breaks a gem into the Java gem-dust range for its purity
  (1–2 cracked, 1–3 chipped, 2–4 flawed, 3–5 normal, 4–7 flawless, 5–10
  perfect).
- **Five sigils** (`sigil_of_socketing`, `sigil_of_withdrawal`,
  `sigil_of_rebirth`, `sigil_of_enhancement`, `sigil_of_unnaming`), each a
  stackable item with its own 16×16 wax-seal texture, en_us/fr_fr name and
  description line, and the Java shaped recipe in gem dust and gem-fused slate
  (with amethyst and gunpowder, ender pearls and a lava bucket, flint,
  godforged pearl), and a place in the creative tab. A sigil is used with the
  affixed piece in the **off hand** and the sigil in the **main hand**, right
  click: socketing adds one socket up to the maximum, withdrawal takes every
  socketed gem back out at the purity it had, rebirth rerolls the piece's
  rarity and affixes (gems kept), enhancement moves the rarity one step up
  (Common→…→Ancient) and rerolls at the new tier, and unnaming removes the
  name the affix roll wrote — never a name the player typed, which leaves the
  sigil unspent. The sigil is used up on the next inventory tick, so a hand
  that changed costs nothing.
- **Four working tables as blocks**, with Apotheosis' ids
  (`gem_cutting_table`, `salvaging_table`, `reforging_table`,
  `simple_reforging_table`): a real block declaration (oak planks template for
  the three old ones, smooth stone for the simple table, own top and side
  textures, `cube_bottom_top` model), item, blockstate, block loot table,
  pickaxe tag, the Java shaped recipe, and the creative tab. Right click with
  the piece in hand (`use_item_on`, API 13): the reforging table charges the
  Java row for every rarity, the simple table only Common/Uncommon/Rare, the
  salvaging table what `/apoth salvage` does (or gem dust for a held gem), and
  the gem cutting table spends the Java purity-upgrade cost (gem dust and
  rarity materials) from the inventory to raise the held gem's purity one step
  (`Host::take_items`, everything refunded if any ingredient is short). The
  click is always claimed, so a misclick shows the table's hint instead of
  opening something else.
- **The four new Java items**: `gem_fused_slate` (8 from deepslate and gem
  dust), the iron, gold and diamond smithing templates (7 diamonds around a
  template, 7 gold, 7 iron), `sigil_of_unnaming` and
  `simple_reforging_table` are declared, craftable and in the creative tab.
  Their Java textures and models come in through the git-ignored
  `content-java/` layer; without it a vanilla placeholder texture is used.
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
- **`sigil_of_malice` and `sigil_of_supremacy` are not ported**: both need an
  overcharged-affix state this port does not have, and the augmenting table
  they feed has no per-affix level.
- **Reforging has no result-slot choice**: Java's reforging menu shows three
  results priced at 1×, 2× and 3× the recipe row; a right click on the table
  charges the 1× row only, since there is no grid to pick a slot in.
- **Plain vanilla gear is not salvaged**: Java also has `recipe/salvaging/other`
  rows for ordinary tools and armour; this port only salvages affixed pieces
  and gems.
- **The 28 sized smithing upgrades are not recipes here**: Java's
  `apotheosis:sized_upgrade_recipe` consumes four ingots or gems per template,
  and the vanilla smithing format cannot express that; shipping it as a
  one-ingot recipe would change what the Java pack costs. The three templates
  are craftable, but not applied through a smithing table in this port.
- **A socket, once added, is permanent**: no sigil takes one away, only the
  gems can be withdrawn.
- **Gear affixed before this slice** carries no `apotheosis:tier`, so it can be
  neither reforged nor salvaged; only pieces rolled (or dropped) after the
  update can.
- **Purity only scales a socketed gem.** An unsocketed gem's off-hand bonus is
  its flat value; there is no purity-scaled off-hand bonus.

## Apothic Attributes et affixes

### Done

- **Eighteen attributes** (`native/apotheosis/api/src/attributes.rs`), stored
  on an affixed stack in `minecraft:custom_data` under `apotheosis:attributes`
  and read back by the hooks: crit chance and crit damage, life steal,
  overheal, dodge chance, armor pierce, armor shred, fire and cold damage,
  experience gained, healing received, arrow damage and velocity, bow draw
  speed, mining speed, current-health damage, protection shred and protection
  pierce.
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
  - *Current-health damage, shred and pierce*: giant slaying adds its share of
    the victim's **current** health as physical damage. Protection shred and
    pierce lower a player victim's real protection points (4 % each, 20 points
    at most) before the hit's reduction is computed, so the mod's damage
    reduction and the vanilla protection line agree.
  - *Mining*: `break_speed` scales the break speed of the held tool.
  - *Experience*: `experience_orb_pickup` adds the experience-gained share to
    the orb's value.
- **Java's affix catalogue is the source**: `tools/gen_java_affixes.py` reads
  the 99 affixes of Apotheosis 8.9.0 and generates
  `native/apotheosis/api/src/java_affixes.rs` (id, categories, rarities and
  per-rarity ranges, plus Java's translation keys). The generator is
  idempotent and only merges its language keys; it copies no asset and no Java
  code. `affixes.rs` turns that table into the roll.
- **The item name is Java's prefix + item + suffix**, composed from
  `misc.apotheosis.affix_name.*` and `affix.apotheosis:<path>` keys, coloured
  with the rarity's Java colour; a player's own custom name is left alone.
- **Tooltips are one `+X Attribute` line per affix**, in the rolled rarity's
  colour (common `#808080`, uncommon `#33FF33`, rare `#5555FF`, epic
  `#BB00BB`, mythic `#ED7014`), translated in en_us and fr_fr. The separate
  attribute block is only a fallback for old pieces without affix lore.
- **Affixes are filtered by Java categories**: melee weapon and trident, bow,
  breaker, shears, shield, and each armour piece. The trident is both melee and
  its own category; shears have their own class. An item with no matching
  affix rolls none rather than falling back to the whole pool.
- **Base modifiers are merged**: `modify_default_components` records each
  item's own `minecraft:attribute_modifiers` before anything edits them, and
  `apply_affixes` writes them ahead of the rolled ones, so an affixed diamond
  sword keeps its innate attack damage.
- **Rarity counts follow Java**: common 1 affix, uncommon 2, rare 2, epic 3,
  mythic 4. `Ancient` is this port's extension and reuses mythic's ranges with
  five affixes; it is documented rather than invented as a Java rarity.
- **Not rolled** (kept in the generated catalogue with their Java ids, kinds
  and values): the four attributes this port has no effect for —
  `neoforge:swim_speed` (aquatic), `cooldown_reduction` (timekeeping),
  `neoforge:creative_flight` (unbound) and `apothic_attributes:elytra_flight`
  (winged) — plus every `attribute_toggle`, `stoneforming`, `radial`,
  `omnetic`, `catalyzing`, `psychic`, `enlightened`, `magical` and `spectral`
  entry, and the `mob_effect` affixes whose target is an arrow hit the port
  has no host for (bursting, spelunkers, swift, bloodletting, caustic,
  omniscient, sophisticated, acidic, grievous, shulkers, devilish). **69 of
  the 99 Java affixes are rolled**; the other 30 are catalogue-only.

### Effets d'affixes et boss Apothic Invader

#### Done

- **Twenty-nine effect affixes** (`native/apotheosis/api/src/affix_effects.rs`)
  on top of the forty attribute ones, using Java's own kinds: fifteen
  `mob_effect` (blinding, bolstering, nimble, revitalizing, elusive,
  weakening, ensnaring, blighted, deathbound, ivy_laced, fleeting, galvanizing,
  reinforcing, venomous, withering), six `damage_reduction` (blast forged,
  blockading, deflective, feathery, grounded, runed), two `enchantment`
  (prosperous: Fortune on a tool, Looting on a bow) and six one-off kinds
  (cleaving, executing, festive, thunderstruck, retreating, telepathic), plus
  the port's own Durable.
- **Their numbers are Java's rolls**, not approximations: each affix keeps its
  per-rarity `duration`, `amplifier`, `chance`, `targets`, `cooldown` (ticks)
  and `rolls` as generated from the 8.9.0 data, and `supported` is true only
  where a hook can actually pay the entry.
- **Their item classes match Java's categories**: a file with no category list
  takes its folder's (`shield/retreating`, `generic/telepathic`), and the
  trident-only arrow targets (ensnaring, blighted, deathbound, ivy laced) are
  paid because a trident hit reaches `attack_target`.
- **Their payouts run in the real hooks**: poison/wither/weakness/slowness/
  blindness and the other mob effects at their rolled duration and amplifier,
  execution under the rolled health share, a cleave to nearby foes, real
  lightning, a knockback away from the attacker, less durability wear, drops
  straight to the inventory, doubled gear from a kill, and the two
  enchantments written as real item enchantment levels. Mob-effect payouts
  respect Java's per-affix cooldown before firing again.
- **Damage reduction is applied to every damage route**: the six armour affixes
  are read per damage class (explosion, physical, projectile, fall, lightning,
  magic) and cached per entity, so the reduction holds for hits v1 and v2 see
  alike. Current-health damage lands as physical damage after the blow.
- They are **extra affixes**, never replacements: `apply_affixes` still rolls
  the same number of attribute modifiers and lore lines, then adds the effect
  affixes Java's rarity guarantees (rare one `basic_effect`, epic one plus one
  `ability`, mythic/ancient two plus one), drawn from that class's supported
  pool and never twice the same. The rolls are stored in
  `minecraft:custom_data` under `apotheosis:effects` and read back per hit.
  Tooltips show the effect line in green, translated in en_us and fr_fr.
  The enchantment levels an effect affix adds are recorded under
  `apotheosis:effect_enchants`, so a reroll takes back only those and leaves
  the item's own enchantments alone.
- **A reforge only replaces the port's own work**: attribute values it wrote
  are recorded under `apotheosis:affix_attrs` and subtracted before the new
  ones, and its lore lines (`apotheosis.affix.*`) and modifiers (Java affix
  ids) are the only ones it removes, so another mod's lore, modifiers or a
  player's anvil-renamed name survive. A kept name is recoloured with the
  tier's colour. Vanilla's `minecraft:rarity` stops at `epic`, so the real
  Java rarity is also written to `apotheosis:rarity`.
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
- Venomous, Withering, Galvanizing, Reinforcing and Retreating fire on any hit
  taken with a shield in the off hand, not only on an actual shield block: the
  hook API does not expose the block event.
- An arrow hit has no weapon stack of its own, so `arrow_target`/`arrow_self`
  `mob_effect` affixes (bursting, spelunkers, swift, bloodletting, caustic,
  omniscient, sophisticated, acidic, grievous, shulkers, devilish) stay
  catalogue-only; only a trident's arrow target is paid.
- Effects on the victim's gear are read for players only: a mob's armour lives
  in its NBT, not in `player_equipment`.
- Protection shred and pierce move a **player** victim's protection points;
  a mob's protection is not modelled.
- Thunderstruck's Java data carries only the strike damage, so this port adds
  its own one-in-five chance of the lightning.
- Telepathy and Festive act on the killer's main hand; a projectile kill reads
  the bow still in hand, not the arrow's own affixes.
- Durable has no Java data entry: it is a port extension at 15 % per tier,
  capped at 50 %.
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
  vanilla's armor points, so the armour bar does not move. Protection shred and
  pierce do move a player's protection points; a mob victim has none to move.
- Dodge is rolled server-side per hit; there is no client animation or sound.
- The mining-speed attribute is applied through `break_speed` only, so it does
  not show on the held tool's vanilla `mining_efficiency` line.

## Missing

- **Affixes from chest loot tables** — chest tables can supply gems, but their
  gear remains un-affixed; `loot_table_load` cannot roll the item components
  the later loot generation creates.
- **Natural ore generation** for gems: gems come from mobs, bosses, recipes
  and reforge only.
- Affix names, lore and command answers are translated, but an affixed item
  keeps its vanilla name: the tier shows as a lore line, not as a prefix.
- No client-side drawing beyond the data-driven items and the tooltip lines.

## Apothic Enchanting

### Done

- **Twenty-six shelf blocks** on `minecraft:bookshelf` (and, for the three
  newest, `minecraft:stone`) templates, with original
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
  | geode_shelf | 5 | 60 | -15 | 0 | 0 |
  | treasure_shelf | 5 | 70 | -15 | -15 | 0 |
  | filtering_shelf | 0 | 30 | 0 | 0 | 0 |

  The three newer shelves keep their Java ids (`apotheosis:geode_shelf`,
  `apotheosis:treasure_shelf`, `apotheosis:filtering_shelf`), their stone
  template and `minecraft:stone` sound, hardness 1.75, the pickaxe tag, their
  Java shaped recipes (`geode_shelf`: calcite corners around a bookshelf with
  a budding amethyst, `treasure_shelf`: gold and diamond above deepshelf over
  emerald, `filtering_shelf`: prismarine bricks around an infused seashelf) and
  are `enchantment_power_provider`s like the wooden ones.

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
  spyglass, emerald, melon slice or polished andesite. Only the Ender Library
  depends on the mod's own infusion table (see below); every other recipe is a
  vanilla shaped one.
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
- **The Ender Library** (`apotheosis:ender_library`), built the way Java builds
  it: an infusion of the ordinary library (`recipe/infusion/ender_library`,
  Eterna 100 / Quanta 45 / Arcana 100 at most 100/50/100). It is the same
  block entity and the same right-click gestures as the library below, with a
  level cap of 31 instead of 30 (Apothic Enchanting builds it with
  `new EnchLibraryBlock(EnderLibraryTile::new, 31)`); the ordinary library's
  Java cap of 16 levels is kept at this port's one-point-per-level 30, so the
  Ender one is exactly one level above it. Own texture, names, creative-tab
  entry, self-drop loot table and axe tag. Java gets the block by infusing a
  placed library; a world where the infusion table is out of reach can craft
  the same upgrade without the table (an ender chest between two books over a
  bookshelf), and the infusion stays the cheaper Eterna-free path.
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
- **Infusion at the Apothic Enchanting Table** (`apotheosis:apothic_enchanting_table`,
  `use_item_on` API 13). API 415 cannot open or drive a block's menu, so the
  table does on a right click what the Java menu's third slot does: the item in
  hand is matched against the mod's `recipe/infusion` entries at the stats the
  surrounding shelves give (`findMatch`, hardest Eterna requirement first), and
  the click applies it at once — one item in, the result in the hand, the same
  experience levels Java charges (the table's cost clamped to the recipe's
  Eterna requirement, at least 3, free in creative). Twelve infusions are
  ported, the ones whose input and result both exist here: budding amethyst,
  golden carrot, deepshelf, the four echo shards, the Ender Library, the two
  infused shelves, the three experience-bottle sizes, and the four-tome
  upgrades (scrap → superior, superior → extraction). Recipes needing an item
  this port does not carry (infused breath, ender leads, the inert trident,
  magic discs) are left out. The block uses the `minecraft:obsidian` template
  so no vanilla screen opens over it, has its own texture, names and creative
  entry, the pickaxe tag, a self-drop loot table and a recipe (obsidian around
  an enchanting table). A click that cannot be infused says what the shelves
  give and what the recipe needs instead of silently doing nothing.

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
- **The filtering shelf filters nothing.** Java gives it 0.5 Eterna and one
  Arcana per enchanted book in its six slots and removes those books'
  enchantments from the table's offers. API 415 has no block-entity inventory
  hook (an item cannot be inserted into a shelf, and its contents cannot be
  read), so the block exists with its recipe, model and names, counts as an
  `enchantment_power_provider` like an empty Java shelf, and contributes no
  stats: the values an empty filtering shelf has in Java. Storing books and
  filtering the offers wait for an inventory hook above API 415.
- **No rectifier and no enchanting screen.** No rectifier block is added, and
  the infusion table and both libraries work by right clicks rather than by
  menus: API 415's menu hooks only reach a LAN host's guests. The infusion
  table also cannot show the Java preview of which recipe the shelves allow;
  the requirements are reported in chat when a click cannot be infused.
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

## Known issues (review of 2026-11-22, not fixed yet)

An independent review found these item duplication or loss risks in the
sockets, sigils, reforge and salvage code. They are **not fixed** in 0.5.0;
avoid these actions on a shared server until they are:

- `sockets.rs:924`: consume_seal() retire le sigil de la pile qui occupe la case sélectionnée au moment du tick, pas de celle qui a été cliquée : use_seal() (l.837-868) ne mémorise ni la case ni l'objet, et add_socket/withdraw/rebirth/enhance ne vérifient même pas que `main` est encore un sigil avant d'appeler consume_seal (l.947, 978, 991, 1025).
- `sockets.rs:1170`: polish() ne vérifie pas que la pile de la main (relue au tick, l.810) est encore une gemme : il applique set_purity() à n'importe quel objet et retire 4 gem dust de la main gauche. Le contrôle de use_item (l.751-757) porte sur la pile du clic, perdue entre-temps.
- `sockets.rs:976`: withdraw() retire les gemmes des chambres et les donne avec `let _ = host.give_stack(...)` : l'échec (inventaire plein) est ignoré, alors que la pièce a déjà été réécrite sans ses chambres (l.968-971) et que le sigil est consommé (l.978).
- `sockets.rs:1238`: salvage() rend les gemmes serties avec `let _ = host.give_stack(player, gem)` : une gemme qui ne rentre pas est perdue, et seul le matériau est contrôlé (`if given < count`, l.1249). La pièce est détruite avant (l.1229).
- `sockets.rs:819`: Les actions différées relisent la pièce de la main gauche au tick (l.819)` et l'appliquent sans vérifier que c'est celle validée au clic (use_seal l.847-857, use_item l.743-756) : sertir, polir, ajouter une chambre, améliorer ou reroll agissent sur la pièce qui s'y trouve alors, et le consommable (gemme, poudre, sigil) est débité quand même.
- `lib.rs:1259`: La reforge d'une pièce affixée débite les 3 niveaux (l.1251) puis pose juste `Action::Reforge` ; sockets::reforge (l.1192-1201) ne rend les niveaux que si la pile tenue n'est pas reforgable. Une autre pièce affixée tenue au tick est donc rerollée à la place de celle payée.
- `lib.rs:1199`: `/apoth salvage` valide la pièce au moment de la commande (begin_salvage, sockets.rs:763-776, contrôle `has_affixes` sur held_stack) puis salvage() (sockets.rs:1216) détruit la pile tenue au tick suivant, sans revérifier que c'est la même.

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

### Build state (2026-11-23)

- v0.5.0 compiles against the pinned commit `a85af177`:
  `bash horizon-work/check.sh apoitems … test` ends with
  `RESULT check=0 test=0`, and **126 unit tests pass**, including the new ones
  for the five Java reforge costs, the simple table's rarity limit, the six gem
  salvage ranges, and the unnaming sigil keeping a player's name. The remaining
  warnings are pre-existing unused helpers (`gem_name_key`, `amount_text`,
  `Purity::multiplier`, a few spawner and affix helpers).
- The content mod has one gem item with 21 Java types and a `minecraft:select`
  model, gem dust, five rarity salvage materials, five sigils, and four
  working tables. It also declares `gem_fused_slate`, the iron, gold and
  diamond smithing templates and `simple_reforging_table`. Mob drops and all
  vanilla chest loot tables provide survival gem routes: type by Java weight
  and dimension, purity above the gem's floor, Flawless and Perfect
  boss-only. All 281 content JSON files parse, and `python tools/pack.py`
  rebuilds `target/apotheosis.hmod` (535 entries, including the new recipes
  and item/block assets) and `target/apotheosis.hnmod`.
- The Java assets were imported locally into the git-ignored `content-java/`
  layer: 28 blockstates, 48 items, 107 models, 115 textures; the new ids have
  Java models (`tools/java_assets_map.md`).

## Licence

Apotheosis is MIT (Shadows_of_Fire). This port's own code and assets are MIT
as well; see `NOTICE.md`.
