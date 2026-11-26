//! Apotheosis for Horizon, the native half.
//!
//! Clean-room reimplementation from Apotheosis' public behaviour (rarity tiers
//! above vanilla's, affixes on gear, higher tiers only from bosses). No
//! Apotheosis code or asset is used: see `../../README.md` and `NOTICE.md`.
//!
//! The hooks below are the real game's trait API (plus `sockets`' own
//! `loot_function` for the chest gems; the tables open menus, `menus`):
//! Seven hooks, all on the real game's trait API:
//! - `modify_default_components`: a gem keeps its own item definition; its
//!   bonuses are written onto the gear it is socketed into, as Apotheosis does.
//! - `tooltip`: a gem's purity, the bonuses its Java data gives the gear it is
//!   socketed into, and how to socket it.
//! - `loot_table_load`: vanilla chest loot tables gain a small chance to
//!   contain a gem, so gems have a survival source beyond mob drops.
//! - `loot_drops`: gear a mob drops is rolled into a rarity with affixes (real
//!   attribute modifiers plus lore), and a kill adds a gem of the level's
//!   dimension, at its Java weight and purity floor.
//! - `register_commands` / `perform_command`: `/apoth salvage`. Reforging is
//!   the reforging tables' own menus (`menus`), as in Java.
//! - `prepare_item_enchant`: Apothic Eterna from nearby shelf blocks raises the
//!   three enchanting offers; shelves are read through Horizon's server Host.
//! - `use_item`: scrapping, superior scrapping and extraction tomes move
//!   enchantments between the two hands, and the Prismatic Cobweb removes
//!   curses; Horizon API 415 has no anvil hook, so a right click uses them.
//!
//! `attributes` holds the Apothic Attributes half: the combat, mining, bow and
//! experience attributes affixes carry, filtered by what a piece of gear is.
//! `gems` makes the Java gem catalogue (`java_gems`, generated) into items and
//! resolves what a gem gives a piece at its purity.
//! `affix_effects` holds the effect affixes (poison, execution, cleaving,
//! spectral arrows, telepathy...) and the Apothic Invader boss.
/// A tooltip line's colour as Horizon draws it: opaque ABGR (`hud::rgba`), from
/// the 0xRRGGBB this crate writes. A bare 0xRRGGBB has no alpha and draws
/// nothing.
pub(crate) const fn tip_colour(rgb: u32) -> u32 {
    let (r, g, b) = ((rgb >> 16) & 0xFF, (rgb >> 8) & 0xFF, rgb & 0xFF);
    0xFF00_0000 | b << 16 | g << 8 | r
}

#[cfg(test)]
mod tip_colour_tests {
    #[test]
    fn tooltip_colours_are_opaque_abgr() {
        assert_eq!(super::tip_colour(0x0AFF0A), 0xFF0A_FF0A);
        assert_eq!(super::tip_colour(0xFFAA00), 0xFF00_AAFF);
        assert_eq!(super::tip_colour(0x55_55FF), 0xFFFF_5555);
    }
}

mod affix_effects;
mod affixes;
mod attributes;
mod enchanting;
mod gems;
mod java_affixes;
mod java_gems;
mod monsters;

use horizon_client::mod_api::{
    ArrowLooseEvent, BreakSpeedEvent, EntityJoinLevelEvent, ExperienceOrbPickupEvent, Flow, Hooks,
    Host, ItemDamagedEvent, ItemStack, LeftClickBlockEvent, LivingEquipmentChangeEvent,
    LivingHealEvent, LivingIncomingDamageEvent, LivingIncomingDamageV2Event, LootDropsEvent,
    MobSpawnedEvent, ModifyDefaultComponentsEvent, NativeCommand, OnEquipEvent, PerformCommandEvent,
    PlaceBlockEvent, PlayerTickEvent, ProjectileLaunchEvent, RegisterCommandsEvent,
    RightClickBlockEvent, Tag, Text, TickEvent, TooltipEvent, UseItemOnEvent, serde_json,
};

mod spawners;
use std::sync::atomic::{AtomicU64, Ordering};

mod menus;
mod sockets;

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Apotheosis)
}

struct Apotheosis;

/// The tiers this slice adds. Vanilla's `minecraft:rarity` component names four
/// (`common`..`epic`); the two above it are this port's own. Java's rarity
/// colours and per-rarity affix counts drive the affix roll; `Ancient` is this
/// port's extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Common,
    Uncommon,
    Rare,
    Epic,
    Mythic,
    Ancient,
}

impl Tier {
    /// How many affixes a piece of gear of this tier rolls: Java's
    /// common 1, uncommon 2, rare 2, epic 3, mythic 4. Ancient is this port's
    /// extension and rolls one more than mythic.
    pub fn affixes(self) -> usize {
        match self {
            Tier::Common => 1,
            Tier::Uncommon => 2,
            Tier::Rare => 2,
            Tier::Epic => 3,
            Tier::Mythic => 4,
            Tier::Ancient => 5,
        }
    }

    /// What an affix's base amount is multiplied by. Java affixes use a range
    /// per rarity instead, so only the effect affixes still use this.
    pub fn scale(self) -> f64 {
        match self {
            Tier::Common => 1.0,
            Tier::Uncommon => 1.0,
            Tier::Rare => 1.5,
            Tier::Epic => 2.0,
            Tier::Mythic => 3.0,
            Tier::Ancient => 4.0,
        }
    }

    /// Java's rarity index, used by the affix table. Ancient reuses mythic.
    pub fn rarity_index(self) -> usize {
        match self {
            Tier::Common => 0,
            Tier::Uncommon => 1,
            Tier::Rare => 2,
            Tier::Epic => 3,
            Tier::Mythic | Tier::Ancient => 4,
        }
    }

