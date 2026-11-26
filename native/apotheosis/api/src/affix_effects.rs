//! Affixes whose payout is an effect, not an attribute, and the Apothic
//! Invader boss.
//!
//! Clean-room: ids and public numbers come from Apotheosis' data files; no
//! Apotheosis source or texture is copied. The catalogue (`java_affixes`)
//! carries Java's per-rarity rolls for every effect affix this port can pay
//! out: duration, amplifier, chance, targets, cooldown and roll count. A piece
//! rolls the ones its rarity guarantees as extra affixes (`effect_counts`), so
//! an item's attribute line count is unchanged; the rolls are written to
//! `minecraft:custom_data`
//! (`apotheosis:effects`) and read back at the hook that pays it out.
//!
//! `supported` in the catalogue means a hook here can pay the affix. A bow's
//! `arrow_target` needs the projectile hit, which this API gives no host for,
//! so only the trident half of those affixes is paid; `break_self`,
//! `stoneforming`, `radial` and Apothic's own effects never enter the pool.
//! See the README for the list.
//!
//! Bosses use the two spawn hooks in order: `entity_join_level` names the
//! entity id before `mob_spawned` says the spawn was natural, so joins wait in
//! a short queue for their spawn. A natural surface spawn at night (overworld)
//! or in the Nether/End may become an invader: a generated name, more health,
//! affixed high-rarity gear, the `apotheosis:boss` tag, and an announcement.

use crate::affixes::roll_range;
use crate::attributes::{self, ItemClass};
use crate::java_affixes::{Affix, AFFIXES};
use crate::tiers::{self, WorldTier};
use crate::{Rng, Tier, apply_affixes, one};
use horizon_client::mod_api::{
    Compound, EntityJoinLevelEvent, EquipmentSlot, Flow, Host, ItemDamagedEvent, ItemStack,
    LivingIncomingDamageV2Event, LootDropsEvent, MobSpawnedEvent, NativeEffect, NativeHeightmap, Tag,
    Text, serde_json,
};
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// `minecraft:custom_data` key holding this port's effect affix rolls.
const EFFECTS_KEY: &str = "apotheosis:effects";

/// `minecraft:custom_data` key recording the enchantment levels the last effect
/// affix added, so the next pass takes them back before adding its own.
const EFFECT_ENCHANTS_KEY: &str = "apotheosis:effect_enchants";

/// The port's own durability affix. Apotheosis 8.9's data has no `durable`
/// affix, so it is not in the generated catalogue; it stays here for the gear
/// rolled before the catalogue reached this slice.
const DURABLE_ID: &str = "apotheosis:durable";

/// What an effect affix does when it is paid out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    MobEffect,
    DamageReduction,
    Enchantment,
    Execution,
    Cleaving,
    Thunderstruck,
    Retreating,
    Telepathic,
    Festive,
    Durable,
}

/// One entry of the effect pool: a catalogue affix, or the port's durable.
pub struct Entry {
    id: &'static str,
    kind: Kind,
    affix: Option<&'static Affix>,
    /// Indices into `Tier` (common..mythic), as the catalogue spells them.
    rarities: &'static [u8],
    /// The item classes it may roll on (`attributes::MELEE` and the like).
    classes: u16,
}

impl Entry {
    /// Java's `affix_type` (`basic_effect` or `ability`). The port's own durable
    /// affix is not in the Java data and fills a basic slot.
    fn affix_type(&self) -> &'static str {
        self.affix.map_or("basic_effect", |affix| affix.affix_type)
    }
}

/// The effect affixes Java's rarity data guarantees at `tier`: the number of
/// `basic_effect` rules and `ability` rules in `data/apotheosis/rarities/*.json`
/// (common 0/0, uncommon 0/0, rare 1/0, epic 1/1, mythic 2/1). Ancient is this
/// port's extension and reuses mythic's row. The chanced extra rules are not
/// modelled, so a piece never rolls more than this.
pub fn effect_counts(tier: Tier) -> (usize, usize) {
    match tier {
        Tier::Common | Tier::Uncommon => (0, 0),
        Tier::Rare => (1, 0),
        Tier::Epic => (1, 1),
        Tier::Mythic | Tier::Ancient => (2, 1),
    }
}

/// The effect kind of a catalogue affix, when a hook can pay it.
pub fn kind_of(affix: &Affix) -> Option<Kind> {
    if !affix.supported {
        return None;
    }
    Some(match affix.kind {
        "mob_effect" => Kind::MobEffect,
        "damage_reduction" => Kind::DamageReduction,
        "enchantment" => Kind::Enchantment,
        "executing" => Kind::Execution,
        "cleaving" => Kind::Cleaving,
        "thunderstruck" => Kind::Thunderstruck,
        "retreating" => Kind::Retreating,
        "telepathic" => Kind::Telepathic,
        "festive" => Kind::Festive,
        _ => return None,
    })
}

/// Every effect affix this port rolls, in catalogue order, plus `durable`.
fn catalogue() -> &'static [Entry] {
    static ENTRIES: OnceLock<Vec<Entry>> = OnceLock::new();
    ENTRIES.get_or_init(|| {
        let mut entries: Vec<Entry> = AFFIXES
            .iter()
            .filter_map(|affix| {
                Some(Entry {
                    id: affix.id,
                    kind: kind_of(affix)?,
                    affix: Some(affix),
                    rarities: affix.rarities,
                    classes: crate::affixes::classes(affix),
                })
            })
            .collect();
        entries.push(Entry {
            id: DURABLE_ID,
            kind: Kind::Durable,
            affix: None,
            rarities: &[3, 4],
            classes: attributes::ARMOR
                | attributes::MELEE
                | attributes::TOOL
                | attributes::RANGED
                | attributes::SHIELD,
        });
        entries
    })
}

/// One rolled effect affix: Java's numbers, drawn once when the piece rolls.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Roll {
    /// Ticks, for a timed effect.
    pub duration: f64,
    pub amplifier: u8,
    /// Ticks between two payouts of this affix on the same entity.
    pub cooldown: u32,
    pub chance: f64,
    pub targets: f64,
    pub rolls: u32,
    /// The plain-value magnitude of the affix (a threshold, damage, ...).
    pub value: f64,
}

/// Draws one affix's roll for `tier` from the catalogue's Java ranges.
fn draw(affix: &Affix, tier: Tier, rng: &mut Rng) -> Roll {
    let index = tier.rarity_index().min(4);
    let mut roll = Roll::default();
    if let Some(effect) = affix.effects.get(index).copied().flatten() {
        if let Some(range) = effect.duration {
            roll.duration = roll_range(range, rng).round();
        }
        if let Some(range) = effect.amplifier {
            roll.amplifier = roll_range(range, rng).round().clamp(0.0, 9.0) as u8;
        }
        if let Some(range) = effect.chance {
            roll.chance = roll_range(range, rng).clamp(0.0, 1.0);
        }
        if let Some(range) = effect.targets {
            roll.targets = roll_range(range, rng).round().max(1.0);
        }
        roll.cooldown = effect.cooldown;
        roll.rolls = effect.rolls;
    }
    if let Some(range) = affix.values.get(index).copied().flatten() {
        roll.value = roll_range(range, rng);
    }
    roll
}

