//! Apothic Spawners for Horizon, the native half.
//!
//! Clean-room reimplementation from Apothic Spawners' public behaviour (the
//! stat names, the modifier items and their steps and bounds, and the NBT keys
//! a spawner stores). No Apothic Spawners code or asset is used.
//!
//! Horizon's pinned hooks API (415) has no hook that reads or writes a block
//! entity's NBT — `block_entity_tag`, `set_block_entity`, `block_entity_nbt`
//! and `run_command_captured` are all still pending fragments. What is there:
//! the console commands (`run_command`, API 21), the item entities of a box
//! (`item_entities`, API 42) and the player hooks. So a spawner's data is read
//! by copying it into a probe item entity with `/data modify` and reading that
//! item back, and written with `/data merge block`.
//!
//! A carried spawner is another matter: its data travels in the item's own
//! `minecraft:block_entity_data` component, so the tooltip reads it on the
//! client with no command at all.
//!
//! Seven hooks, all on the real game's trait API:
//! - `tooltip`: a carried spawner shows its mob and its stats (API 12).
//! - `right_click_block`: a click on a spawner asks for its stats (API 21).
//! - `use_item_on`: a modifier item on a spawner changes a stat (API 13).
//! - `left_click_block`: the data of a spawner is read before it breaks (API 30).
//! - `loot_drops`: a silk-touch break drops the spawner with that data (API 75).
//! - `place_block`: a placed spawner gets its data back, for survival too (API 14).
//! - `tick`: the queued reads, writes and messages run where commands may (API 1).

use horizon_client::mod_api::{
    Flow, Host, ItemStack, LootDropsEvent, PlaceBlockEvent, Tag, Text, TooltipEvent,
    UseItemOnEvent,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// The vanilla spawner block, and the block entity type it saves under.
pub const SPAWNER: &str = "minecraft:spawner";
const BLOCK_ENTITY_ID: &str = "minecraft:mob_spawner";
const CUSTOM_DATA: &str = "minecraft:custom_data";
const BLOCK_ENTITY_DATA: &str = "minecraft:block_entity_data";
/// The tag the probe item entity carries, so it is found again.
const PROBE: &str = "apoth_spawner_probe";
/// The key the probe item carries its own name under: a read can leave a probe
/// behind (a killed server, a command that failed), and the exact one has to be
/// read back, not a stale one that sits in the same block.
const PROBE_KEY: &str = "apoth_probe";
const PROBE_ITEM: &str = "minecraft:stone";
/// How many queued actions one tick takes at most.
const BATCH: usize = 64;
/// How many read spawners wait to be dropped, and how long one waits (ticks).
const CACHE: usize = 32;
const CACHE_TICKS: u64 = 1200;

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

/// A numeric spawner stat: the seven vanilla numbers Apothic Spawners changes,
/// plus its own `Echoing` (0 to 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stat {
    MinDelay,
    MaxDelay,
    SpawnCount,
    MaxNearby,
    PlayerRange,
    SpawnRange,
    InitialHealth,
    Echoing,
}

impl Stat {
    pub const ALL: [Stat; 8] = [
        Stat::MinDelay,
        Stat::MaxDelay,
        Stat::SpawnCount,
        Stat::MaxNearby,
        Stat::PlayerRange,
        Stat::SpawnRange,
        Stat::InitialHealth,
        Stat::Echoing,
    ];

    /// Its key in the spawner's block entity.
    pub fn key(self) -> &'static str {
        match self {
            Stat::MinDelay => "MinSpawnDelay",
            Stat::MaxDelay => "MaxSpawnDelay",
            Stat::SpawnCount => "SpawnCount",
            Stat::MaxNearby => "MaxNearbyEntities",
            Stat::PlayerRange => "RequiredPlayerRange",
            Stat::SpawnRange => "SpawnRange",
            Stat::InitialHealth => "InitialHealth",
            Stat::Echoing => "Echoing",
        }
    }

    /// The lowest and highest value Apothic Spawners keeps this at.
    pub fn bounds(self) -> (f32, f32) {
        match self {
            Stat::MinDelay | Stat::MaxDelay => (20.0, 1600.0),
            Stat::SpawnCount => (1.0, 16.0),
            Stat::MaxNearby => (1.0, 32.0),
            Stat::PlayerRange => (1.0, 48.0),
            Stat::SpawnRange => (1.0, 32.0),
            Stat::InitialHealth => (0.20, 1.0),
            Stat::Echoing => (0.0, 3.0),
        }
    }

    fn index(self) -> usize {
        match self {
            Stat::MinDelay => 0,
            Stat::MaxDelay => 1,
            Stat::SpawnCount => 2,
            Stat::MaxNearby => 3,
            Stat::PlayerRange => 4,
            Stat::SpawnRange => 5,
            Stat::InitialHealth => 6,
            Stat::Echoing => 7,
        }
    }

    /// `true` for a whole number, as every stat but the initial health is.
    fn whole(self) -> bool {
        self != Stat::InitialHealth
    }
}

/// A spawner flag Apothic Spawners adds: one NBT byte per flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flag {
    IgnorePlayers,
    IgnoreLight,
    IgnoreConditions,
    NoAi,
    Silent,
    RedstoneControl,
    Youthful,
    Burning,
}

impl Flag {
    pub const ALL: [Flag; 8] = [
        Flag::IgnorePlayers,
        Flag::IgnoreLight,
        Flag::IgnoreConditions,
        Flag::NoAi,
        Flag::Silent,
        Flag::RedstoneControl,
        Flag::Youthful,
        Flag::Burning,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Flag::IgnorePlayers => "IgnorePlayers",
            Flag::IgnoreLight => "IgnoreLight",
            Flag::IgnoreConditions => "IgnoreConditions",
            Flag::NoAi => "NoAI",
            Flag::Silent => "Silent",
            Flag::RedstoneControl => "RedstoneControl",
            Flag::Youthful => "Youthful",
            Flag::Burning => "Burning",
        }
    }

    /// The language key and its English text, for the action bar.
    pub fn label(self) -> (&'static str, &'static str) {
        match self {
            Flag::IgnorePlayers => (
                "apotheosis.spawner.flag.ignore_players",
                "ignores players",
            ),
            Flag::IgnoreLight => ("apotheosis.spawner.flag.ignore_light", "ignores light"),
            Flag::IgnoreConditions => (
                "apotheosis.spawner.flag.ignore_conditions",
                "ignores conditions",
            ),
            Flag::NoAi => ("apotheosis.spawner.flag.no_ai", "mobs have no AI"),
            Flag::Silent => ("apotheosis.spawner.flag.silent", "silent"),
            Flag::RedstoneControl => (
                "apotheosis.spawner.flag.redstone_control",
                "redstone controlled",
            ),
            Flag::Youthful => ("apotheosis.spawner.flag.youthful", "young mobs"),
            Flag::Burning => ("apotheosis.spawner.flag.burning", "burning mobs"),
        }
    }

    fn bit(self) -> u8 {
        1 << (self as u8)
    }
}