    /// `minecraft:rarity`, the component that colours the item's name. The two
    /// tiers above vanilla's take epic's colour; their name says the tier.
    pub fn vanilla_rarity(self) -> &'static str {
        match self {
            Tier::Common => "common",
            Tier::Uncommon => "uncommon",
            Tier::Rare => "rare",
            Tier::Epic | Tier::Mythic | Tier::Ancient => "epic",
        }
    }

    /// The rarity's Java id: the five `data/apotheosis/rarities/*.json` names,
    /// plus this port's `ancient` extension. Vanilla's component cannot spell
    /// the last two, so it is also written to `apotheosis:rarity`.
    pub fn id(self) -> &'static str {
        match self {
            Tier::Common => "common",
            Tier::Uncommon => "uncommon",
            Tier::Rare => "rare",
            Tier::Epic => "epic",
            Tier::Mythic => "mythic",
            Tier::Ancient => "ancient",
        }
    }

    /// The tier's name, as a translation key the content mod's lang files carry.
    pub fn name_key(self) -> &'static str {
        match self {
            Tier::Common => "apotheosis.tier.common",
            Tier::Uncommon => "apotheosis.tier.uncommon",
            Tier::Rare => "apotheosis.tier.rare",
            Tier::Epic => "apotheosis.tier.epic",
            Tier::Mythic => "apotheosis.tier.mythic",
            Tier::Ancient => "apotheosis.tier.ancient",
        }
    }

    /// The tier's name in English, for a client without the key.
    pub fn name(self) -> &'static str {
        match self {
            Tier::Common => "Common",
            Tier::Uncommon => "Uncommon",
            Tier::Rare => "Rare",
            Tier::Epic => "Epic",
            Tier::Mythic => "Mythic",
            Tier::Ancient => "Ancient",
        }
    }

    /// The tier's colour, `0xRRGGBB`, using Java's five rarity colours.
    /// Ancient is this port's extension and reuses mythic's values.
    pub fn colour(self) -> u32 {
        match self {
            Tier::Common => 0x80_8080,
            Tier::Uncommon => 0x33_FF33,
            Tier::Rare => 0x55_55FF,
            Tier::Epic => 0xBB_00BB,
            Tier::Mythic | Tier::Ancient => 0xED_7014,
        }
    }

    /// The tier's colour as a text component's `color`.
    pub fn hex(self) -> String {
        format!("#{:06X}", self.colour())
    }
}

/// A string tag (`JavaString` is built from `&str`).
fn string(value: &str) -> Tag {
    Tag::String(value.into())
}

/// Mobs whose death drops a mythic or ancient gem.
const BOSSES: [&str; 4] = [
    "minecraft:wither",
    "minecraft:ender_dragon",
    "minecraft:warden",
    "minecraft:elder_guardian",
];

/// The gem chance for a kill, in 1000: Java's 4% for a real player kill and
/// 0.5% otherwise, but never for a passive entity.  Horizon API 415 has no
/// fake-player distinction, so every player kill is the 4% branch.
fn gem_kill_chance(kind: &str, boss: bool, killed_by_player: bool) -> Option<u64> {
    if !boss && !monsters::is_monster(kind) {
        return None;
    }
    Some(if killed_by_player { 40 } else { 5 })
}

/// Whether a dropped item can carry affixes: a weapon, a tool or a piece of
/// armour. Recognised from the item id, which is all a `loot_drops` hook is
/// given about an item.
pub fn is_equipment(item: &str) -> bool {
    const SUFFIXES: [&str; 14] = [
        "_sword",
        "_axe",
        "_pickaxe",
        "_shovel",
        "_hoe",
        "_helmet",
        "_chestplate",
        "_leggings",
        "_boots",
        "_bow",
        "_crossbow",
        "_spear",
        "_trident",
        "_mace",
    ];
    item == "minecraft:bow"
        || item == "minecraft:crossbow"
        || item == "minecraft:trident"
        || item == "minecraft:shield"
        || SUFFIXES.iter().any(|s| item.ends_with(s))
}

/// Whether a piece of equipment is worn as armour (the `armor` slot group),
/// as opposed to held in a hand (`hand`). Shields are worn in the off hand and
/// carry armour-group modifiers.
pub fn is_armor(item: &str) -> bool {
    item == "minecraft:shield"
        || ["_helmet", "_chestplate", "_leggings", "_boots"]
            .iter()
            .any(|s| item.ends_with(s))
}

/// A stand-in for a random source: the hook API has none, so rolls come from a
/// counter mixed with what dropped (splitmix64). Two kills of one mob at one
/// place in different ticks roll differently; the stream is this mod's own, not
/// the level's.
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// A number in `0..n`, uniformly enough for a loot roll.
    pub(crate) fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next() % n }
    }
    /// Whether a roll of `chance` in 100 comes up.
    pub(crate) fn chance(&mut self, chance: u64) -> bool {
        self.below(100) < chance
    }
    /// One of `weights`' tiers, by weight.
    pub(crate) fn tier(&mut self, weights: &[(Tier, u64)]) -> Tier {
        let total: u64 = weights.iter().map(|(_, w)| *w).sum();
        let mut roll = self.below(total.max(1));
        for (tier, weight) in weights {
            if roll < *weight {
                return *tier;
            }
            roll -= *weight;
        }
        weights.last().map_or(Tier::Uncommon, |(tier, _)| *tier)
    }
}

/// What an ordinary mob killed by a player can roll: never above epic, so the
/// two high tiers stay boss-only.
const ORDINARY_TIERS: [(Tier, u64); 3] = [(Tier::Uncommon, 55), (Tier::Rare, 30), (Tier::Epic, 15)];

/// What a boss's gear rolls.
pub(crate) const BOSS_TIERS: [(Tier, u64); 4] = [
    (Tier::Rare, 25),
    (Tier::Epic, 40),
    (Tier::Mythic, 25),
    (Tier::Ancient, 10),
];

const CHEST_GEM_POOL: &str = "apotheosis:chest_gems";

/// A counter that makes every loot roll's seed its own.
static DROPS: AtomicU64 = AtomicU64::new(0);