/// The effect affixes an item of `class` at `tier` can roll, for README checks.
pub fn available(class: ItemClass, tier: Tier) -> Vec<&'static str> {
    catalogue()
        .iter()
        .filter(|entry| {
            entry.rarities.contains(&(tier.rarity_index() as u8))
                && attributes::accepts(entry.classes, class)
        })
        .map(|entry| entry.id)
        .collect()
}

/// Picks an effect affix of `affix_type` for `class` at `tier`, if one is
/// available that the piece does not already carry.
fn choose(
    class: ItemClass,
    tier: Tier,
    affix_type: &str,
    taken: &[&'static str],
    rng: &mut Rng,
) -> Option<&'static Entry> {
    let pool: Vec<&'static Entry> = catalogue()
        .iter()
        .filter(|entry| {
            entry.affix_type() == affix_type
                && !taken.contains(&entry.id)
                && entry.rarities.contains(&(tier.rarity_index() as u8))
                && attributes::accepts(entry.classes, class)
        })
        .collect();
    if pool.is_empty() {
        return None;
    }
    Some(pool[rng.below(pool.len() as u64) as usize])
}

/// `7` for 7%, `12.5` for a twelfth of a point over.
fn percent(value: f64) -> String {
    let pct = value * 100.0;
    if (pct - pct.round()).abs() < 0.05 {
        format!("{pct:.0}")
    } else {
        format!("{pct:.1}")
    }
}

/// `6` for six seconds, `7.5` when the roll kept a half tick.
fn seconds(ticks: f64) -> String {
    let value = ticks / 20.0;
    if (value - value.round()).abs() < 0.05 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

fn roman(amplifier: u8) -> &'static str {
    ["", " II", " III", " IV", " V", " VI", " VII", " VIII", " IX", " X"]
        .get(usize::from(amplifier))
        .copied()
        .unwrap_or("")
}

/// A vanilla effect's translation key and English fallback. The key is the
/// game's own `effect.<namespace>.<path>`, so the effect name translates like
/// any other component rather than staying English in the lore.
fn effect_name(effect: &str, amplifier: u8) -> (String, String) {
    let path = effect.rsplit(':').next().unwrap_or(effect);
    let fallback = match path {
        "blindness" => "Blindness",
        "poison" => "Poison",
        "weakness" => "Weakness",
        "slowness" => "Slowness",
        "wither" => "Wither",
        "speed" => "Speed",
        "resistance" => "Resistance",
        "instant_health" => "Instant Health",
        "levitation" => "Levitation",
        "haste" => "Haste",
        _ => path,
    };
    (
        format!("effect.{}", effect.replacen(':', ".", 1)),
        format!("{fallback}{}", roman(amplifier)),
    )
}

/// An effect as a translatable argument: the game's own effect key, with the
/// amplifier as vanilla's `enchantment.level.*` line after it.
fn effect_arg(effect: &str, amplifier: u8) -> Tag {
    let (key, fallback) = effect_name(effect, amplifier);
    let mut fields: Vec<(&'static str, Tag)> = vec![
        ("translate", string(&key)),
        ("fallback", string(&fallback)),
    ];
    if amplifier > 0 {
        fields.push((
            "extra",
            Tag::List(vec![
                string(" "),
                Tag::compound([
                    (
                        "translate",
                        string(&format!("enchantment.level.{}", amplifier + 1)),
                    ),
                    ("fallback", string(roman(amplifier).trim())),
                ]),
            ]),
        ));
    }
    Tag::compound(fields)
}

