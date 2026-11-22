use horizon_client::mod_api::{
    EquipmentSlot, Flow, Hand, Host, ItemStack, PrepareItemEnchantEvent, Text, UseItemEvent,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq)]
struct ShelfStats {
    eterna: f32,
    max_eterna: f32,
    quanta: f32,
    arcana: f32,
    clues: i32,
}

impl ShelfStats {
    // Fields absent from a shelf's data file default to maxEterna 30 and zero.
    const DEFAULT: ShelfStats = ShelfStats {
        eterna: 0.0,
        max_eterna: 30.0,
        quanta: 0.0,
        arcana: 0.0,
        clues: 0,
    };
}

// Public 1.21 Apothic Enchanting enchanting_stats values. Fields omitted in
// its data files use maxEterna=30 and zero for the remaining stats.
const SHELVES: &[(&str, ShelfStats)] = &[
    ("amethyst_cluster", ShelfStats { eterna: 1.0, max_eterna: 40.0, quanta: -1.0, ..ShelfStats::DEFAULT }),
    ("basic_bookshelf", ShelfStats { eterna: 2.0, ..ShelfStats::DEFAULT }),
    ("basic_skulls", ShelfStats { quanta: 5.0, ..ShelfStats::DEFAULT }),
    ("beeshelf", ShelfStats { eterna: -30.0, quanta: 100.0, ..ShelfStats::DEFAULT }),
    ("blazing_hellshelf", ShelfStats { eterna: 10.0, max_eterna: 65.0, quanta: 10.0, clues: -1, ..ShelfStats::DEFAULT }),
    ("crystal_seashelf", ShelfStats { eterna: 5.0, max_eterna: 60.0, quanta: 3.0, arcana: 5.0, ..ShelfStats::DEFAULT }),
    ("deepshelf", ShelfStats { eterna: 5.0, max_eterna: 70.0, quanta: 5.0, arcana: 5.0, ..ShelfStats::DEFAULT }),
    ("dormant_deepshelf", ShelfStats { eterna: 2.0, ..ShelfStats::DEFAULT }),
    ("draconic_endshelf", ShelfStats { eterna: 20.0, max_eterna: 100.0, ..ShelfStats::DEFAULT }),
    ("echoing_deepshelf", ShelfStats { eterna: 5.0, max_eterna: 75.0, arcana: 15.0, ..ShelfStats::DEFAULT }),
    ("echoing_sculkshelf", ShelfStats { eterna: 10.0, max_eterna: 80.0, quanta: 10.0, arcana: 15.0, clues: 1, ..ShelfStats::DEFAULT }),
    ("endshelf", ShelfStats { eterna: 5.0, max_eterna: 90.0, quanta: 5.0, arcana: 5.0, ..ShelfStats::DEFAULT }),
    ("glowing_hellshelf", ShelfStats { eterna: 5.0, max_eterna: 60.0, quanta: 5.0, arcana: 3.3333, ..ShelfStats::DEFAULT }),
    ("heart_seashelf", ShelfStats { eterna: 15.0, max_eterna: 60.0, arcana: 20.0, ..ShelfStats::DEFAULT }),
    ("hellshelf", ShelfStats { eterna: 3.0, max_eterna: 45.0, quanta: 3.0, ..ShelfStats::DEFAULT }),
    ("infused_hellshelf", ShelfStats { eterna: 5.0, max_eterna: 60.0, quanta: 5.0, ..ShelfStats::DEFAULT }),
    ("infused_seashelf", ShelfStats { eterna: 5.0, max_eterna: 60.0, arcana: 5.0, ..ShelfStats::DEFAULT }),
    ("melonshelf", ShelfStats { eterna: -2.0, quanta: -10.0, ..ShelfStats::DEFAULT }),
    ("pearl_endshelf", ShelfStats { eterna: 10.0, max_eterna: 90.0, quanta: 7.5, arcana: 7.5, ..ShelfStats::DEFAULT }),
    ("seashelf", ShelfStats { eterna: 3.0, max_eterna: 45.0, arcana: 3.0, ..ShelfStats::DEFAULT }),
    ("sightshelf", ShelfStats { clues: 1, ..ShelfStats::DEFAULT }),
    ("sightshelf_t2", ShelfStats { clues: 2, ..ShelfStats::DEFAULT }),
    ("soul_touched_deepshelf", ShelfStats { eterna: 5.0, max_eterna: 75.0, quanta: 15.0, ..ShelfStats::DEFAULT }),
    ("soul_touched_sculkshelf", ShelfStats { eterna: 10.0, max_eterna: 80.0, quanta: 15.0, arcana: 10.0, clues: 1, ..ShelfStats::DEFAULT }),
    ("stoneshelf", ShelfStats { eterna: -3.0, arcana: -7.5, ..ShelfStats::DEFAULT }),
    ("wither_skull", ShelfStats { quanta: 10.0, ..ShelfStats::DEFAULT }),
];

