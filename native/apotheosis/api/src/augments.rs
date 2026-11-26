//! The world tiers' augments, step 3: what a tier gives a monster when it
//! spawns, and what it gives a player who has it active.
//!
//! Java keeps these as data (`data/apotheosis/tier_augments/<tier>/*.json`)
//! and applies them as attribute modifiers: `add_value` on a monster's armour,
//! armour toughness and Apothic Attributes, `add_multiplied_total` on a
//! player's experience and luck, and `DamageReductionAugment`s on a monster's
//! physical and magic damage. Only the active tier's augments apply; they do
//! not add up from tier to tier. The "max Eterna" rows only concern Apothic
//! Enchanting and are left out.
//!
//! Horizon 415 has no named, removable attribute modifier for an entity. Its
//! one write, `entity_set_attribute_base`, fills a base that the game's mob
//! damage reads only for health, speed and the like: armour comes from the
//! type's own default and from an attribute map that no hook writes
//! (`mobs::armor::values`), so a mob's armour base would never reduce a hit.
//! Every monster augment therefore lives in a per-entity cache that
//! `attributes` reads from the damage hooks, as it already reads the affix
//! attributes there: the tier's armour points are applied as the share
//! vanilla's armour formula gives them (armour and toughness against the
//! hit's size, 4% a point for a mob with no armour of its own), and the
//! physical and magic reductions as shares of their damage class. A mob is
//! marked when it spawns, so a second hook for the same mob adds nothing.
//!
//! Players have a real `minecraft:luck` attribute and
//! `native_set_attribute_base`. Java's tier luck is `add_multiplied_total` on a
//! base of zero, which weighs nothing, so the bonus's own value is the coherent
//! equivalent: the port's rolls take it as their luck, and the player's
//! `minecraft:luck` base is set to it. The base is written absolutely, so a
//! tier change replaces the old bonus instead of adding the new one to it.
//! Experience is multiplied in the `player_xp_change` hook (API 394), which
//! sees every point a player gains.

use horizon_client::mod_api::{Host, MobSpawnedEvent, PlayerXpChangeEvent};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::tiers::WorldTier;

/// What a tier gives a monster, from Java's `tier_augments/<tier>` files: the
/// armour and toughness `add_value`s and the Apothic Attributes Java adds the
/// same way. Every one of them reaches a hit through `reduction` and `pierce`
/// rather than the game's attribute registry.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MonsterAugment {
    /// `minecraft:armor`, `add_value`.
    pub armor: f64,
    /// `minecraft:armor_toughness`, `add_value`.
    pub armor_toughness: f64,
    /// `apothic_attributes:armor_pierce`, `add_value`.
    pub armor_pierce: f64,
    /// `apothic_attributes:prot_pierce`, `add_value`.
    pub prot_pierce: f64,
    /// Java's `DamageReductionAugment` for the physical damage class.
    pub physical_reduction: f64,
    /// The same for magic damage.
    pub magic_reduction: f64,
}

impl MonsterAugment {
    /// The augments of `tier` alone. Nothing sums them from the tiers below.
    pub fn of(tier: WorldTier) -> Self {
        match tier {
            WorldTier::Haven => Self::default(),
            WorldTier::Frontier => Self {
                armor: 4.0,
                ..Self::default()
            },
            WorldTier::Ascent => Self {
                armor: 8.0,
                armor_pierce: 5.0,
                ..Self::default()
            },
            WorldTier::Summit => Self {
                armor: 12.0,
                armor_toughness: 10.0,
                armor_pierce: 15.0,
                prot_pierce: 10.0,
                physical_reduction: 0.1,
                magic_reduction: 0.1,
            },
            WorldTier::Pinnacle => Self {
                armor: 16.0,
                armor_toughness: 20.0,
                armor_pierce: 25.0,
                prot_pierce: 20.0,
                physical_reduction: 0.15,
                magic_reduction: 0.15,
            },
        }
    }

    /// Whether the tier gives a monster nothing at all (Haven's).
    pub fn is_empty(self) -> bool {
        self == Self::default()
    }
}

/// How long an augment is kept before its entity id is forgotten, in case a
/// mob despawned without dying (a death forgets it at once).
const TRACKED_FOR: Duration = Duration::from_secs(30 * 60);

/// The map is pruned once it reaches this many mobs.
const CAP: usize = 4096;

/// The augments the monsters alive now carry, by entity id, read from here by
/// the damage hooks.
static MONSTERS: OnceLock<Mutex<HashMap<u32, (MonsterAugment, Instant)>>> = OnceLock::new();

fn monsters() -> &'static Mutex<HashMap<u32, (MonsterAugment, Instant)>> {
    MONSTERS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The augments `entity` carries, none for a mob without any.