/// A spawner as it stands in the world: its mob and its stats.
#[derive(Clone, Debug, PartialEq)]
pub struct Stats {
    /// The entity the spawner spawns (`minecraft:zombie`), empty when unset.
    pub mob: String,
    /// One value per [`Stat`], in [`Stat::ALL`] order.
    pub nums: [f32; 8],
    /// The [`Flag`]s that are on.
    pub flags: u8,
}

impl Default for Stats {
    /// What a fresh vanilla spawner holds.
    fn default() -> Self {
        Stats {
            mob: String::new(),
            nums: [200.0, 800.0, 4.0, 6.0, 16.0, 4.0, 1.0, 0.0],
            flags: 0,
        }
    }
}

impl Stats {
    pub fn get(&self, stat: Stat) -> f32 {
        self.nums[stat.index()]
    }

    pub fn set(&mut self, stat: Stat, value: f32) {
        let (min, max) = stat.bounds();
        let value = value.clamp(min, max);
        self.nums[stat.index()] = if stat.whole() { value.round() } else { value };
    }

    pub fn flag(&self, flag: Flag) -> bool {
        self.flags & flag.bit() != 0
    }

    pub fn set_flag(&mut self, flag: Flag, on: bool) {
        if on {
            self.flags |= flag.bit();
        } else {
            self.flags &= !flag.bit();
        }
    }

    /// Reads a spawner's saved data. Missing keys keep vanilla's defaults,
    /// which is what Apothic Spawners itself does for a spawner it has not
    /// touched yet.
    pub fn from_tag(tag: &Tag) -> Stats {
        let mut stats = Stats::default();
        for stat in Stat::ALL {
            if let Some(value) = number(tag.get(stat.key())) {
                stats.set(stat, value);
            }
        }
        for flag in Flag::ALL {
            stats.set_flag(flag, number(tag.get(flag.key())).unwrap_or(0.0) != 0.0);
        }
        if let Some(id) = spawn_entity(tag) {
            stats.mob = id;
        }
        stats
    }

    /// The `/data merge block` compound for these stats: every key, so a stat
    /// that was clamped down is written back as it now stands.
    pub fn merge_snbt(&self) -> String {
        let mut out = String::from("{");
        for (n, stat) in Stat::ALL.into_iter().enumerate() {
            if n > 0 {
                out.push(',');
            }
            let value = self.get(stat);
            if stat.whole() {
                out.push_str(&format!("{}:{}", stat.key(), value as i32));
            } else {
                out.push_str(&format!("{}:{}f", stat.key(), fmt_float(value)));
            }
        }
        for flag in Flag::ALL {
            out.push_str(&format!(
                ",{}:{}b",
                flag.key(),
                if self.flag(flag) { 1 } else { 0 }
            ));
        }
        out.push('}');
        out
    }
}

/// The entity a spawner spawns: `SpawnData.entity.id`, else the first of its
/// `SpawnPotentials`.
fn spawn_entity(tag: &Tag) -> Option<String> {
    if let Some(id) = tag
        .get("SpawnData")
        .and_then(|data| data.get("entity"))
        .and_then(|entity| entity.get("id"))
        .and_then(tag_string)
    {
        return Some(id);
    }
    let potentials = match tag.get("SpawnPotentials") {
        Some(Tag::List(list)) => list,
        _ => return None,
    };
    potentials
        .iter()
        .find_map(|entry| entry.get("data"))
        .and_then(|data| data.get("entity"))
        .and_then(|entity| entity.get("id"))
        .and_then(tag_string)
}

fn tag_string(tag: &Tag) -> Option<String> {
    match tag {
        Tag::String(text) => Some(text.to_string_lossy()),
        _ => None,
    }
}

/// A number out of any of NBT's numeric types.
pub fn number(tag: Option<&Tag>) -> Option<f32> {
    match tag? {
        Tag::Byte(v) => Some(f32::from(*v)),
        Tag::Short(v) => Some(f32::from(*v)),
        Tag::Int(v) => Some(*v as f32),
        Tag::Long(v) => Some(*v as f32),
        Tag::Float(bits) => Some(f32::from_bits(*bits)),
        Tag::Double(bits) => Some(f64::from_bits(*bits) as f32),
        _ => None,
    }
}

fn fmt_float(value: f32) -> String {
    let text = format!("{value:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text.is_empty() {
        "0".to_owned()
    } else {
        text.to_owned()
    }
}

// ---------------------------------------------------------------------------
// Modifiers
// ---------------------------------------------------------------------------

/// Which stack a modifier answers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selector {
    /// One item id.
    Item(&'static str),
    /// Any `minecraft:*_wool`.
    Wool,
}

/// What a modifier does to a spawner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    /// Adds `step` to a stat, clamped to the stat's bounds.
    Number { stat: Stat, step: f32 },
    /// Turns a flag on.
    Flag(Flag),
}

/// One of Apothic Spawners' modifiers: the item, the stat and the step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Modifier {
    pub selector: Selector,
    pub effect: Effect,
}

impl Modifier {
    /// The stat it moves, if it moves one.
    pub fn stat(&self) -> Option<Stat> {
        match self.effect {
            Effect::Number { stat, .. } => Some(stat),
            Effect::Flag(_) => None,
        }
    }

    /// The step it takes, in the increasing direction.
    pub fn step(&self) -> f32 {
        match self.effect {
            Effect::Number { step, .. } => step,
            Effect::Flag(_) => 0.0,
        }
    }
}

const fn num(selector: Selector, stat: Stat, step: f32) -> Modifier {
    Modifier {
        selector,
        effect: Effect::Number { stat, step },
    }
}

const fn flag(selector: Selector, flag: Flag) -> Modifier {
    Modifier {
        selector,
        effect: Effect::Flag(flag),
    }
}

/// The modifiers of Apothic Spawners, item by item and value by value, as its
/// `spawner_modifiers` recipes define them.
pub const MODIFIERS: [Modifier; 16] = [
    // Sugar and a clock: the spawner's own delays, shorter.
    num(Selector::Item("minecraft:sugar"), Stat::MinDelay, -10.0),
    num(Selector::Item("minecraft:clock"), Stat::MaxDelay, -20.0),
    // A wave holds more mobs, and the spawner allows more of them nearby.
    num(
        Selector::Item("minecraft:fermented_spider_eye"),
        Stat::SpawnCount,
        2.0,
    ),
    num(
        Selector::Item("minecraft:ghast_tear"),
        Stat::MaxNearby,
        2.0,
    ),
    // A player has to come closer, and mobs appear further out.
    num(
        Selector::Item("minecraft:prismarine_crystals"),
        Stat::PlayerRange,
        4.0,
    ),
    num(Selector::Item("minecraft:piston"), Stat::SpawnRange, 2.0),
    // Frailer mobs, and the spawner's own echo (up to three).
    num(
        Selector::Item("minecraft:pointed_dripstone"),
        Stat::InitialHealth,
        -0.05,
    ),
    num(
        Selector::Item("minecraft:echo_shard"),
        Stat::Echoing,
        1.0,
    ),
    // The flags.
    flag(
        Selector::Item("minecraft:nether_star"),
        Flag::IgnorePlayers,
    ),
    flag(
        Selector::Item("minecraft:soul_lantern"),
        Flag::IgnoreLight,
    ),
    flag(Selector::Item("minecraft:conduit"), Flag::IgnoreConditions),
    flag(Selector::Item("minecraft:chorus_fruit"), Flag::NoAi),
    flag(Selector::Wool, Flag::Silent),
    flag(Selector::Item("minecraft:comparator"), Flag::RedstoneControl),
    flag(Selector::Item("minecraft:turtle_egg"), Flag::Youthful),
    flag(Selector::Item("minecraft:campfire"), Flag::Burning),
];