// Vanilla checks two blocks out at the 32 perimeter positions, reading the
// outer block only when the intervening position is air.
const OFFSETS: &[(i32, i32)] = &[
    (-2, -1), (-2, 0), (-2, 1), (-1, -2), (-1, 2),
    (0, -2), (0, 2), (1, -2), (1, 2), (2, -1), (2, 0), (2, 1),
];

fn shelf_stats(name: &str) -> Option<ShelfStats> {
    SHELVES.iter().find(|(id, _)| *id == name).map(|(_, stats)| *stats)
}

fn aggregate(stats: impl IntoIterator<Item = ShelfStats>) -> ShelfStats {
    // Vanilla defaults: 15 Quanta and one clue. Eterna caps are applied in
    // ascending cap order; non-positive-cap entries are uncapped.
    let mut values = ShelfStats { quanta: 15.0, clues: 1, ..ShelfStats::DEFAULT };
    let mut capped = Vec::new();
    for shelf in stats {
        values.quanta += shelf.quanta;
        values.arcana += shelf.arcana;
        values.clues += shelf.clues;
        if shelf.max_eterna > 0.0 {
            capped.push((shelf.max_eterna, shelf.eterna));
        } else {
            values.eterna += shelf.eterna;
        }
    }
    capped.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (cap, contribution) in capped {
        values.eterna = (values.eterna + contribution).min(cap);
    }
    values.eterna = values.eterna.clamp(0.0, 100.0);
    values.quanta = values.quanta.clamp(0.0, 100.0);
    values.arcana = values.arcana.clamp(0.0, 100.0);
    values.clues = values.clues.max(0);
    values
}

fn read_table_stats(host: &dyn Host, pos: [i32; 3]) -> ShelfStats {
    let mut shelves = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for &(dx, dz) in OFFSETS {
        for dy in [0, 1] {
            let middle = [pos[0] + dx / 2, pos[1] + dy, pos[2] + dz / 2];
            if host.block(middle) != "minecraft:air" {
                continue;
            }
            let shelf_pos = [pos[0] + dx, pos[1] + dy, pos[2] + dz];
            if !seen.insert(shelf_pos) {
                continue;
            }
            if let Some(stats) = shelf_stats(&host.block(shelf_pos)) {
                shelves.push(stats);
            }
        }
    }
    aggregate(shelves)
}

fn apply_costs(costs: &mut [i32; 3], eterna: f32) {
    let eterna = eterna.max(1.5).round() as i32;
    // Apothic Enchanting scales its three offers from Eterna. Keep each offer
    // at or above vanilla's original eligibility threshold (1, 2, 3).
    let factors = [(0.2_f32, 0.4_f32), (0.6, 0.8)];
    for (slot, cost) in costs.iter_mut().enumerate() {
        let min_cost = slot as i32 + 1;
        let scaled = match slot {
            2 => eterna.max(min_cost),
            0 | 1 => {
                let (low, high) = factors[slot];
                let floor = (eterna as f32 * low).round() as i32;
                let ceil = (eterna as f32 * high).round() as i32;
                ((floor + ceil) / 2).max(min_cost)
            }
            _ => *cost,
        };
        // Eterna raises offers; it must not undercut a table that vanilla
        // already made expensive through its ordinary bookshelves.
        *cost = (*cost).max(scaled);
    }
}

pub(super) fn on_prepare_item_enchant(host: &dyn Host, event: &mut PrepareItemEnchantEvent<'_>) {
    let stats = read_table_stats(host, event.pos);
    apply_costs(event.costs, stats.eterna);
}

// ---------------------------------------------------------------------------
// Tomes and the Prismatic Cobweb
// ---------------------------------------------------------------------------
// Horizon API 415 has no anvil hook, so Apothic Enchanting's tomes are used on
// a right click instead: the tome in one hand, the enchanted item in the other.