/// The English text of a lore argument, for the fallback string.
fn arg_text(arg: &Tag) -> String {
    match arg {
        Tag::String(text) => text.to_string_lossy(),
        Tag::Compound(fields) => match fields
            .get(&"fallback".into())
            .or_else(|| fields.get(&"text".into()))
        {
            Some(Tag::String(text)) => text.to_string_lossy(),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

/// A damage class's name key and English fallback.
fn damage_name(kind: &str) -> (String, String) {
    let fallback = match kind {
        "explosion" => "Explosion",
        "physical" => "Physical",
        "projectile" => "Projectile",
        "fall" => "Fall",
        "lightning" => "Lightning",
        "magic" => "Magic",
        _ => kind,
    };
    (format!("apotheosis.damage.{kind}"), fallback.to_string())
}

fn fill(template: &str, args: &[String]) -> String {
    let mut text = template.to_string();
    for arg in args {
        if let Some(at) = text.find("%s") {
            text.replace_range(at..at + 2, arg);
        }
    }
    text
}

/// The lore line, its English fallback, and the values the language file
/// fills in. The keys are this port's own; Java's line is computed, not keyed.
fn description(entry: &Entry, roll: Roll) -> (String, String, Vec<Tag>) {
    let generic = |kind: &str, template: &str, args: Vec<Tag>| {
        let text: Vec<String> = args.iter().map(arg_text).collect();
        (
            format!("apotheosis.affix.effect.{kind}"),
            fill(template, &text),
            args,
        )
    };
    let literal = |value: String| Tag::String(value.as_str().into());
    match entry.kind {
        Kind::MobEffect => {
            let effect = entry.affix.and_then(|a| a.effect).unwrap_or("minecraft:unknown");
            generic(
                "effect",
                "%s for %s s",
                vec![
                    effect_arg(effect, roll.amplifier),
                    literal(seconds(roll.duration)),
                ],
            )
        }
        Kind::DamageReduction => {
            let kind = entry.affix.and_then(|a| a.damage_kind).unwrap_or("physical");
            let (_, name) = damage_name(kind);
            generic(
                "reduction",
                "-%s%% %s damage",
                vec![literal(percent(roll.value)), literal(name)],
            )
        }
        Kind::Execution => generic(
            "executing",
            "Executes below %s%% health",
            vec![literal(percent(roll.value))],
        ),
        Kind::Cleaving => generic(
            "cleaving",
            "%s%% chance to hit %s foes",
            vec![
                literal(percent(roll.chance)),
                literal(format!("{:.0}", roll.targets)),
            ],
        ),
        Kind::Thunderstruck => generic(
            "thunderstruck",
            "Lightning for %s damage",
            vec![literal(format!("{:.0}", roll.value))],
        ),
        Kind::Festive => generic(
            "festive",
            "%s%% chance of extra loot",
            vec![literal(percent(roll.chance))],
        ),
        Kind::Durable => generic(
            "durable",
            "%s%% less wear",
            vec![literal(percent(roll.value))],
        ),
        Kind::Enchantment => {
            let enchantment = entry.affix.and_then(|a| a.enchantment).unwrap_or("minecraft:fortune");
            let path = enchantment.rsplit(':').next().unwrap_or(enchantment);
            let fallback = if path == "fortune" { "Fortune" } else { path };
            generic(
                "enchantment",
                "+%s %s",
                vec![
                    literal(format!("{:.0}", roll.value)),
                    literal(fallback.to_string()),
                ],
            )
        }
        Kind::Retreating => (
            "apotheosis.affix.effect.retreating".into(),
            "Knocks attackers back on block".into(),
            Vec::new(),
        ),
        Kind::Telepathic => (
            "apotheosis.affix.effect.telepathic".into(),
            "Drops go to your inventory".into(),
            Vec::new(),
        ),
    }
}

/// Adds the effect affixes Java's rarity guarantees for `tier`, each with its
/// lore line and its roll in `minecraft:custom_data`. Called by `apply_affixes`
/// after the attribute affixes, so their count is untouched.
pub fn add_effect_affix(stack: &mut ItemStack, tier: Tier, rng: &mut Rng) -> Vec<&'static str> {
    let class = attributes::class_of(&stack.item);
    // A reroll drops the previous effects: their lore, rolls and damage
    // reductions, and the enchantment levels they added.
    clear_effects(stack);
    attributes::clear_reduction(stack);
    let (basic, abilities) = effect_counts(tier);
    let mut added: Vec<&'static str> = Vec::with_capacity(basic + abilities);
    for affix_type in std::iter::repeat_n("basic_effect", basic)
        .chain(std::iter::repeat_n("ability", abilities))
    {
        let Some(entry) = choose(class, tier, affix_type, &added, rng) else {
            // No supported affix of that kind for this class and tier: a tool
            // has no supported ability at all, for instance.
            continue;
        };
        let roll = match entry.affix {
            Some(affix) => draw(affix, tier, rng),
            None => Roll {
                value: (0.15 * tier.scale()).min(0.5),
                ..Roll::default()
            },
        };
        let (key, fallback, args) = description(entry, roll);

        // The enchantment affix is permanent, not a hit effect: it adds levels
        // to the item's own component, which the game then applies itself.
        if entry.kind == Kind::Enchantment {
            if let Some(affix) = entry.affix {
                add_enchantment(stack, affix.enchantment.unwrap_or("minecraft:fortune"), roll.value);
            }
        }
        // Damage reduction is read from the armour cache, not from a hit hook.
        if entry.kind == Kind::DamageReduction {
            if let Some(affix) = entry.affix {
                attributes::attach_reduction(
                    stack,
                    affix.damage_kind.unwrap_or("physical"),
                    roll.value,
                );
            }
        }
        store(stack, entry.id, roll);
        append_lore(stack, &key, &fallback, &args);
        added.push(entry.id);
    }
    added
}

/// Appends one effect line after the attribute lines.
fn append_lore(stack: &mut ItemStack, key: &str, fallback: &str, args: &[Tag]) {
    let mut lore = match stack
        .data()
        .and_then(|data| data.other("minecraft:lore"))
    {
        Some(Tag::List(lines)) => lines.clone(),
        _ => Vec::new(),
    };
    lore.push(Tag::compound([
        ("translate", string(key)),
        ("fallback", string(fallback)),
        ("color", string("#55FF55")),
        ("italic", Tag::Byte(0)),
        ("with", Tag::List(args.to_vec())),
    ]));
    stack
        .data_mut()
        .set_other("minecraft:lore", Some(Tag::List(lore)));
}

/// Writes one roll into the stack's `minecraft:custom_data`.
fn store(stack: &mut ItemStack, id: &str, roll: Roll) {
    let mut custom = custom_data(stack);
    let mut effects = match custom.get(&EFFECTS_KEY.into()) {
        Some(Tag::Compound(compound)) => compound.clone(),
        _ => Compound::new(),
    };
    effects.insert(
        id.into(),
        Tag::compound([
            ("duration", Tag::Double(roll.duration.to_bits())),
            ("amplifier", Tag::Byte(roll.amplifier as i8)),
            ("cooldown", Tag::Int(roll.cooldown as i32)),
            ("chance", Tag::Double(roll.chance.to_bits())),
            ("targets", Tag::Double(roll.targets.to_bits())),
            ("rolls", Tag::Int(roll.rolls as i32)),
            ("value", Tag::Double(roll.value.to_bits())),
        ]),
    );
    custom.insert(EFFECTS_KEY.into(), Tag::Compound(effects));
    stack
        .data_mut()
        .set_other("minecraft:custom_data", Some(Tag::Compound(custom)));
}

fn custom_data(stack: &ItemStack) -> Compound {
    match stack
        .data()
        .and_then(|data| data.other("minecraft:custom_data"))
    {
        Some(Tag::Compound(compound)) => compound.clone(),
        _ => Compound::new(),
    }
}

/// Drops the effect rolls a previous affix pass wrote, and the enchantment
/// levels that pass added.
fn clear_effects(stack: &mut ItemStack) {
    remove_effect_enchantments(stack);
    let mut custom = custom_data(stack);
    if custom.remove(&EFFECTS_KEY.into()).is_some() {
        stack
            .data_mut()
            .set_other("minecraft:custom_data", Some(Tag::Compound(custom)));
    }
}

/// The effect affix rolls an item carries.
pub fn effects_of(stack: &ItemStack) -> Vec<(&'static Entry, Roll)> {
    let mut found = Vec::new();
    let Some(Tag::Compound(custom)) = stack.data().and_then(|data| data.other("minecraft:custom_data"))
    else {
        return found;
    };
    let Some(Tag::Compound(effects)) = custom.get(&EFFECTS_KEY.into()) else {
        return found;
    };
    for entry in catalogue() {
        let Some(Tag::Compound(roll)) = effects.get(&entry.id.into()) else {
            continue;
        };
        found.push((
            entry,
            Roll {
                duration: number(roll.get(&"duration".into())).unwrap_or(0.0),
                amplifier: number(roll.get(&"amplifier".into())).unwrap_or(0.0) as u8,
                cooldown: number(roll.get(&"cooldown".into())).unwrap_or(0.0) as u32,
                chance: number(roll.get(&"chance".into())).unwrap_or(0.0),
                targets: number(roll.get(&"targets".into())).unwrap_or(0.0),
                rolls: number(roll.get(&"rolls".into())).unwrap_or(0.0) as u32,
                value: number(roll.get(&"value".into())).unwrap_or(0.0),
            },
        ));
    }
    found
}

/// Adds whole levels of `id` to the item's own enchantments, the typed
/// component the game reads for Fortune and Looting, and records them so the
/// next pass can take them back without touching the item's own levels.
fn add_enchantment(stack: &mut ItemStack, id: &str, levels: f64) {
    let add = levels.round().max(0.0) as u32;
    if add == 0 {
        return;
    }
    let mut table: Vec<(std::sync::Arc<str>, u32)> = stack.enchantments().to_vec();
    match table.iter_mut().find(|(key, _)| &**key == id) {
        Some((_, level)) => *level += add,
        None => table.push((std::sync::Arc::from(id), add)),
    }
    stack.data_mut().enchantments = table;
    let mut custom = custom_data(stack);
    let mut recorded = match custom.get(&EFFECT_ENCHANTS_KEY.into()) {
        Some(Tag::Compound(compound)) => compound.clone(),
        _ => Compound::new(),
    };
    let before = number(recorded.get(&id.into())).unwrap_or(0.0) as u32;
    recorded.insert(id.into(), Tag::Int((before + add) as i32));
    custom.insert(EFFECT_ENCHANTS_KEY.into(), Tag::Compound(recorded));
    stack
        .data_mut()
        .set_other("minecraft:custom_data", Some(Tag::Compound(custom)));
}

/// Takes back the enchantment levels the last effect affix added, leaving the
/// item's own enchantments (or another module's) where they were.
fn remove_effect_enchantments(stack: &mut ItemStack) {
    let mut custom = custom_data(stack);
    let Some(Tag::Compound(recorded)) = custom.remove(&EFFECT_ENCHANTS_KEY.into()) else {
        return;
    };
    let mut table: Vec<(std::sync::Arc<str>, u32)> = stack.enchantments().to_vec();
    for (id, value) in recorded.iter() {
        let Tag::Int(level) = value else {
            continue;
        };
        let key = id.to_string_lossy();
        if let Some(index) = table.iter().position(|(name, _)| &**name == key.as_str()) {
            let left = table[index]
                .1
                .saturating_sub(u32::try_from(*level).unwrap_or(0));
            if left == 0 {
                table.remove(index);
            } else {
                table[index].1 = left;
            }
        }
    }
    stack.data_mut().enchantments = table;
    stack
        .data_mut()
        .set_other("minecraft:custom_data", Some(Tag::Compound(custom)));
}

/// Per-entity, per-affix cooldowns, in the ticks Java's data names.
static COOLDOWNS: OnceLock<Mutex<HashMap<(u32, &'static str), Instant>>> = OnceLock::new();

fn cooldowns() -> &'static Mutex<HashMap<(u32, &'static str), Instant>> {
    COOLDOWNS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Whether `source` may pay `id` now; records the payout when it may. A zero
/// cooldown means every hit.
fn ready(source: u32, id: &'static str, ticks: u32) -> bool {
    if ticks == 0 {
        return true;
    }
    let wait = Duration::from_millis(u64::from(ticks) * 50);
    let mut map = cooldowns()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    map.retain(|_, at| at.elapsed() < Duration::from_secs(120));
    if map
        .get(&(source, id))
        .is_some_and(|at| at.elapsed() < wait)
    {
        return false;
    }
    map.insert((source, id), Instant::now());
    true
}

fn number(tag: Option<&Tag>) -> Option<f64> {
    match tag? {
        Tag::Double(bits) => Some(f64::from_bits(*bits)),
        Tag::Float(bits) => Some(f64::from(f32::from_bits(*bits))),
        Tag::Int(value) => Some(f64::from(*value)),
        _ => None,
    }
}

fn string(value: &str) -> Tag {
    Tag::String(value.into())
}

/// A roll stream for effect hooks: nothing in the event is random, so mix a
/// counter with the entities and the amount.
static ROLLS: AtomicU64 = AtomicU64::new(0);

fn roll(seed: u64) -> Rng {
    Rng::new(seed ^ ROLLS.fetch_add(1, Ordering::Relaxed).wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

fn entity_of(id: i32) -> Option<u32> {
    u32::try_from(id).ok()
}

// ---------------------------------------------------------------------------
// Effects on a victim
// ---------------------------------------------------------------------------

/// Adds `effect` to any living entity: players through the effect API, other
/// mobs by merging their `active_effects` list (the shape `/data merge` reads).
fn add_effect(host: &mut dyn Host, id: u32, effect: &str, duration: i32, amplifier: u8) {
    let Some(entity) = host.entity(id) else {
        return;
    };
    if host.entity_kind(entity).as_deref() == Some("minecraft:player") {
        if let Some(uuid) = host.entity_uuid(entity) {
            let _ = host.add_effect(
                uuid,
                &NativeEffect {
                    id: effect.into(),
                    amplifier,
                    duration,
                    ambient: false,
                    particles: true,
                },
            );
        }
        return;
    }
    let Some(mut nbt) = host.entity_nbt(entity) else {
        return;
    };
    let mut list = match nbt.get(&"active_effects".into()) {
        Some(Tag::List(list)) => list.clone(),
        _ => Vec::new(),
    };
    list.push(Tag::compound([
        ("id", string(effect)),
        ("duration", Tag::Int(duration)),
        ("amplifier", Tag::Byte(amplifier as i8)),
        ("ambient", Tag::Byte(0)),
        ("show_particles", Tag::Byte(1)),
    ]));
    nbt.insert("active_effects".into(), Tag::List(list));
    let _ = host.entity_merge_nbt(entity, nbt);
}

/// The victim's health and maximum, from its own NBT.
fn health(host: &mut dyn Host, id: u32) -> Option<(f32, f32)> {
    let entity = host.entity(id)?;
    let nbt = host.entity_nbt(entity)?;
    let current = match nbt.get(&"Health".into()) {
        Some(Tag::Float(bits)) => f32::from_bits(*bits),
        Some(Tag::Double(bits)) => f64::from_bits(*bits) as f32,
        _ => return None,
    };
    let mut max = 20.0f32;
    if let Some(Tag::List(attributes)) = nbt.get(&"attributes".into()) {
        for entry in attributes {
            let is_max = match entry.get("id") {
                Some(Tag::String(id)) => id.to_string_lossy() == "minecraft:max_health",
                _ => false,
            };
            if is_max {
                if let Some(base) = number(entry.get("base")) {
                    max = base as f32;
                }
            }
        }
    }
    Some((current, max))
}

/// The victim's worn or held stack in `slot`, for a player. Mobs' armour is in
/// their NBT instead; effect affixes on mob gear are not paid out.
fn player_slot(host: &mut dyn Host, victim: u32, slot: EquipmentSlot) -> Option<ItemStack> {
    let entity = host.entity(victim)?;
    if host.entity_kind(entity).as_deref() != Some("minecraft:player") {
        return None;
    }
    let uuid = host.entity_uuid(entity)?;
    host.player_equipment(uuid, slot)
}

/// The combat side of effect affixes, from the attacker's weapon and the
/// victim's helmet/shield. Runs after the attribute half of the hook.
pub fn on_hit(host: &mut dyn Host, event: &mut LivingIncomingDamageV2Event<'_>) {
    if event.causing == 0 || event.causing == event.entity {
        return;
    }
    let Some(attacker) = entity_of(event.causing) else {
        return;
    };
    let Some(victim) = entity_of(event.entity) else {
        return;
    };
    let mut rng = roll(
        ((u64::from(victim) << 32) | u64::from(attacker))
            ^ u64::from(event.amount.to_bits()).rotate_left(13),
    );
    let trident = event
        .weapon
        .is_some_and(|weapon| attributes::class_of(&weapon.item) == ItemClass::Trident);

    if let Some(weapon) = event.weapon {
        for (entry, roll) in effects_of(weapon) {
            let target = match entry.affix.and_then(|affix| affix.effect_target) {
                Some("attack_target") => Some(victim),
                Some("arrow_target") if trident => Some(victim),
                Some("attack_self") | Some("arrow_self") => Some(attacker),
                _ => None,
            };
            match entry.kind {
                Kind::MobEffect => {
                    if let Some(target) = target {
                        pay_effect(host, entry, roll, attacker, target, &mut rng);
                    }
                }
                Kind::Execution => {
                    if rng.chance((roll.chance * 100.0).round() as u64) {
                        if let Some((current, max)) = health(host, victim) {
                            let after = current - event.amount;
                            if after > 0.0 && f64::from(after) <= f64::from(max) * roll.value {
                                event.amount = current + 1.0;
                            }
                        }
                    }
                }
                Kind::Cleaving => {
                    if rng.chance((roll.chance * 100.0).round() as u64) {
                        cleave(host, event, attacker, victim, roll.targets);
                    }
                }
                Kind::Thunderstruck => {
                    // Java's data carries the damage, not a chance: the strike
                    // is this port's own one-in-five.
                    if rng.chance(20) {
                        let at = event.victim_position;
                        let _ = host.lightning_bolt(host.dimension(), at, false);
                        let _ = host.hurt_entity(
                            host.dimension(),
                            victim,
                            roll.value as f32,
                            "minecraft:lightning_bolt",
                            Some(attacker),
                        );
                    }
                }
                _ => {}
            }
        }
    }

    // The victim's own gear: armour pays `hurt_*` effects, a shield in the off
    // hand the `block_*` ones (there is no block event: the README says so).
    let mut gear: Vec<(EquipmentSlot, ItemStack)> = Vec::new();
    for slot in [
        EquipmentSlot::Head,
        EquipmentSlot::Chest,
        EquipmentSlot::Legs,
        EquipmentSlot::Feet,
        EquipmentSlot::OffHand,
    ] {
        if let Some(stack) = player_slot(host, victim, slot) {
            gear.push((slot, stack));
        }
    }
    for (_slot, stack) in gear {
        for (entry, roll) in effects_of(&stack) {
            match entry.affix.and_then(|affix| affix.effect_target) {
                Some("hurt_attacker") => {
                    pay_effect(host, entry, roll, victim, attacker, &mut rng);
                }
                Some("hurt_self") | Some("block_self") => {
                    pay_effect(host, entry, roll, victim, victim, &mut rng);
                }
                Some("block_attacker") => {
                    pay_effect(host, entry, roll, victim, attacker, &mut rng);
                }
                _ => {}
            }
            if entry.kind == Kind::Retreating {
                retreat(host, event, victim);
            }
        }
    }
}

/// Applies one `mob_effect` affix: chance, then cooldown, then the effect at
/// the rolled duration and amplifier.
fn pay_effect(
    host: &mut dyn Host,
    entry: &Entry,
    roll: Roll,
    source: u32,
    target: u32,
    rng: &mut Rng,
) {
    if roll.chance > 0.0 && !rng.chance((roll.chance * 100.0).round() as u64) {
        return;
    }
    if !ready(source, entry.id, roll.cooldown) {
        return;
    }
    let effect = entry.affix.and_then(|affix| affix.effect).unwrap_or("minecraft:poison");
    add_effect(
        host,
        target,
        effect,
        (roll.duration as i32).max(1),
        roll.amplifier,
    );
}

/// Retreating: pushes the holder away from the attacker.
fn retreat(host: &mut dyn Host, event: &LivingIncomingDamageV2Event<'_>, victim: u32) {
    let away = [
        event.victim_position[0] - event.source_position.unwrap_or(event.victim_position)[0],
        0.0,
        event.victim_position[2] - event.source_position.unwrap_or(event.victim_position)[2],
    ];
    let length = (away[0] * away[0] + away[2] * away[2]).sqrt().max(1.0e-4);
    if let Some(entity) = host.entity(victim) {
        let _ = host.entity_push(
            entity,
            [away[0] / length * 0.6, 0.25, away[2] / length * 0.6],
        );
    }
}

/// Cleaving: up to `count` other living entities around the victim take half
/// the blow, with the same attacker.
fn cleave(
    host: &mut dyn Host,
    event: &LivingIncomingDamageV2Event<'_>,
    attacker: u32,
    victim: u32,
    count: f64,
) {
    let centre = event.victim_position;
    let min = [centre[0] - 3.5, centre[1] - 3.0, centre[2] - 3.5];
    let max = [centre[0] + 3.5, centre[1] + 3.0, centre[2] + 3.5];
    let mut targets: Vec<u32> = host
        .entities_in(host.dimension(), min, max)
        .into_iter()
        .filter(|id| *id != victim && *id != attacker)
        .filter(|id| {
            host.entity(*id)
                .and_then(|entity| host.entity_kind(entity))
                .is_some_and(|kind| kind != "minecraft:item" && !kind.contains("arrow"))
        })
        .take(count.max(1.0) as usize)
        .collect();
    targets.sort_unstable();
    for target in targets {
        let _ = host.hurt_entity(
            host.dimension(),
            target,
            event.amount * 0.5,
            "minecraft:player_attack",
            Some(attacker),
        );
    }
}

// ---------------------------------------------------------------------------
// Durability, arrows, drops
// ---------------------------------------------------------------------------

/// Durable: finds a worn or held copy of the damaged item that carries the
/// affix, and takes its share off the wear.
pub fn item_damaged(host: &mut dyn Host, event: &mut ItemDamagedEvent<'_>) -> Flow {
    let mut best = 0.0f64;
    for slot in EquipmentSlot::ALL {
        let Some(stack) = host.player_equipment(event.player, slot) else {
            continue;
        };
        if stack.item.as_ref() != event.item {
            continue;
        }
        for (entry, roll) in effects_of(&stack) {
            if entry.kind == Kind::Durable {
                best = best.max(roll.value);
            }
        }
    }
    if best > 0.0 {
        let reduction = (f64::from(event.amount) * best).floor() as i32;
        event.amount = (event.amount - reduction.max(0)).max(0);
    }
    Flow::Continue
}

/// Telepathic: a tool or weapon with the affix hands its block or kill drops
/// straight to the player. Festive: a weapon may double the gear a kill drops.
pub fn loot_drops(host: &mut dyn Host, event: &mut LootDropsEvent<'_>) {
    let Some(player) = event.context.player else {
        return;
    };
    let has = |stack: &ItemStack, kind: Kind| effects_of(stack).iter().any(|(e, _)| e.kind == kind);
    let telepathic = event
        .context
        .tool
        .map(|tool| has(tool, Kind::Telepathic))
        .unwrap_or(false)
        || host
            .player_equipment(player, EquipmentSlot::MainHand)
            .map(|stack| has(&stack, Kind::Telepathic))
            .unwrap_or(false);
    if telepathic && !event.drops.is_empty() {
        let drops = std::mem::take(event.drops);
        for stack in drops {
            if let Some(left) = host.give_stack(player, stack) {
                event.drops.push(left);
            }
        }
    }

    // Festive: the killer's weapon may duplicate what the mob dropped.
    if event.context.entity.is_none() || event.context.used {
        return;
    }
    let Some(weapon) = host.player_equipment(player, EquipmentSlot::MainHand) else {
        return;
    };
    let Some((_, rolled)) = effects_of(&weapon)
        .into_iter()
        .find(|(entry, _)| entry.kind == Kind::Festive)
    else {
        return;
    };
    let seed = event
        .context
        .entity
        .map(|(id, _)| u64::from(id))
        .unwrap_or_default()
        ^ player as u64;
    let mut rng = roll(seed);
    if rng.chance((rolled.chance * 100.0).round() as u64) {
        // Gear only: a doubled gem would inflate the boss table.
        let extra: Vec<ItemStack> = event
            .drops
            .iter()
            .filter(|stack| crate::is_equipment(&stack.item))
            .cloned()
            .collect();
        event.drops.extend(extra);
    }
}

// ---------------------------------------------------------------------------
// Apothic Invader
// ---------------------------------------------------------------------------

/// The tag a converted boss carries, so its death drops the high gem.
pub const BOSS_TAG: &str = "apotheosis:boss";

struct Join {
    dimension: String,
    entity: u32,
    kind: String,
    pos: [f64; 3],
    at: Instant,
}

fn joins() -> &'static Mutex<Vec<Join>> {
    static JOINS: OnceLock<Mutex<Vec<Join>>> = OnceLock::new();
    JOINS.get_or_init(|| Mutex::new(Vec::new()))
}

/// `entity_join_level` names the entity before `mob_spawned` says the spawn was
/// natural; the join waits here for that second hook. Disk loads never do.
pub fn entity_join_level(event: &mut EntityJoinLevelEvent<'_>) -> Flow {
    if event.loaded_from_disk || event.kind == "minecraft:player" {
        return Flow::Continue;
    }
    let mut joins = joins().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    joins.push(Join {
        dimension: event.dimension.to_string(),
        entity: event.entity,
        kind: event.kind.to_string(),
        pos: event.pos,
        at: Instant::now(),
    });
    // Keep the queue short: a join is consumed by the next spawn hook.
    let excess = joins.len().saturating_sub(64);
    if excess > 0 {
        joins.drain(..excess);
    }
    Flow::Continue
}

/// Takes the fresh join matching `event`, if one is waiting.
fn take_join(event: &MobSpawnedEvent<'_>) -> Option<u32> {
    let mut joins = joins().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    joins.retain(|join| join.at.elapsed().as_secs() < 5);
    let mut found = None;
    for (index, join) in joins.iter().enumerate() {
        if join.kind == event.kind
            && join.dimension == event.dimension
            && (join.pos[0] - event.pos[0]).abs() < 8.0
            && (join.pos[1] - event.pos[1]).abs() < 8.0
            && (join.pos[2] - event.pos[2]).abs() < 8.0
        {
            found = Some(index);
            break;
        }
    }
    found.map(|index| joins.remove(index).entity)
}

/// Java's `bossSpawnCooldown`: 3600 ticks (three minutes) between two natural
/// invaders. Java keeps it per player; the spawn event names no player, so
/// this port uses the nearest one, the same player whose world tier decides
/// whether the spawn may be an invader at all.
const INVADER_COOLDOWN: Duration = Duration::from_secs(3600 / 20);

static LAST_INVADER: OnceLock<Mutex<HashMap<u128, Instant>>> = OnceLock::new();

/// Whether `player` may have an invader at `now`; if so, starts their
/// cooldown.
fn invader_ready(player: u128, now: Instant) -> bool {
    let mut last = LAST_INVADER
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if last
        .get(&player)
        .is_some_and(|then| now.duration_since(*then) < INVADER_COOLDOWN)
    {
        return false;
    }
    last.insert(player, now);
    true
}

/// Whether the spawn is on the surface: at or above the column's highest
/// motion-blocking block.
fn on_surface(host: &mut dyn Host, dimension: &str, pos: [f64; 3]) -> bool {
    let column = [(pos[0].floor() as i32, pos[2].floor() as i32)];
    match host.surface_heights(dimension, &column, NativeHeightmap::MotionBlockingNoLeaves) {
        Ok(heights) => heights
            .first()
            .copied()
            .flatten()
            .is_some_and(|height| pos[1] >= f64::from(height) - 1.0),
        // Without a heightmap, do not block the boss.
        Err(_) => true,
    }
}

/// `mob_spawned`: a fresh natural mob on the surface, at night in the
/// overworld, may become an Apothic Invader.
pub fn mob_spawned(host: &mut dyn Host, event: &MobSpawnedEvent<'_>) {
    if !event.natural {
        return;
    }
    let Some(entity) = take_join(event) else {
        return;
    };
    let night = (host.day_time().rem_euclid(24_000) >= 13_000)
        && (host.day_time().rem_euclid(24_000) <= 23_000);
    let overworld = event.dimension == "minecraft:overworld";
    if overworld && (!night || !on_surface(host, event.dimension, event.pos)) {
        return;
    }
    // The nearest player decides both the chance (Haven never spawns one) and
    // whose cooldown the invader takes.
    let Some(player) = tiers::nearest_player(host, event.dimension, event.pos) else {
        return;
    };
    let world = tiers::tier_of_with(host, player);
    let mut rng = roll(u64::from(entity) ^ event.pos[0].to_bits().rotate_left(9));
    if rng.below(10_000) >= tiers::invader_chance(world, event.dimension)
        || !invader_ready(player, Instant::now())
    {
        return;
    }
    become_boss(host, event.dimension, entity, event.kind, world, &mut rng);
}

/// Extra health by rarity, so an invader is a fight.
fn bonus_health(tier: Tier) -> f64 {
    match tier {
        Tier::Uncommon => 20.0,
        Tier::Rare => 40.0,
        Tier::Epic => 70.0,
        Tier::Mythic => 110.0,
        Tier::Common | Tier::Ancient => 20.0,
    }
}

/// The base name of a mob kind: `minecraft:wither_skeleton` -> "Wither Skeleton".
fn base_name(kind: &str) -> (String, String) {
    let name = kind.rsplit(':').next().unwrap_or(kind);
    let words: Vec<String> = name
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect();
    (format!("apotheosis.boss.base.{name}"), words.join(" "))
}

const PREFIXES: [(&str, &str); 8] = [
    ("grim", "Grim"),
    ("vile", "Vile"),
    ("ancient", "Ancient"),
    ("cursed", "Cursed"),
    ("hollow", "Hollow"),
    ("blighted", "Blighted"),
    ("savage", "Savage"),
    ("ethereal", "Ethereal"),
];

const SUFFIXES: [(&str, &str); 8] = [
    ("devourer", "the Devourer"),
    ("abyss", "of the Abyss"),
    ("unbroken", "the Unbroken"),
    ("embers", "of Embers"),
    ("silent", "the Silent"),
    ("deep", "of the Deep"),
    ("ravager", "the Ravager"),
    ("twilight", "of Twilight"),
];

/// A generated name, as the text component a `CustomName` holds and as a
/// `Text` the announcement can use.
fn generated_name(rng: &mut Rng, kind: &str) -> (String, Text) {
    let (base_key, base) = base_name(kind);
    let prefix = if rng.chance(30) {
        Some(PREFIXES[rng.below(PREFIXES.len() as u64) as usize])
    } else {
        None
    };
    let suffix = if rng.chance(80) {
        Some(SUFFIXES[rng.below(SUFFIXES.len() as u64) as usize])
    } else {
        None
    };
    let (title_key, title) = match (prefix, suffix) {
        (Some(_), Some(_)) => ("apotheosis.boss.title.full", "%s %s %s"),
        (Some(_), None) => ("apotheosis.boss.title.prefix", "%s %s"),
        (None, Some(_)) => ("apotheosis.boss.title.suffix", "%s %s"),
        (None, None) => ("", "%s"),
    };
    let base_text = Text::translate_with_fallback(&base_key, &base, &[]);
    let mut args = Vec::new();
    let mut json_args = Vec::new();
    if let Some((key, word)) = prefix {
        args.push(Text::translate_with_fallback(
            &format!("apotheosis.boss.prefix.{key}"),
            word,
            &[],
        ));
        json_args.push(serde_json::json!({
            "translate": format!("apotheosis.boss.prefix.{key}"),
            "fallback": word,
        }));
    }
    args.push(base_text);
    json_args.push(serde_json::json!({"translate": base_key, "fallback": base}));
    if let Some((key, word)) = suffix {
        args.push(Text::translate_with_fallback(
            &format!("apotheosis.boss.suffix.{key}"),
            word,
            &[],
        ));
        json_args.push(serde_json::json!({
            "translate": format!("apotheosis.boss.suffix.{key}"),
            "fallback": word,
        }));
    }
    let text = if title_key.is_empty() {
        args.remove(0)
    } else {
        Text::translate_with_fallback(title_key, title, &args)
    };
    let json = if title_key.is_empty() {
        json_args.remove(0)
    } else {
        serde_json::json!({"translate": title_key, "fallback": title, "with": json_args})
    };
    (json.to_string(), text)
}

/// Converts the freshly spawned mob into an invader whose own rarity and gear
/// roll from the nearby player's world tier.
fn become_boss(
    host: &mut dyn Host,
    dimension: &str,
    entity: u32,
    kind: &str,
    world: WorldTier,
    rng: &mut Rng,
) {
    let Some(reference) = host.entity(entity) else {
        return;
    };
    let tier = tiers::roll_rarity(world, 0.0, rng);
    let (name, name_text) = generated_name(rng, kind);
    let health = 20.0 + bonus_health(tier);
    let _ = host.entity_set_attribute_base(reference, "minecraft:max_health", health);
    let _ = host.set_mob_health(dimension, entity, health as f32);
    let mut boss_nbt = Compound::new();
    boss_nbt.insert("CustomName".into(), Tag::String(name.as_str().into()));
    boss_nbt.insert("CustomNameVisible".into(), Tag::Byte(1));
    boss_nbt.insert("Health".into(), Tag::Float((health as f32).to_bits()));
    let _ = host.entity_merge_nbt(reference, boss_nbt);
    let _ = host.entity_add_tag(reference, BOSS_TAG);

    // Gear: a high-rarity affixed piece in every empty equipment slot.
    if let Some(living) = host.living(reference) {
        let slots = [
            EquipmentSlot::MainHand,
            EquipmentSlot::Head,
            EquipmentSlot::Chest,
            EquipmentSlot::Legs,
            EquipmentSlot::Feet,
        ];
        for slot in slots {
            let mut stack = one(boss_gear(kind, slot, rng));
            let rarity = tiers::roll_rarity(world, 0.0, rng);
            apply_affixes(&mut stack, rarity, rng);
            let _ = host.living_set_equipment(living, slot, Some(stack));
        }
    }
    host.broadcast(Text::translate_with_fallback(
        "apotheosis.boss.awakened",
        "%s has awoken nearby!",
        &[name_text],
    ));
}

/// Which piece an invader wears in `slot`: its own weapon where it has one,
/// otherwise diamond gear.
fn boss_gear(kind: &str, slot: EquipmentSlot, rng: &mut Rng) -> &'static str {
    match slot {
        EquipmentSlot::MainHand => match kind {
            "minecraft:skeleton" | "minecraft:stray" | "minecraft:bogged" => "minecraft:bow",
            "minecraft:piglin" => "minecraft:golden_sword",
            "minecraft:wither_skeleton" => "minecraft:stone_sword",
            _ => "minecraft:diamond_sword",
        },
        EquipmentSlot::Head => "minecraft:diamond_helmet",
        EquipmentSlot::Chest => "minecraft:diamond_chestplate",
        EquipmentSlot::Legs => "minecraft:diamond_leggings",
        EquipmentSlot::Feet => "minecraft:diamond_boots",
        _ => {
            let _ = rng;
            "minecraft:diamond_sword"
        }
    }
}

/// Whether a mob carries the boss tag, read from the entity the loot hook
/// names.
pub fn is_boss(host: &mut dyn Host, entity: u32) -> bool {
    host.entity(entity)
        .and_then(|reference| host.entity_tags(reference))
        .is_some_and(|tags| tags.iter().any(|tag| tag == BOSS_TAG))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn affix(path: &str) -> &'static Affix {
        AFFIXES
            .iter()
            .find(|affix| affix.path == path)
            .unwrap_or_else(|| panic!("no {path}"))
    }

    /// The reported bug: the lore showed the effect as a bare English word
    /// ("Weakness"), so a client's language never translated it. The argument
    /// must be the game's own `effect.minecraft.*` key.
    #[test]
    fn an_effect_lore_line_carries_the_vanilla_effect_key() {
        let entry = catalogue()
            .iter()
            .find(|entry| {
                entry.kind == Kind::MobEffect
                    && entry.affix.and_then(|affix| affix.effect).is_some()
            })
            .expect("a mob-effect affix");
        let (_, _, args) = description(
            entry,
            Roll {
                amplifier: 1,
                duration: 140.0,
                ..Roll::default()
            },
        );
        let key = args.iter().find_map(|arg| match arg {
            Tag::Compound(fields) => match fields.get(&"translate".into()) {
                Some(Tag::String(key)) => Some(key.to_string_lossy()),
                _ => None,
            },
            _ => None,
        });
        let effect = entry.affix.and_then(|affix| affix.effect).unwrap();
        let expected = format!("effect.{}", effect.replacen(':', ".", 1));
        assert_eq!(key.as_deref(), Some(expected.as_str()));
        assert!(expected.starts_with("effect.minecraft."), "{expected}");
    }

    #[test]
    fn every_effect_affix_is_in_the_catalogue_and_rolls() {
        for entry in catalogue() {
            assert!(entry.id.starts_with("apotheosis:"), "{}", entry.id);
            if let Some(affix) = entry.affix {
                assert!(kind_of(affix).is_some(), "{}", affix.id);
                assert!(matches!(affix.kind, "mob_effect" | "damage_reduction" | "enchantment")
                    || matches!(
                        affix.kind,
                        "executing" | "cleaving" | "thunderstruck" | "retreating"
                            | "telepathic" | "festive"
                    ));
            }
        }
    }

    #[test]
    fn effect_affixes_respect_their_item_classes() {
        assert!(available(ItemClass::Helmet, Tier::Epic).contains(&"apotheosis:armor/mob_effect/blinding"));
        assert!(!available(ItemClass::Sword, Tier::Epic).contains(&"apotheosis:armor/mob_effect/blinding"));
        assert!(available(ItemClass::Shield, Tier::Epic).contains(&"apotheosis:shield/retreating"));
        assert!(available(ItemClass::Shield, Tier::Uncommon).contains(&"apotheosis:shield/mob_effect/venomous"));
        assert!(!available(ItemClass::Sword, Tier::Uncommon).contains(&"apotheosis:shield/mob_effect/venomous"));
        assert!(available(ItemClass::Sword, Tier::Rare).contains(&"apotheosis:generic/telepathic"));
        assert!(!available(ItemClass::Sword, Tier::Epic).contains(&"apotheosis:shield/mob_effect/withering"));
        assert!(!available(ItemClass::Bow, Tier::Epic).contains(&"apotheosis:melee/cleaving"));
        assert!(available(ItemClass::Sword, Tier::Epic).contains(&"apotheosis:durable"));
    }

    #[test]
    fn rolls_follow_the_java_ranges() {
        let venomous = affix("shield/mob_effect/venomous");
        for seed in 0..40 {
            let mut rng = Rng::new(seed);
            let roll = draw(venomous, Tier::Epic, &mut rng);
            // Epic: 120..200 by 20, amplifier 0..1, cooldown 120.
            assert!((120.0..=200.0).contains(&roll.duration), "{roll:?}");
            assert_eq!(roll.duration % 20.0, 0.0);
            assert!(roll.amplifier <= 1, "{roll:?}");
            assert_eq!(roll.cooldown, 120);
        }
        let blinding = affix("armor/mob_effect/blinding");
        let mut rng = Rng::new(1);
        assert_eq!(draw(blinding, Tier::Mythic, &mut rng).cooldown, 200);
        let cleaving = affix("melee/cleaving");
        let roll = draw(cleaving, Tier::Epic, &mut rng);
        assert!((0.3..=0.5).contains(&roll.chance));
        assert!((2.0..=5.0).contains(&roll.targets));
        let executing = affix("melee/executing");
        let roll = draw(executing, Tier::Mythic, &mut rng);
        assert!((0.15..=0.25).contains(&roll.value));
        let reduction = affix("armor/dmg_reduction/feathery");
        let roll = draw(reduction, Tier::Mythic, &mut rng);
        assert!((0.15..=0.4).contains(&roll.value));
    }

    /// The effect affixes of one kind an item carries.
    fn count_kind(stack: &ItemStack, kind: &str) -> usize {
        effects_of(stack)
            .iter()
            .filter(|(entry, _)| entry.affix_type() == kind)
            .count()
    }

    #[test]
    fn effect_counts_follow_java_rarities() {
        assert_eq!(effect_counts(Tier::Common), (0, 0));
        assert_eq!(effect_counts(Tier::Uncommon), (0, 0));
        assert_eq!(effect_counts(Tier::Rare), (1, 0));
        assert_eq!(effect_counts(Tier::Epic), (1, 1));
        assert_eq!(effect_counts(Tier::Mythic), (2, 1));
        assert_eq!(effect_counts(Tier::Ancient), (2, 1));
    }

    #[test]
    fn a_piece_rolls_the_effects_its_rarity_guarantees() {
        for seed in 0..12 {
            let mut rare = one("minecraft:diamond_sword");
            apply_affixes(&mut rare, Tier::Rare, &mut Rng::new(seed));
            assert_eq!(count_kind(&rare, "basic_effect"), 1, "rare/seed {seed}");
            assert_eq!(count_kind(&rare, "ability"), 0, "rare/seed {seed}");

            let mut epic = one("minecraft:diamond_sword");
            apply_affixes(&mut epic, Tier::Epic, &mut Rng::new(seed));
            assert_eq!(count_kind(&epic, "basic_effect"), 1, "epic/seed {seed}");
            assert_eq!(count_kind(&epic, "ability"), 1, "epic/seed {seed}");

            let mut mythic = one("minecraft:diamond_sword");
            apply_affixes(&mut mythic, Tier::Mythic, &mut Rng::new(seed));
            assert_eq!(count_kind(&mythic, "basic_effect"), 2, "mythic/seed {seed}");
            assert_eq!(count_kind(&mythic, "ability"), 1, "mythic/seed {seed}");
        }
    }

    #[test]
    fn an_effect_affix_keeps_the_attribute_lines() {
        let mut stack = one("minecraft:diamond_sword");
        let mut rng = Rng::new(7);
        apply_affixes(&mut stack, Tier::Ancient, &mut rng);
        let data = stack.data().unwrap();
        let Some(Tag::List(modifiers)) = data.other("minecraft:attribute_modifiers") else {
            panic!("no modifiers");
        };
        assert_eq!(modifiers.len(), Tier::Ancient.affixes());
        let Some(Tag::List(lore)) = data.other("minecraft:lore") else {
            panic!("no lore");
        };
        // Five stat lines and ancient's three effect lines.
        assert_eq!(lore.len(), modifiers.len() + 3);
    }

    #[test]
    fn a_reroll_clears_the_previous_effects() {
        let mut stack = one("minecraft:diamond_sword");
        let mut rng = Rng::new(11);
        apply_affixes(&mut stack, Tier::Ancient, &mut rng);
        assert_eq!(effects_of(&stack).len(), 3);
        // A second ancient roll replaces the three, never stacks three more.
        apply_affixes(&mut stack, Tier::Ancient, &mut rng);
        assert_eq!(effects_of(&stack).len(), 3);
        // And a rare reroll drops the extra effects instead of keeping them.
        apply_affixes(&mut stack, Tier::Rare, &mut rng);
        assert_eq!(effects_of(&stack).len(), 1);
    }

    #[test]
    fn a_reroll_does_not_stack_the_enchantment_levels() {
        // The item's own Fortune II is not this module's to remove.
        let mut stack = one("minecraft:diamond_pickaxe");
        stack.data_mut().enchantments = vec![(std::sync::Arc::from("minecraft:fortune"), 2)];
        for seed in 0..40 {
            let mut rng = Rng::new(seed);
            apply_affixes(&mut stack, Tier::Epic, &mut rng);
            let added: u32 = effects_of(&stack)
                .iter()
                .filter(|(entry, _)| entry.kind == Kind::Enchantment)
                .map(|(_, roll)| roll.value.round().max(0.0) as u32)
                .sum();
            assert_eq!(
                stack.enchantment_level("minecraft:fortune"),
                2 + added,
                "seed {seed}: the last pass's levels stayed"
            );
        }
    }

    #[test]
    fn clearing_the_effects_takes_back_only_the_added_levels() {
        let mut stack = one("minecraft:diamond_pickaxe");
        stack.data_mut().enchantments = vec![(std::sync::Arc::from("minecraft:fortune"), 2)];
        add_enchantment(&mut stack, "minecraft:fortune", 3.0);
        assert_eq!(stack.enchantment_level("minecraft:fortune"), 5);
        clear_effects(&mut stack);
        assert_eq!(stack.enchantment_level("minecraft:fortune"), 2);
    }

    #[test]
    fn the_enchantment_affix_writes_real_levels() {
        let mut stack = one("minecraft:diamond_pickaxe");
        add_enchantment(&mut stack, "minecraft:fortune", 3.0);
        assert_eq!(stack.enchantment_level("minecraft:fortune"), 3);
        add_enchantment(&mut stack, "minecraft:fortune", 1.0);
        assert_eq!(stack.enchantment_level("minecraft:fortune"), 4);
    }

    #[test]
    fn boss_names_are_generated_from_parts() {
        for seed in 0..20 {
            let mut rng = Rng::new(seed);
            let (json, text) = generated_name(&mut rng, "minecraft:zombie");
            assert!(json.contains("apotheosis.boss.base.zombie"));
            assert!(!text.plain().trim().is_empty());
        }
    }

    #[test]
    fn boss_health_rises_with_rarity() {
        assert!(bonus_health(Tier::Rare) > bonus_health(Tier::Uncommon));
        assert!(bonus_health(Tier::Mythic) > bonus_health(Tier::Epic));
        assert!(bonus_health(Tier::Mythic) >= 100.0);
    }

    #[test]
    fn the_boss_tag_is_the_one_the_loot_hook_reads() {
        assert_eq!(BOSS_TAG, "apotheosis:boss");
    }

    #[test]
    fn base_names_read_as_words() {
        assert_eq!(base_name("minecraft:wither_skeleton").1, "Wither Skeleton");
        assert_eq!(base_name("minecraft:zombie").1, "Zombie");
    }
}

#[cfg(test)]
mod invader_rate_tests {
    use super::{invader_ready, INVADER_COOLDOWN};
    use std::time::{Duration, Instant};

    #[test]
    fn the_invader_cooldown_is_per_player() {
        let t0 = Instant::now();
        let one = 0xAAAA;
        let two = 0xBBBB;
        assert!(invader_ready(one, t0));
        assert!(!invader_ready(one, t0 + Duration::from_secs(60)));
        assert!(invader_ready(two, t0), "another player has their own cooldown");
        assert!(invader_ready(one, t0 + INVADER_COOLDOWN));
    }
}