/// The off-hand inverter of Apothic Spawners.
pub const INVERTER: &str = "minecraft:quartz";

/// The modifier an item is, if it is one.
pub fn modifier_for(item: &str) -> Option<&'static Modifier> {
    MODIFIERS.iter().find(|modifier| match modifier.selector {
        Selector::Item(id) => id == item,
        Selector::Wool => item.starts_with("minecraft:") && item.ends_with("_wool"),
    })
}

/// How a click on a spawner with `modifier` changes `stats`. `inverse` is the
/// inverter in the off hand: it takes the step back, and does nothing to a
/// flag (a flag is only ever turned on, as in Apothic Spawners).
///
/// Answers whether anything changed.
pub fn apply(stats: &mut Stats, modifier: &Modifier, inverse: bool) -> bool {
    match modifier.effect {
        Effect::Number { stat, step } => {
            let before = stats.get(stat);
            let delta = if inverse { -step } else { step };
            stats.set(stat, before + delta);
            stats.get(stat) != before
        }
        Effect::Flag(flag) => {
            if inverse || stats.flag(flag) {
                return false;
            }
            stats.set_flag(flag, true);
            true
        }
    }
}

// ---------------------------------------------------------------------------
// SNBT
// ---------------------------------------------------------------------------

/// A tag as SNBT, for a `/data merge block` argument.
pub fn snbt(tag: &Tag) -> String {
    let mut out = String::new();
    write_tag(&mut out, tag);
    out
}

fn write_tag(out: &mut String, tag: &Tag) {
    match tag {
        Tag::End => out.push_str("null"),
        Tag::Byte(v) => out.push_str(&format!("{v}b")),
        Tag::Short(v) => out.push_str(&format!("{v}s")),
        Tag::Int(v) => out.push_str(&format!("{v}")),
        Tag::Long(v) => out.push_str(&format!("{v}L")),
        Tag::Float(bits) => out.push_str(&format!("{}f", fmt_float(f32::from_bits(*bits)))),
        Tag::Double(bits) => out.push_str(&format!("{}d", fmt_float(f64::from_bits(*bits) as f32))),
        Tag::ByteArray(values) => {
            out.push_str("[B;");
            for (n, value) in values.iter().enumerate() {
                if n > 0 {
                    out.push(',');
                }
                out.push_str(&format!("{value}b"));
            }
            out.push(']');
        }
        Tag::IntArray(values) => {
            out.push_str("[I;");
            for (n, value) in values.iter().enumerate() {
                if n > 0 {
                    out.push(',');
                }
                out.push_str(&format!("{value}"));
            }
            out.push(']');
        }
        Tag::LongArray(values) => {
            out.push_str("[L;");
            for (n, value) in values.iter().enumerate() {
                if n > 0 {
                    out.push(',');
                }
                out.push_str(&format!("{value}L"));
            }
            out.push(']');
        }
        Tag::String(text) => out.push_str(&quote(&text.to_string_lossy())),
        Tag::List(values) => {
            out.push('[');
            for (n, value) in values.iter().enumerate() {
                if n > 0 {
                    out.push(',');
                }
                write_tag(out, value);
            }
            out.push(']');
        }
        Tag::Compound(entries) => {
            out.push('{');
            for (n, (key, value)) in entries.iter().enumerate() {
                if n > 0 {
                    out.push(',');
                }
                out.push_str(&write_key(&key.to_string_lossy()));
                out.push(':');
                write_tag(out, value);
            }
            out.push('}');
        }
    }
}

fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// A compound key as SNBT writes it: bare while it is a plain name, quoted
/// otherwise.
fn write_key(key: &str) -> String {
    let plain = !key.is_empty()
        && key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '+'));
    if plain {
        key.to_owned()
    } else {
        quote(key)
    }
}

/// A spawner's saved data without the keys a block entity's own position and
/// type put there: `/data merge block` refuses another type's `id`.
pub fn mergeable(tag: &Tag) -> String {
    let mut out = String::from("{");
    let Tag::Compound(entries) = tag else {
        return snbt(tag);
    };
    let mut first = true;
    for (key, value) in entries.iter() {
        let key = key.to_string_lossy();
        let key = key.as_ref();
        if matches!(key, "id" | "x" | "y" | "z" | "keepPacked") {
            continue;
        }
        if !first {
            out.push(',');
        }
        first = false;
        out.push_str(&write_key(key));
        out.push(':');
        write_tag(&mut out, value);
    }
    out.push('}');
    out
}

// ---------------------------------------------------------------------------
// Items
// ---------------------------------------------------------------------------

/// The silk-touch level of a tool: `minecraft:enchantments`' `levels` map, as
/// 1.21 writes it, or a plain `minecraft:silk_touch` key on an older one.
pub fn silk_touch(stack: &ItemStack) -> i32 {
    let Some(data) = stack.data() else {
        return 0;
    };
    let Some(enchantments) = data.other("minecraft:enchantments") else {
        return 0;
    };
    let mut levels = enchantments;
    if let Some(inner) = enchantments.get("levels") {
        levels = inner;
    }
    number(levels.get("minecraft:silk_touch")).unwrap_or(0.0) as i32
}

/// The spawner item a broken spawner drops: `minecraft:block_entity_data`
/// carries the block entity's data, so placing it restores the mob and the
/// stats.
pub fn spawner_item(tag: &Tag) -> ItemStack {
    let mut stack = ItemStack::new(SPAWNER, 1);
    let mut data = match tag {
        Tag::Compound(entries) => Tag::Compound(entries.clone()),
        _ => {
            return stack;
        }
    };
    if data.get("id").is_none() {
        if let Tag::Compound(entries) = &mut data {
            entries.insert(
                "id".into(),
                Tag::String(BLOCK_ENTITY_ID.into()),
            );
        }
    }
    stack
        .data_mut()
        .set_other(BLOCK_ENTITY_DATA, Some(data));
    stack
}

// ---------------------------------------------------------------------------
// Action bar
// ---------------------------------------------------------------------------

