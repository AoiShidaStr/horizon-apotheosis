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
//! The salvaging table (Java's `SalvagingMenu`) follows the same frame: its
//! results go into output slots the player can take from but not place into,
//! and only after the salvage button destroyed the inputs.
//!
//! The gem cutting table (Java's `GemCuttingMenu`) follows it too: a gem, a
//! second one of the same type and purity, gem dust and a rarity material; its
//! button turns the first gem into the next purity's item.
//!
//! Deviation: Java gives the gear back to the player when the menu closes
//! (`removed()`); taking a slot out of a block entity from a close hook needs a
//! host helper past API 415, so the gear stays in the table, as in a furnace.
//! The choices show their rarity and cost, not the rolled piece: the client
//! has only the data slots, and the roll is made again on the server.

use horizon_client::mod_api::{
    BlockEntityContainerEvent, BlockEntityEvent, BlockEntityTickEvent, BlockEntityTicksEvent,
    Compound, Host, HudBatch, ItemStack, MenuButtonEvent, MenuSlotRulesEvent, ModContainer, NativeScreen,
    ScreenClickEvent, ScreenDrawEvent, ScreenEvent, ScreenTooltipEvent, Text,
};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::gems;
use crate::sockets::{self, Purity, Table, REFORGE_COSTS, UPGRADE_COSTS};
use crate::{is_equipment, Rng, Tier};

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
    match sockets::table(block)? {
        Table::SimpleReforging | Table::Reforging => Some(REFORGING),
        Table::Salvaging => Some(SALVAGING),
        Table::Cutting => Some(CUTTING),
    }
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
        slots: match menu {
            SALVAGING => SALVAGE_SLOTS,
            CUTTING => CUT_SLOTS,
            _ => 3,
        },
        faces: Default::default(),
        inputs: Vec::new(),
        outputs: Vec::new(),
        menu: Some(menu.to_owned()),
    })
}

