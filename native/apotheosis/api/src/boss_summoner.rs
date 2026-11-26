//! The boss summoner item (`apotheosis:boss_summoner`): Java's
//! `BossSummonerItem`, which spawns an Apothic Invader at the block clicked.
//!
//! Java picks a random entry of its invader registry whose dimension
//! constraints allow the player's level, then tries the block clicked + the
//! face, else the cell above it, and fails without consuming the item when
//! nothing fits. The port keeps those rules: the base mobs are the registry's
//! entries for the Overworld, the Nether and the End that are still monsters
//! here (a wolf is not one), the placement check reads the collision boxes the
//! API offers, and the item is taken once, only after `summon` returned an
//! entity. The invader itself is `affix_effects::become_invader`, the same
//! conversion the natural spawn uses, at the rarity the player's world tier
//! rolls; the natural spawn's three-minute invader cooldown does not apply.
//!
//! Java also calls `boss.setTarget(player)`; Horizon API 415 has no helper to
//! set a mob's target, so the summoned invader acquires one through its own
//! goals. It is summoned as `/summon` does, so no drop or spawn event is
//! invented for it.

use crate::affix_effects;
use crate::augments;
use crate::tiers::{self, WorldTier};
use crate::{Rng, Tier};
use horizon_client::mod_api::{
    Compound, EquipmentSlot, Flow, Host, Text, TooltipEvent, UseItemOnEvent,
};
use std::sync::atomic::{AtomicU64, Ordering};

/// The item's id.
pub const ITEM: &str = "apotheosis:boss_summoner";

/// The registry's Overworld invaders (`data/apotheosis/apothic_invaders/
/// overworld`), minus the wolf: it is not a `MobCategory.MONSTER`.
const OVERWORLD: [&str; 8] = [
    "minecraft:bogged",
    "minecraft:breeze",
    "minecraft:enderman",
    "minecraft:husk",
    "minecraft:skeleton",
    "minecraft:stray",
    "minecraft:vindicator",
    "minecraft:zombie",
];

/// The Nether's registry entries.
const NETHER: [&str; 6] = [
    "minecraft:blaze",
    "minecraft:piglin",
    "minecraft:piglin_brute",
    "minecraft:wither_skeleton",
    "minecraft:zoglin",
    "minecraft:zombified_piglin",
];

/// The End's registry entries.
const END: [&str; 5] = [
    "minecraft:enderman",
    "minecraft:endermite",
    "minecraft:evoker",
    "minecraft:phantom",
    "minecraft:shulker",
];

/// The base mobs an invader may be in `dimension`; a dimension Java's registry
/// gives no entry (a mod's) allows none, so the item fails without consuming.
pub fn base_mobs(dimension: &str) -> Option<&'static [&'static str]> {
    Some(match dimension {
        "minecraft:overworld" => &OVERWORLD,
        "minecraft:the_nether" => &NETHER,
        "minecraft:the_end" => &END,
        _ => return None,
    })
}

/// One admissible base mob for `dimension`, by Java's registry weights (an
/// equal draw per entry is enough here: the registry's weights are its ids'
/// defaults).
pub fn base_mob(dimension: &str, rng: &mut Rng) -> Option<&'static str> {
    let pool = base_mobs(dimension)?;
    Some(pool[rng.below(pool.len() as u64) as usize])
}

/// Where Java's click goes: the block clicked plus the clicked face, then the
/// cell above that. The first cell the level lets an entity stand in is the
/// spawn's; None when neither is free.
pub fn spot(pos: [i32; 3], face: [i32; 3], blocked: impl Fn([i32; 3]) -> bool) -> Option<[i32; 3]> {
    let first = [pos[0] + face[0], pos[1] + face[1], pos[2] + face[2]];
    let above = [first[0], first[1] + 1, first[2]];
    [first, above].into_iter().find(|cell| !blocked(*cell))
}

/// Whether `cell` has no room for the summoned mob: the cell or the one above
/// it carries a collision box (the registry's mobs are two blocks tall).
fn blocked(host: &dyn Host, cell: [i32; 3]) -> bool {
    let above = [cell[0], cell[1] + 1, cell[2]];
    !host.block_collision(cell).is_empty() || !host.block_collision(above).is_empty()
}

/// The feet position of a mob standing in `cell`.
fn feet(cell: [i32; 3]) -> [f64; 3] {
    [f64::from(cell[0]) + 0.5, f64::from(cell[1]), f64::from(cell[2]) + 0.5]
}

/// The invader's rarity at `world`: the same Java rarity table the natural
/// spawn rolls, with the tier's own luck (`augments::player_luck`).
pub fn invader_rarity(world: WorldTier, rng: &mut Rng) -> Tier {
    tiers::roll_rarity(world, augments::player_luck(world), rng)
}

/// `native_inventory_tick`'s slot number for the off hand.
const OFF_HAND: i32 = -106;

/// Uses one of the summoner in `slot`, as Java's `ItemStack.shrink(1)` does on
/// the hand that was used: a renamed summoner still counts.
fn consume(host: &mut dyn Host, slot: i32) {
    if let Some(current) = host.carried_stack(slot) {
        current.count = current.count.saturating_sub(1);
    }
}