/// The mob's name: the game's own name for that entity, which is translated
/// into the player's language, and a readable fallback for an id the game does
/// not know.
fn mob_text(stats: &Stats) -> Text {
    if stats.mob.is_empty() {
        return Text::literal("?");
    }
    let (key, fallback) = match stats.mob.split_once(':') {
        Some((namespace, path)) => (
            format!("entity.{namespace}.{path}"),
            path.replace('_', " "),
        ),
        None => (
            format!("entity.{}", stats.mob),
            stats.mob.replace('_', " "),
        ),
    };
    Text::translate_with_fallback(&key, &fallback, &[])
}

fn ticks(stats: &Stats, stat: Stat) -> Text {
    Text::literal((stats.get(stat) as i32).to_string())
}

/// The line a spawner's stats are shown as: the mob, the delays, the wave, the
/// ranges, and the flags that are on.
pub fn summary(stats: &Stats) -> Text {
    let mut line = Text::translate_with_fallback(
        "apotheosis.spawner.stats",
        "Spawner: %s · delay %s–%s ticks · %s per wave · player range %s · nearby max %s · spawn range %s",
        &[
            mob_text(stats),
            ticks(stats, Stat::MinDelay),
            ticks(stats, Stat::MaxDelay),
            ticks(stats, Stat::SpawnCount),
            ticks(stats, Stat::PlayerRange),
            ticks(stats, Stat::MaxNearby),
            ticks(stats, Stat::SpawnRange),
        ],
    );
    for flag in Flag::ALL {
        if !stats.flag(flag) {
            continue;
        }
        let (key, fallback) = flag.label();
        line = line.append(Text::literal(" · "));
        line = line.append(Text::translate_with_fallback(key, fallback, &[]));
    }
    line
}

// ---------------------------------------------------------------------------
// Tooltip
// ---------------------------------------------------------------------------

/// The colour of the mob's line: aqua, the colour vanilla writes a spawner's
/// own line in.
pub const MOB_COLOUR: u32 = 0x55_FF_FF;
/// The colour of a stat's line.
pub const STAT_COLOUR: u32 = 0xAA_AA_AA;
/// The colour of the line that lists the flags.
pub const FLAG_COLOUR: u32 = 0x55_FF_55;

/// A carried spawner's tooltip: one line per fact, so a narrow tooltip still
/// reads well. The mob, the two delays, the wave, the three ranges, the mob
/// health and the echo while they are not the vanilla ones, then the flags.
pub fn tooltip(stats: &Stats) -> Vec<(Text, u32)> {
    let mut lines = Vec::with_capacity(7);
    let mut line = |key: &str, fallback: &str, args: &[Text], colour: u32| {
        let text = Text::translate_with_fallback(key, fallback, args);
        lines.push((text.with_color(colour), colour));
    };
    line(
        "apotheosis.spawner.tooltip.mob",
        "Mob: %s",
        &[mob_text(stats)],
        MOB_COLOUR,
    );
    line(
        "apotheosis.spawner.tooltip.delay",
        "Delay: %s–%s ticks",
        &[ticks(stats, Stat::MinDelay), ticks(stats, Stat::MaxDelay)],
        STAT_COLOUR,
    );
    line(
        "apotheosis.spawner.tooltip.wave",
        "Mobs per wave: %s",
        &[ticks(stats, Stat::SpawnCount)],
        STAT_COLOUR,
    );
    line(
        "apotheosis.spawner.tooltip.ranges",
        "Player range %s · nearby max %s · spawn range %s",
        &[
            ticks(stats, Stat::PlayerRange),
            ticks(stats, Stat::MaxNearby),
            ticks(stats, Stat::SpawnRange),
        ],
        STAT_COLOUR,
    );
    // Only worth a line when a modifier moved it away from vanilla.
    if stats.get(Stat::InitialHealth) < 0.999 {
        line(
            "apotheosis.spawner.tooltip.health",
            "Mob health: %s×",
            &[Text::literal(fmt_float(stats.get(Stat::InitialHealth)))],
            STAT_COLOUR,
        );
    }
    if stats.get(Stat::Echoing) > 0.0 {
        line(
            "apotheosis.spawner.tooltip.echo",
            "Echo: %s",
            &[ticks(stats, Stat::Echoing)],
            STAT_COLOUR,
        );
    }
    // The flags, as one list: an empty line would say nothing.
    let mut flags: Option<Text> = None;
    for flag in Flag::ALL {
        if !stats.flag(flag) {
            continue;
        }
        let (key, fallback) = flag.label();
        let label = Text::translate_with_fallback(key, fallback, &[]);
        flags = Some(match flags {
            None => label,
            Some(joined) => joined.append(Text::literal(" · ")).append(label),
        });
    }
    if let Some(flags) = flags {
        line(
            "apotheosis.spawner.tooltip.options",
            "Options: %s",
            &[flags],
            FLAG_COLOUR,
        );
    }
    lines
}

/// An item tooltip, on the client: a spawner item that carries a block
/// entity's data says which mob it will spawn and with which stats.
pub fn on_tooltip(event: &mut TooltipEvent<'_>) {
    if event.stack.item.as_ref() != SPAWNER {
        return;
    }
    let Some(data) = event.stack.data().and_then(|d| d.other(BLOCK_ENTITY_DATA)) else {
        return;
    };
    event.lines.extend(tooltip(&Stats::from_tag(data)));
}

// ---------------------------------------------------------------------------
// Hooks
// ---------------------------------------------------------------------------

/// What a hook asked the tick to do once commands may run.
enum Job {
    /// Read a spawner's stats and show them.
    Show { player: u128 },
    /// Apply a modifier item to a spawner.
    Modify {
        player: u128,
        item: String,
        inverse: bool,
    },
    /// Read a spawner's data, to drop it with.
    Probe,
    /// Give a placed spawner its data back.
    Restore(Tag),
}

struct Queued {
    dimension: String,
    pos: [i32; 3],
    job: Job,
}

struct Cached {
    dimension: String,
    pos: [i32; 3],
    tag: Tag,
    tick: u64,
}

static QUEUE: Mutex<Vec<Queued>> = Mutex::new(Vec::new());
static CACHED: Mutex<Vec<Cached>> = Mutex::new(Vec::new());
static TICKS: AtomicU64 = AtomicU64::new(0);
/// How many probes have been made, so each one can be named.
static PROBES: AtomicU64 = AtomicU64::new(0);

fn push(dimension: &str, pos: [i32; 3], job: Job) {
    let Ok(mut queue) = QUEUE.lock() else {
        return;
    };
    if queue.len() >= BATCH {
        queue.remove(0);
    }
    queue.push(Queued {
        dimension: dimension.to_owned(),
        pos,
        job,
    });
}

/// The data of the spawner that `dimension`/`pos` holds, as it was read for a
/// break, if it is still waiting for its drop.
pub fn cached_drop(dimension: &str, pos: [i32; 3]) -> Option<Tag> {
    let mut cached = CACHED.lock().ok()?;
    let index = cached
        .iter()
        .position(|entry| entry.dimension == dimension && entry.pos == pos)?;
    Some(cached.remove(index).tag)
}