fn bonus_of(entity: u32) -> Option<MonsterAugment> {
    monsters()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&entity)
        .map(|(bonus, _)| *bonus)
}

/// Marks `entity` as carrying `tier`'s augments, which are then returned. A
/// mob already marked answers `None`, and so does a tier with none.
fn plan_spawn(tier: WorldTier, entity: u32) -> Option<MonsterAugment> {
    let bonus = MonsterAugment::of(tier);
    if bonus.is_empty() {
        return None;
    }
    let mut tracked = monsters()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if tracked.contains_key(&entity) {
        return None;
    }
    if tracked.len() >= CAP {
        tracked.retain(|_, (_, at)| at.elapsed() < TRACKED_FOR);
    }
    tracked.insert(entity, (bonus, Instant::now()));
    Some(bonus)
}

/// Drops a dead mob's augments.
pub fn forget(entity: u32) {
    monsters()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&entity);
}

/// `mob_spawned`: a monster takes the active tier's augments when it spawns,
/// the tier of the player nearest it, as Java's `GenContext.of(level, entity)`
/// reads it. `entity` is the id `affix_effects::take_spawn` matched to the
/// event, which names no entity itself.
pub fn monster_spawned(
    host: &mut dyn Host,
    event: &MobSpawnedEvent<'_>,
    entity: Option<u32>,
) {
    if !crate::monsters::is_monster(event.kind) {
        return;
    }
    let Some(entity) = entity else {
        return;
    };
    let Some(player) = crate::tiers::nearest_player(host, event.dimension, event.pos) else {
        return;
    };
    let tier = crate::tiers::tier_of_with(host, player);
    // No augment changes max health, so the mob keeps the health share it
    // spawned with, as Java's `tryAugmentations` preserves it by hand.
    plan_spawn(tier, entity);
}

/// The share the excess armour removes from a hit of `amount`, as vanilla's
/// `CombatRules.getDamageAfterAbsorb` works it out for the tier's points alone:
/// `clamp(armour - amount / (2 + toughness / 4), armour * 0.2, 20) / 25`. With
/// no armour of its own, 4% a point at most.
fn armor_share(armor: f64, toughness: f64, amount: f64) -> f64 {
    if armor <= 0.0 || amount <= 0.0 {
        return 0.0;
    }
    let effective = (armor - amount / (2.0 + toughness / 4.0)).clamp(armor * 0.2, 20.0);
    (effective / 25.0).clamp(0.0, 0.8)
}

/// The share a spawned monster removes from a hit of `damage_type` and
/// `amount`: its tier's armour, plus its physical or magic reduction, zero for
/// any other class. Read from the damage hook, which has no host.
pub fn reduction(entity: u32, damage_type: &str, amount: f64) -> f64 {
    let Some(bonus) = bonus_of(entity) else {
        return 0.0;
    };
    if crate::attributes::is_physical(damage_type) {
        armor_share(bonus.armor, bonus.armor_toughness, amount) + bonus.physical_reduction
    } else if crate::attributes::damage_class(damage_type) == "magic" {
        bonus.magic_reduction
    } else {
        0.0
    }
}

/// A spawned monster's armour perforation and protection perforation, for the
/// blow it deals: `(armor_pierce, prot_pierce)`, none for a mob without
/// augments.
pub fn pierce(entity: u32) -> Option<(f64, f64)> {
    let bonus = bonus_of(entity)?;
    Some((bonus.armor_pierce, bonus.prot_pierce))
}

// ---------------------------------------------------------------------------
// Player bonuses
// ---------------------------------------------------------------------------

/// The experience a tier's player bonus adds, Java's `add_multiplied_total` on
/// `apothic_attributes:experience_gained`. `0.35` is `+35%`.
pub fn player_xp_bonus(tier: WorldTier) -> f64 {
    match tier {
        WorldTier::Haven => 0.0,
        WorldTier::Frontier => 0.35,
        WorldTier::Ascent => 0.55,
        WorldTier::Summit => 0.75,
        WorldTier::Pinnacle => 1.25,
    }
}

/// The multiplier a player of `tier` gains experience at.
pub fn xp_multiplier(tier: WorldTier) -> f64 {
    1.0 + player_xp_bonus(tier)
}

/// The luck a player of `tier` has, Java's `add_multiplied_total` on
/// `minecraft:luck`. A base of zero would make that operation weigh nothing,
/// so the bonus's own value is used; it is both the luck the port's rolls take
/// and the value of the player's luck attribute.
pub fn player_luck(tier: WorldTier) -> f32 {
    match tier {
        WorldTier::Haven => 0.0,
        WorldTier::Frontier => 0.2,
        WorldTier::Ascent => 0.3,
        WorldTier::Summit => 0.5,
        WorldTier::Pinnacle => 1.0,
    }
}