/// The chance Java's `gem_loot_injection.json` gives a chest table, by the
/// table's domain and path: minecraft chests 25%, every other domain's chests
/// 20%, and Twilight Forest structures 20%.  A table no entry matches gets no
/// pool at all.
fn chest_gem_chance(id: &str) -> Option<f64> {
    let (domain, path) = id.split_once(':')?;
    if path.starts_with("chests") {
        return Some(if domain == "minecraft" { 0.25 } else { 0.2 });
    }
    if domain == "twilightforest" && path.starts_with("structures") {
        return Some(0.2);
    }
    None
}

/// Adds Java's gem injection to a chest table: one generic `apotheosis:gem`
/// entry whose function draws the type by Java weight and the purity.  The
/// name guard makes the edit safe if a table is passed through the hook more
/// than once.
fn add_chest_gem_pool(id: &str, table: &mut serde_json::Value) {
    let Some(chance) = chest_gem_chance(id) else {
        return;
    };
    let Some(pools) = table
        .get_mut("pools")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return;
    };
    if pools
        .iter()
        .any(|pool| pool.get("name").and_then(serde_json::Value::as_str) == Some(CHEST_GEM_POOL))
    {
        return;
    }

    // One entry, as Java's `apotheosis:gems` GLM adds one: its function draws
    // the gem type by Java weight and dimensions, then clamps the purity to the
    // type's floor.  Twenty-one identical weighted entries would only be dead
    // weight, because the function would draw the type again.
    pools.push(serde_json::json!({
        "name": CHEST_GEM_POOL,
        "rolls": 1,
        "condition": {
            "type": "minecraft:random_chance",
            "chance": chance,
        },
        "entries": [{
            "type": "minecraft:item",
            "name": gems::ITEM,
            "weight": 1,
            "functions": [{ "function": "apotheosis:gem_purity" }],
        }],
    }));
}

/// A stack of one item.
pub(crate) fn one(item: &str) -> ItemStack {
    ItemStack::new(item, 1)
}

/// The gear an affixed drop can be.
const ORDINARY_GEAR: [&str; 8] = [
    "minecraft:iron_sword",
    "minecraft:iron_axe",
    "minecraft:bow",
    "minecraft:iron_helmet",
    "minecraft:iron_chestplate",
    "minecraft:iron_leggings",
    "minecraft:iron_boots",
    "minecraft:shield",
];

const BOSS_GEAR: [&str; 5] = [
    "minecraft:diamond_sword",
    "minecraft:diamond_axe",
    "minecraft:diamond_chestplate",
    "minecraft:diamond_leggings",
    "minecraft:bow",
];

/// `minecraft:custom_data` key recording the attribute values the last affix
/// pass wrote, so a reroll takes them back instead of stacking them.
const AFFIX_ATTRS_KEY: &str = "apotheosis:affix_attrs";

/// Every lore line this module writes is a translated `apotheosis.affix.*`
/// line: that prefix is how a reroll tells its own lines from someone else's.
const AFFIX_LORE_PREFIX: &str = "apotheosis.affix.";

/// The id of a `minecraft:attribute_modifiers` entry, when it names one.
fn modifier_id(entry: &Tag) -> Option<String> {
    match entry.get("id")? {
        Tag::String(id) => Some(id.to_string_lossy()),
        _ => None,
    }
}

/// Whether a modifier entry was written by an affix pass of this module. The
/// ids are the Java affix ids; socket modifiers use `apotheosis:socket_*` and
/// are left to `sockets`.
fn is_affix_modifier(entry: &Tag) -> bool {
    modifier_id(entry)
        .is_some_and(|id| java_affixes::AFFIXES.iter().any(|affix| affix.id == id))
}

/// Whether a lore line was written by an affix pass of this module.
fn is_affix_lore(entry: &Tag) -> bool {
    matches!(
        entry.get("translate"),
        Some(Tag::String(key)) if key.to_string_lossy().starts_with(AFFIX_LORE_PREFIX)
    )
}

/// The modifiers and lore already on `stack`, minus this module's own entries:
/// anything a datapack, another mod or the player wrote survives a reforge.
fn foreign_entries(stack: &ItemStack) -> (Vec<Tag>, Vec<Tag>) {
    let modifiers = match stack
        .data()
        .and_then(|data| data.other("minecraft:attribute_modifiers"))
    {
        Some(Tag::List(entries)) => entries
            .iter()
            .filter(|entry| !is_affix_modifier(entry))
            .cloned()
            .collect(),
        _ => Vec::new(),
    };
    let lore = match stack.data().and_then(|data| data.other("minecraft:lore")) {
        Some(Tag::List(lines)) => lines
            .iter()
            .filter(|line| !is_affix_lore(line))
            .cloned()
            .collect(),
        _ => Vec::new(),
    };
    (modifiers, lore)
}

