//! The working tables' own screens, as Java's `ReforgingMenu`/`ReforgingScreen`.
//!
//! The shared frame, all on API <= 415:
//! - the menu type is content (`data/apotheosis/horizon/menu/<name>.json`) and
//!   the table block's template opens a menu (`minecraft:crafting_table`);
//! - `block_entity` gives the table a block entity whose `items` are the
//!   menu's slots, `block_entity_container` names the menu type;
//! - `block_entity_tick` writes what the screen shows into the data slots
//!   (`data`, 16-bit on the wire), recomputed from the slots every tick;
//! - `screen`/`screen_draw`/`screen_click`/`screen_tooltip` lay out and draw
//!   the screen on the client from those data slots, with the Java texture
//!   when the local Java layer has it (`apotheosis:gui/<name>`) and drawn
//!   panels otherwise (the `--no-java` pack);
//! - a click on a drawn button is a menu button (`menu_button`), applied on
//!   the server to the block entity's slots;
//! - `menu_slot_rules` says what each slot takes.
//!
//! A salvaging or gem cutting screen adds its table to `menu_of`, its layout
//! to `screen`, its data to `block_entity_tick` and its buttons to
//! `menu_button`.
//!
//! Deviation: Java gives the gear back to the player when the menu closes
//! (`removed()`); taking a slot out of a block entity from a close hook needs a
//! host helper past API 415, so the gear stays in the table, as in a furnace.
//! The choices show their rarity and cost, not the rolled piece: the client
//! has only the data slots, and the roll is made again on the server.

use horizon_client::mod_api::{
    BlockEntityContainerEvent, BlockEntityEvent, BlockEntityTickEvent, BlockEntityTicksEvent,
    Compound, Host, ItemStack, MenuButtonEvent, MenuSlotRulesEvent, ModContainer, NativeScreen,
    ScreenClickEvent, ScreenDrawEvent, ScreenEvent, ScreenTooltipEvent,
};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::sockets::{self, Table, REFORGE_COSTS};
use crate::{is_equipment, Tier};

/// The reforging menu type, shared by both reforging tables.
pub const REFORGING: &str = "apotheosis:reforging";
/// Its slots: the gear, the rarity material, the sigils of rebirth.
const GEAR: usize = 0;
const MATERIAL: usize = 1;
const SIGILS: usize = 2;
const SIGIL: &str = "apotheosis:sigil_of_rebirth";
/// Its data slots: sigil, material and level cost of the first choice, then
/// the flags (`VALID`, a choice's materials present, the rarity row).
const DATA_SLOTS: usize = 4;
const VALID: i32 = 1;

/// The reforging table a block is, if it is one.
fn reforge_table(block: &str) -> Option<Table> {
    sockets::table(block).filter(|t| matches!(t, Table::SimpleReforging | Table::Reforging))
}

/// The menu type a block opens.
fn menu_of(block: &str) -> Option<&'static str> {
    reforge_table(block).map(|_| REFORGING)
}

/// The rarity a material names (Java's `RarityRegistry.getMaterialRarity`).
pub fn material_tier(item: &str) -> Option<Tier> {
    REFORGE_COSTS
        .iter()
        .map(|(tier, ..)| *tier)
        .find(|tier| sockets::salvage_material(*tier).0 == item)
}

fn count_of(slot: Option<&Option<ItemStack>>, item: Option<&str>) -> u32 {
    match (slot, item) {
        (Some(Some(stack)), Some(item)) if &*stack.item == item => stack.count,
        (Some(Some(stack)), None) => stack.count,
        _ => 0,
    }
}

/// What the screen shows, from the table's slots: `[sigils, material, levels,
/// flags]`, the costs of the first choice (choice `i` costs `i + 1` times as
/// much, Java's `getSigilCost(slot)`...). Flags: `VALID` when a piece and a
/// material with a recipe at this table are in; bit `1 + i` when the slots hold
/// what choice `i` takes; bits 4.. the material's row in `REFORGE_COSTS`, + 1.
pub fn reforge_data(table: Table, items: &[Option<ItemStack>]) -> [i32; DATA_SLOTS] {
    let material = items.get(MATERIAL).and_then(|s| s.as_ref());
    let Some(tier) = material.and_then(|m| material_tier(&m.item)) else {
        return [0; DATA_SLOTS];
    };
    let Some((levels, mats, sigils)) = sockets::reforge_cost(tier, table) else {
        return [0; DATA_SLOTS];
    };
    let row = REFORGE_COSTS.iter().position(|(t, ..)| *t == tier).unwrap_or(0) as i32;
    let mut flags = (row + 1) << 4;
    let gear = items.get(GEAR).and_then(|s| s.as_ref());
    if gear.is_some_and(|g| is_equipment(&g.item)) {
        flags |= VALID;
        let have_mats = count_of(items.get(MATERIAL), None);
        let have_sigils = count_of(items.get(SIGILS), Some(SIGIL));
        for i in 0..3u32 {
            if have_mats >= mats * (i + 1) && have_sigils >= sigils * (i + 1) {
                flags |= 1 << (1 + i);
            }
        }
    }
    [sigils as i32, mats as i32, levels, flags]
}