/// The luck last written into each player's `minecraft:luck`, so a tick that
/// changes nothing writes nothing. The write is absolute, so a lost entry
/// only costs one rewrite.
static PLAYERS: OnceLock<Mutex<HashMap<u128, f32>>> = OnceLock::new();

fn players() -> &'static Mutex<HashMap<u128, f32>> {
    PLAYERS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// `player_tick`: writes the active tier's luck into the player's
/// `minecraft:luck` base when it differs from the one already written. An
/// absolute base, so changing tier takes the old bonus away and puts the new
/// one in its place; Haven's zero restores vanilla's.
pub fn player_tick(host: &mut dyn Host, player: u128, tier: WorldTier) {
    let luck = player_luck(tier);
    let applied = players()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&player)
        .copied();
    if applied == Some(luck) {
        return;
    }
    let mut written = players()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if host.set_attribute_base(player, "minecraft:luck", f64::from(luck)) {
        written.insert(player, luck);
    } else {
        // A dead or unreachable player: try again next tick.
        written.remove(&player);
    }
}

/// `player_xp_change` (API 394): every point a player gains carries the active
/// tier's experience bonus. A loss, `/xp add -5`, is left alone, as Java's
/// multiplier only raises a gain.
pub fn xp_change(host: &mut dyn Host, event: &mut PlayerXpChangeEvent<'_>) {
    let tier = crate::tiers::tier_of_with(host, event.player);
    event.amount = xp_gain(event.amount, tier);
}

