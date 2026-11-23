//! Affixes whose payout is an effect, not an attribute, and the Apothic
//! Invader boss.
//!
//! Clean-room: ids and public numbers come from Apotheosis' data files; no
//! Apotheosis source or texture is copied. Effect affixes are extra affixes:
//! they never replace an attribute affix, so an item's attribute line count is
//! unchanged. Their minimum tier follows Java's per-affix values (uncommon for
//! the shield/melee/ranged debuffs, rare for telepathy, epic for the rest), and
//! they live in `minecraft:custom_data`
//! (`apotheosis:effects`), and are read back at the hooks that pay them out.
//!
//! Bosses use the two spawn hooks in order: `entity_join_level` names the
//! entity id before `mob_spawned` says the spawn was natural, so joins wait in
//! a short queue for their spawn. A natural surface spawn at night (overworld)
//! or in the Nether/End may become an invader: a generated name, more health,
//! affixed high-rarity gear, the `apotheosis:boss` tag, and an announcement.

use crate::attributes::{self, ItemClass};
use crate::{BOSS_TIERS, Rng, Tier, apply_affixes, one};
use horizon_client::mod_api::{
    Compound, EntityJoinLevelEvent, EquipmentSlot, Flow, Host, ItemStack, ItemDamagedEvent,
    LivingIncomingDamageV2Event, LootDropsEvent, MobSpawnedEvent, NativeEffect, NativeHeightmap,
    ProjectileLaunchEvent, Tag, Text, serde_json,
};
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// `minecraft:custom_data` key holding this port's effect affix rolls.
const EFFECTS_KEY: &str = "apotheosis:effects";

/// What an effect affix does when it is paid out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Venomous,
    Withering,
    Weakening,
    Ensnaring,
    Blinding,
    Execution,
    Cleaving,
    Thunderstruck,
    Retreating,
    Durable,
    Telepathic,
    Spectral,
    Magical,
    Festive,
}

/// An effect affix. `base` is the value at epic (a fraction for chances, ticks
/// for effects, targets for cleaving); higher tiers scale it by `Tier::scale`.
pub struct EffectAffix {
    pub id: &'static str,
    pub name_key: &'static str,
    pub name: &'static str,
    pub kind: Kind,
    /// The item classes it may roll on (`attributes::MELEE` and the like).
    pub classes: u16,
    /// The lowest tier it appears at.
    pub min_tier: Tier,
    pub base: f64,
    /// The lore line's English fallback, `{}` filled with the rolled value.
    pub desc: &'static str,
}

const fn effect(
    id: &'static str,
    name: &'static str,
    kind: Kind,
    classes: u16,
    min_tier: Tier,
    base: f64,
    desc: &'static str,
) -> EffectAffix {
    EffectAffix {
        id,
        name_key: "",
        name,
        kind,
        classes,
        min_tier,
        base,
        desc,
    }
}