/// Each player's reforge seed, renewed after every reforge (Java's
/// `REFORGE_SEED` in the player's persistent data).
fn seeds() -> &'static Mutex<HashMap<u128, u64>> {
    static SEEDS: OnceLock<Mutex<HashMap<u128, u64>>> = OnceLock::new();
    SEEDS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn player_seed(player: u128, item: &str) -> u64 {
    let mut seeds = seeds().lock().unwrap_or_else(|e| e.into_inner());
    *seeds.entry(player).or_insert_with(|| sockets::reforge_seed(player, item))
}

fn renew_seed(player: u128, item: &str) {
    let seed = sockets::reforge_seed(player, item);
    seeds().lock().unwrap_or_else(|e| e.into_inner()).insert(player, seed);
}

/// Choice `choice` (0..3) at `table` for a player with `xp_level` levels:
/// what it costs, checked against the slots. The levels charged are
/// `choice + 1`, the experience Java's `getExpCostForSlot` takes from the top
/// of the required level; the requirement is the recipe's levels times
/// `choice + 1`. None when the slots or the levels fall short.
pub fn plan_choice(
    table: Table,
    items: &[Option<ItemStack>],
    choice: usize,
    xp_level: i32,
    creative: bool,
) -> Option<(Tier, u32, u32, i32)> {
    if choice > 2 {
        return None;
    }
    let data = reforge_data(table, items);
    if data[3] & VALID == 0 {
        return None;
    }
    let tier = material_tier(&items[MATERIAL].as_ref()?.item)?;
    let n = choice as u32 + 1;
    if creative {
        return Some((tier, 0, 0, 0));
    }
    if data[3] & (1 << (1 + choice)) == 0 || xp_level < data[2] * n as i32 {
        return None;
    }
    Some((tier, data[1] as u32 * n, data[0] as u32 * n, n as i32))
}

/// Puts the reforged piece in the gear slot and takes the materials and
/// sigils `plan_choice` counted.
pub fn apply_choice(items: &mut [Option<ItemStack>], tier: Tier, mats: u32, sigils: u32, seed: u64) {
    let Some(gear) = items[GEAR].clone() else {
        return;
    };
    items[GEAR] = Some(sockets::reforged(&gear, tier, seed));
    for (slot, n) in [(MATERIAL, mats), (SIGILS, sigils)] {
        if let Some(stack) = &mut items[slot] {
            stack.count = stack.count.saturating_sub(n);
            if stack.count == 0 {
                items[slot] = None;
            }
        }
    }
}

/// The seed of choice `choice` for `item`: Java's `seed ^ itemKey.hashCode() + slot`.
pub fn choice_seed(seed: u64, item: &str, choice: usize) -> u64 {
    seed ^ sockets::item_id(item).wrapping_add(choice as u64)
}

// --- Hooks -----------------------------------------------------------------

pub fn on_block_entity(event: &BlockEntityEvent<'_>) -> Option<(String, Compound)> {
    menu_of(event.block).map(|_| (event.block.to_owned(), Compound::new()))
}

pub fn on_block_entity_ticks(event: &mut BlockEntityTicksEvent<'_>) {
    if menu_of(event.block).is_some() {
        event.ticks = true;
    }
}

pub fn on_block_entity_container(event: &BlockEntityContainerEvent<'_>) -> Option<ModContainer> {
    let menu = menu_of(event.block)?;
    Some(ModContainer {
        slots: 3,
        faces: Default::default(),
        inputs: Vec::new(),
        outputs: Vec::new(),
        menu: Some(menu.to_owned()),
    })
}