/// Drops what was read for `dimension`/`pos`: the block there is not the
/// spawner that was read any more, so its data must not come back.
fn forget(dimension: &str, pos: [i32; 3]) {
    let Ok(mut cached) = CACHED.lock() else {
        return;
    };
    cached.retain(|entry| !(entry.dimension == dimension && entry.pos == pos));
}

/// What a probe found, or nothing when the spawner was gone or its chunk was
/// not loaded: a stale entry at that position would hand a fresh spawner's
/// break the old one's data, so a failed read takes it out.
fn settle(dimension: &str, pos: [i32; 3], tag: Option<Tag>, tick: u64) {
    match tag {
        Some(tag) => remember(dimension, pos, tag, tick),
        None => forget(dimension, pos),
    }
}

/// A right click on a block: on a spawner it asks for the stats, unless a
/// modifier item's own click (below) already answers for it this tick.
pub fn on_right_click(host: &mut dyn Host, pos: [i32; 3]) -> Flow {
    if host.block(pos) != SPAWNER {
        return Flow::Continue;
    }
    let Some(player) = host.acting_player() else {
        return Flow::Continue;
    };
    push(
        &player.dimension,
        pos,
        Job::Show {
            player: player.uuid,
        },
    );
    Flow::Continue
}

/// An item used on a block: a modifier item changes the spawner's stat, the
/// off-hand inverter turns its step round. The item's own use is claimed.
pub fn on_use_item_on(host: &mut dyn Host, event: &mut UseItemOnEvent<'_>) -> Flow {
    if host.block(event.pos) != SPAWNER {
        return Flow::Continue;
    }
    let Some(player) = host.acting_player() else {
        return Flow::Continue;
    };
    let item = event.stack.item.as_ref();
    let inverse = item == INVERTER;
    if !inverse && modifier_for(item).is_none() {
        return Flow::Continue;
    }
    push(
        &player.dimension,
        event.pos,
        Job::Modify {
            player: player.uuid,
            item: if inverse {
                INVERTER.to_owned()
            } else {
                item.to_owned()
            },
            inverse,
        },
    );
    Flow::Cancel
}

/// A left click on a spawner: read its data now, so the break can drop it.
pub fn on_left_click(host: &mut dyn Host, pos: [i32; 3]) -> Flow {
    if host.block(pos) != SPAWNER {
        return Flow::Continue;
    }
    push(host.dimension(), pos, Job::Probe);
    Flow::Continue
}

/// A spawner item placed: its data goes back into the block, for a player who
/// may not set a spawner's data themselves.
pub fn on_place(host: &mut dyn Host, event: &PlaceBlockEvent<'_>) {
    if event.stack.item.as_ref() != SPAWNER {
        return;
    }
    let Some(data) = event.stack.data().and_then(|d| d.other(BLOCK_ENTITY_DATA)) else {
        return;
    };
    push(host.dimension(), event.pos, Job::Restore(data.clone()));
}

/// A broken spawner that a silk-touch tool broke: it drops itself with its mob
/// and its stats.
pub fn on_loot_drops(event: &mut LootDropsEvent<'_>) {
    let Some((pos, _)) = event.context.block else {
        return;
    };
    let Some(tool) = event.context.tool else {
        return;
    };
    if silk_touch(tool) < 1 {
        return;
    }
    let Some(tag) = cached_drop(event.context.dimension, pos) else {
        return;
    };
    event.drops.push(spawner_item(&tag));
}

/// Once a tick: the queued reads, writes and messages, where commands may run.
pub fn on_tick(host: &mut dyn Host) {
    let tick = TICKS.fetch_add(1, Ordering::Relaxed) + 1;
    let queued: Vec<Queued> = match QUEUE.lock() {
        Ok(mut queue) => std::mem::take(&mut *queue),
        Err(_) => return,
    };
    // A quartz click in the off hand inverts the modifier clicks of the same
    // spawner this tick.
    let inverter: Vec<(u128, [i32; 3])> = queued
        .iter()
        .filter_map(|entry| match entry.job {
            Job::Modify {
                player,
                inverse: true,
                ..
            } => Some((player, entry.pos)),
            _ => None,
        })
        .collect();
    for entry in &queued {
        match &entry.job {
            Job::Probe => {
                // A spawner broken between the click and this tick, or a chunk
                // that is not loaded, is not read: nothing of it is left to
                // drop, and what an older read cached goes with it.
                let tag = read(host, &entry.dimension, entry.pos);
                settle(&entry.dimension, entry.pos, tag, tick);
            }
            Job::Restore(tag) => {
                write(host, &entry.dimension, entry.pos, &mergeable(tag));
            }
            Job::Modify { player, item, .. } => {
                let inverse = inverter
                    .iter()
                    .any(|(who, pos)| who == player && *pos == entry.pos);
                let Some(modifier) = modifier_for(item) else {
                    // A quartz click with no modifier item to invert: show the
                    // stats, as any other click on a spawner does.
                    if let Some(stats) = read_stats(host, &entry.dimension, entry.pos) {
                        host.action_bar(*player, summary(&stats));
                    }
                    continue;
                };
                let Some(mut stats) = read_stats(host, &entry.dimension, entry.pos) else {
                    continue;
                };
                if apply(&mut stats, modifier, inverse) {
                    write(host, &entry.dimension, entry.pos, &stats.merge_snbt());
                }
                host.action_bar(*player, summary(&stats));
            }
            Job::Show { player } => {
                let answered = queued.iter().any(|other| {
                    other.pos == entry.pos
                        && matches!(other.job, Job::Modify { player: who, .. } if who == *player)
                });
                if answered {
                    continue;
                }
                if let Some(stats) = read_stats(host, &entry.dimension, entry.pos) {
                    host.action_bar(*player, summary(&stats));
                }
            }
        }
    }
    forget_old(tick);
}

fn remember(dimension: &str, pos: [i32; 3], tag: Tag, tick: u64) {
    let Ok(mut cached) = CACHED.lock() else {
        return;
    };
    cached.retain(|entry| !(entry.dimension == dimension && entry.pos == pos));
    if cached.len() >= CACHE {
        cached.remove(0);
    }
    cached.push(Cached {
        dimension: dimension.to_owned(),
        pos,
        tag,
        tick,
    });
}

fn forget_old(tick: u64) {
    let Ok(mut cached) = CACHED.lock() else {
        return;
    };
    cached.retain(|entry| tick.saturating_sub(entry.tick) < CACHE_TICKS);
}

fn read_stats(host: &mut dyn Host, dimension: &str, pos: [i32; 3]) -> Option<Stats> {
    read(host, dimension, pos).map(|tag| Stats::from_tag(&tag))
}