/// The effect affixes this port knows, by upstream id.
pub const EFFECT_AFFIXES: [EffectAffix; 14] = [
    effect(
        "venomous",
        "Venomous",
        Kind::Venomous,
        attributes::SHIELD,
        Tier::Uncommon,
        80.0,
        "Poison for {} s",
    ),
    effect(
        "withering",
        "Withering",
        Kind::Withering,
        attributes::SHIELD,
        Tier::Epic,
        35.0,
        "Wither for {} s",
    ),
    effect(
        "weakening",
        "Weakening",
        Kind::Weakening,
        attributes::MELEE,
        Tier::Uncommon,
        65.0,
        "Weakness for {} s",
    ),
    effect(
        "ensnaring",
        "Ensnaring",
        Kind::Ensnaring,
        attributes::RANGED,
        Tier::Uncommon,
        40.0,
        "Slowness for {} s",
    ),
    effect(
        "blinding",
        "Blinding",
        Kind::Blinding,
        attributes::HELMET,
        Tier::Uncommon,
        30.0,
        "Blinds attackers for {} s",
    ),
    effect(
        "executing",
        "Executing",
        Kind::Execution,
        attributes::MELEE,
        Tier::Epic,
        0.075,
        "Executes foes below {}% health",
    ),
    effect(
        "cleaving",
        "Cleaving",
        Kind::Cleaving,
        attributes::MELEE,
        Tier::Epic,
        1.75,
        "Hits {} nearby foes",
    ),
    effect(
        "thunderstruck",
        "Thunderstruck",
        Kind::Thunderstruck,
        attributes::MELEE,
        Tier::Epic,
        2.25,
        "{}% chance of lightning",
    ),
    effect(
        "retreating",
        "Retreating",
        Kind::Retreating,
        attributes::SHIELD,
        Tier::Epic,
        1.0,
        "Knocks you back when hit",
    ),
    effect(
        "durable",
        "Durable",
        Kind::Durable,
        attributes::ARMOR | attributes::MELEE | attributes::TOOL | attributes::RANGED,
        Tier::Epic,
        0.15,
        "{}% less wear",
    ),
    effect(
        "telepathic",
        "Telepathic",
        Kind::Telepathic,
        attributes::ARMOR
            | attributes::MELEE
            | attributes::RANGED
            | attributes::TOOL
            | attributes::SHIELD
            | attributes::SHEARS,
        Tier::Rare,
        1.0,
        "Drops go to your inventory",
    ),
    effect(
        "spectral",
        "Spectral",
        Kind::Spectral,
        attributes::RANGED,
        Tier::Epic,
        0.175,
        "{}% chance of a spectral arrow",
    ),
    effect(
        "magical",
        "Magical",
        Kind::Magical,
        attributes::RANGED,
        Tier::Epic,
        0.25,
        "Arrows pierce armour as magic",
    ),
    effect(
        "festive",
        "Festive",
        Kind::Festive,
        attributes::MELEE,
        Tier::Epic,
        0.0175,
        "{}% chance of extra loot",
    ),
];

/// The affix's translation key, `apotheosis.affix.<id>`.
pub fn name_key(affix: &EffectAffix) -> String {
    if affix.name_key.is_empty() {
        format!("apotheosis.affix.{}", affix.id)
    } else {
        affix.name_key.to_string()
    }
}

/// The lore line's translation key, `apotheosis.affix.<id>.desc`.
pub fn desc_key(affix: &EffectAffix) -> String {
    format!("apotheosis.affix.{}.desc", affix.id)
}

/// The rolled magnitude at `tier`: ticks for effects, a fraction for chances,
/// a target count for cleaving.
pub fn value_at(affix: &EffectAffix, tier: Tier) -> f64 {
    let raw = affix.base * tier.scale();
    match affix.kind {
        Kind::Venomous | Kind::Withering | Kind::Weakening | Kind::Ensnaring | Kind::Blinding => {
            (raw / 20.0).round() * 20.0
        }
        Kind::Execution | Kind::Durable | Kind::Spectral | Kind::Festive => {
            ((raw.min(1.0)) * 100.0).round() / 100.0
        }
        Kind::Cleaving | Kind::Thunderstruck => raw.round(),
        Kind::Retreating | Kind::Telepathic | Kind::Magical => raw,
    }
}

/// The value as a lore string: seconds, percent or a count.
fn display(affix: &EffectAffix, value: f64) -> String {
    match affix.kind {
        Kind::Venomous | Kind::Withering | Kind::Weakening | Kind::Ensnaring | Kind::Blinding => {
            format!("{:.0}", value / 20.0)
        }
        Kind::Execution | Kind::Durable | Kind::Spectral | Kind::Festive
        | Kind::Thunderstruck => format!("{:.0}", value * 100.0),
        Kind::Cleaving => format!("{value:.0}"),
        Kind::Retreating | Kind::Telepathic | Kind::Magical => String::new(),
    }
}

/// Picks an effect affix for `class` at `tier`, if one is available.
fn choose(class: ItemClass, tier: Tier, rng: &mut Rng) -> Option<&'static EffectAffix> {
    let pool: Vec<&'static EffectAffix> = EFFECT_AFFIXES
        .iter()
        .filter(|a| tier >= a.min_tier && attributes::accepts(a.classes, class))
        .collect();
    if pool.is_empty() {
        return None;
    }
    Some(pool[rng.below(pool.len() as u64) as usize])
}