/// True when the entity was one of ours.
pub fn on_block_entity_tick(event: &mut BlockEntityTickEvent<'_>) -> bool {
    let Some(table) = reforge_table(&event.entity.id) else {
        return false;
    };
    let data = reforge_data(table, &event.entity.items);
    if event.entity.data.len() != DATA_SLOTS || event.entity.data[..] != data[..] {
        event.entity.data = data.to_vec();
    }
    true
}

pub fn on_menu_button(host: &mut dyn Host, event: &mut MenuButtonEvent<'_>) {
    let Some(table) = reforge_table(&event.entity.id) else {
        return;
    };
    let Some(player) = host.acting_player() else {
        return;
    };
    let choice = event.id.max(0) as usize;
    if event.entity.items.len() < 3 {
        return;
    }
    let creative = player.game_mode == 1;
    let Some((tier, mats, sigils, levels)) =
        plan_choice(table, &event.entity.items, choice, player.xp_level, creative)
    else {
        return;
    };
    if levels > 0 && !host.give_levels(player.uuid, -levels) {
        return;
    }
    let item = event.entity.items[GEAR].as_ref().map(|g| g.item.to_string()).unwrap_or_default();
    let seed = choice_seed(player_seed(player.uuid, &item), &item, choice);
    apply_choice(&mut event.entity.items, tier, mats, sigils, seed);
    renew_seed(player.uuid, &item);
    event.entity.data = reforge_data(table, &event.entity.items).to_vec();
}

/// Java's slot rules: one piece of gear, a rarity material, sigils of rebirth.
pub fn on_menu_slot_rules(event: &mut MenuSlotRulesEvent<'_>) {
    if event.menu != REFORGING {
        return;
    }
    if let Some((may_place, max_stack)) = slot_rule(event.slot, &event.stack.item) {
        event.may_place = may_place;
        event.max_stack = max_stack.unwrap_or(event.max_stack);
    }
}

/// Whether reforging slot `slot` takes `item`, and its cap when it has one.
pub fn slot_rule(slot: usize, item: &str) -> Option<(bool, Option<u32>)> {
    match slot {
        GEAR => Some((is_equipment(item), Some(1))),
        MATERIAL => Some((material_tier(item).is_some(), None)),
        SIGILS => Some((item == SIGIL, None)),
        _ => None,
    }
}

// --- Client screen -----------------------------------------------------------

const TEXTURE: &str = "apotheosis:gui/reforge";
const WIDTH: f32 = 176.0;
const HEIGHT: f32 = 266.0;
/// Java's slots: gear (81, 62), material (39, 40), sigils (123, 86); the
/// choices' circles at (27 + 54i, 135).
const SLOTS: [[f32; 2]; 3] = [[81.0, 62.0], [39.0, 40.0], [123.0, 86.0]];

fn choice_x(i: usize) -> f32 {
    27.0 + 54.0 * i as f32
}

/// The choice under `at` (GUI pixels from the screen's top left).
pub fn choice_at(at: [f32; 2]) -> Option<usize> {
    (0..3).find(|&i| {
        let x = choice_x(i) - 4.0;
        at[0] >= x && at[0] < x + 24.0 && at[1] >= 131.0 && at[1] < 155.0
    })
}

fn get(data: &[i32], i: usize) -> i32 {
    data.get(i).copied().unwrap_or(0)
}

pub fn on_screen(event: &ScreenEvent<'_>) -> Option<NativeScreen> {
    menu_of(event.block)?;
    Some(NativeScreen {
        width: WIDTH,
        height: HEIGHT,
        slots: SLOTS.iter().map(|s| Some(*s)).collect(),
        inventory_y: 184.0,
        inventory_x: 8.0,
        own_labels: false,
        own_background: true,
    })
}