const SCRAP_TOME: &str = "apotheosis:scrap_tome";
const IMPROVED_TOME: &str = "apotheosis:improved_scrap_tome";
const EXTRACTION_TOME: &str = "apotheosis:extraction_tome";
const PRISMATIC_WEB: &str = "apotheosis:prismatic_web";

const SCRAP_COST: i32 = 6;
const IMPROVED_COST: i32 = 10;
const EXTRACTION_COST: i32 = 16;
const WEB_COST: i32 = 30;

const CURSES: &[&str] = &["minecraft:binding_curse", "minecraft:vanishing_curse"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tome {
    Scrap,
    Improved,
    Extraction,
    Web,
}

fn tome_kind(item: &str) -> Option<Tome> {
    match item {
        SCRAP_TOME => Some(Tome::Scrap),
        IMPROVED_TOME => Some(Tome::Improved),
        EXTRACTION_TOME => Some(Tome::Extraction),
        PRISMATIC_WEB => Some(Tome::Web),
        _ => None,
    }
}

/// Keeps `ceil(n / 2)` enchantments. A seeded xorshift draws which ones, so
/// the same player sees the same result for the same item.
fn scrap_selection(levels: &[(Arc<str>, u32)], seed: u64) -> Vec<(Arc<str>, u32)> {
    let keep = levels.len().div_ceil(2);
    let mut pool = levels.to_vec();
    let mut state = seed | 1;
    for index in 0..keep {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let pick = index + (state % (pool.len() - index) as u64) as usize;
        pool.swap(index, pick);
    }
    pool.truncate(keep);
    pool
}

fn strip_curses(levels: &[(Arc<str>, u32)]) -> (Vec<(Arc<str>, u32)>, usize) {
    let kept: Vec<_> = levels
        .iter()
        .filter(|(id, _)| !CURSES.contains(&&**id))
        .cloned()
        .collect();
    let removed = levels.len() - kept.len();
    (kept, removed)
}

fn tome_cost(tome: Tome, enchantments: usize) -> i32 {
    match tome {
        Tome::Scrap => enchantments as i32 * SCRAP_COST,
        Tome::Improved => enchantments as i32 * IMPROVED_COST,
        Tome::Extraction => enchantments as i32 * EXTRACTION_COST,
        Tome::Web => WEB_COST,
    }
}

fn output_stack(tome: Tome, target: &ItemStack, kept: Vec<(Arc<str>, u32)>) -> ItemStack {
    if tome == Tome::Web {
        let mut stack = target.clone();
        stack.set_crafting_enchantments(kept);
        stack
    } else {
        let mut book = ItemStack::new("minecraft:enchanted_book", 1);
        book.set_crafting_enchantments(kept);
        book
    }
}

fn dice(uuid: u128) -> u64 {
    (uuid as u64) ^ ((uuid >> 64) as u64)
}

pub(super) fn on_use_item(host: &mut dyn Host, event: &mut UseItemEvent<'_>) -> Flow {
    let Some(tome) = tome_kind(&event.stack.item) else {
        return Flow::Continue;
    };
    let Some(player) = host.acting_player() else {
        return Flow::Continue;
    };
    if player.entity_id <= 0 {
        return Flow::Continue;
    }
    let Some(entity) = host.entity(player.entity_id as u32) else {
        return Flow::Continue;
    };
    let Some(living) = host.living(entity) else {
        return Flow::Continue;
    };

    // The used stack is a tome; the other hand holds the item it acts on.
    let main = living.hand_item(host, Hand::Main);
    let off = living.hand_item(host, Hand::Off);
    let (tome_slot, target_slot, target) = if main
        .as_ref()
        .is_some_and(|stack| stack.item == event.stack.item)
    {
        (EquipmentSlot::MainHand, EquipmentSlot::OffHand, off)
    } else if off
        .as_ref()
        .is_some_and(|stack| stack.item == event.stack.item)
    {
        (EquipmentSlot::OffHand, EquipmentSlot::MainHand, main)
    } else {
        return Flow::Continue;
    };

    let Some(target) = target else {
        host.tell(
            player.uuid,
            Text::translate_with_fallback(
                "apotheosis.tome.no_target",
                "Offhand must hold an enchanted item or a cursed item.",
                &[],
            ),
        );
        return Flow::Cancel;
    };
    if tome_kind(&target.item).is_some() {
        host.tell(
            player.uuid,
            Text::translate_with_fallback(
                "apotheosis.tome.no_target",
                "Offhand must hold an enchanted item or a cursed item.",
                &[],
            ),
        );
        return Flow::Cancel;
    }

    let (kept, removed) = if tome == Tome::Web {
        strip_curses(target.crafting_enchantments())
    } else {
        (target.crafting_enchantments().to_vec(), 0)
    };
    if tome == Tome::Web && removed == 0 {
        host.tell(
            player.uuid,
            Text::translate_with_fallback(
                "apotheosis.tome.no_curse",
                "That item carries no curse.",
                &[],
            ),
        );
        return Flow::Cancel;
    }
    if tome != Tome::Web && kept.is_empty() {
        host.tell(
            player.uuid,
            Text::translate_with_fallback(
                "apotheosis.tome.not_enchanted",
                "That item has no enchantments.",
                &[],
            ),
        );
        return Flow::Cancel;
    }

    let kept = if tome == Tome::Scrap {
        scrap_selection(&kept, dice(player.uuid))
    } else {
        kept
    };
    let cost = tome_cost(tome, kept.len());
    if player.game_mode != 1 && player.xp_level < cost {
        host.tell(
            player.uuid,
            Text::translate_with_fallback(
                "apotheosis.tome.levels",
                "You need %s experience levels.",
                &[Text::literal(cost.to_string())],
            ),
        );
        return Flow::Cancel;
    }

    let replacement = output_stack(tome, &target, kept.clone());
    let no_room = Text::translate_with_fallback(
        "apotheosis.tome.no_room",
        "No room for the extracted book; nothing was changed.",
        &[],
    );
    if tome == Tome::Extraction {
        // The item stays in the hand, stripped, while the book goes to the
        // inventory. Restore it if there is no room for the book.
        let mut stripped = target.clone();
        stripped.set_crafting_enchantments(Vec::new());
        if !living.set_equipment(host, target_slot, Some(stripped)) {
            return Flow::Continue;
        }
        if host.give_stack(player.uuid, replacement).is_some() {
            let _ = living.set_equipment(host, target_slot, Some(target));
            host.tell(player.uuid, no_room);
            return Flow::Cancel;
        }
    } else if tome == Tome::Web {
        if !living.set_equipment(host, target_slot, Some(replacement)) {
            return Flow::Continue;
        }
    } else {
        // Scrapping consumes the item; take it before handing out the book so
        // a full inventory leaves it in the hand instead of losing it.
        if !living.set_equipment(host, target_slot, None) {
            return Flow::Continue;
        }
        if host.give_stack(player.uuid, replacement).is_some() {
            let _ = living.set_equipment(host, target_slot, Some(target));
            host.tell(player.uuid, no_room);
            return Flow::Cancel;
        }
    }

    let left = event.stack.count.saturating_sub(1);
    let tome_left = if left == 0 {
        None
    } else {
        let mut stack = event.stack.clone();
        stack.count = left;
        Some(stack)
    };
    let _ = living.set_equipment(host, tome_slot, tome_left);

    if player.game_mode != 1 {
        let _ = host.give_levels(player.uuid, -cost);
    }
    let message = match tome {
        Tome::Scrap | Tome::Improved => (
            "apotheosis.tome.scrapped",
            "Scrapped %s enchantments.",
        ),
        Tome::Extraction => ("apotheosis.tome.extracted", "Extracted %s enchantments."),
        Tome::Web => ("apotheosis.tome.cleansed", "Curses removed."),
    };
    let count = Text::literal(kept.len().to_string());
    let args: &[Text] = if tome == Tome::Web {
        &[]
    } else {
        std::slice::from_ref(&count)
    };
    host.tell(
        player.uuid,
        Text::translate_with_fallback(message.0, message.1, args),
    );
    Flow::Cancel
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upstream_shelf_stats_keep_their_caps_and_modifiers() {
        assert_eq!(shelf_stats("heart_seashelf").unwrap().eterna, 15.0);
        assert_eq!(shelf_stats("heart_seashelf").unwrap().max_eterna, 60.0);
        assert_eq!(shelf_stats("heart_seashelf").unwrap().arcana, 20.0);
        assert_eq!(shelf_stats("beeshelf").unwrap().quanta, 100.0);
        assert_eq!(shelf_stats("stoneshelf").unwrap().eterna, -3.0);
    }

    #[test]
    fn capped_eterna_is_accumulated_in_ascending_cap_order() {
        let stats = aggregate([
            shelf_stats("draconic_endshelf").unwrap(),
            shelf_stats("hellshelf").unwrap(),
            shelf_stats("basic_bookshelf").unwrap(),
        ]);
        // Lowest caps first: 2 at cap 30, 5 at cap 45, then 25 at cap 100.
        assert_eq!(stats.eterna, 25.0);
    }

    #[test]
    fn cost_offers_scale_with_eterna_and_respect_slot_minimums() {
        let mut costs = [1, 2, 3];
        apply_costs(&mut costs, 45.0);
        assert_eq!(costs, [13, 31, 45]);
    }

    #[test]
    fn vanilla_offers_are_never_lowered() {
        let mut costs = [8, 16, 30];
        apply_costs(&mut costs, 0.0);
        assert_eq!(costs, [8, 16, 30]);
    }

    #[test]
    fn low_eterna_keeps_offers_eligible_at_vanilla_minimums() {
        let mut costs = [0, 0, 0];
        apply_costs(&mut costs, 0.0);
        assert_eq!(costs, [1, 2, 3]);
    }

    fn levels(ids: &[(&str, u32)]) -> Vec<(Arc<str>, u32)> {
        ids.iter().map(|(id, level)| (Arc::from(*id), *level)).collect()
    }

    #[test]
    fn every_tome_and_the_web_are_recognised() {
        assert_eq!(tome_kind(SCRAP_TOME), Some(Tome::Scrap));
        assert_eq!(tome_kind(IMPROVED_TOME), Some(Tome::Improved));
        assert_eq!(tome_kind(EXTRACTION_TOME), Some(Tome::Extraction));
        assert_eq!(tome_kind(PRISMATIC_WEB), Some(Tome::Web));
        assert_eq!(tome_kind("minecraft:book"), None);
    }

    #[test]
    fn scrapping_keeps_half_rounded_up_and_is_stable() {
        let all = levels(&[("minecraft:sharpness", 3), ("minecraft:unbreaking", 2),
                           ("minecraft:mending", 1), ("minecraft:looting", 2),
                           ("minecraft:fire_aspect", 1)]);
        let kept = scrap_selection(&all, 42);
        assert_eq!(kept.len(), 3);
        for (id, _) in &kept {
            assert!(all.iter().any(|(other, _)| other == id));
        }
        assert_eq!(scrap_selection(&all, 42), kept);
        assert_eq!(scrap_selection(&all[..2], 7).len(), 1);
        assert_eq!(scrap_selection(&all[..1], 7).len(), 1);
    }

    #[test]
    fn prismatic_web_removes_only_curses() {
        let all = levels(&[("minecraft:sharpness", 2),
                           ("minecraft:binding_curse", 1),
                           ("minecraft:vanishing_curse", 1)]);
        let (kept, removed) = strip_curses(&all);
        assert_eq!(removed, 2);
        assert_eq!(kept.len(), 1);
        assert_eq!(&*kept[0].0, "minecraft:sharpness");
        assert_eq!(strip_curses(&levels(&[("minecraft:sharpness", 1)])).1, 0);
    }

    #[test]
    fn tome_costs_scale_with_the_enchantments_involved() {
        assert_eq!(tome_cost(Tome::Scrap, 3), 18);
        assert_eq!(tome_cost(Tome::Improved, 3), 30);
        assert_eq!(tome_cost(Tome::Extraction, 3), 48);
        assert_eq!(tome_cost(Tome::Web, 2), 30);
    }

    #[test]
    fn tomes_move_enchantments_between_the_item_and_a_book() {
        let mut sword = ItemStack::new("minecraft:diamond_sword", 1);
        sword.set_crafting_enchantments(levels(&[("minecraft:sharpness", 2)]));
        let book = output_stack(Tome::Improved, &sword, sword.crafting_enchantments().to_vec());
        assert_eq!(&*book.item, "minecraft:enchanted_book");
        assert_eq!(book.crafting_enchantments().len(), 1);

        let extracted = output_stack(Tome::Extraction, &sword, sword.crafting_enchantments().to_vec());
        assert_eq!(&*extracted.item, "minecraft:enchanted_book");

        let cleaned = output_stack(Tome::Web, &sword, levels(&[]));
        assert_eq!(&*cleaned.item, "minecraft:diamond_sword");
        assert!(cleaned.crafting_enchantments().is_empty());
    }
}