/// Adds one effect affix to a piece being affixed: its lore line, and its
/// roll in `minecraft:custom_data`. Called by `apply_affixes` at epic and
/// above, after the attribute affixes, so their count is untouched.
pub fn add_effect_affix(stack: &mut ItemStack, tier: Tier, rng: &mut Rng) -> Option<&'static str> {
    let class = attributes::class_of(&stack.item);
    // One effect affix on most high-tier pieces; ancient always gets one.
    let chance = if tier >= Tier::Ancient { 100 } else { 55 };
    if !rng.chance(chance) {
        return None;
    }
    let affix = choose(class, tier, rng)?;
    let value = value_at(affix, tier);
    let text = display(affix, value);
    let fallback = if text.is_empty() {
        affix.desc.to_string()
    } else {
        affix.desc.replace("{}", &text)
    };

    // Lore, after the attribute lines.
    let mut lore = match stack
        .data()
        .and_then(|data| data.other("minecraft:lore"))
    {
        Some(Tag::List(lines)) => lines.clone(),
        _ => Vec::new(),
    };
    lore.push(Tag::compound([
        (
            "translate",
            string(&format!("apotheosis.affix.{}.desc", affix.id)),
        ),
        ("fallback", string(&fallback)),
        ("color", string("#55FF55")),
        ("italic", Tag::Byte(0)),
        ("with", Tag::List(vec![string(&text)])),
    ]));
    stack
        .data_mut()
        .set_other("minecraft:lore", Some(Tag::List(lore)));

    // The roll, for the hooks.
    let mut custom = match stack.data().and_then(|data| data.other("minecraft:custom_data")) {
        Some(Tag::Compound(compound)) => compound.clone(),
        _ => Compound::new(),
    };
    let mut effects = match custom.get(&EFFECTS_KEY.into()) {
        Some(Tag::Compound(compound)) => compound.clone(),
        _ => Compound::new(),
    };
    effects.insert(affix.id.into(), Tag::Double(value.to_bits()));
    custom.insert(EFFECTS_KEY.into(), Tag::Compound(effects));
    stack
        .data_mut()
        .set_other("minecraft:custom_data", Some(Tag::Compound(custom)));
    Some(affix.id)
}

/// The effect affix rolls an item carries, by id.
pub fn effects_of(stack: &ItemStack) -> HashMap<&'static str, f64> {
    let mut found = HashMap::new();
    let Some(Tag::Compound(custom)) = stack.data().and_then(|data| data.other("minecraft:custom_data"))
    else {
        return found;
    };
    let Some(Tag::Compound(effects)) = custom.get(&EFFECTS_KEY.into()) else {
        return found;
    };
    for affix in &EFFECT_AFFIXES {
        if let Some(value) = number(effects.get(&affix.id.into())) {
            found.insert(affix.id, value);
        }
    }
    found
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

    if let Some(weapon) = event.weapon {
        let effects = effects_of(weapon);
        if !effects.is_empty() {
            for (id, value) in &effects {
                match *id {
                    "weakening" => add_effect(host, victim, "minecraft:weakness", *value as i32, 0),
                    "ensnaring" => add_effect(host, victim, "minecraft:slowness", *value as i32, 0),
                    "executing" => {
                        if let Some((current, max)) = health(host, victim) {
                            let after = current - event.amount;
                            if after > 0.0 && f64::from(after) <= f64::from(max) * value {
                                event.amount = current + 1.0;
                            }
                        }
                    }
                    "cleaving" => cleave(host, event, attacker, victim, *value),
                    "thunderstruck" => {
                        if rng.chance((value * 100.0).clamp(0.0, 100.0) as u64) {
                            let at = event.victim_position;
                            let _ = host.lightning_bolt(host.dimension(), at, false);
                            let _ = host.hurt_entity(
                                host.dimension(),
                                victim,
                                5.0,
                                "minecraft:lightning_bolt",
                                Some(attacker),
                            );
                        }
                    }
                    "magical" => {
                        // Armour and enchantment reduction is not replayed;
                        // the magic share lands as extra damage instead.
                        event.amount += event.amount * *value as f32;
                    }
                    _ => {}
                }
            }
        }
    }

    // The victim's own gear: a helmet may blind the attacker, a shield may
    // throw the holder back.
    if let Some(helmet) = player_slot(host, victim, EquipmentSlot::Head) {
        if let Some(value) = effects_of(&helmet).get("blinding").copied() {
            add_effect(host, attacker, "minecraft:blindness", value as i32, 0);
        }
    }
    if let Some(offhand) = player_slot(host, victim, EquipmentSlot::OffHand) {
        let shield_effects = effects_of(&offhand);
        // Java pays these on an actual block; the hook API has no block event,
        // so a shield in the off hand is the closest signal. See the README.
        if let Some(value) = shield_effects.get("venomous").copied() {
            add_effect(host, attacker, "minecraft:poison", value as i32, 0);
        }
        if let Some(value) = shield_effects.get("withering").copied() {
            add_effect(host, attacker, "minecraft:wither", value as i32, 0);
        }
        if shield_effects.contains_key("retreating") {
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
        if let Some(value) = effects_of(&stack).get("durable").copied() {
            best = best.max(value);
        }
    }
    if best > 0.0 {
        let reduction = (f64::from(event.amount) * best).floor() as i32;
        event.amount = (event.amount - reduction.max(0)).max(0);
    }
    Flow::Continue
}

