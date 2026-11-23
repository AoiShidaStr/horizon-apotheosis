//! Apotheosis for Horizon, the native half.
//!
//! Clean-room reimplementation from Apotheosis' public behaviour (rarity tiers
//! above vanilla's, affixes on gear, higher tiers only from bosses). No
//! Apotheosis code or asset is used: see `../../README.md` and `NOTICE.md`.
//!
//! The hooks below are the real game's trait API (plus `sockets`' own
//! `use_item_on` and `loot_function` for the tables and the chest gems):
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
//! - `register_commands` / `perform_command`: `/apoth reforge` rolls affixes
//!   onto plain gear, for experience levels.
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
mod affix_effects;
mod affixes;
mod attributes;
mod enchanting;
mod gems;
mod java_affixes;
mod java_gems;

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

/// Adds a modest, weighted gem roll to vanilla chest tables. The name guard
/// makes the edit safe if a table is passed through the hook more than once.
fn add_chest_gem_pool(id: &str, table: &mut serde_json::Value) {
    if !id.starts_with("minecraft:chests/") {
        return;
    }
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

    // One entry per Java gem, at its Java weight: the loot function picks the
    // gem's type and purity, so a chest gem is a real gem of the level's table.
    let entries: Vec<_> = java_gems::GEMS
        .iter()
        .map(|gem| {
            serde_json::json!({
                "type": "minecraft:item",
                "name": gems::ITEM,
                "weight": gem.weight,
                "functions": [{ "function": "apotheosis:gem_purity" }],
            })
        })
        .collect();
    pools.push(serde_json::json!({
        "name": CHEST_GEM_POOL,
        "rolls": 1,
        "condition": {
            "type": "minecraft:random_chance",
            "chance": 0.08,
        },
        "entries": entries,
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

/// Rolls `tier`'s affixes onto `stack`: real `minecraft:attribute_modifiers`, a
/// lore that says them, Java's prefix/suffix name, and the vanilla rarity that
/// colours the name. Only affixes this port can actually pay out enter the
/// pool; the rest of the Java catalogue stays in `java_affixes` for later.
pub(crate) fn apply_affixes(stack: &mut ItemStack, tier: Tier, rng: &mut Rng) {
    let count = tier.affixes();
    if count == 0 {
        return;
    }
    let class = attributes::class_of(&stack.item);
    let mut candidates = affixes::pool(class, tier);
    if candidates.is_empty() {
        return;
    }
    // The item's own modifiers come first: writing the component replaces the
    // defaults, so an affixed sword would otherwise lose its base damage.
    let mut modifiers = attributes::base_modifiers(&stack.item);
    modifiers.reserve(count);
    let mut custom = Vec::with_capacity(count);
    let mut lore = Vec::with_capacity(count);
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
    // A reroll replaces the name our last roll wrote, but never a player's.
    let keep_name = stack
        .data()
        .and_then(|data| data.other("minecraft:custom_name"))
        .is_some_and(|name| !affixes::is_generated_name(name));
    let name = affixes::item_name_component(&stack.item, tier, &chosen);
    let data = stack.data_mut();
    data.set_other("minecraft:attribute_modifiers", Some(Tag::List(modifiers)));
    data.set_other("minecraft:lore", Some(Tag::List(lore)));
    if !keep_name {
        data.set_other("minecraft:custom_name", name);
    }
    data.set_other("minecraft:rarity", Some(string(tier.vanilla_rarity())));
    attributes::attach(stack, &custom);
    // An effect affix is extra, never a replacement: the attribute lines and
    // their count stay as they were.
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
        if let Some((gem, purity)) = gems::of_stack(event.stack) {
            for (line, colour) in gems::description(gem, purity) {
                event.lines.push((
                    Text::translate_with_fallback("apotheosis.gem.bonus", &line, &[])
                        .with_color(colour),
                    colour,
                ));
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
        if !boss && !killed_by_player {
            return;
        }
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
            } else if rng.chance(35) {
                let tier = rng.tier(&ORDINARY_TIERS);
                apply_affixes(stack, tier, &mut rng);
                sockets::after_affixes(stack, tier, &mut rng);
            }
        }
        // A low gem from an ordinary mob, often enough to matter.
        if !boss && rng.chance(12) {
            let (gem, purity) = gems::roll(event.context.dimension, false, &mut rng);
            event.drops.push(gems::stack(gem, purity));
        }
        // And one of its own: a boss always, an ordinary mob rarely.
        let gear = |rng: &mut Rng| -> &'static str {
            if boss {
                BOSS_GEAR[rng.below(BOSS_GEAR.len() as u64) as usize]
            } else {
                ORDINARY_GEAR[rng.below(ORDINARY_GEAR.len() as u64) as usize]
            }
        };
        let drop = if boss { true } else { rng.chance(4) };
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

    fn projectile_launch(&self, host: &mut dyn Host, event: &mut ProjectileLaunchEvent<'_>) {
        attributes::projectile_launch(event);
        affix_effects::projectile_launch(host, event);
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
        enchanting::on_block_entity(event)
    }

    fn block_entity_ticks(
        &self,
        _host: &dyn Host,
        event: &mut horizon_client::mod_api::BlockEntityTicksEvent<'_>,
    ) {
        enchanting::on_block_entity_ticks(event);
    }

    fn block_entity_tick(
        &self,
        host: &mut dyn Host,
        event: &mut horizon_client::mod_api::BlockEntityTickEvent<'_>,
    ) {
        enchanting::on_block_entity_tick(host, event);
    }

    fn use_item_on(&self, host: &mut dyn Host, event: &mut UseItemOnEvent<'_>) -> Flow {
        // The Apotheosis tables (reforge, salvage, gem cutting) first, then a
        // spawner modifier on a spawner block.
        match sockets::use_item_on(host, event) {
            Flow::Continue => spawners::on_use_item_on(host, event),
            Flow::Cancel => Flow::Cancel,
        }
    }

    fn left_click_block(&self, host: &mut dyn Host, event: &mut LeftClickBlockEvent) -> Flow {
        spawners::on_left_click(host, event.pos)
    }

    fn place_block(&self, host: &mut dyn Host, event: &mut PlaceBlockEvent<'_>) {
        spawners::on_place(host, event);
    }

    fn register_commands(&self, _host: &mut dyn Host, event: &mut RegisterCommandsEvent<'_>) {
        event.commands.push(NativeCommand::new("apoth"));
        event.commands.push(NativeCommand::new("apoth reforge"));
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
                "Apotheosis: /apoth reforge — roll affixes onto the gear you hold (3 levels); /apoth salvage — break an affixed piece into rarity materials.",
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
        if words[1] != "reforge" {
            host.feedback(Text::translate_with_fallback(
                "apotheosis.command.unknown",
                "Unknown Apotheosis command.",
                &[],
            ));
            return Flow::Cancel;
        }
        reforge(
            host,
            event.player.uuid,
            event.player.xp_level,
            i32::from(event.player.selected_slot),
        );
        Flow::Cancel
    }
}

/// How many levels a reforge costs.
pub const REFORGE_COST: i32 = 3;

/// `/apoth reforge`: takes one plain piece of gear the player holds, gives it
/// back with a rolled rarity, and charges `REFORGE_COST` levels.
fn reforge(host: &mut dyn Host, player: u128, xp_level: i32, slot: i32) {
    let Some((item, count, plain)) = host.held() else {
        host.feedback(Text::translate_with_fallback(
            "apotheosis.reforge.empty",
            "Hold the piece of gear you want reforged.",
            &[],
        ));
        return;
    };
    if !is_equipment(&item) {
        host.feedback(Text::translate_with_fallback(
            "apotheosis.reforge.not_gear",
            "That is not a weapon, a tool or a piece of armour.",
            &[],
        ));
        return;
    }
    if !plain || count == 0 {
        // An affixed piece is rerolled in place, on its next inventory tick.
        if host
            .held_stack(player)
            .is_some_and(|stack| sockets::can_reforge(&stack))
        {
            if xp_level < REFORGE_COST {
                host.feedback(Text::translate_with_fallback(
                    "apotheosis.reforge.levels",
                    "Reforging costs 3 experience levels.",
                    &[],
                ));
                return;
            }
            if !host.give_levels(player, -REFORGE_COST) {
                host.feedback(Text::translate_with_fallback(
                    "apotheosis.reforge.levels",
                    "Reforging costs 3 experience levels.",
                    &[],
                ));
                return;
            }
            sockets::begin_reforge(host, player, slot);
            host.feedback(Text::translate_with_fallback(
                "apotheosis.reforge.rerolled",
                "Reforged: the piece in your hand rolls again.",
                &[],
            ));
            return;
        }
        host.feedback(Text::translate_with_fallback(
            "apotheosis.reforge.already",
            "That piece is neither plain gear nor an affixed piece of this mod.",
            &[],
        ));
        return;
    }
    if xp_level < REFORGE_COST {
        host.feedback(Text::translate_with_fallback(
            "apotheosis.reforge.levels",
            "Reforging costs 3 experience levels.",
            &[],
        ));
        return;
    }
    if host.take_items(&item, 1) != 1 {
        host.feedback(Text::translate_with_fallback(
            "apotheosis.reforge.empty",
            "Hold the piece of gear you want reforged.",
            &[],
        ));
        return;
    }
    let n = DROPS.fetch_add(1, Ordering::Relaxed);
    let mut rng = Rng::new(n);
    let tier = rng.tier(&ORDINARY_TIERS);
    let mut stack = one(&item);
    apply_affixes(&mut stack, tier, &mut rng);
    sockets::after_affixes(&mut stack, tier, &mut rng);
    if host.give_stack(player, stack).is_some() {
        // No room: hand the plain piece back rather than lose it.
        let _ = host.give_stack(player, one(&item));
        host.feedback(Text::translate_with_fallback(
            "apotheosis.reforge.no_room",
            "No room for the reforged piece; nothing was charged.",
            &[],
        ));
        return;
    }
    let _ = host.give_levels(player, -REFORGE_COST);
    host.feedback(Text::translate_with_fallback(
        "apotheosis.reforge.done",
        "Reforged: %s.",
        &[Text::translate_with_fallback(
            tier.name_key(),
            tier.name(),
            &[],
        )],
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_java_gem_names_itself() {
        for gem in &java_gems::GEMS {
            let stack = gems::stack(gem, crate::sockets::Purity::Normal);
            assert_eq!(gems::id_of(&stack), Some(gem.id));
            assert!(stack
                .data()
                .and_then(|data| data.other("minecraft:custom_name"))
                .is_some());
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
        assert_eq!(lore.len(), 2, "one Java line per affix, no tier line");
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

    #[test]
    fn a_seed_follows_its_drop() {
        let a = 1u64;
        assert_ne!(
            seed_of_fake(a, [0.0, 64.0, 0.0]),
            seed_of_fake(a, [2.0, 64.0, 0.0])
        );
    }

    #[test]
    fn vanilla_chests_get_a_weighted_low_gem_pool() {
        let mut table = serde_json::json!({"pools": []});
        add_chest_gem_pool("minecraft:chests/simple_dungeon", &mut table);
        let pools = table["pools"].as_array().unwrap();
        assert_eq!(pools.len(), 1);
        assert_eq!(pools[0]["name"], CHEST_GEM_POOL);
        assert_eq!(pools[0]["condition"]["type"], "minecraft:random_chance");
        assert_eq!(pools[0]["condition"]["chance"], 0.08);
        let entries = pools[0]["entries"].as_array().unwrap();
        assert_eq!(entries.len(), java_gems::GEMS.len());
        assert!(entries
            .iter()
            .all(|entry| entry["name"].as_str() == Some(gems::ITEM)));
        assert!(entries.iter().all(|entry| entry["weight"].as_u64().is_some()));
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