pub fn on_screen_draw(host: &dyn Host, event: &mut ScreenDrawEvent<'_>) {
    if menu_of(event.block).is_none() {
        return;
    }
    let [ox, oy] = event.origin;
    let java = host.sprite(event.batch, TEXTURE, [0.0, 0.0, WIDTH, HEIGHT], [ox, oy, WIDTH, HEIGHT]);
    if !java {
        // The port's own panel for the `--no-java` pack.
        host.fill(event.batch, [ox, oy, WIDTH, HEIGHT], 0xFF2E_2A33);
        host.fill(event.batch, [ox + 4.0, oy + 4.0, WIDTH - 8.0, 168.0], 0xFF3B_3542);
        let frame = |host: &dyn Host, batch: &mut _, x: f32, y: f32| {
            host.fill(batch, [ox + x - 1.0, oy + y - 1.0, 18.0, 18.0], 0xFF1A_181D);
        };
        for [x, y] in SLOTS {
            frame(host, event.batch, x, y);
        }
        for row in 0..4 {
            let y = if row == 3 { 242.0 } else { 184.0 + 18.0 * row as f32 };
            for col in 0..9 {
                frame(host, event.batch, 8.0 + 18.0 * col as f32, y);
            }
        }
    }
    let flags = get(event.data, 3);
    let levels = get(event.data, 2);
    let colour = match (flags >> 4) - 1 {
        row if row >= 0 => REFORGE_COSTS.get(row as usize).map_or(0xA0A0A0, |r| r.0.colour()),
        _ => 0xA0A0A0,
    };
    for i in 0..3 {
        let x = choice_x(i);
        let ready = flags & VALID != 0 && flags & (1 << (1 + i)) != 0;
        if ready {
            let src = [x - 9.0, 273.0, 34.0, 35.0];
            if !(java && host.sprite(event.batch, TEXTURE, src, [ox + x - 9.0, oy + 129.0, 34.0, 35.0])) {
                host.fill(event.batch, [ox + x - 2.0, oy + 133.0, 20.0, 20.0], 0x80A0_50E0);
            }
        } else if !java {
            host.fill(event.batch, [ox + x - 2.0, oy + 133.0, 20.0, 20.0], 0xFF1A_181D);
        }
        if flags & VALID != 0 {
            let cost = (levels * (i as i32 + 1)).to_string();
            let w = host.text_width(&cost);
            let rgb = if ready { 0x80FF20 } else { 0x808080 };
            host.text(event.batch, &cost, [ox + x + 8.0 - w / 2.0, oy + 156.0], rgb, true);
            host.fill(event.batch, [ox + x + 2.0, oy + 139.0, 12.0, 4.0], 0xFF00_0000 | colour);
        }
    }
}

pub fn on_screen_click(event: &ScreenClickEvent<'_>) -> Option<i32> {
    menu_of(event.block)?;
    if event.right || get(event.data, 3) & VALID == 0 {
        return None;
    }
    choice_at(event.at).map(|i| i as i32)
}