/// The name this read's probe is given. A counter, so two reads never answer
/// for each other, and a probe an interrupted read left behind is never taken
/// for the one this read just made.
fn probe_tag() -> String {
    format!("{PROBE}_{}", PROBES.fetch_add(1, Ordering::Relaxed))
}

/// The commands one read runs: put a probe in the spawner's own block, copy
/// the block entity's data into it, then take it out again.
///
/// The first two are gated on the block still being a spawner, so a spawner
/// broken between the click and this tick, or a chunk that is not loaded,
/// fails quietly instead of writing an error into the log.
fn read_commands(dimension: &str, pos: [i32; 3], probe: &str) -> [String; 3] {
    let [x, y, z] = pos;
    let gate = format!("execute in {dimension} if block {x} {y} {z} {SPAWNER} run");
    let summon = format!(
        "{gate} summon minecraft:item {} {} {} \
         {{Item:{{id:\"{PROBE_ITEM}\",count:1b,components:{{\"{CUSTOM_DATA}\":\
         {{\"{PROBE_KEY}\":\"{probe}\"}}}}}},NoGravity:1b,Invulnerable:1b,Age:-32768s,\
         PickupDelay:32767s,Tags:[\"{PROBE}\",\"{probe}\"]}}",
        f64::from(x) + 0.5,
        f64::from(y) + 0.5,
        f64::from(z) + 0.5
    );
    let copy = format!(
        "{gate} data modify entity @e[type=item,tag={probe},limit=1] \
         Item.components.\"{CUSTOM_DATA}\".spawner set from block {x} {y} {z}"
    );
    let kill = format!("execute in {dimension} run kill @e[type=item,tag={probe}]");
    [summon, copy, kill]
}

/// Kills the probes of earlier reads in this dimension: one left behind by a
/// command that never ran would sit in the spawner for good.
fn sweep_command(dimension: &str) -> String {
    format!("execute in {dimension} run kill @e[type=item,tag={PROBE}]")
}

/// A spawner's saved data, through a probe item entity: Horizon 415 has no
/// block entity NBT helper, but `/data modify ... set from block` copies the
/// data and `item_entities` reads the item back. The probe lives for the one
/// tick of the call, and it is found by the name it carries, not by where it
/// stands.
fn read(host: &mut dyn Host, dimension: &str, pos: [i32; 3]) -> Option<Tag> {
    let probe = probe_tag();
    let [summon, copy, kill] = read_commands(dimension, pos, &probe);
    let mut found = None;
    if host.run_command(&summon) && host.run_command(&copy) {
        let [x, y, z] = pos;
        let (cx, cy, cz) = (
            f64::from(x) + 0.5,
            f64::from(y) + 0.5,
            f64::from(z) + 0.5,
        );
        let near = |d: f64| -> [f64; 3] { [cx + d, cy + d, cz + d] };
        for entity in host.item_entities(dimension, near(-1.0), near(1.0)) {
            let Some(data) = entity.stack.data().and_then(|d| d.other(CUSTOM_DATA)) else {
                continue;
            };
            if data.get(PROBE_KEY).and_then(tag_string).as_deref() != Some(probe.as_str()) {
                continue;
            }
            if let Some(tag) = data.get("spawner") {
                found = Some(tag.clone());
                break;
            }
        }
    }
    host.run_command(&kill);
    host.run_command(&sweep_command(dimension));
    found
}

/// The `/data merge block` that puts `snbt` into a spawner, gated on the block
/// still being one: a spawner broken between the click and this tick leaves
/// nothing for the merge to land in.
fn merge_command(dimension: &str, pos: [i32; 3], snbt: &str) -> String {
    let [x, y, z] = pos;
    format!(
        "execute in {dimension} if block {x} {y} {z} {SPAWNER} run \
         data merge block {x} {y} {z} {snbt}"
    )
}