/// Spectral: a bow's arrows may split off a free spectral arrow, as upstream's
/// `SpectralShotAffix` does. Magical is paid out in the damage hook.
pub fn projectile_launch(host: &mut dyn Host, event: &mut ProjectileLaunchEvent<'_>) {
    let Some(player) = event.player else {
        return;
    };
    if !event.kind.contains("arrow") {
        return;
    }
    let Some(weapon) = host.player_equipment(player, EquipmentSlot::MainHand) else {
        return;
    };
    let Some(value) = effects_of(&weapon).get("spectral").copied() else {
        return;
    };
    let mut rng = roll(u64::from(event.entity) ^ player as u64);
    if !rng.chance((value * 100.0).clamp(0.0, 100.0) as u64) {
        return;
    }
    let Some(entity) = host.entity(event.entity) else {
        return;
    };
    let Some(pos) = host.entity_position(entity) else {
        return;
    };
    let motion = host
        .entity_velocity(entity)
        .unwrap_or([0.0, 0.0, 0.0])
        .map(|v| (v * 1000.0).round() / 1000.0);
    let motion = format!("[{},{},{}]", motion[0], motion[1], motion[2]);
    let line = format!(
        "summon minecraft:spectral_arrow {} {} {} {{Motion:{},pickup:2b}}",
        pos[0], pos[1], pos[2], motion
    );
    let _ = host.run_command(&line);
}