/// Rolls `tier`'s affixes onto `stack`: real `minecraft:attribute_modifiers`, a
/// lore that says them, Java's prefix/suffix name, and the rarity that colours
/// the name. Only affixes this port can actually pay out enter the pool; the
/// rest of the Java catalogue stays in `java_affixes` for later.
///
/// A reroll replaces this module's own entries and nobody else's: other lore
/// lines and attribute modifiers stay, the previous affix values are taken back
/// before the new ones are added, and a player's name is kept (recoloured with
/// the tier).
pub(crate) fn apply_affixes(stack: &mut ItemStack, tier: Tier, rng: &mut Rng) {
    let count = tier.affixes();
    // Strip last pass's attributes even if this roll ends up empty, so they can
    // never be read twice.
    attributes::attach_recorded(stack, &[], AFFIX_ATTRS_KEY);
    let (mut modifiers, mut lore) = foreign_entries(stack);
    if count == 0 {
        return;
    }
    let class = attributes::class_of(&stack.item);
    let mut candidates = affixes::pool(class, tier);
    if candidates.is_empty() {
        return;
    }
    // The item's own modifiers come first: writing the component replaces the
    // defaults, so an affixed sword would otherwise lose its base damage. A
    // piece rolled before already carries them, so only add the missing ones.
    for entry in attributes::base_modifiers(&stack.item) {
        let id = modifier_id(&entry);
        if id.is_none() || !modifiers.iter().any(|kept| modifier_id(kept) == id) {
            modifiers.push(entry);
        }
    }
    modifiers.reserve(count);
    let mut custom = Vec::with_capacity(count);
    lore.reserve(count);
    let mut chosen: Vec<&'static java_affixes::Affix> = Vec::with_capacity(count);
    for _ in 0..count.min(candidates.len()) {
        let pick = rng.below(candidates.len() as u64) as usize;
        let affix = candidates.swap_remove(pick);
        let Some(amount) = affixes::amount(affix, tier, rng) else {
            continue;
        };
        let (Some(attribute), Some(operation)) = (affix.attribute, affix.operation) else {
            continue;
        };
        modifiers.push(Tag::compound([
            ("type", string(attribute)),
            ("id", string(affix.id)),
            ("amount", Tag::Double(amount.to_bits())),
            ("operation", string(operation)),
            ("slot", string(affixes::slot(class))),
        ]));
        lore.push(affixes::stat_line(affix, tier, amount));
        if let Some(attr) = affix.attr {
            custom.push((attr, amount));
        }
        chosen.push(affix);
    }
    if chosen.is_empty() {
        return;
    }
    // A reroll replaces the name our last roll wrote, but never a player's; a
    // kept name still carries the tier's colour, as Java's rarity component
    // gives it, so a mythic piece stays orange after an anvil rename.
    let kept_name = stack
        .data()
        .and_then(|data| data.other("minecraft:custom_name"))
        .cloned();
    let keep_name = kept_name
        .as_ref()
        .is_some_and(|name| !affixes::is_generated_name(name));
    let name = affixes::item_name_component(&stack.item, tier, &chosen);
    let data = stack.data_mut();
    data.set_other(
        "minecraft:attribute_modifiers",
        (!modifiers.is_empty()).then_some(Tag::List(modifiers)),
    );
    data.set_other("minecraft:lore", (!lore.is_empty()).then_some(Tag::List(lore)));
    if keep_name {
        if let Some(kept) = kept_name {
            data.set_other(
                "minecraft:custom_name",
                Some(affixes::recolour_name(&kept, tier)),
            );
        }
    } else {
        data.set_other("minecraft:custom_name", name);
    }
    data.set_other("minecraft:rarity", Some(string(tier.vanilla_rarity())));
    // Vanilla's component stops at epic; this one keeps the five Java rarities
    // (and this port's ancient).
    data.set_other("apotheosis:rarity", Some(string(tier.id())));
    attributes::attach_recorded(stack, &custom, AFFIX_ATTRS_KEY);
    // Effect affixes are extra, never replacements: the attribute lines and
    // their count stay as they were, and the effects follow Java's rarity
    // counts.
    affix_effects::add_effect_affix(stack, tier, rng);
}

/// A seed for one drop: the counter, the position and the mob, so two kills
/// never roll the same loot.
fn seed_of(context: &horizon_client::mod_api::NativeLootDrops<'_>, n: u64) -> u64 {
    let (x, y, z) = (context.origin[0], context.origin[1], context.origin[2]);
    let mut seed = n.wrapping_add(0x2545_F491_4F6C_DD1D);
    for part in [x.to_bits(), y.to_bits(), z.to_bits()] {
        seed ^= part.rotate_left(17);
    }
    if let Some((id, kind)) = context.entity {
        seed ^= u64::from(id) << 8;
        for byte in kind.bytes() {
            seed = seed.rotate_left(5) ^ u64::from(byte);
        }
    }
    seed
}

impl Hooks for Apotheosis {
    fn prepare_item_enchant(
        &self,
        host: &mut dyn Host,
        event: &mut horizon_client::mod_api::PrepareItemEnchantEvent<'_>,
    ) {
        enchanting::on_prepare_item_enchant(host, event);
    }

    fn modify_default_components(&self, event: &mut ModifyDefaultComponentsEvent<'_>) {
        // Before anything edits them: the item's own modifiers are what an
        // affixed copy must keep.
        attributes::remember_default_modifiers(event);
    }

    fn tooltip(&self, _host: &dyn Host, event: &mut TooltipEvent<'_>) {
        attributes::tooltip(event);
        spawners::on_tooltip(event);
        enchanting::tooltip(event);
        // Java's gem tooltip, built here: Horizon's client shows neither the
        // lore nor the name colour a gem item's declaration would carry.
        if let Some((gem, purity)) = gems::of_stack(event.stack) {
            let colour = purity.colour();
            if let Some((name, line_colour)) = event.lines.first_mut() {
                *name = std::mem::take(name).with_color(colour);
                *line_colour = tip_colour(colour);
            }
            for line in gems::tooltip_lines(gem, purity) {
                event
                    .lines
                    .push((line.piece.text().with_color(line.colour), tip_colour(line.colour)));
            }
        }
        sockets::tooltip(event);
    }

    fn use_item(&self, host: &mut dyn Host, event: &mut horizon_client::mod_api::UseItemEvent<'_>) -> Flow {
        // Two right-click gestures share this hook: socketing and sigils (gem or
        // sigil in the main hand) first, then the enchanting tomes and cobweb.
        match sockets::use_item(host, event) {
            Flow::Continue => enchanting::on_use_item(host, event),
            Flow::Cancel => Flow::Cancel,
        }
    }

    fn inventory_tick(&self, host: &mut dyn Host, event: &mut horizon_client::mod_api::InventoryTickEvent) {
        sockets::inventory_tick(host, event);
        enchanting::inventory_tick(host, event);
    }

    fn loot_table_load(&self, event: &mut horizon_client::mod_api::LootTableLoadEvent<'_>) {
        add_chest_gem_pool(event.id, event.table);
    }

    fn loot_function(&self, event: &mut horizon_client::mod_api::LootFunctionEvent<'_>) {
        sockets::loot_function(event)
    }