fn write(host: &mut dyn Host, dimension: &str, pos: [i32; 3], snbt: &str) -> bool {
    host.run_command(&merge_command(dimension, pos, snbt))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(pairs: impl IntoIterator<Item = (&'static str, Tag)>) -> Tag {
        Tag::compound(pairs)
    }

    fn text(value: &str) -> Tag {
        Tag::String(value.into())
    }

    #[test]
    fn a_fresh_spawner_holds_vanillas_numbers() {
        let stats = Stats::from_tag(&tag([]));
        assert_eq!(stats.get(Stat::MinDelay), 200.0);
        assert_eq!(stats.get(Stat::MaxDelay), 800.0);
        assert_eq!(stats.get(Stat::SpawnCount), 4.0);
        assert_eq!(stats.get(Stat::MaxNearby), 6.0);
        assert_eq!(stats.get(Stat::PlayerRange), 16.0);
        assert_eq!(stats.get(Stat::SpawnRange), 4.0);
        assert_eq!(stats.get(Stat::InitialHealth), 1.0);
        assert_eq!(stats.flags, 0);
    }

    #[test]
    fn every_stat_reads_its_own_key() {
        let stats = Stats::from_tag(&tag([
            ("MinSpawnDelay", Tag::Int(60)),
            ("MaxSpawnDelay", Tag::Int(120)),
            ("SpawnCount", Tag::Int(2)),
            ("MaxNearbyEntities", Tag::Int(9)),
            ("RequiredPlayerRange", Tag::Int(24)),
            ("SpawnRange", Tag::Int(6)),
            ("InitialHealth", Tag::Float(0.5_f32.to_bits())),
            ("Echoing", Tag::Int(2)),
        ]));
        assert_eq!(stats.get(Stat::MinDelay), 60.0);
        assert_eq!(stats.get(Stat::MaxDelay), 120.0);
        assert_eq!(stats.get(Stat::SpawnCount), 2.0);
        assert_eq!(stats.get(Stat::MaxNearby), 9.0);
        assert_eq!(stats.get(Stat::PlayerRange), 24.0);
        assert_eq!(stats.get(Stat::SpawnRange), 6.0);
        assert_eq!(stats.get(Stat::InitialHealth), 0.5);
        assert_eq!(stats.get(Stat::Echoing), 2.0);
    }

    #[test]
    fn the_mob_is_spawn_data_or_the_first_potential() {
        let direct = tag([(
            "SpawnData",
            tag([("entity", tag([("id", text("minecraft:zombie"))]))]),
        )]);
        assert_eq!(Stats::from_tag(&direct).mob, "minecraft:zombie");

        let potential = tag([(
            "SpawnPotentials",
            Tag::List(vec![tag([
                ("data", tag([("entity", tag([("id", text("minecraft:skeleton"))]))])),
                ("weight", Tag::Int(1)),
            ])]),
        )]);
        assert_eq!(Stats::from_tag(&potential).mob, "minecraft:skeleton");
    }

    #[test]
    fn every_recipe_item_is_a_modifier_and_its_step_is_the_recipes() {
        assert_eq!(
            modifier_for("minecraft:sugar").unwrap().stat(),
            Some(Stat::MinDelay)
        );
        assert_eq!(modifier_for("minecraft:sugar").unwrap().step(), -10.0);
        assert_eq!(modifier_for("minecraft:clock").unwrap().step(), -20.0);
        assert_eq!(
            modifier_for("minecraft:fermented_spider_eye").unwrap().step(),
            2.0
        );
        assert_eq!(modifier_for("minecraft:ghast_tear").unwrap().step(), 2.0);
        assert_eq!(
            modifier_for("minecraft:prismarine_crystals").unwrap().step(),
            4.0
        );
        assert_eq!(modifier_for("minecraft:piston").unwrap().step(), 2.0);
        assert_eq!(
            modifier_for("minecraft:pointed_dripstone").unwrap().step(),
            -0.05
        );
        assert_eq!(modifier_for("minecraft:echo_shard").unwrap().step(), 1.0);
        assert!(modifier_for("minecraft:diamond").is_none());
        assert!(modifier_for("minecraft:red_wool").is_some());
        assert!(modifier_for("minecraft:white_wool").is_some());
        assert!(modifier_for("minecraft:wool").is_none());
    }

    #[test]
    fn flags_are_modifiers_of_their_own() {
        let cases = [
            ("minecraft:nether_star", Flag::IgnorePlayers),
            ("minecraft:soul_lantern", Flag::IgnoreLight),
            ("minecraft:conduit", Flag::IgnoreConditions),
            ("minecraft:chorus_fruit", Flag::NoAi),
            ("minecraft:blue_wool", Flag::Silent),
            ("minecraft:comparator", Flag::RedstoneControl),
            ("minecraft:turtle_egg", Flag::Youthful),
            ("minecraft:campfire", Flag::Burning),
        ];
        for (item, flag) in cases {
            let modifier = modifier_for(item).unwrap();
            let mut stats = Stats::default();
            assert!(apply(&mut stats, modifier, false), "{item} sets {flag:?}");
            assert!(stats.flag(flag));
            // A flag is only ever turned on, and the inverter leaves it alone.
            assert!(!apply(&mut stats, modifier, false));
            assert!(!apply(&mut stats, modifier, true));
            assert!(stats.flag(flag));
        }
    }

    #[test]
    fn a_step_stops_at_the_bound_and_the_inverter_takes_it_back() {
        let sugar = modifier_for("minecraft:sugar").unwrap();
        let mut stats = Stats::default();
        stats.set(Stat::MinDelay, 20.0);
        assert!(!apply(&mut stats, sugar, false), "20 is the floor");
        assert_eq!(stats.get(Stat::MinDelay), 20.0);
        assert!(apply(&mut stats, sugar, true));
        assert_eq!(stats.get(Stat::MinDelay), 30.0);

        let clock = modifier_for("minecraft:clock").unwrap();
        stats.set(Stat::MaxDelay, 1600.0);
        assert!(!apply(&mut stats, clock, true), "1600 is the ceiling");
        assert!(apply(&mut stats, clock, false));
        assert_eq!(stats.get(Stat::MaxDelay), 1580.0);

        let eye = modifier_for("minecraft:fermented_spider_eye").unwrap();
        stats.set(Stat::SpawnCount, 16.0);
        assert!(!apply(&mut stats, eye, false));
        assert!(apply(&mut stats, eye, true));
        assert_eq!(stats.get(Stat::SpawnCount), 14.0);
    }

    #[test]
    fn the_initial_health_moves_by_a_twentieth_and_rounds_to_two_places() {
        let dripstone = modifier_for("minecraft:pointed_dripstone").unwrap();
        let mut stats = Stats::default();
        assert!(apply(&mut stats, dripstone, false));
        assert_eq!(stats.get(Stat::InitialHealth), 0.95);
        assert!(apply(&mut stats, dripstone, true));
        assert_eq!(stats.get(Stat::InitialHealth), 1.0);
        stats.set(Stat::InitialHealth, 0.20);
        assert!(!apply(&mut stats, dripstone, false));
        assert_eq!(stats.get(Stat::InitialHealth), 0.20);
    }

    #[test]
    fn the_merge_compound_holds_every_key_and_stays_integer_where_it_should() {
        let mut stats = Stats::default();
        stats.set(Stat::MinDelay, 90.0);
        stats.set_flag(Flag::Silent, true);
        stats.set(Stat::InitialHealth, 0.75);
        let merged = stats.merge_snbt();
        assert!(merged.starts_with('{') && merged.ends_with('}'));
        assert!(merged.contains("MinSpawnDelay:90,"));
        assert!(merged.contains("InitialHealth:0.75f"));
        assert!(merged.contains("Silent:1b"));
        assert!(merged.contains("NoAI:0b"));
        assert!(!merged.contains("MinDelay"));
    }

    #[test]
    fn snbt_writes_every_tag_type_and_quotes_text() {
        let value = tag([
            ("byte", Tag::Byte(3)),
            ("int", Tag::Int(-7)),
            ("float", Tag::Float(0.5_f32.to_bits())),
            ("text", text("a \"quoted\" name")),
            ("list", Tag::List(vec![Tag::Int(1), Tag::Int(2)])),
            ("empty", Tag::List(Vec::new())),
        ]);
        let written = snbt(&value);
        assert!(written.contains("byte:3b"));
        assert!(written.contains("int:-7"));
        assert!(written.contains("float:0.5f"));
        assert!(written.contains("text:\"a \\\"quoted\\\" name\""));
        assert!(written.contains("list:[1,2]"));
        assert!(written.contains("empty:[]"));
    }

    #[test]
    fn a_restore_leaves_the_type_and_position_out() {
        let data = tag([
            ("id", text(BLOCK_ENTITY_ID)),
            ("x", Tag::Int(1)),
            ("y", Tag::Int(2)),
            ("z", Tag::Int(3)),
            ("MinSpawnDelay", Tag::Int(40)),
            ("SpawnData", tag([("entity", tag([("id", text("minecraft:cow"))]))])),
        ]);
        let merged = mergeable(&data);
        assert!(merged.starts_with("{MinSpawnDelay:40,"), "{merged}");
        assert!(!merged.contains(BLOCK_ENTITY_ID));
        assert!(!merged.contains("x:1"));
        assert!(!merged.contains("y:2"));
        assert!(!merged.contains("z:3"));
        assert!(merged.contains("minecraft:cow"));
    }

    #[test]
    fn a_spawner_item_carries_its_data_with_its_type() {
        let data = tag([("MinSpawnDelay", Tag::Int(40))]);
        let stack = spawner_item(&data);
        assert_eq!(&*stack.item, SPAWNER);
        assert_eq!(stack.count, 1);
        let carried = stack
            .data()
            .and_then(|d| d.other(BLOCK_ENTITY_DATA))
            .expect("block entity data");
        assert_eq!(carried.get("MinSpawnDelay"), Some(&Tag::Int(40)));
        assert_eq!(
            carried.get("id").and_then(tag_string).as_deref(),
            Some(BLOCK_ENTITY_ID)
        );
    }

    #[test]
    fn silk_touch_is_read_from_the_enchantments_component() {
        let mut stack = ItemStack::new("minecraft:diamond_pickaxe", 1);
        assert_eq!(silk_touch(&stack), 0);
        stack.data_mut().set_other(
            "minecraft:enchantments",
            Some(tag([(
                "levels",
                tag([("minecraft:silk_touch", Tag::Int(1))]),
            )])),
        );
        assert_eq!(silk_touch(&stack), 1);
        let mut plain = ItemStack::new("minecraft:diamond_pickaxe", 1);
        plain.data_mut().set_other(
            "minecraft:enchantments",
            Some(tag([("levels", tag([("minecraft:fortune", Tag::Int(3))]))])),
        );
        assert_eq!(silk_touch(&plain), 0);
    }

    #[test]
    fn the_cache_hands_out_what_was_read_once() {
        assert!(cached_drop("minecraft:overworld", [1, 2, 3]).is_none());
        remember("minecraft:overworld", [1, 2, 3], tag([("a", Tag::Int(1))]), 0);
        let tag = cached_drop("minecraft:overworld", [1, 2, 3]).expect("cached");
        assert_eq!(tag.get("a"), Some(&Tag::Int(1)));
        assert!(cached_drop("minecraft:overworld", [1, 2, 3]).is_none());
    }

    #[test]
    fn a_read_names_its_probe_and_gates_its_commands_on_the_spawner() {
        let [summon, copy, kill] = read_commands(
            "minecraft:overworld",
            [10, 64, -3],
            "apoth_spawner_probe_7",
        );
        assert!(
            summon.starts_with(
                "execute in minecraft:overworld if block 10 64 -3 minecraft:spawner run \
                 summon minecraft:item 10.5 64.5 -2.5"
            ),
            "{summon}"
        );
        // The probe carries its own name, so a probe left behind by an earlier
        // read is never mistaken for this one.
        assert!(summon.contains("apoth_probe\":\"apoth_spawner_probe_7"), "{summon}");
        assert!(!summon.contains("apoth_spawner_probe_6"), "{summon}");
        assert!(
            copy.contains("if block 10 64 -3 minecraft:spawner run data modify entity"),
            "{copy}"
        );
        assert!(copy.contains("@e[type=item,tag=apoth_spawner_probe_7,limit=1]"), "{copy}");
        assert!(copy.ends_with("set from block 10 64 -3"), "{copy}");
        // The probe goes even when the block is not a spawner any more.
        assert!(
            kill.ends_with("kill @e[type=item,tag=apoth_spawner_probe_7]"),
            "{kill}"
        );
        assert!(!kill.contains("if block"), "{kill}");
    }

    #[test]
    fn every_read_finds_a_probe_name_of_its_own() {
        let first = probe_tag();
        let second = probe_tag();
        assert_ne!(first, second);
        assert!(first.starts_with(PROBE), "{first}");
        assert!(sweep_command("minecraft:the_end").ends_with("kill @e[type=item,tag=apoth_spawner_probe]"));
    }

    #[test]
    fn a_merge_only_lands_in_a_spawner() {
        let merge = merge_command("minecraft:the_nether", [1, 2, 3], "{MinSpawnDelay:20}");
        assert!(
            merge.starts_with(
                "execute in minecraft:the_nether if block 1 2 3 minecraft:spawner run \
                 data merge block 1 2 3 "
            ),
            "{merge}"
        );
        assert!(merge.ends_with("{MinSpawnDelay:20}"), "{merge}");
    }

    #[test]
    fn a_failed_read_takes_the_old_entry_out() {
        let (dimension, pos) = ("minecraft:overworld", [21, 22, 23]);
        remember(dimension, pos, tag([("a", Tag::Int(1))]), 10);
        // The spawner that was read is gone: its data must not come back for
        // whatever stands there now.
        settle(dimension, pos, None, 11);
        assert!(cached_drop(dimension, pos).is_none());
        // A fresh read of that block replaces what an older one left.
        remember(dimension, pos, tag([("a", Tag::Int(1))]), 12);
        settle(dimension, pos, Some(tag([("a", Tag::Int(2))])), 13);
        let cached = cached_drop(dimension, pos).expect("cached");
        assert_eq!(cached.get("a"), Some(&Tag::Int(2)));
    }

    #[test]
    fn a_read_that_waited_too_long_for_its_break_is_forgotten() {
        let (dimension, pos) = ("minecraft:overworld", [31, 32, 33]);
        remember(dimension, pos, tag([("a", Tag::Int(1))]), 0);
        forget_old(CACHE_TICKS - 1);
        assert!(cached_drop(dimension, pos).is_some());
        remember(dimension, pos, tag([("a", Tag::Int(1))]), 0);
        forget_old(CACHE_TICKS);
        assert!(cached_drop(dimension, pos).is_none());
    }

    #[test]
    fn the_tooltip_says_which_mob_and_which_stats() {
        let mut stats = Stats::default();
        stats.mob = "minecraft:zombie".into();
        stats.set(Stat::MinDelay, 60.0);
        stats.set_flag(Flag::Burning, true);
        let lines = tooltip(&stats);
        assert_eq!(lines[0].1, MOB_COLOUR);
        assert_eq!(lines[1].1, STAT_COLOUR);
        let rendered = lines
            .iter()
            .map(|(text, _)| format!("{:?}", text.spans))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("zombie"), "{rendered}");
        assert!(rendered.contains("60"), "{rendered}");
        assert!(rendered.contains("burning"), "{rendered}");
        assert!(rendered.contains("Mob: "), "{rendered}");
        // Neither line appears while health and echo are the vanilla ones.
        assert!(!rendered.contains("Echo"), "{rendered}");
        assert!(!rendered.contains("health"), "{rendered}");
        assert_eq!(lines.last().expect("a flag line").1, FLAG_COLOUR);
    }

    #[test]
    fn a_tooltip_shows_the_health_and_the_echo_once_a_modifier_moved_them() {
        let mut stats = Stats::default();
        stats.mob = "minecraft:cow".into();
        stats.set_flag(Flag::Silent, true);
        stats.set(Stat::InitialHealth, 0.8);
        stats.set(Stat::Echoing, 2.0);
        let rendered = tooltip(&stats)
            .iter()
            .map(|(text, _)| format!("{:?}", text.spans))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("0.8"), "{rendered}");
        assert!(rendered.contains("Echo"), "{rendered}");
        assert!(rendered.contains("silent"), "{rendered}");
    }

    #[test]
    fn the_summary_names_the_mob_and_shows_the_delays() {
        let mut stats = Stats::default();
        stats.mob = "minecraft:zombie".into();
        stats.set(Stat::MinDelay, 60.0);
        stats.set_flag(Flag::Silent, true);
        let line = summary(&stats);
        let rendered = format!("{:?}", line.spans);
        assert!(rendered.contains("zombie"));
        assert!(rendered.contains("silent"));
    }
}