/// A counter that makes every use roll its own mob and rarity.
static USES: AtomicU64 = AtomicU64::new(0);

/// A seed for one click: the player, the block, its face and the use counter.
fn seed_of(player: u128, pos: [i32; 3], face: [i32; 3], n: u64) -> u64 {
    let mut seed = (player as u64) ^ n.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for value in [pos[0], pos[1], pos[2], face[0], face[1], face[2]] {
        seed = seed.rotate_left(7) ^ (value as u32 as u64);
    }
    seed
}

/// The click's whole effect, apart from reading the acting player: chooses the
/// mob, finds a free cell, summons it, makes it an invader and consumes one
/// item from `slot`. False, and nothing consumed, when the dimension allows no
/// mob, no cell is free or the summon was refused.
pub fn use_at(
    host: &mut dyn Host,
    player: u128,
    world: WorldTier,
    dimension: &str,
    pos: [i32; 3],
    face: [i32; 3],
    slot: i32,
) -> bool {
    let n = USES.fetch_add(1, Ordering::Relaxed);
    let mut rng = Rng::new(seed_of(player, pos, face, n));
    let Some(kind) = base_mob(dimension, &mut rng) else {
        return false;
    };
    let Some(cell) = spot(pos, face, |cell| blocked(host, cell)) else {
        return false;
    };
    let Some(entity) = host.summon(dimension, kind, feet(cell), Compound::new()) else {
        return false;
    };
    let tier = invader_rarity(world, &mut rng);
    affix_effects::become_invader(host, dimension, entity, kind, world, tier, &mut rng);
    consume(host, slot);
    true
}

/// The created item's one tooltip line: what the click does. The item has no
/// Java description, so this is the port's own text, in each lang file.
pub fn tooltip(event: &mut TooltipEvent<'_>) {
    if &*event.stack.item != ITEM {
        return;
    }
    event.lines.push((
        Text::translate_with_fallback(
            "item.apotheosis.boss_summoner.desc",
            "Summons an Apothic Invader on the block you click",
            &[],
        )
        .with_color(0x80_8080),
        crate::tip_colour(0x80_8080),
    ));
}