/// Telepathic: a tool or weapon with the affix hands its block or kill drops
/// straight to the player. Festive: a weapon may double the gear a kill drops.
pub fn loot_drops(host: &mut dyn Host, event: &mut LootDropsEvent<'_>) {
    let Some(player) = event.context.player else {
        return;
    };
    let telepathic = event
        .context
        .tool
        .map(|tool| effects_of(tool).contains_key("telepathic"))
        .unwrap_or(false)
        || host
            .player_equipment(player, EquipmentSlot::MainHand)
            .map(|stack| effects_of(&stack).contains_key("telepathic"))
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
    let Some(value) = effects_of(&weapon).get("festive").copied() else {
        return;
    };
    let seed = event
        .context
        .entity
        .map(|(id, _)| u64::from(id))
        .unwrap_or_default()
        ^ player as u64;
    let mut rng = roll(seed);
    if rng.chance((value * 100.0).clamp(0.0, 100.0) as u64) {
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

/// The chance a natural spawn becomes an invader, per dimension.
fn chance(dimension: &str) -> u64 {
    match dimension {
        "minecraft:the_nether" => 2,
        "minecraft:the_end" => 2,
        _ => 2,
    }
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
    let mut rng = roll(u64::from(entity) ^ event.pos[0].to_bits().rotate_left(9));
    if !rng.chance(chance(event.dimension)) {
        return;
    }
    become_boss(host, event.dimension, entity, event.kind, &mut rng);
}

/// The rarity a converted invader rolls.
const BOSS_RARITIES: [(Tier, u64); 4] = [
    (Tier::Uncommon, 35),
    (Tier::Rare, 40),
    (Tier::Epic, 20),
    (Tier::Mythic, 5),
];

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

/// Converts the freshly spawned mob into an invader.
fn become_boss(host: &mut dyn Host, dimension: &str, entity: u32, kind: &str, rng: &mut Rng) {
    let Some(reference) = host.entity(entity) else {
        return;
    };
    let tier = rng.tier(&BOSS_RARITIES);
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
            apply_affixes(&mut stack, rng.tier(&BOSS_TIERS), rng);
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

/// The boss loot tiers, exposed for `loot_drops`' gear upgrade.
pub fn boss_tiers() -> &'static [(Tier, u64)] {
    &BOSS_TIERS
}

/// The effect affixes an item of `class` can roll, for README checks and tests.
pub fn available(class: ItemClass, tier: Tier) -> Vec<&'static str> {
    EFFECT_AFFIXES
        .iter()
        .filter(|a| tier >= a.min_tier && attributes::accepts(a.classes, class))
        .map(|a| a.id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_effect_affix_names_and_describes_itself() {
        for affix in &EFFECT_AFFIXES {
            assert!(!affix.name.is_empty());
            assert!(name_key(affix).starts_with("apotheosis.affix."));
            assert!(desc_key(affix).ends_with(".desc"));
            assert!(affix.base > 0.0);
        }
    }

    #[test]
    fn effect_ids_are_unique_and_absent_from_attribute_affixes() {
        let mut seen = std::collections::HashSet::new();
        for affix in &EFFECT_AFFIXES {
            assert!(seen.insert(affix.id), "{} twice", affix.id);
            assert!(
                !crate::java_affixes::AFFIXES.iter().any(|a| {
                    a.kind == "attribute"
                        && a.path.rsplit('/').next() == Some(affix.id)
                }),
                "{} is both an attribute and an effect affix",
                affix.id
            );
        }
    }

    #[test]
    fn effect_affixes_respect_their_item_classes() {
        assert!(available(ItemClass::Bow, Tier::Epic).contains(&"spectral"));
        assert!(!available(ItemClass::Bow, Tier::Epic).contains(&"cleaving"));
        assert!(available(ItemClass::Helmet, Tier::Epic).contains(&"blinding"));
        assert!(!available(ItemClass::Sword, Tier::Epic).contains(&"blinding"));
        assert!(available(ItemClass::Shield, Tier::Epic).contains(&"retreating"));
        assert!(available(ItemClass::Shield, Tier::Uncommon).contains(&"venomous"));
        assert!(!available(ItemClass::Sword, Tier::Uncommon).contains(&"venomous"));
        assert!(available(ItemClass::Sword, Tier::Rare).contains(&"telepathic"));
        assert!(!available(ItemClass::Sword, Tier::Rare).contains(&"withering"));
    }

    #[test]
    fn values_follow_the_tier() {
        let venomous = EFFECT_AFFIXES.iter().find(|a| a.id == "venomous").unwrap();
        assert_eq!(value_at(venomous, Tier::Epic), 160.0);
        assert!(value_at(venomous, Tier::Mythic) > value_at(venomous, Tier::Epic));
        let executing = EFFECT_AFFIXES.iter().find(|a| a.id == "executing").unwrap();
        assert!((value_at(executing, Tier::Epic) - 0.15).abs() < 1.0e-9);
        assert!(value_at(executing, Tier::Mythic) <= 1.0);
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
        assert!(lore.len() >= modifiers.len() + 1);
        assert!(lore.len() <= modifiers.len() + 2, "one effect affix at most");
    }

    #[test]
    fn effect_rolls_round_trip_through_custom_data() {
        let mut stack = one("minecraft:diamond_sword");
        let mut rng = Rng::new(3);
        // Roll until the piece takes an effect affix (55% at epic).
        for _ in 0..40 {
            stack = one("minecraft:diamond_sword");
            apply_affixes(&mut stack, Tier::Epic, &mut rng);
            if !effects_of(&stack).is_empty() {
                break;
            }
        }
        let effects = effects_of(&stack);
        assert!(!effects.is_empty(), "no effect affix in forty pieces");
        for (_, value) in effects {
            assert!(value > 0.0);
        }
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
    fn every_boss_rarity_is_below_ancient() {
        for (tier, _) in BOSS_RARITIES {
            assert!(tier < Tier::Ancient, "{tier:?} should not be a boss roll");
        }
        assert!(boss_tiers().iter().any(|(tier, _)| *tier == Tier::Ancient));
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