/// The points `amount` becomes at `tier`, rounded to the nearest point.
fn xp_gain(amount: i32, tier: WorldTier) -> i32 {
    if amount <= 0 {
        return amount;
    }
    let multiplier = xp_multiplier(tier);
    if (multiplier - 1.0).abs() < f64::EPSILON {
        return amount;
    }
    (f64::from(amount) * multiplier).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Fake {
        written: Vec<(String, f64)>,
    }

    impl Host for Fake {
        fn set_attribute_base(&mut self, _uuid: u128, attribute: &str, base: f64) -> bool {
            self.written.push((attribute.to_owned(), base));
            true
        }
    }

    #[test]
    fn the_monster_table_is_javas() {
        assert_eq!(MonsterAugment::of(WorldTier::Haven), MonsterAugment::default());
        assert_eq!(
            MonsterAugment::of(WorldTier::Frontier),
            MonsterAugment {
                armor: 4.0,
                ..MonsterAugment::default()
            }
        );
        assert_eq!(
            MonsterAugment::of(WorldTier::Ascent),
            MonsterAugment {
                armor: 8.0,
                armor_pierce: 5.0,
                ..MonsterAugment::default()
            }
        );
        let summit = MonsterAugment::of(WorldTier::Summit);
        assert_eq!(summit.armor, 12.0);
        assert_eq!(summit.armor_toughness, 10.0);
        assert_eq!(summit.armor_pierce, 15.0);
        assert_eq!(summit.prot_pierce, 10.0);
        assert_eq!(summit.physical_reduction, 0.1);
        assert_eq!(summit.magic_reduction, 0.1);
        let pinnacle = MonsterAugment::of(WorldTier::Pinnacle);
        assert_eq!(pinnacle.armor, 16.0);
        assert_eq!(pinnacle.armor_toughness, 20.0);
        assert_eq!(pinnacle.armor_pierce, 25.0);
        assert_eq!(pinnacle.prot_pierce, 20.0);
        assert_eq!(pinnacle.physical_reduction, 0.15);
        assert_eq!(pinnacle.magic_reduction, 0.15);
    }

    #[test]
    fn the_player_table_is_javas() {
        assert_eq!(player_xp_bonus(WorldTier::Haven), 0.0);
        assert_eq!(player_xp_bonus(WorldTier::Frontier), 0.35);
        assert_eq!(player_xp_bonus(WorldTier::Ascent), 0.55);
        assert_eq!(player_xp_bonus(WorldTier::Summit), 0.75);
        assert_eq!(player_xp_bonus(WorldTier::Pinnacle), 1.25);

        let luck: Vec<f32> = WorldTier::ALL.iter().map(|tier| player_luck(*tier)).collect();
        assert_eq!(luck, vec![0.0, 0.2, 0.3, 0.5, 1.0]);
    }

    #[test]
    fn only_the_active_tiers_augments_apply() {
        // Summit's armour is 12, not Frontier's 4 + Ascent's 8 + its own 12.
        assert_eq!(MonsterAugment::of(WorldTier::Summit).armor, 12.0);
        assert_eq!(MonsterAugment::of(WorldTier::Pinnacle).armor, 16.0);
        // The player's luck is Pinnacle's 1.0, not the sum of every tier's.
        assert_eq!(player_luck(WorldTier::Pinnacle), 1.0);
        assert!((xp_multiplier(WorldTier::Pinnacle) - 2.25).abs() < 1e-9);
    }

    #[test]
    fn a_mob_takes_its_augments_once() {
        let entity = 0x00C0_FFEE;
        forget(entity);
        assert_eq!(plan_spawn(WorldTier::Ascent, entity), Some(MonsterAugment::of(WorldTier::Ascent)));
        assert_eq!(plan_spawn(WorldTier::Ascent, entity), None, "a second spawn applied them again");
        assert_eq!(
            plan_spawn(WorldTier::Pinnacle, entity),
            None,
            "a higher tier re-applied them to an existing mob"
        );
        // A death forgets them, so a reused id starts clean.
        forget(entity);
        assert!(plan_spawn(WorldTier::Frontier, entity).is_some());
        forget(entity);
    }

    #[test]
    fn haven_marks_no_mob() {
        let entity = 0x00C0_FFEF;
        forget(entity);
        assert_eq!(plan_spawn(WorldTier::Haven, entity), None);
        assert_eq!(pierce(entity), None);
    }

    #[test]
    fn the_armour_share_is_vanillas_formula_for_the_tiers_points() {
        // No toughness, 4 points: 4 - 10 / 2 floors on the 20% minimum.
        assert!((armor_share(4.0, 0.0, 10.0) - 0.032).abs() < 1e-9);
        // A small hit lets more of the armour count: 4 - 1 / 2 = 3.5 points.
        assert!((armor_share(4.0, 0.0, 1.0) - 0.14).abs() < 1e-9);
        // Pinnacle's 16 points and 20 toughness: 16 - 10 / 7 = 14.57 points.
        assert!((armor_share(16.0, 20.0, 10.0) - 14.571_428_571_428_572 / 25.0).abs() < 1e-9);
        assert_eq!(armor_share(0.0, 20.0, 10.0), 0.0, "Haven adds none");
    }

    #[test]
    fn reductions_and_pierce_are_read_by_entity() {
        let entity = 0x00C0_FFF0;
        forget(entity);
        let summit = plan_spawn(WorldTier::Summit, entity).expect("Summit augments a mob");
        let physical = armor_share(summit.armor, summit.armor_toughness, 10.0)
            + summit.physical_reduction;
        assert!((reduction(entity, "minecraft:player_attack", 10.0) - physical).abs() < 1e-9);
        assert_eq!(
            reduction(entity, "minecraft:arrow", 10.0),
            reduction(entity, "minecraft:player_attack", 10.0)
        );
        assert_eq!(
            reduction(entity, "minecraft:magic", 10.0),
            summit.magic_reduction
        );
        assert_eq!(reduction(entity, "minecraft:fall", 10.0), 0.0);
        assert_eq!(pierce(entity), Some((15.0, 10.0)));
        forget(entity);
        assert_eq!(reduction(entity, "minecraft:player_attack", 10.0), 0.0);
        assert_eq!(pierce(entity), None);
    }

    #[test]
    fn changing_tier_replaces_the_players_luck() {
        let player = 0x0000_ABCD_0000_0001;
        let mut host = Fake::default();
        player_tick(&mut host, player, WorldTier::Frontier);
        player_tick(&mut host, player, WorldTier::Frontier);
        assert_eq!(host.written.len(), 1, "the same tier writes once");
        assert_eq!(host.written[0].0, "minecraft:luck");
        assert!((host.written[0].1 - 0.2).abs() < 1e-6);
        player_tick(&mut host, player, WorldTier::Summit);
        assert_eq!(host.written.len(), 2);
        assert!(
            (host.written[1].1 - 0.5).abs() < 1e-6,
            "the new tier's bonus replaces the old one"
        );
        player_tick(&mut host, player, WorldTier::Haven);
        assert_eq!(host.written.last().unwrap().1, 0.0, "Haven restores vanilla's zero");
        players()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&player);
    }

    #[test]
    fn experience_gains_are_multiplied_and_losses_are_not() {
        assert_eq!(xp_gain(10, WorldTier::Haven), 10);
        assert_eq!(xp_gain(10, WorldTier::Frontier), 14);
        assert_eq!(xp_gain(10, WorldTier::Ascent), 16);
        assert_eq!(xp_gain(10, WorldTier::Summit), 18);
        assert_eq!(xp_gain(10, WorldTier::Pinnacle), 23);
        assert_eq!(xp_gain(-5, WorldTier::Pinnacle), -5);
        assert_eq!(xp_gain(0, WorldTier::Pinnacle), 0);
    }
}