/// True when the entity was one of ours.
pub fn on_block_entity_tick(event: &mut BlockEntityTickEvent<'_>) -> bool {
    if menu_of(&event.entity.id) == Some(CUTTING) {
        let data = cut_data(&event.entity.items);
        if event.entity.data[..] != data[..] {
            event.entity.data = data.to_vec();
        }
        return true;
    }
    if menu_of(&event.entity.id) == Some(SALVAGING) {
        let data = salvage_data(&event.entity.items);
        if event.entity.data[..] != data[..] {
            event.entity.data = data.to_vec();
        }
        return true;
    }
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
    if menu_of(&event.entity.id) == Some(SALVAGING) {
        return salvage_button(host, event);
    }
    if menu_of(&event.entity.id) == Some(CUTTING) {
        if event.id == 0 && host.acting_player().is_some() && cut(&mut event.entity.items) {
            event.entity.data = cut_data(&event.entity.items).to_vec();
        }
        return;
    }
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
    let rule = match event.menu {
        REFORGING => slot_rule(event.slot, &event.stack.item),
        SALVAGING => salvage_slot_rule(event.slot, event.stack),
        CUTTING => cut_slot_rule(event.slot, event.stack),
        _ => return,
    };
    if let Some((may_place, max_stack)) = rule {
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
    match menu_of(event.block)? {
        SALVAGING => return Some(salvage_screen()),
        CUTTING => return Some(cut_screen()),
        _ => {}
    }
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
    match menu_of(event.block) {
        Some(REFORGING) => {}
        Some(SALVAGING) => return salvage_draw(host, event),
        Some(CUTTING) => return cut_draw(host, event),
        _ => return,
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
    if menu_of(event.block)? == SALVAGING {
        let ready = !event.right && get(event.data, 0) & VALID != 0;
        return (ready && on_salvage_button(event.at)).then_some(0);
    }
    if menu_of(event.block)? == CUTTING {
        let ready = !event.right && get(event.data, 0) & VALID != 0;
        return (ready && on_cut_button(event.at)).then_some(0);
    }
    if event.right || get(event.data, 3) & VALID == 0 {
        return None;
    }
    choice_at(event.at).map(|i| i as i32)
}

pub fn on_screen_tooltip(event: &mut ScreenTooltipEvent<'_>) {
    if menu_of(event.block) == Some(SALVAGING) {
        return salvage_tooltip(event);
    }
    if menu_of(event.block) == Some(CUTTING) {
        return cut_tooltip(event);
    }
    if menu_of(event.block).is_none() || get(event.data, 3) & VALID == 0 {
        return;
    }
    let Some(i) = choice_at(event.at) else {
        return;
    };
    push_lines(event.lines, &reforge_lines(event.data, i));
}

// --- Tooltip lines -------------------------------------------------------------

/// An argument of a tooltip line: a number, or an item named in the player's
/// language.
#[derive(Clone, Debug, PartialEq)]
enum Arg {
    Num(i32),
    Item(&'static str),
    Key(&'static str, &'static str),
}

/// One tooltip line: a legacy colour prefix (and any text before the
/// translation), then a translation key with its English fallback and its
/// arguments. Resolved on the client, in its language (`Text::translate`).
#[derive(Clone, Debug, PartialEq)]
struct Line {
    prefix: String,
    key: String,
    fallback: String,
    args: Vec<Arg>,
}

fn line(key: &str, fallback: &str, prefix: impl Into<String>, args: Vec<Arg>) -> Line {
    Line { prefix: prefix.into(), key: key.to_owned(), fallback: fallback.to_owned(), args }
}

/// An item's English name from its id, when the language lacks its key.
fn english(item: &str) -> String {
    item.rsplit(':').next().unwrap_or(item).replace('_', " ")
}

/// The key `Item.getDescriptionId` gives an item id (`item.apotheosis.gem_dust`).
fn item_key(item: &str) -> String {
    let (namespace, path) = item.split_once(':').unwrap_or(("minecraft", item));
    format!("item.{namespace}.{path}")
}

fn arg_text(arg: &Arg) -> Text {
    match arg {
        Arg::Num(n) => Text::literal(n.to_string()),
        Arg::Item(item) => Text::translate_with_fallback(&item_key(item), &english(item), &[]),
        Arg::Key(key, fallback) => Text::translate_with_fallback(key, fallback, &[]),
    }
}

fn push_lines(out: &mut Vec<String>, lines: &[Line]) {
    for l in lines {
        let args: Vec<Text> = l.args.iter().map(arg_text).collect();
        let text = Text::translate_with_fallback(&l.key, &l.fallback, &args);
        out.push(format!("{}{}", l.prefix, text.plain()));
    }
}

/// Reforge choice `i`'s tooltip (Java's `ReforgingScreen`): the rarity, then
/// what it costs: sigils, the rarity's material, levels, the level required.
fn reforge_lines(data: &[i32], i: usize) -> Vec<Line> {
    let n = i as i32 + 1;
    let mut lines = Vec::new();
    let tier = REFORGE_COSTS.get(((get(data, 3) >> 4) - 1).max(0) as usize).map(|(t, ..)| *t);
    if let Some(tier) = tier {
        lines.push(line(tier.name_key(), tier.name(), "§f", vec![]));
    }
    lines.push(line("text.apotheosis.reforge_cost", "Reforge Cost", "§7", vec![]));
    let sigils = get(data, 0) * n;
    if sigils > 0 {
        lines.push(line("text.apotheosis.cost", "%sx %s", "§7", vec![Arg::Num(sigils), Arg::Item(SIGIL)]));
    }
    if let Some(tier) = tier {
        let material = Arg::Item(sockets::salvage_material(tier).0);
        lines.push(line("text.apotheosis.cost", "%sx %s", "§7", vec![Arg::Num(get(data, 1) * n), material]));
    }
    let (key, fallback) = if n == 1 { ("misc.apotheosis.level", "level") } else { ("misc.apotheosis.level.many", "levels") };
    lines.push(line(key, fallback, format!("§7{n} "), vec![]));
    lines.push(line(
        "container.enchant.level.requirement",
        "Level Requirement: %s",
        "§8",
        vec![Arg::Num(get(data, 2) * n)],
    ));
    lines
}

/// The salvage button's tooltip: what the inputs can give, or why it is off.
fn salvage_lines(data: &[i32]) -> Vec<Line> {
    if get(data, 0) & VALID == 0 {
        return vec![line("button.apotheosis.no_salvage", "No items selected", "§c", vec![])];
    }
    let mut lines = vec![line("text.apotheosis.salvage_results", "Salvage Results", "§e§n", vec![])];
    for (i, item) in SALVAGE_OUTPUTS.iter().enumerate() {
        let (low, high) = (get(data, 1 + 2 * i), get(data, 2 + 2 * i));
        if high > 0 {
            lines.push(line(&item_key(item), &english(item), format!("§7{low}-{high} "), vec![]));
        }
    }
    lines
}

/// The upgrade button's tooltip: the purity reached and the dust it takes.
fn cut_lines(data: &[i32]) -> Vec<Line> {
    if get(data, 0) & VALID == 0 {
        return vec![line("button.apotheosis.upgrade.no", "Missing Inputs", "§c", vec![])];
    }
    let purity = Purity::from_index(get(data, 2) as u8);
    vec![
        line("apotheosis.gem_cutting.upgrade_to", "Upgrade to %s", "§e§n", vec![Arg::Key(purity.name_key(), purity.name())]),
        line("text.apotheosis.cost", "%sx %s", "§7", vec![Arg::Num(get(data, 1)), Arg::Item(GEM_DUST)]),
    ]
}

// --- Salvaging table -----------------------------------------------------------

/// The salvaging menu type: Java's `SalvagingMenu`.
pub const SALVAGING: &str = "apotheosis:salvaging";
/// Twelve inputs (one piece each), then six outputs.
const SALVAGE_INPUTS: usize = 12;
const SALVAGE_SLOTS: usize = 18;
/// What a salvage can give, in the data slots' order: the five rarity
/// materials (Java's `recipe/salvaging/affix_item`), then gem dust
/// (`recipe/salvaging/gem`).
const SALVAGE_OUTPUTS: [&str; 6] = [
    "apotheosis:mysterious_scrap_metal",
    "apotheosis:timeworn_fabric",
    "apotheosis:luminous_crystal_shard",
    "apotheosis:arcane_sands",
    "apotheosis:godforged_pearl",
    "apotheosis:gem_dust",
];
/// Its data slots: flags (`VALID` when an input salvages), then the least and
/// the most of each output, summed over the inputs (Java's results tooltip).
const SALVAGE_DATA: usize = 1 + 2 * SALVAGE_OUTPUTS.len();

/// What salvaging `stack` can give: `(output, least, most)`, by
/// `SALVAGE_OUTPUTS` index. Empty when nothing salvages it.
pub fn salvage_ranges(stack: &ItemStack) -> Vec<(usize, u32, u32)> {
    if let Some((_, purity)) = gems::of_stack(stack) {
        return sockets::GEM_SALVAGE
            .iter()
            .find(|(p, ..)| *p == purity)
            .map(|(_, low, high)| vec![(5, *low, *high)])
            .unwrap_or_default();
    }
    if !sockets::has_affixes(stack) {
        return Vec::new();
    }
    let (material, max) = sockets::salvage_material(sockets::tier_of(stack).unwrap_or(Tier::Common));
    SALVAGE_OUTPUTS
        .iter()
        .position(|o| *o == material)
        .map(|i| vec![(i, 1, max as u32)])
        .unwrap_or_default()
}

/// The data slots, from the inputs.
pub fn salvage_data(items: &[Option<ItemStack>]) -> [i32; SALVAGE_DATA] {
    let mut data = [0; SALVAGE_DATA];
    for stack in items.iter().take(SALVAGE_INPUTS).flatten() {
        for (i, low, high) in salvage_ranges(stack) {
            data[0] |= VALID;
            data[1 + 2 * i] += low as i32;
            data[2 + 2 * i] += high as i32;
        }
    }
    data
}

/// What one input breaks into: the roll of `sockets::salvage_yield` or
/// `gem_salvage_yield`, and, as `/apoth salvage` does, the gems socketed in it.
pub fn salvage_results(stack: &ItemStack, rng: &mut Rng) -> Vec<ItemStack> {
    if let Some((_, purity)) = gems::of_stack(stack) {
        let dust = sockets::gem_salvage_yield(purity, rng) * stack.count.max(1);
        return vec![ItemStack::new(SALVAGE_OUTPUTS[5], dust)];
    }
    if !sockets::has_affixes(stack) {
        return Vec::new();
    }
    let (material, count) = sockets::salvage_yield(sockets::tier_of(stack).unwrap_or(Tier::Common), rng);
    let mut out = vec![ItemStack::new(material.as_str(), count)];
    for socket in sockets::socketed(stack) {
        if let Some(gem) = gems::get(&socket.gem) {
            out.push(gems::stack(gem, socket.purity));
        }
    }
    out
}

fn same_kind(a: &ItemStack, b: &ItemStack) -> bool {
    a.item == b.item && a.damage == b.damage && a.extra == b.extra
}

/// Puts `stack` into `slots`, topping up its own kind first, then the empty
/// ones. False (with `slots` partly filled) when it does not all fit.
fn insert(slots: &mut [Option<ItemStack>], mut stack: ItemStack, max_stack: &dyn Fn(&str) -> u32) -> bool {
    let max = max_stack(&stack.item).max(1);
    for slot in slots.iter_mut().flatten() {
        if stack.count > 0 && same_kind(slot, &stack) && slot.count < max {
            let moved = (max - slot.count).min(stack.count);
            slot.count += moved;
            stack.count -= moved;
        }
    }
    for slot in slots.iter_mut().filter(|s| s.is_none()) {
        if stack.count == 0 {
            break;
        }
        let moved = max.min(stack.count);
        let mut part = stack.clone();
        part.count = moved;
        *slot = Some(part);
        stack.count -= moved;
    }
    stack.count == 0
}

/// The salvage button (Java's `salvageAll`): each input that salvages is
/// destroyed and its results go into the output slots. Deviation: Java hands
/// what does not fit to the player; here an input whose results do not fit
/// stays whole, so nothing is ever lost. Returns how many inputs salvaged.
pub fn salvage_all(items: &mut [Option<ItemStack>], rng: &mut Rng, max_stack: &dyn Fn(&str) -> u32) -> usize {
    if items.len() < SALVAGE_SLOTS {
        return 0;
    }
    let mut done = 0;
    for i in 0..SALVAGE_INPUTS {
        let Some(stack) = items[i].clone() else {
            continue;
        };
        let results = salvage_results(&stack, rng);
        if results.is_empty() {
            continue;
        }
        let mut outputs = items[SALVAGE_INPUTS..SALVAGE_SLOTS].to_vec();
        if results.into_iter().all(|r| insert(&mut outputs, r, max_stack)) {
            items[SALVAGE_INPUTS..SALVAGE_SLOTS].clone_from_slice(&outputs);
            items[i] = None;
            done += 1;
        }
    }
    done
}

fn salvage_button(host: &mut dyn Host, event: &mut MenuButtonEvent<'_>) {
    if event.id != 0 || host.acting_player().is_none() {
        return;
    }
    let mut rng = Rng::new(sockets::next_roll());
    let sizes: HashMap<String, u32> = event
        .entity
        .items
        .iter()
        .flatten()
        .map(|s| s.item.to_string())
        .chain(SALVAGE_OUTPUTS.iter().map(|s| s.to_string()))
        .map(|item| {
            let max = host.max_stack_size(&item).unwrap_or(64);
            (item, max)
        })
        .collect();
    let max_stack = |item: &str| sizes.get(item).copied().unwrap_or(64);
    salvage_all(&mut event.entity.items, &mut rng, &max_stack);
    event.entity.data = salvage_data(&event.entity.items).to_vec();
}

/// Java's slots: an input takes one piece that salvages; an output takes
/// nothing (it is only taken from).
pub fn salvage_slot_rule(slot: usize, stack: &ItemStack) -> Option<(bool, Option<u32>)> {
    match slot {
        s if s < SALVAGE_INPUTS => Some((!salvage_ranges(stack).is_empty(), Some(1))),
        s if s < SALVAGE_SLOTS => Some((false, None)),
        _ => None,
    }
}

const SALVAGE_TEXTURE: &str = "apotheosis:gui/salvage";
const SALVAGE_HEIGHT: f32 = 174.0;
/// Java's salvage button, at (98, 34), 18 square; its sprite at (238, 0),
/// the inactive one 18 below.
const SALVAGE_BUTTON: [f32; 2] = [98.0, 34.0];

fn salvage_slot(i: usize) -> [f32; 2] {
    if i < SALVAGE_INPUTS {
        [17.0 + (i % 4) as f32 * 19.0, 17.0 + (i / 4) as f32 * 19.0]
    } else {
        let j = i - SALVAGE_INPUTS;
        [124.0 + (j % 2) as f32 * 19.0, 17.0 + (j / 2) as f32 * 19.0]
    }
}

pub fn on_salvage_button(at: [f32; 2]) -> bool {
    let [x, y] = SALVAGE_BUTTON;
    at[0] >= x && at[0] < x + 18.0 && at[1] >= y && at[1] < y + 18.0
}

fn salvage_screen() -> NativeScreen {
    NativeScreen {
        width: WIDTH,
        height: SALVAGE_HEIGHT,
        slots: (0..SALVAGE_SLOTS).map(|i| Some(salvage_slot(i))).collect(),
        inventory_y: 92.0,
        inventory_x: 8.0,
        own_labels: false,
        own_background: true,
    }
}

/// The port's own panel for the `--no-java` pack: the background, the slot
/// frames and the player's inventory frames.
fn draw_panel(host: &dyn Host, batch: &mut HudBatch, origin: [f32; 2], height: f32, slots: &[[f32; 2]], inventory_y: f32) {
    let [ox, oy] = origin;
    host.fill(batch, [ox, oy, WIDTH, height], 0xFF2E_2A33);
    host.fill(batch, [ox + 4.0, oy + 4.0, WIDTH - 8.0, inventory_y - 14.0], 0xFF3B_3542);
    let mut frame = |x: f32, y: f32| host.fill(batch, [ox + x - 1.0, oy + y - 1.0, 18.0, 18.0], 0xFF1A_181D);
    for [x, y] in slots {
        frame(*x, *y);
    }
    for row in 0..4 {
        let y = if row == 3 { inventory_y + 58.0 } else { inventory_y + 18.0 * row as f32 };
        for col in 0..9 {
            frame(8.0 + 18.0 * col as f32, y);
        }
    }
}

fn salvage_draw(host: &dyn Host, event: &mut ScreenDrawEvent<'_>) {
    let [ox, oy] = event.origin;
    let java = host.sprite(event.batch, SALVAGE_TEXTURE, [0.0, 0.0, WIDTH, SALVAGE_HEIGHT], [ox, oy, WIDTH, SALVAGE_HEIGHT]);
    if !java {
        let slots: Vec<[f32; 2]> = (0..SALVAGE_SLOTS).map(salvage_slot).collect();
        draw_panel(host, event.batch, event.origin, SALVAGE_HEIGHT, &slots, 92.0);
    }
    let ready = get(event.data, 0) & VALID != 0;
    let [bx, by] = SALVAGE_BUTTON;
    let src = [238.0, if ready { 0.0 } else { 18.0 }, 18.0, 18.0];
    if !(java && host.sprite(event.batch, SALVAGE_TEXTURE, src, [ox + bx, oy + by, 18.0, 18.0])) {
        host.fill(event.batch, [ox + bx, oy + by, 18.0, 18.0], if ready { 0xFF6A_6070 } else { 0xFF1A_181D });
        host.text(event.batch, ">", [ox + bx + 6.0, oy + by + 5.0], if ready { 0xFFFFFF } else { 0x606060 }, true);
    }
}

fn salvage_tooltip(event: &mut ScreenTooltipEvent<'_>) {
    if !on_salvage_button(event.at) {
        return;
    }
    push_lines(event.lines, &salvage_lines(event.data));
}

// --- Gem cutting table ---------------------------------------------------------

/// The gem cutting menu type: Java's `GemCuttingMenu`.
pub const CUTTING: &str = "apotheosis:gem_cutting";
/// Its slots: the gem cut (base), a second gem of the same type and purity
/// (top), gem dust (left), a rarity material (right).
const CUT_BASE: usize = 0;
const CUT_TOP: usize = 1;
const CUT_DUST: usize = 2;
const CUT_MATERIAL: usize = 3;
const CUT_SLOTS: usize = 4;
const GEM_DUST: &str = "apotheosis:gem_dust";
/// Its data slots: flags (`VALID` when the button cuts), the dust the cut
/// takes, and the purity it reaches (`Purity::index`).
const CUT_DATA: usize = 3;

/// What cutting the gems in `items` takes (Java's `PurityUpgradeRecipe`): the
/// recipe of the base gem's current purity, its dust, and the first of its
/// alternative materials the right slot holds enough of (any one suffices).
/// Returns `(reached purity, dust, material count)`, or none when the slots
/// do not make a cut.
pub fn cut_plan(items: &[Option<ItemStack>]) -> Option<(Purity, u32, u32)> {
    let slot = |i: usize| items.get(i).and_then(|s| s.as_ref());
    let (gem, purity) = gems::of_stack(slot(CUT_BASE)?)?;
    let (top_gem, top_purity) = gems::of_stack(slot(CUT_TOP)?)?;
    if top_gem.id != gem.id || top_purity != purity {
        return None;
    }
    let (dust, materials) = sockets::upgrade_cost(purity)?;
    if count_of(items.get(CUT_DUST), Some(GEM_DUST)) < dust {
        return None;
    }
    let right = slot(CUT_MATERIAL)?;
    let (_, count) = materials
        .iter()
        .find(|(item, count)| &*right.item == *item && right.count >= *count)?;
    // The gem must have an item at the purity it reaches.
    gems::item_id(gem, purity.next())?;
    Some((purity.next(), dust, *count))
}

/// The data slots, from the slots.
pub fn cut_data(items: &[Option<ItemStack>]) -> [i32; CUT_DATA] {
    match cut_plan(items) {
        Some((purity, dust, _)) => [VALID, dust as i32, purity.index() as i32],
        None => [0; CUT_DATA],
    }
}

fn take(items: &mut [Option<ItemStack>], slot: usize, n: u32) {
    if let Some(stack) = &mut items[slot] {
        stack.count = stack.count.saturating_sub(n);
        if stack.count == 0 {
            items[slot] = None;
        }
    }
}

/// The upgrade button (Java's `GemCuttingMenu.clickMenuButton`): the base gem
/// becomes the next purity's item; one second gem, the dust and the material
/// are spent. False, with nothing changed, when the slots do not make a cut.
pub fn cut(items: &mut [Option<ItemStack>]) -> bool {
    if items.len() < CUT_SLOTS {
        return false;
    }
    let Some((purity, dust, material)) = cut_plan(items) else {
        return false;
    };
    let Some(mut gem) = items[CUT_BASE].clone() else {
        return false;
    };
    if !gems::change_purity(&mut gem, purity) {
        return false;
    }
    items[CUT_BASE] = Some(gem);
    take(items, CUT_TOP, 1);
    take(items, CUT_DUST, dust);
    take(items, CUT_MATERIAL, material);
    true
}

/// Java's slots: a gem below perfect in the base (one) and the top, gem dust
/// on the left, a material some purity upgrade takes on the right.
pub fn cut_slot_rule(slot: usize, stack: &ItemStack) -> Option<(bool, Option<u32>)> {
    let cuttable = gems::of_stack(stack).is_some_and(|(_, p)| p < Purity::Perfect);
    match slot {
        CUT_BASE => Some((cuttable, Some(1))),
        CUT_TOP => Some((cuttable, None)),
        CUT_DUST => Some((&*stack.item == GEM_DUST, None)),
        CUT_MATERIAL => Some((
            UPGRADE_COSTS.iter().any(|(_, _, mats)| mats.iter().any(|(m, _)| *m == &*stack.item)),
            None,
        )),
        _ => None,
    }
}

const CUT_TEXTURE: &str = "apotheosis:gui/gem_cutting";
const CUT_HEIGHT: f32 = 180.0;
const CUT_INVENTORY_Y: f32 = 98.0;
/// Java's slots: base (62, 45), top (62, 12), left (33, 64), right (90, 64).
const CUT_LAYOUT: [[f32; 2]; CUT_SLOTS] = [[62.0, 45.0], [62.0, 12.0], [33.0, 64.0], [90.0, 64.0]];
/// Java's upgrade button, at (135, 44), 18 square; its sprite at (238, 0),
/// the inactive one 18 below.
const CUT_BUTTON: [f32; 2] = [135.0, 44.0];

pub fn on_cut_button(at: [f32; 2]) -> bool {
    let [x, y] = CUT_BUTTON;
    at[0] >= x && at[0] < x + 18.0 && at[1] >= y && at[1] < y + 18.0
}

fn cut_screen() -> NativeScreen {
    NativeScreen {
        width: WIDTH,
        height: CUT_HEIGHT,
        slots: CUT_LAYOUT.iter().map(|s| Some(*s)).collect(),
        inventory_y: CUT_INVENTORY_Y,
        inventory_x: 8.0,
        own_labels: false,
        own_background: true,
    }
}

fn cut_draw(host: &dyn Host, event: &mut ScreenDrawEvent<'_>) {
    let [ox, oy] = event.origin;
    let java = host.sprite(event.batch, CUT_TEXTURE, [0.0, 0.0, WIDTH, CUT_HEIGHT], [ox, oy, WIDTH, CUT_HEIGHT]);
    if !java {
        draw_panel(host, event.batch, event.origin, CUT_HEIGHT, &CUT_LAYOUT, CUT_INVENTORY_Y);
    }
    let ready = get(event.data, 0) & VALID != 0;
    let [bx, by] = CUT_BUTTON;
    let src = [238.0, if ready { 0.0 } else { 18.0 }, 18.0, 18.0];
    if !(java && host.sprite(event.batch, CUT_TEXTURE, src, [ox + bx, oy + by, 18.0, 18.0])) {
        host.fill(event.batch, [ox + bx, oy + by, 18.0, 18.0], if ready { 0xFF6A_6070 } else { 0xFF1A_181D });
        host.text(event.batch, "^", [ox + bx + 6.0, oy + by + 5.0], if ready { 0xFFFFFF } else { 0x606060 }, true);
    }
}

fn cut_tooltip(event: &mut ScreenTooltipEvent<'_>) {
    if !on_cut_button(event.at) {
        return;
    }
    push_lines(event.lines, &cut_lines(event.data));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(lines: &[Line]) -> Vec<&str> {
        lines.iter().map(|l| l.key.as_str()).collect()
    }

    #[test]
    fn reforge_tooltip_is_translation_keys_with_the_rarity_material() {
        // Rare row (index 2, + 1 in bits 4..), choice 1 (n = 2): 3 sigils, 2 material, 5 levels each.
        let data = [3, 2, 5, VALID | (3 << 4)];
        let lines = reforge_lines(&data, 1);
        assert_eq!(
            keys(&lines),
            [
                "apotheosis.tier.rare",
                "text.apotheosis.reforge_cost",
                "text.apotheosis.cost",
                "text.apotheosis.cost",
                "misc.apotheosis.level.many",
                "container.enchant.level.requirement",
            ]
        );
        assert_eq!(lines[2].args, [Arg::Num(6), Arg::Item(SIGIL)]);
        assert_eq!(lines[3].args, [Arg::Num(4), Arg::Item(sockets::salvage_material(Tier::Rare).0)]);
        assert_eq!(lines[4].prefix, "§72 ");
        assert_eq!(lines[5].args, [Arg::Num(10)]);
        assert_eq!(reforge_lines(&data, 0)[4].key, "misc.apotheosis.level");
        assert!(lines.iter().all(|l| !l.prefix.contains('/') && !l.fallback.contains('/')));
    }

    #[test]
    fn salvage_and_cut_tooltips_name_items_by_their_keys() {
        assert_eq!(keys(&salvage_lines(&[0])), ["button.apotheosis.no_salvage"]);
        let mut data = vec![0; SALVAGE_DATA];
        data[0] = VALID;
        (data[1], data[2]) = (1, 3);
        let lines = salvage_lines(&data);
        assert_eq!(keys(&lines), ["text.apotheosis.salvage_results", "item.apotheosis.mysterious_scrap_metal"]);
        assert_eq!(lines[1].prefix, "§71-3 ");
        assert_eq!(lines[1].fallback, "mysterious scrap metal");
        assert_eq!(keys(&cut_lines(&[0])), ["button.apotheosis.upgrade.no"]);
        let lines = cut_lines(&[VALID, 7, Purity::Flawed.index() as i32]);
        assert_eq!(keys(&lines), ["apotheosis.gem_cutting.upgrade_to", "text.apotheosis.cost"]);
        assert_eq!(lines[0].args, [Arg::Key("apotheosis.purity.flawed", "Flawed")]);
        assert_eq!(lines[1].args, [Arg::Num(7), Arg::Item(GEM_DUST)]);
    }

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

    fn affixed_sword(tier: Tier) -> ItemStack {
        sockets::reforged(&ItemStack::new("minecraft:diamond_sword", 1), tier, 7)
    }

    fn gem(purity: sockets::Purity) -> ItemStack {
        gems::stack(gems::get("apotheosis:core/ballast").unwrap(), purity)
    }

    fn salvage_table(inputs: &[ItemStack]) -> Vec<Option<ItemStack>> {
        let mut items = vec![None; SALVAGE_SLOTS];
        for (i, s) in inputs.iter().enumerate() {
            items[i] = Some(s.clone());
        }
        items
    }

    #[test]
    fn salvaging_reads_the_java_recipes_ranges() {
        assert_eq!(salvage_ranges(&affixed_sword(Tier::Rare)), vec![(2, 1, 4)]);
        assert_eq!(salvage_ranges(&gem(sockets::Purity::Cracked)), vec![(5, 1, 2)]);
        assert!(salvage_ranges(&ItemStack::new("minecraft:diamond_sword", 1)).is_empty());
        assert!(salvage_ranges(&ItemStack::new("minecraft:stone", 1)).is_empty());
    }

    #[test]
    fn the_salvage_data_sums_every_input() {
        let items = salvage_table(&[affixed_sword(Tier::Rare), affixed_sword(Tier::Rare), gem(sockets::Purity::Normal)]);
        let data = salvage_data(&items);
        assert_eq!(data[0] & VALID, VALID);
        assert_eq!(&data[5..7], &[2, 8], "two rare pieces: 2-8 shards");
        assert_eq!(&data[11..13], &[3, 5], "a normal gem: 3-5 dust");
        assert_eq!(salvage_data(&salvage_table(&[])), [0; SALVAGE_DATA]);
    }

    #[test]
    fn the_salvage_button_destroys_the_inputs_and_fills_the_outputs() {
        let mut items = salvage_table(&[affixed_sword(Tier::Rare), gem(sockets::Purity::Cracked), ItemStack::new("minecraft:stone", 1)]);
        let mut rng = Rng::new(3);
        assert_eq!(salvage_all(&mut items, &mut rng, &|_| 64), 2);
        assert!(items[0].is_none() && items[1].is_none());
        assert!(items[2].is_some(), "what does not salvage stays");
        let outputs: Vec<&ItemStack> = items[SALVAGE_INPUTS..].iter().flatten().collect();
        let shards = outputs.iter().find(|s| &*s.item == "apotheosis:luminous_crystal_shard").unwrap();
        assert!((1..=4).contains(&shards.count));
        let dust = outputs.iter().find(|s| &*s.item == "apotheosis:gem_dust").unwrap();
        assert!((1..=2).contains(&dust.count));
    }

    #[test]
    fn a_piece_whose_results_do_not_fit_stays_whole() {
        let mut items = salvage_table(&[affixed_sword(Tier::Rare)]);
        for slot in &mut items[SALVAGE_INPUTS..] {
            *slot = Some(ItemStack::new("minecraft:stone", 64));
        }
        assert_eq!(salvage_all(&mut items, &mut Rng::new(1), &|_| 64), 0);
        assert!(items[0].is_some());
        assert!(items[SALVAGE_INPUTS..].iter().flatten().all(|s| &*s.item == "minecraft:stone" && s.count == 64));
    }

    #[test]
    fn salvage_outputs_stack_up_to_their_size() {
        let mut slots = vec![Some(ItemStack::new("apotheosis:gem_dust", 62)), None];
        assert!(insert(&mut slots, ItemStack::new("apotheosis:gem_dust", 5), &|_| 64));
        assert_eq!(slots[0].as_ref().unwrap().count, 64);
        assert_eq!(slots[1].as_ref().unwrap().count, 3);
        assert!(!insert(&mut slots, ItemStack::new("apotheosis:arcane_sands", 1), &|_| 64));
    }

    #[test]
    fn salvage_slots_take_one_salvageable_piece_and_outputs_take_nothing() {
        assert_eq!(salvage_slot_rule(0, &affixed_sword(Tier::Common)), Some((true, Some(1))));
        assert_eq!(salvage_slot_rule(11, &gem(sockets::Purity::Flawed)), Some((true, Some(1))));
        assert_eq!(salvage_slot_rule(3, &ItemStack::new("minecraft:stone", 1)), Some((false, Some(1))));
        assert_eq!(salvage_slot_rule(12, &gem(sockets::Purity::Flawed)), Some((false, None)));
        assert_eq!(salvage_slot_rule(18, &gem(sockets::Purity::Flawed)), None);
    }

    #[test]
    fn the_salvage_layout_and_button_follow_java() {
        assert_eq!(salvage_slot(5), [36.0, 36.0]);
        assert_eq!(salvage_slot(17), [143.0, 55.0]);
        assert!(on_salvage_button([100.0, 40.0]));
        assert!(!on_salvage_button([90.0, 40.0]));
        assert_eq!(menu_of("apotheosis:salvaging_table"), Some(SALVAGING));
    }

    fn cut_table(base: Purity, top: Purity, dust: u32, material: (&str, u32)) -> Vec<Option<ItemStack>> {
        vec![Some(gem(base)), Some(gem(top)), stack(GEM_DUST, dust), stack(material.0, material.1)]
    }

    #[test]
    fn a_cut_costs_the_recipe_of_the_current_purity() {
        // cracked.json: a cracked gem, 1 dust and 3 scrap OR 1 fabric.
        let scrap = cut_table(Purity::Cracked, Purity::Cracked, 1, ("apotheosis:mysterious_scrap_metal", 3));
        assert_eq!(cut_plan(&scrap), Some((Purity::Chipped, 1, 3)));
        let fabric = cut_table(Purity::Cracked, Purity::Cracked, 1, ("apotheosis:timeworn_fabric", 1));
        assert_eq!(cut_plan(&fabric), Some((Purity::Chipped, 1, 1)), "one alternative suffices");
        // flawed.json: a flawed gem, 5 dust and a single arcane sands.
        let flawed = cut_table(Purity::Flawed, Purity::Flawed, 5, ("apotheosis:arcane_sands", 1));
        assert_eq!(cut_plan(&flawed), Some((Purity::Normal, 5, 1)));
        assert_eq!(cut_plan(&cut_table(Purity::Flawed, Purity::Flawed, 4, ("apotheosis:arcane_sands", 1))), None);
        assert_eq!(cut_plan(&cut_table(Purity::Flawed, Purity::Flawed, 5, ("apotheosis:godforged_pearl", 9))), None);
        assert_eq!(
            cut_plan(&cut_table(Purity::Cracked, Purity::Cracked, 1, ("apotheosis:mysterious_scrap_metal", 2))),
            None
        );
    }

    #[test]
    fn flawless_cuts_to_perfect_and_perfect_does_not_cut() {
        let flawless = cut_table(Purity::Flawless, Purity::Flawless, 9, ("apotheosis:godforged_pearl", 3));
        assert_eq!(cut_plan(&flawless), Some((Purity::Perfect, 9, 3)));
        assert_eq!(cut_plan(&cut_table(Purity::Perfect, Purity::Perfect, 64, ("apotheosis:godforged_pearl", 64))), None);
    }

    #[test]
    fn the_second_gem_must_match_type_and_purity() {
        assert_eq!(cut_plan(&cut_table(Purity::Cracked, Purity::Chipped, 1, ("apotheosis:timeworn_fabric", 1))), None);
        let mut other = cut_table(Purity::Cracked, Purity::Cracked, 1, ("apotheosis:timeworn_fabric", 1));
        other[CUT_TOP] = Some(gems::stack(gems::get("apotheosis:core/brawlers").unwrap(), Purity::Cracked));
        assert_eq!(cut_plan(&other), None);
    }

    #[test]
    fn the_upgrade_button_gives_the_next_purity_item_and_spends_the_inputs() {
        let mut items = cut_table(Purity::Chipped, Purity::Chipped, 5, ("apotheosis:luminous_crystal_shard", 2));
        items[CUT_TOP].as_mut().unwrap().count = 2;
        assert_eq!(cut_data(&items), [VALID, 3, Purity::Flawed.index() as i32]);
        assert!(cut(&mut items));
        let base = items[CUT_BASE].as_ref().unwrap();
        assert_eq!(&*base.item, "apotheosis:gem_ballast_flawed");
        assert_eq!(gems::of_stack(base).map(|(_, p)| p), Some(Purity::Flawed));
        assert_eq!(items[CUT_TOP].as_ref().map(|s| s.count), Some(1));
        assert_eq!(items[CUT_DUST].as_ref().map(|s| s.count), Some(2));
        assert_eq!(items[CUT_MATERIAL].as_ref().map(|s| s.count), Some(1));
        assert!(!cut(&mut items), "the second gem is now of another purity");
        assert_eq!(cut_data(&items), [0; CUT_DATA]);
    }

    #[test]
    fn cut_slots_follow_java() {
        assert_eq!(cut_slot_rule(CUT_BASE, &gem(Purity::Normal)), Some((true, Some(1))));
        assert_eq!(cut_slot_rule(CUT_BASE, &gem(Purity::Perfect)).map(|r| r.0), Some(false));
        assert_eq!(cut_slot_rule(CUT_TOP, &gem(Purity::Flawed)), Some((true, None)));
        assert_eq!(cut_slot_rule(CUT_DUST, &ItemStack::new(GEM_DUST, 1)).map(|r| r.0), Some(true));
        assert_eq!(cut_slot_rule(CUT_DUST, &gem(Purity::Flawed)).map(|r| r.0), Some(false));
        let fabric = ItemStack::new("apotheosis:timeworn_fabric", 1);
        assert_eq!(cut_slot_rule(CUT_MATERIAL, &fabric).map(|r| r.0), Some(true));
        assert_eq!(cut_slot_rule(CUT_MATERIAL, &ItemStack::new("minecraft:stone", 1)).map(|r| r.0), Some(false));
        assert_eq!(cut_slot_rule(4, &gem(Purity::Flawed)), None);
        assert_eq!(menu_of("apotheosis:gem_cutting_table"), Some(CUTTING));
        assert!(on_cut_button([140.0, 50.0]));
        assert!(!on_cut_button([130.0, 50.0]));
    }
}