    fn loot_drops(&self, host: &mut dyn Host, event: &mut LootDropsEvent<'_>) {
        // A silk-touched spawner's own drop first, so telepathy carries it too.
        spawners::on_loot_drops(event);
        // Telepathy moves a tool's drops to the player, festive may double a
        // kill's gear: both happen on their own, kill or use.
        affix_effects::loot_drops(host, event);
        let Some((id, kind)) = event.context.entity else {
            return;
        };
        // A player's use (shears, a brush) is not a kill.
        if event.context.used {
            return;
        }
        let boss = BOSSES.contains(&kind) || affix_effects::is_boss(host, id);
        let killed_by_player = event.context.player.is_some();
        let Some(gem_rate) = gem_kill_chance(kind, boss, killed_by_player) else {
            return;
        };
        let n = DROPS.fetch_add(1, Ordering::Relaxed);
        let mut rng = Rng::new(seed_of(&event.context, n));

        // A gem of the level's dimension, at a purity at or above its Java
        // floor: a boss draws from better steps than an ordinary mob.
        if boss {
            let (gem, purity) = gems::roll(event.context.dimension, true, &mut rng);
            event.drops.push(gems::stack(gem, purity));
        }
        // The gear this mob dropped, upgraded in place: a zombie's sword, a
        // skeleton's bow.
        for stack in event.drops.iter_mut() {
            if !is_equipment(&stack.item) || stack.extra.is_some() || stack.damage != 0 {
                continue;
            }
            if boss {
                let tier = rng.tier(&BOSS_TIERS);
                apply_affixes(stack, tier, &mut rng);
                sockets::after_affixes(stack, tier, &mut rng);
            } else if killed_by_player && rng.chance(35) {
                let tier = rng.tier(&ORDINARY_TIERS);
                apply_affixes(stack, tier, &mut rng);
                sockets::after_affixes(stack, tier, &mut rng);
            }
        }
        // A low gem from an ordinary monster: 4% for a player kill, 0.5%
        // otherwise, as the two Java GLMs do.
        if !boss && rng.below(1000) < gem_rate {
            let (gem, purity) = gems::roll(event.context.dimension, false, &mut rng);
            event.drops.push(gems::stack(gem, purity));
        }
        // And one of its own: a boss always, an ordinary mob killed by a player
        // rarely.
        let gear = |rng: &mut Rng| -> &'static str {
            if boss {
                BOSS_GEAR[rng.below(BOSS_GEAR.len() as u64) as usize]
            } else {
                ORDINARY_GEAR[rng.below(ORDINARY_GEAR.len() as u64) as usize]
            }
        };
        let drop = boss || (killed_by_player && rng.chance(4));
        if drop {
            let tier = if boss {
                rng.tier(&BOSS_TIERS)
            } else {
                rng.tier(&ORDINARY_TIERS)
            };
            let item = gear(&mut rng);
            let mut stack = one(item);
            apply_affixes(&mut stack, tier, &mut rng);
            sockets::after_affixes(&mut stack, tier, &mut rng);
            event.drops.push(stack);
        }
        // A gem's purity is rolled where it drops; the gear above is done.
        for stack in event.drops.iter_mut() {
            if stack.data().is_none() {
                sockets::roll_dropped_purity(stack, &mut rng, boss);
            }
        }    }

    fn living_incoming_damage(&self, event: &mut LivingIncomingDamageEvent<'_>) -> Flow {
        if attributes::incoming_damage(event) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn living_incoming_damage_v2(
        &self,
        host: &mut dyn Host,
        event: &mut LivingIncomingDamageV2Event<'_>,
    ) -> Flow {
        if attributes::incoming_damage_v2(host, event) {
            return Flow::Cancel;
        }
        affix_effects::on_hit(host, event);
        Flow::Continue
    }

    fn item_damaged(&self, host: &mut dyn Host, event: &mut ItemDamagedEvent<'_>) -> Flow {
        affix_effects::item_damaged(host, event)
    }

    fn projectile_launch(&self, _host: &mut dyn Host, event: &mut ProjectileLaunchEvent<'_>) {
        attributes::projectile_launch(event);
    }

    fn entity_join_level(&self, _host: &mut dyn Host, event: &mut EntityJoinLevelEvent<'_>) -> Flow {
        affix_effects::entity_join_level(event)
    }

    fn mob_spawned(&self, host: &mut dyn Host, event: &mut MobSpawnedEvent<'_>) {
        affix_effects::mob_spawned(host, event);
    }

    fn living_heal(&self, event: &mut LivingHealEvent) -> Flow {
        attributes::living_heal(event);
        Flow::Continue
    }

    fn experience_orb_pickup(
        &self,
        host: &mut dyn Host,
        event: &mut ExperienceOrbPickupEvent<'_>,
    ) -> Flow {
        attributes::orb(host, event);
        Flow::Continue
    }

    fn break_speed(&self, host: &dyn Host, event: &mut BreakSpeedEvent) {
        attributes::break_speed(host, event);
    }

    fn arrow_loose(&self, host: &mut dyn Host, event: &mut ArrowLooseEvent<'_>) -> Flow {
        attributes::arrow_loose(host, event);
        Flow::Continue
    }

    fn living_equipment_change(
        &self,
        _host: &mut dyn Host,
        event: &mut LivingEquipmentChangeEvent<'_>,
    ) {
        attributes::equipment_changed(event);
    }

    fn on_equip(&self, host: &mut dyn Host, event: &mut OnEquipEvent<'_>) {
        attributes::equipped(host, event);
    }

    fn player_tick(&self, host: &mut dyn Host, event: &mut PlayerTickEvent) {
        attributes::tick(host, event);
    }

    fn tick(&self, host: &mut dyn Host, _event: &mut TickEvent) {
        spawners::on_tick(host);
    }

    fn right_click_block(&self, host: &mut dyn Host, event: &mut RightClickBlockEvent) -> Flow {
        match spawners::on_right_click(host, event.pos) {
            Flow::Continue => enchanting::on_right_click_block(host, event),
            Flow::Cancel => Flow::Cancel,
        }
    }

    fn block_entity(
        &self,
        _host: &mut dyn Host,
        event: &mut horizon_client::mod_api::BlockEntityEvent<'_>,
    ) -> Option<(String, horizon_client::mod_api::Compound)> {
        enchanting::on_block_entity(event).or_else(|| menus::on_block_entity(event))
    }

    fn block_entity_ticks(
        &self,
        _host: &dyn Host,
        event: &mut horizon_client::mod_api::BlockEntityTicksEvent<'_>,
    ) {
        enchanting::on_block_entity_ticks(event);
        menus::on_block_entity_ticks(event);
    }

    fn block_entity_tick(
        &self,
        host: &mut dyn Host,
        event: &mut horizon_client::mod_api::BlockEntityTickEvent<'_>,
    ) {
        if !menus::on_block_entity_tick(event) {
            enchanting::on_block_entity_tick(host, event);
        }
    }

    fn block_entity_container(
        &self,
        _host: &dyn Host,
        event: &mut horizon_client::mod_api::BlockEntityContainerEvent<'_>,
    ) -> Option<horizon_client::mod_api::ModContainer> {
        menus::on_block_entity_container(event)
    }

    fn screen(
        &self,
        _host: &dyn Host,
        event: &mut horizon_client::mod_api::ScreenEvent<'_>,
    ) -> Option<horizon_client::mod_api::NativeScreen> {
        menus::on_screen(event)
    }

    fn screen_draw(&self, host: &dyn Host, event: &mut horizon_client::mod_api::ScreenDrawEvent<'_>) {
        menus::on_screen_draw(host, event);
    }

    fn screen_click(
        &self,
        _host: &dyn Host,
        event: &mut horizon_client::mod_api::ScreenClickEvent<'_>,
    ) -> Option<i32> {
        menus::on_screen_click(event)
    }

    fn screen_tooltip(&self, _host: &dyn Host, event: &mut horizon_client::mod_api::ScreenTooltipEvent<'_>) {
        menus::on_screen_tooltip(event);
    }

    fn menu_button(&self, host: &mut dyn Host, event: &mut horizon_client::mod_api::MenuButtonEvent<'_>) {
        menus::on_menu_button(host, event);
    }

    fn menu_slot_rules(&self, event: &mut horizon_client::mod_api::MenuSlotRulesEvent<'_>) {
        menus::on_menu_slot_rules(event);
    }

    fn use_item_on(&self, host: &mut dyn Host, event: &mut UseItemOnEvent<'_>) -> Flow {
        // A spawner modifier on a spawner block, then an infusion on the
        // apothic enchanting table. The Apotheosis tables open their menus.
        match spawners::on_use_item_on(host, event) {
            Flow::Continue => enchanting::on_use_item_on(host, event),
            Flow::Cancel => Flow::Cancel,
        }
    }

    fn left_click_block(&self, host: &mut dyn Host, event: &mut LeftClickBlockEvent) -> Flow {
        spawners::on_left_click(host, event.pos, event.state)
    }

    fn place_block(&self, host: &mut dyn Host, event: &mut PlaceBlockEvent<'_>) {
        spawners::on_place(host, event);
    }

    fn register_commands(&self, _host: &mut dyn Host, event: &mut RegisterCommandsEvent<'_>) {
        event.commands.push(NativeCommand::new("apoth"));
        event.commands.push(NativeCommand::new("apoth salvage"));
    }

    fn perform_command(&self, host: &mut dyn Host, event: &mut PerformCommandEvent<'_>) -> Flow {
        let command = event.command.trim();
        let words: Vec<&str> = command.split_whitespace().collect();
        if words.first() != Some(&"apoth") {
            return Flow::Continue;
        }
        if words.len() < 2 {
            host.feedback(Text::translate_with_fallback(
                "apotheosis.command.help",
                "Apotheosis: /apoth salvage — break an affixed piece into rarity materials. Reforge at a reforging table.",
                &[],
            ));
            return Flow::Cancel;
        }
        if words[1] == "salvage" {
            sockets::begin_salvage(
                host,
                event.player.uuid,
                i32::from(event.player.selected_slot),
            );
            return Flow::Cancel;
        }
        host.feedback(Text::translate_with_fallback(
            "apotheosis.command.unknown",
            "Unknown Apotheosis command.",
            &[],
        ));
        Flow::Cancel
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_java_gem_names_itself() {
        for gem in &java_gems::GEMS {
            let stack = gems::stack(gem, crate::sockets::Purity::Normal);
            assert_eq!(gems::id_of(&stack), Some(gem.id));
            // A gem item's name is a default component of its declaration
            // (`tools/test_content.py` checks it); only the generic gem, below
            // a purity floor, carries it in its patch.
            if &*stack.item != gems::ITEM {
                assert!(stack.data().is_none(), "{}", stack.item);
                continue;
            }
            // A gem's name is an item name, never an anvil-style custom name:
            // custom_name is what renders italics like a player-given name.
            assert!(stack
                .data()
                .and_then(|data| data.other("minecraft:item_name"))
                .is_some());
            assert!(stack
                .data()
                .and_then(|data| data.other("minecraft:custom_name"))
                .is_none());
        }
    }

    #[test]
    fn a_gem_is_not_equipment() {
        assert!(!is_equipment(gems::ITEM));
    }

    #[test]
    fn gear_is_recognised_by_its_id() {
        assert!(is_equipment("minecraft:diamond_sword"));
        assert!(is_equipment("minecraft:iron_chestplate"));
        assert!(is_equipment("minecraft:bow"));
        assert!(is_equipment("minecraft:netherite_pickaxe"));
        assert!(!is_equipment("minecraft:dirt"));
        assert!(!is_equipment(gems::ITEM));
    }

    #[test]
    fn higher_tiers_carry_more_and_bigger_affixes() {
        assert_eq!(Tier::Common.affixes(), 1);
        assert_eq!(Tier::Uncommon.affixes(), 2);
        assert_eq!(Tier::Rare.affixes(), 2);
        assert_eq!(Tier::Epic.affixes(), 3);
        assert_eq!(Tier::Mythic.affixes(), 4);
        assert_eq!(Tier::Ancient.affixes(), 5);
        assert_eq!(Tier::Ancient.rarity_index(), Tier::Mythic.rarity_index());
        assert!(Tier::Ancient.scale() > Tier::Epic.scale());
    }

    #[test]
    fn ordinary_loot_never_reaches_the_boss_tiers() {
        for (tier, _) in ORDINARY_TIERS {
            assert!(tier <= Tier::Epic, "{tier:?} is boss-only");
        }
    }

    #[test]
    fn affixing_writes_modifiers_lore_and_rarity() {
        let mut stack = one("minecraft:diamond_sword");
        let mut rng = Rng::new(7);
        apply_affixes(&mut stack, Tier::Rare, &mut rng);
        let data = stack.data().expect("components");
        let Some(Tag::List(modifiers)) = data.other("minecraft:attribute_modifiers") else {
            panic!("no attribute modifiers");
        };
        assert_eq!(modifiers.len(), 2);
        for entry in modifiers {
            for key in ["type", "id", "amount", "operation", "slot"] {
                assert!(entry.get(key).is_some(), "modifier without {key}");
            }
        }
        let Some(Tag::List(lore)) = data.other("minecraft:lore") else {
            panic!("no lore");
        };
        // Two stat lines, and the one basic effect affix Java's rare rarity
        // guarantees; no tier line.
        assert_eq!(lore.len(), 3, "one Java line per affix, no tier line");
        assert_eq!(
            data.other("minecraft:rarity"),
            Some(&Tag::String("rare".into()))
        );
        let Some(Tag::Compound(name)) = data.other("minecraft:custom_name") else {
            panic!("no composed name");
        };
        assert_eq!(
            name.get(&"translate".into()),
            Some(&Tag::String(
                "misc.apotheosis.affix_name.three".into()
            ))
        );
    }

    #[test]
    fn affixes_of_one_item_are_distinct() {
        let mut stack = one("minecraft:diamond_chestplate");
        let mut rng = Rng::new(11);
        apply_affixes(&mut stack, Tier::Ancient, &mut rng);
        let data = stack.data().expect("components");
        let Some(Tag::List(modifiers)) = data.other("minecraft:attribute_modifiers") else {
            panic!("no attribute modifiers");
        };
        assert_eq!(modifiers.len(), 5);
        let ids: Vec<String> = modifiers
            .iter()
            .filter_map(|m| match m.get("id") {
                Some(Tag::String(s)) => Some(s.to_string_lossy()),
                _ => None,
            })
            .collect();
        let unique: std::collections::HashSet<&String> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len(), "an affix was rolled twice");
    }

    #[test]
    fn affix_ids_are_the_java_ids() {
        let mut stack = one("minecraft:diamond_sword");
        let mut rng = Rng::new(13);
        apply_affixes(&mut stack, Tier::Mythic, &mut rng);
        let data = stack.data().expect("components");
        let Some(Tag::List(modifiers)) = data.other("minecraft:attribute_modifiers") else {
            panic!("no attribute modifiers");
        };
        let affixed: Vec<String> = modifiers
            .iter()
            .filter_map(|modifier| match modifier.get("id") {
                Some(Tag::String(id)) if id.to_string_lossy().starts_with("apotheosis:") => {
                    Some(id.to_string_lossy())
                }
                _ => None,
            })
            .collect();
        assert_eq!(affixed.len(), 4, "mythic rolls four Java affixes");
        for id in affixed {
            assert!(
                java_affixes::AFFIXES.iter().any(|affix| affix.id == id),
                "{id} is not a Java affix id"
            );
        }
    }

    /// The Apothic attribute values an item carries.
    fn apothic_attributes(stack: &ItemStack) -> std::collections::BTreeMap<String, f64> {
        let Some(Tag::Compound(custom)) = stack
            .data()
            .and_then(|data| data.other("minecraft:custom_data"))
        else {
            return Default::default();
        };
        let Some(Tag::Compound(attrs)) = custom.get(&"apotheosis:attributes".into()) else {
            return Default::default();
        };
        attrs
            .iter()
            .filter_map(|(key, value)| match value {
                Tag::Double(bits) => Some((key.to_string_lossy(), f64::from_bits(*bits))),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_reroll_replaces_its_own_attribute_values() {
        let seed = (0..100)
            .find(|seed| {
                let mut fresh = one("minecraft:diamond_sword");
                apply_affixes(&mut fresh, Tier::Mythic, &mut Rng::new(*seed));
                !apothic_attributes(&fresh).is_empty()
            })
            .expect("mythic gear rolls Apothic attributes");
        let mut stack = one("minecraft:diamond_sword");
        apply_affixes(&mut stack, Tier::Mythic, &mut Rng::new(seed));
        let first = apothic_attributes(&stack);
        apply_affixes(&mut stack, Tier::Mythic, &mut Rng::new(seed));
        assert_eq!(
            apothic_attributes(&stack),
            first,
            "the second pass stacked its own bonuses"
        );
    }

    #[test]
    fn a_reforge_keeps_foreign_lore_and_modifiers() {
        let mut stack = one("minecraft:diamond_sword");
        let foreign_lore = Tag::compound([
            ("translate", string("othermod.note")),
            ("fallback", string("Kept")),
        ]);
        let foreign_modifier = Tag::compound([
            ("type", string("minecraft:attack_damage")),
            ("id", string("othermod:test")),
            ("amount", Tag::Double(1.5f64.to_bits())),
            ("operation", string("add_value")),
            ("slot", string("hand")),
        ]);
        {
            let data = stack.data_mut();
            data.set_other(
                "minecraft:lore",
                Some(Tag::List(vec![foreign_lore.clone()])),
            );
            data.set_other(
                "minecraft:attribute_modifiers",
                Some(Tag::List(vec![foreign_modifier.clone()])),
            );
        }
        let mut rng = Rng::new(5);
        apply_affixes(&mut stack, Tier::Rare, &mut rng);
        apply_affixes(&mut stack, Tier::Rare, &mut rng);
        let data = stack.data().expect("components");
        let Some(Tag::List(lore)) = data.other("minecraft:lore") else {
            panic!("no lore");
        };
        assert!(
            lore.contains(&foreign_lore),
            "the foreign lore line was erased"
        );
        assert_eq!(
            lore.iter().filter(|line| is_affix_lore(line)).count(),
            3,
            "rare: two stat lines and one effect line, replaced not stacked"
        );
        let Some(Tag::List(modifiers)) = data.other("minecraft:attribute_modifiers") else {
            panic!("no modifiers");
        };
        assert!(
            modifiers.contains(&foreign_modifier),
            "the foreign modifier was erased"
        );
        assert_eq!(
            modifiers.iter().filter(|entry| is_affix_modifier(entry)).count(),
            2,
            "the affix modifiers replace each other"
        );
    }

    #[test]
    fn a_renamed_mythic_piece_is_coloured_mythic() {
        let mut stack = one("minecraft:diamond_sword");
        stack
            .data_mut()
            .set_other("minecraft:custom_name", Some(string("Bob's Blade")));
        apply_affixes(&mut stack, Tier::Mythic, &mut Rng::new(9));
        let data = stack.data().expect("components");
        let Some(Tag::Compound(name)) = data.other("minecraft:custom_name") else {
            panic!("the player's name was replaced");
        };
        let colour = string(&Tier::Mythic.hex());
        assert_eq!(
            name.get(&"text".into()),
            Some(&Tag::String("Bob's Blade".into())),
            "the player's text stays"
        );
        assert_eq!(
            name.get(&"color".into()),
            Some(&colour),
            "a kept name must carry the tier's colour"
        );
        assert_eq!(
            data.other("apotheosis:rarity"),
            Some(&Tag::String("mythic".into())),
            "the real Java rarity is recorded"
        );
        // Vanilla's own component still names the closest of its four.
        assert_eq!(
            data.other("minecraft:rarity"),
            Some(&Tag::String("epic".into()))
        );
    }

    #[test]
    fn every_tier_names_its_java_rarity() {
        let tiers = [
            (Tier::Common, "common"),
            (Tier::Uncommon, "uncommon"),
            (Tier::Rare, "rare"),
            (Tier::Epic, "epic"),
            (Tier::Mythic, "mythic"),
            (Tier::Ancient, "ancient"),
        ];
        for (tier, id) in tiers {
            assert_eq!(tier.id(), id);
            assert_eq!(tier.hex(), format!("#{:06X}", tier.colour()));
        }
    }

    #[test]
    fn a_seed_follows_its_drop() {
        let a = 1u64;
        assert_ne!(
            seed_of_fake(a, [0.0, 64.0, 0.0]),
            seed_of_fake(a, [2.0, 64.0, 0.0])
        );
    }

    #[test]
    fn chests_get_java_rates_and_one_generic_gem_entry() {
        for (id, chance) in [
            ("minecraft:chests/simple_dungeon", 0.25),
            ("minecraft:chests/end_city_treasure", 0.25),
            ("twilightforest:structures/labyrinth", 0.2),
            ("othermod:chests/reward", 0.2),
        ] {
            let mut table = serde_json::json!({"pools": []});
            add_chest_gem_pool(id, &mut table);
            let pools = table["pools"].as_array().unwrap();
            assert_eq!(pools.len(), 1, "{id}");
            assert_eq!(pools[0]["name"], CHEST_GEM_POOL);
            assert_eq!(pools[0]["condition"]["type"], "minecraft:random_chance");
            assert_eq!(pools[0]["condition"]["chance"], chance, "{id}");
            let entries = pools[0]["entries"].as_array().unwrap();
            assert_eq!(entries.len(), 1, "{id}: one entry, not one per gem");
            assert_eq!(entries[0]["name"], gems::ITEM);
            assert_eq!(entries[0]["functions"][0]["function"], "apotheosis:gem_purity");
        }
        assert_eq!(chest_gem_chance("minecraft:entities/zombie"), None);
        assert_eq!(chest_gem_chance("twilightforest:chests/foo"), Some(0.2));
        assert_eq!(chest_gem_chance("twilightforest:structures/foo"), Some(0.2));
        assert_eq!(chest_gem_chance("minecraft:gameplay/fishing"), None);
    }

    #[test]
    fn chest_gem_pool_is_idempotent_and_does_not_touch_other_tables() {
        let mut table = serde_json::json!({"pools": []});
        add_chest_gem_pool("minecraft:chests/ancient_city", &mut table);
        add_chest_gem_pool("minecraft:chests/ancient_city", &mut table);
        assert_eq!(table["pools"].as_array().unwrap().len(), 1);

        let mut other = serde_json::json!({"pools": []});
        add_chest_gem_pool("minecraft:entities/zombie", &mut other);
        assert!(other["pools"].as_array().unwrap().is_empty());
    }

    #[test]
    fn only_a_monster_can_drop_a_gem_and_a_player_doubles_the_odds() {
        assert_eq!(gem_kill_chance("minecraft:cow", false, true), None);
        assert_eq!(gem_kill_chance("minecraft:pig", false, false), None);
        assert_eq!(gem_kill_chance("minecraft:zombie", false, true), Some(40));
        assert_eq!(gem_kill_chance("minecraft:zombie", false, false), Some(5));
        assert_eq!(gem_kill_chance("minecraft:wither", true, false), Some(5));
        // A boss tagged by the other module still counts even if its id is
        // outside the vanilla monster list.
        assert_eq!(gem_kill_chance("mymod:boss", true, true), Some(40));
    }

    /// `seed_of` over a bare context, for the test above.
    fn seed_of_fake(n: u64, origin: [f64; 3]) -> u64 {
        let (x, y, z) = (origin[0], origin[1], origin[2]);
        let mut seed = n.wrapping_add(0x2545_F491_4F6C_DD1D);
        for part in [x.to_bits(), y.to_bits(), z.to_bits()] {
            seed ^= part.rotate_left(17);
        }
        seed
    }
}