/// A right click with the summoner on a block. Claimed only when the summon
/// happened, so a failed use leaves the click (and the item) alone, as Java's
/// `InteractionResult.FAIL` does.
pub fn on_use_item_on(host: &mut dyn Host, event: &mut UseItemOnEvent<'_>) -> Flow {
    if &*event.stack.item != ITEM {
        return Flow::Continue;
    }
    let Some(player) = host.acting_player() else {
        return Flow::Continue;
    };
    // The hook names no hand: the used stack is the main hand's when the two
    // match, the off hand's otherwise.
    let main = host.player_equipment(player.uuid, EquipmentSlot::MainHand);
    let slot = match &main {
        Some(main) if main.item == event.stack.item && main.count == event.stack.count => {
            i32::from(player.selected_slot)
        }
        _ => OFF_HAND,
    };
    let world = tiers::tier_of_with(host, player.uuid);
    if use_at(
        host,
        player.uuid,
        world,
        &player.dimension,
        event.pos,
        event.face,
        slot,
    ) {
        Flow::Cancel
    } else {
        Flow::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monsters;
    use horizon_client::mod_api::{EntityRef, ItemStack};
    use std::cell::{Cell, RefCell};
    use std::collections::HashSet;

    #[test]
    fn every_base_mob_is_a_monster_of_its_dimension() {
        for dimension in [
            "minecraft:overworld",
            "minecraft:the_nether",
            "minecraft:the_end",
        ] {
            let pool = base_mobs(dimension).expect(dimension);
            assert!(!pool.is_empty(), "{dimension}");
            for kind in pool {
                assert!(monsters::is_monster(kind), "{kind} is not a Monster");
            }
        }
        // Java's registry lists a wolf in the Overworld; a wolf is no monster.
        assert!(!OVERWORLD.contains(&"minecraft:wolf"));
        // A mod's dimension has no registry entry, so nothing can be summoned.
        assert!(base_mobs("mymod:void").is_none());
    }

    #[test]
    fn the_clicked_face_is_tried_before_the_cell_above() {
        let pos = [4, 64, 4];
        let top = [0, 1, 0];
        assert_eq!(
            spot(pos, top, |_| false),
            Some([4, 65, 4]),
            "the cell on the clicked face comes first"
        );
        assert_eq!(
            spot(pos, [1, 0, 0], |_| false),
            Some([5, 64, 4]),
            "a side face places beside the block"
        );
        // The first cell occupied: the one above it is tried.
        assert_eq!(
            spot(pos, top, |cell| cell == [4, 65, 4]),
            Some([4, 66, 4])
        );
        // Both occupied: the click fails.
        assert_eq!(
            spot(pos, top, |cell| cell[1] == 65 || cell[1] == 66),
            None
        );
        // A mob is two blocks tall: the free cell must have room for both, and
        // only the clicked face and the cell above it are tried.
        let mut host = Fake::default();
        host.blocked.insert([4, 66, 4]);
        assert_eq!(
            spot(pos, top, |cell| blocked(&host, cell)),
            None,
            "the head room of the first cell is taken, and the second is blocked"
        );
        let mut host = Fake::default();
        host.blocked.insert([4, 65, 4]);
        assert_eq!(
            spot(pos, top, |cell| blocked(&host, cell)),
            Some([4, 66, 4]),
            "a collision line at the bottom of the target cell moves the mob up"
        );
    }

    #[test]
    fn the_rarity_follows_the_world_tier() {
        // Haven's table weighs only common, uncommon and rare.
        for seed in 0..500 {
            let tier = invader_rarity(WorldTier::Haven, &mut Rng::new(seed));
            assert!(tier <= Tier::Rare, "Haven rolled {tier:?}");
        }
        // Pinnacle's table has no common or uncommon, and reaches mythic.
        let mut mythic = false;
        for seed in 0..500 {
            let tier = invader_rarity(WorldTier::Pinnacle, &mut Rng::new(seed));
            assert!(tier >= Tier::Rare, "Pinnacle rolled {tier:?}");
            mythic |= tier == Tier::Mythic;
        }
        assert!(mythic, "Pinnacle never reached mythic");
    }

    #[test]
    fn two_uses_in_a_row_both_summon_without_the_natural_cooldown() {
        let mut host = Fake::default();
        let stack = ItemStack::new(ITEM, 3);
        host.slots.insert(0, stack.clone());
        let pos = [0, 64, 0];
        let face = [0, 1, 0];
        for _ in 0..2 {
            assert!(use_at(
                &mut host,
                7,
                WorldTier::Summit,
                "minecraft:overworld",
                pos,
                face,
                0,
            ));
        }
        assert_eq!(host.summoned.borrow().len(), 2, "the second use was refused");
        assert_eq!(host.slots.get(&0).unwrap().count, 1, "two uses took two items");
    }

    #[test]
    fn a_failed_use_consumes_nothing() {
        let stack = ItemStack::new(ITEM, 1);
        let count = |host: &Fake| host.slots.get(&0).map_or(0, |stack| stack.count);
        let used = |mut host: Fake, dimension: &str| {
            host.slots.insert(0, stack.clone());
            assert!(!use_at(
                &mut host,
                7,
                WorldTier::Summit,
                dimension,
                [0, 64, 0],
                [0, 1, 0],
                0,
            ));
            assert!(host.summoned.borrow().is_empty());
            assert_eq!(count(&host), 1, "a failed use consumed the item");
        };

        // A dimension Java's registry gives no invader for.
        used(Fake::default(), "mymod:void");

        // No room: the clicked face and the cell above it are both occupied.
        let mut host = Fake::default();
        for y in 65..=67 {
            host.blocked.insert([0, y, 0]);
        }
        host.slots.insert(0, stack.clone());
        assert!(!use_at(
            &mut host,
            7,
            WorldTier::Summit,
            "minecraft:overworld",
            [0, 64, 0],
            [0, 1, 0],
            0,
        ));
        assert!(host.summoned.borrow().is_empty());
        assert_eq!(count(&host), 1);

        // The level refused the summon: no item is taken either.
        let mut host = Fake::default();
        host.refuse_summon.set(true);
        host.slots.insert(0, stack.clone());
        assert!(!use_at(
            &mut host,
            7,
            WorldTier::Summit,
            "minecraft:overworld",
            [0, 64, 0],
            [0, 1, 0],
            0,
        ));
        assert_eq!(count(&host), 1);
    }

    /// A host for the summoner: collision boxes at some cells, and a summon
    /// that succeeds unless told otherwise. `entity` gives nothing, so the
    /// conversion stops before touching the mob (its own host calls are not
    /// what these tests are about).
    #[derive(Default)]
    struct Fake {
        /// The cells that collide with an entity.
        blocked: HashSet<[i32; 3]>,
        refuse_summon: Cell<bool>,
        summoned: RefCell<Vec<(String, String, [f64; 3])>>,
        slots: std::collections::HashMap<i32, ItemStack>,
    }

    impl Host for Fake {
        fn carried_stack(&mut self, slot: i32) -> Option<&mut ItemStack> {
            self.slots.get_mut(&slot)
        }

        fn block_collision(&self, pos: [i32; 3]) -> Vec<[f64; 6]> {
            if self.blocked.contains(&pos) {
                vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]
            } else {
                Vec::new()
            }
        }

        fn summon(
            &mut self,
            dimension: &str,
            kind: &str,
            pos: [f64; 3],
            _nbt: Compound,
        ) -> Option<u32> {
            if self.refuse_summon.get() {
                return None;
            }
            self.summoned
                .borrow_mut()
                .push((dimension.to_owned(), kind.to_owned(), pos));
            Some(u32::try_from(self.summoned.borrow().len()).unwrap())
        }

        fn entity(&self, _id: u32) -> Option<EntityRef> {
            None
        }

    }

}