pub fn on_screen_tooltip(event: &mut ScreenTooltipEvent<'_>) {
    if menu_of(event.block).is_none() || get(event.data, 3) & VALID == 0 {
        return;
    }
    let Some(i) = choice_at(event.at) else {
        return;
    };
    let n = i as i32 + 1;
    let flags = get(event.data, 3);
    if let Some((tier, ..)) = REFORGE_COSTS.get(((flags >> 4) - 1).max(0) as usize) {
        event.lines.push(format!("§f{}", tier.name()));
    }
    event.lines.push("§7Reforge cost / Coût de reforge :".to_owned());
    let sigils = get(event.data, 0) * n;
    if sigils > 0 {
        event.lines.push(format!("§7{sigils} x Sigil of Rebirth"));
    }
    event.lines.push(format!("§7{} x {}", get(event.data, 1) * n, "rarity material"));
    event.lines.push(format!("§7{n} level(s) / niveau(x)"));
    event.lines.push(format!("§8Requires level {} / Niveau requis {}", get(event.data, 2) * n, get(event.data, 2) * n));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(item: &str, count: u32) -> Option<ItemStack> {
        Some(ItemStack::new(item, count))
    }

    #[test]
    fn a_material_names_its_rarity() {
        assert_eq!(material_tier("apotheosis:luminous_crystal_shard"), Some(Tier::Rare));
        assert_eq!(material_tier("apotheosis:godforged_pearl"), Some(Tier::Mythic));
        assert_eq!(material_tier("minecraft:stone"), None);
    }

    #[test]
    fn the_data_slots_carry_the_java_recipe_costs() {
        // Rare: 15 levels, 2 shards, 2 sigils (recipe/reforging/rare.json).
        let items = [
            stack("minecraft:diamond_sword", 1),
            stack("apotheosis:luminous_crystal_shard", 4),
            stack(SIGIL, 5),
        ];
        let data = reforge_data(Table::Reforging, &items);
        assert_eq!(&data[..3], &[2, 2, 15]);
        assert!(data[3] & VALID != 0);
        // Choices 1 and 2 have what they need (2 and 4 shards); 3 needs 6.
        assert_eq!(data[3] & 0b1110, 0b0110);
    }

    #[test]
    fn the_simple_table_refuses_epic_materials() {
        let items = [stack("minecraft:diamond_sword", 1), stack("apotheosis:arcane_sands", 9), None];
        assert_eq!(reforge_data(Table::SimpleReforging, &items), [0; 4]);
        assert!(reforge_data(Table::Reforging, &items)[3] & VALID != 0);
    }

    #[test]
    fn a_choice_costs_its_rank_times_the_recipe() {
        let items = [
            stack("minecraft:diamond_sword", 1),
            stack("apotheosis:luminous_crystal_shard", 6),
            stack(SIGIL, 6),
        ];
        assert_eq!(plan_choice(Table::Reforging, &items, 2, 45, false), Some((Tier::Rare, 6, 6, 3)));
        assert_eq!(plan_choice(Table::Reforging, &items, 2, 44, false), None, "requires 45 levels");
        assert_eq!(plan_choice(Table::Reforging, &items, 0, 15, false), Some((Tier::Rare, 2, 2, 1)));
        assert_eq!(plan_choice(Table::Reforging, &items, 3, 99, false), None);
        assert_eq!(plan_choice(Table::Reforging, &items, 2, 0, true), Some((Tier::Rare, 0, 0, 0)));
    }

    #[test]
    fn no_gear_no_choice() {
        let items = [None, stack("apotheosis:luminous_crystal_shard", 6), stack(SIGIL, 6)];
        assert_eq!(plan_choice(Table::Reforging, &items, 0, 99, false), None);
        let not_gear = [stack("minecraft:stone", 1), stack("apotheosis:luminous_crystal_shard", 6), None];
        assert_eq!(plan_choice(Table::Reforging, &not_gear, 0, 99, true), None);
    }

    #[test]
    fn applying_a_choice_reforges_in_place_and_takes_the_costs() {
        let mut items = vec![
            stack("minecraft:diamond_sword", 1),
            stack("apotheosis:luminous_crystal_shard", 2),
            stack(SIGIL, 3),
        ];
        apply_choice(&mut items, Tier::Rare, 2, 2, 7);
        let gear = items[GEAR].as_ref().expect("the piece stays in the table");
        assert_eq!(&*gear.item, "minecraft:diamond_sword");
        assert_eq!(sockets::tier_of(gear), Some(Tier::Rare), "the material's rarity");
        assert!(sockets::can_reforge(gear));
        assert!(items[MATERIAL].is_none(), "the last shards are used up");
        assert_eq!(items[SIGILS].as_ref().map(|s| s.count), Some(1));
    }

    #[test]
    fn the_same_seed_gives_the_same_choice_and_choices_differ() {
        let gear = ItemStack::new("minecraft:diamond_sword", 1);
        let a = sockets::reforged(&gear, Tier::Epic, choice_seed(42, "minecraft:diamond_sword", 0));
        let b = sockets::reforged(&gear, Tier::Epic, choice_seed(42, "minecraft:diamond_sword", 0));
        assert_eq!(a, b);
        assert_ne!(choice_seed(42, "x", 0), choice_seed(42, "x", 1));
    }

    #[test]
    fn slot_rules_follow_java() {
        let check = |slot: usize, item: &str| {
            let (may_place, max) = slot_rule(slot, item).unwrap();
            (may_place, max.unwrap_or(99))
        };
        assert_eq!(check(GEAR, "minecraft:diamond_sword"), (true, 1));
        assert!(!check(GEAR, "minecraft:stone").0);
        assert!(check(MATERIAL, "apotheosis:timeworn_fabric").0);
        assert!(!check(MATERIAL, SIGIL).0);
        assert!(check(SIGILS, SIGIL).0);
        assert!(!check(SIGILS, "apotheosis:timeworn_fabric").0);
    }

    #[test]
    fn a_click_lands_on_the_choice_under_it() {
        assert_eq!(choice_at([30.0, 140.0]), Some(0));
        assert_eq!(choice_at([84.0, 140.0]), Some(1));
        assert_eq!(choice_at([138.0, 140.0]), Some(2));
        assert_eq!(choice_at([60.0, 140.0]), None);
        assert_eq!(choice_at([30.0, 100.0]), None);
    }
}
