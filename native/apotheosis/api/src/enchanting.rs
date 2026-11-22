use horizon_client::mod_api::{Host, PrepareItemEnchantEvent};

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
        *cost = match slot {
            2 => eterna.max(min_cost),
            0 | 1 => {
                let (low, high) = factors[slot];
                let floor = (eterna as f32 * low).round() as i32;
                let ceil = (eterna as f32 * high).round() as i32;
                ((floor + ceil) / 2).max(min_cost)
            }
            _ => *cost,
        };
    }
}

pub(super) fn on_prepare_item_enchant(host: &dyn Host, event: &mut PrepareItemEnchantEvent<'_>) {
    let stats = read_table_stats(host, event.pos);
    apply_costs(event.costs, stats.eterna);
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
    fn low_eterna_keeps_offers_eligible_at_vanilla_minimums() {
        let mut costs = [0, 0, 0];
        apply_costs(&mut costs, 0.0);
        assert_eq!(costs, [1, 2, 3]);
    }
}
