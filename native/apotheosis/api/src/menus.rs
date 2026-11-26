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
//! The augmenting table (Java's `AugmentingMenu`) lists the affixes of the
//! piece in its first slot, sorted by id, six to a page (Java uses a dropdown).
//! A row selects an affix; Upgrade raises it by 0.25 up to 1.0, Reroll replaces
//! it with another affix of the same type that the piece's rarity and category
//! allow, at a level drawn in `[0, 1)`. Both cost Sigils of Enhancement and
//! experience levels, free in creative. Deviation: the port keeps no level for
//! effect affixes, so only the attribute affixes the `affixes` module ties to a
//! level are listed and upgradable; Java lists both kinds.
//!
//! Deviation: Java gives the gear back to the player when the menu closes
//! (`removed()`); taking a slot out of a block entity from a close hook needs a
//! host helper past API 415, so the gear stays in the table, as in a furnace.
//! The choices show their rarity and cost, not the rolled piece: the client
//! has only the data slots, and the roll is made again on the server.

use horizon_client::mod_api::{
    BlockEntityContainerEvent, BlockEntityEvent, BlockEntityTickEvent, BlockEntityTicksEvent,
    Compound, Host, HudBatch, ItemStack, MenuButtonEvent, MenuSlotRulesEvent, ModContainer, NativeScreen,
    ScreenClickEvent, ScreenDrawEvent, ScreenEvent, ScreenTooltipEvent, Tag, Text,
};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::affixes::{self, Instance};
use crate::attributes;
use crate::gems;
use crate::java_affixes::{Affix, AFFIXES};
use crate::sockets::{self, Purity, Table, REFORGE_COSTS, UPGRADE_COSTS};
use crate::tiers::{self, WorldTier};
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
        Table::Augmenting => Some(AUGMENTING),
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
            AUGMENTING => AUG_SLOTS,
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
    if menu_of(&event.entity.id) == Some(AUGMENTING) {
        let selected = clamp_selection(
            &event.entity.items,
            augmenting_selected(&event.entity.nbt),
        );
        if augmenting_selected(&event.entity.nbt) != selected {
            set_augmenting_selected(&mut event.entity.nbt, selected);
        }
        let data = augmenting_data(&event.entity.items, selected);
        if event.entity.data[..] != data[..] {
            event.entity.data = data.to_vec();
        }
        return true;
    }
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
    if menu_of(&event.entity.id) == Some(AUGMENTING) {
        return augmenting_button(host, event);
    }
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
        AUGMENTING => augmenting_slot_rule(event.slot, event.stack),
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
        AUGMENTING => return Some(augmenting_screen()),
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
        Some(AUGMENTING) => return augmenting_draw(host, event),
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
    if menu_of(event.block)? == AUGMENTING {
        return augmenting_click(event);
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
    if menu_of(event.block) == Some(AUGMENTING) {
        return augmenting_tooltip(event);
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

// --- Augmenting table ----------------------------------------------------------

/// The augmenting menu type: Java's `AugmentingMenu`.
pub const AUGMENTING: &str = "apotheosis:augmenting";
/// Its slots: the affixed piece, the sigils of enhancement.
const AUG_GEAR: usize = 0;
const AUG_SIGILS: usize = 1;
const AUG_SLOTS: usize = 2;
const AUG_SIGIL: &str = "apotheosis:sigil_of_enhancement";
/// Java's `AdventureConfig` defaults (`upgradeSigilCost`, `upgradeLevelCost`,
/// `rerollSigilCost`, `rerollLevelCost`).
pub const UPGRADE_SIGILS: u32 = 2;
pub const UPGRADE_LEVELS: i32 = 225;
pub const REROLL_SIGILS: u32 = 1;
pub const REROLL_LEVELS: i32 = 175;
/// The affix rows one page shows.
pub const AUG_ROWS: usize = 6;
/// Its data slots: flags, count, selected, first row, both costs, then each
/// row's `AFFIXES` index + 1 and its level in hundredths.
const AUG_DATA: usize = 8 + 2 * AUG_ROWS;
const AUG_VALID: i32 = 1;
const AUG_UPGRADE: i32 = 2;
const AUG_REROLL: i32 = 4;
const AUG_PREV: i32 = 8;
const AUG_NEXT: i32 = 16;
/// The block entity's `nbt` key holding the selected affix, so the selection
/// survives the screen closing.
const AUG_SELECTED_KEY: &str = "augmenting_selected";

fn augmenting_selected(nbt: &Compound) -> usize {
    match nbt.get(&AUG_SELECTED_KEY.into()) {
        Some(Tag::Int(selected)) if *selected > 0 => *selected as usize,
        _ => 0,
    }
}

fn set_augmenting_selected(nbt: &mut Compound, selected: usize) {
    nbt.insert(AUG_SELECTED_KEY.into(), Tag::Int(selected as i32));
}

/// The affixes a piece carries, sorted by id as Java's `computeItemAffixes`.
pub fn affixes_of(stack: &ItemStack) -> Vec<Instance> {
    let mut instances = affixes::instances(stack);
    instances.sort_by(|a, b| a.affix.id.cmp(b.affix.id));
    instances
}

/// Java's `Affix.STANDARD_MAX_LEVEL`-bounded upgrade: a quarter of a level,
/// never past 1.0. `None` at or above the cap, so an overcharged affix (a Sigil
/// of Malice raised it) cannot be upgraded.
pub fn upgrade(level: f64) -> Option<f64> {
    if level < 1.0 {
        Some((level + 0.25).min(1.0))
    } else {
        None
    }
}

/// Java's `canAugment`: a level-dependent affix below the standard cap.
pub fn can_upgrade(instance: &Instance, tier: Tier) -> bool {
    instance.level_dependent(tier) && upgrade(instance.level_at(tier)).is_some()
}

/// Two affixes are compatible unless either names the other in Java's
/// `exclusive_set`.
pub fn compatible(a: &Affix, b: &Affix) -> bool {
    a.id != b.id && !a.exclusive.contains(&b.id) && !b.exclusive.contains(&a.id)
}

/// Java's `getAlternativeAffixes`: every affix of `pool` of the same type as
/// `chosen`, with a value at `tier`, that stays compatible with the piece's
/// other affixes; the chosen one is never in it.
pub fn reroll_candidates(
    pool: &[&'static Affix],
    chosen: &Affix,
    remaining: &[&'static Affix],
    tier: Tier,
) -> Vec<&'static Affix> {
    let rarity = affixes::rarity_index(tier);
    pool.iter()
        .copied()
        .filter(|candidate| candidate.id != chosen.id)
        .filter(|candidate| candidate.affix_type == chosen.affix_type)
        .filter(|candidate| candidate.values[rarity].is_some())
        .filter(|candidate| remaining.iter().all(|other| compatible(candidate, other)))
        .collect()
}

/// The affixes a reroll of `chosen` may draw at the piece's own rarity.
pub fn alternatives(stack: &ItemStack, chosen: &Instance) -> Vec<&'static Affix> {
    let Some(tier) = sockets::tier_of(stack) else {
        return Vec::new();
    };
    let class = attributes::class_of(&stack.item);
    let remaining: Vec<&'static Affix> = affixes_of(stack)
        .iter()
        .map(|instance| instance.affix)
        .filter(|affix| affix.id != chosen.affix.id)
        .collect();
    reroll_candidates(&affixes::pool(class, tier), chosen.affix, &remaining, tier)
}

/// Java's `TieredWeights.getWeight(tier, luck)` for an affix, clamped at 0.
pub fn weight_at(affix: &Affix, world: WorldTier, luck: f32) -> i64 {
    affix.weights[world.index()].map_or(0, |weight| {
        let bonus = (luck * weight.quality + 0.5).floor() as i64;
        (i64::from(weight.weight) + bonus).max(0)
    })
}

/// Picks a candidate by weight; when every weight is zero (Java's
/// `WeightedRandom` finding nothing), uniformly instead.
pub fn choose_weighted(
    candidates: &[&'static Affix],
    weights: &[i64],
    rng: &mut Rng,
) -> Option<usize> {
    if candidates.is_empty() {
        return None;
    }
    let total: u64 = (0..candidates.len())
        .map(|i| weights.get(i).copied().unwrap_or(0).max(0) as u64)
        .sum();
    if total == 0 {
        return Some(rng.below(candidates.len() as u64) as usize);
    }
    let mut pick = rng.below(total);
    for i in 0..candidates.len() {
        let weight = weights.get(i).copied().unwrap_or(0).max(0) as u64;
        if pick < weight {
            return Some(i);
        }
        pick -= weight;
    }
    Some(candidates.len() - 1)
}

/// Vanilla's total experience needed to reach `level` (`Player.getXpNeededForLevel`'s running sum).
fn total_xp(level: i32) -> i64 {
    let level = i64::from(level.max(0));
    if level <= 16 {
        level * level + 6 * level
    } else if level <= 31 {
        (5 * level * level - 81 * level + 720) / 2
    } else {
        (9 * level * level - 325 * level + 4440) / 2
    }
}

/// Vanilla's experience the next level of `level` takes.
fn xp_to_next(level: i32) -> i64 {
    let level = i64::from(level.max(0));
    if level >= 30 {
        9 * level - 158
    } else if level >= 15 {
        5 * level - 38
    } else {
        2 * level + 7
    }
}

/// The highest level `xp` points reach, at most `cap`.
fn level_for_xp(xp: i64, cap: i32) -> i32 {
    let mut low = 0;
    let mut high = cap.max(0);
    while low < high {
        let mid = (low + high + 1) / 2;
        if total_xp(mid) <= xp {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    low
}

/// The levels `give_levels` must take off to mirror Java's charge of
/// `cost` levels' worth of experience points (`EnchantmentUtils.chargeExperience`
/// with `getTotalExperienceForLevel`): the level the player is left on is the
/// one their remaining points reach, which is not simply `level - cost` (the
/// total grows faster than that). The progress bar is kept, as `give_levels`
/// does; only the level matches Java exactly.
pub fn levels_charged(level: i32, progress: f32, cost: i32) -> i32 {
    let level = level.max(0);
    let points = total_xp(level)
        + (f64::from(progress.clamp(0.0, 1.0)) * xp_to_next(level) as f64).round() as i64;
    let remaining = (points - total_xp(cost.max(0))).max(0);
    (level - level_for_xp(remaining, level)).max(0)
}

/// What a transaction takes from the player: the sigils and the levels, free
/// in creative. `None` when the player is short of either, so nothing happens.
pub fn charge(
    sigil_cost: u32,
    level_cost: i32,
    sigils: u32,
    xp_level: i32,
    xp_progress: f32,
    creative: bool,
) -> Option<(u32, i32)> {
    if creative {
        return Some((0, 0));
    }
    if sigils < sigil_cost || xp_level < level_cost {
        return None;
    }
    Some((sigil_cost, levels_charged(xp_level, xp_progress, level_cost)))
}

/// What an upgrade of `selected` would do: its new level, and the sigils and
/// levels it takes. `None` for an affix that cannot be upgraded or a player
/// who cannot pay.
pub fn upgrade_plan(
    stack: &ItemStack,
    selected: usize,
    sigils: u32,
    xp_level: i32,
    xp_progress: f32,
    creative: bool,
) -> Option<(usize, f64, u32, i32)> {
    let tier = sockets::tier_of(stack)?;
    let instances = affixes_of(stack);
    let instance = instances.get(selected)?;
    let next = upgrade(instance.level_at(tier)).filter(|_| instance.level_dependent(tier))?;
    let (sigils, levels) = charge(
        UPGRADE_SIGILS,
        UPGRADE_LEVELS,
        sigils,
        xp_level,
        xp_progress,
        creative,
    )?;
    Some((selected, next, sigils, levels))
}

/// What a reroll of `selected` would do: the affix drawn and the level it lands
/// at, plus what it takes.
#[derive(Clone, Copy, Debug)]
pub struct Reroll {
    pub affix: &'static Affix,
    pub level: f64,
    pub sigils: u32,
    pub levels: i32,
}

/// Java's reroll: one alternative of the same type, drawn by its weight for the
/// player's world tier and luck (uniformly when they all weigh nothing), at a
/// level in `[0, 1)`.
pub fn reroll_plan(
    stack: &ItemStack,
    selected: usize,
    world: WorldTier,
    luck: f32,
    sigils: u32,
    xp_level: i32,
    xp_progress: f32,
    creative: bool,
    rng: &mut Rng,
) -> Option<Reroll> {
    let instances = affixes_of(stack);
    let chosen = instances.get(selected)?;
    let candidates = alternatives(stack, chosen);
    if candidates.is_empty() {
        return None;
    }
    let (sigils, levels) = charge(
        REROLL_SIGILS,
        REROLL_LEVELS,
        sigils,
        xp_level,
        xp_progress,
        creative,
    )?;
    let weights: Vec<i64> = candidates
        .iter()
        .map(|affix| weight_at(affix, world, luck))
        .collect();
    let index = choose_weighted(&candidates, &weights, rng)?;
    Some(Reroll {
        affix: candidates[index],
        level: affixes::roll_level(rng),
        sigils,
        levels,
    })
}

/// The piece with affix `selected` raised to `next`; every other affix and
/// every other piece of data is left as it was.
pub fn upgraded(stack: &ItemStack, selected: usize, next: f64) -> Option<ItemStack> {
    let tier = sockets::tier_of(stack)?;
    let mut instances = affixes_of(stack);
    instances.get_mut(selected)?.set_level(tier, next);
    let mut upgraded = stack.clone();
    affixes::rewrite(&mut upgraded, tier, &instances);
    Some(upgraded)
}

/// The piece with affix `selected` replaced by `affix` at `level`, its affixes
/// kept sorted by id as Java keeps them.
pub fn rerolled(
    stack: &ItemStack,
    selected: usize,
    affix: &'static Affix,
    level: f64,
) -> Option<ItemStack> {
    let tier = sockets::tier_of(stack)?;
    let value = affixes::value_at(affix, tier, level)?;
    let mut instances = affixes_of(stack);
    *instances.get_mut(selected)? = Instance {
        affix,
        level: Some(level),
        value,
    };
    instances.sort_by(|a, b| a.affix.id.cmp(b.affix.id));
    let mut rerolled = stack.clone();
    affixes::rewrite(&mut rerolled, tier, &instances);
    Some(rerolled)
}

/// The affixes of the piece in the gear slot, clamped to what `selected` may
/// be (0 for an empty piece).
pub fn clamp_selection(items: &[Option<ItemStack>], selected: usize) -> usize {
    let count = items
        .get(AUG_GEAR)
        .and_then(|slot| slot.as_ref())
        .map_or(0, |stack| affixes_of(stack).len());
    if count == 0 { 0 } else { selected.min(count - 1) }
}

/// The data slots, from the slots and the selected affix.
pub fn augmenting_data(items: &[Option<ItemStack>], selected: usize) -> [i32; AUG_DATA] {
    let mut data = [0; AUG_DATA];
    let Some(stack) = items.get(AUG_GEAR).and_then(|slot| slot.as_ref()) else {
        return data;
    };
    let Some(tier) = sockets::tier_of(stack) else {
        return data;
    };
    let instances = affixes_of(stack);
    if instances.is_empty() {
        return data;
    }
    let selected = selected.min(instances.len() - 1);
    let first = selected / AUG_ROWS * AUG_ROWS;
    let chosen = &instances[selected];
    let mut flags = AUG_VALID;
    if can_upgrade(chosen, tier) {
        flags |= AUG_UPGRADE;
    }
    if !alternatives(stack, chosen).is_empty() {
        flags |= AUG_REROLL;
    }
    if first > 0 {
        flags |= AUG_PREV;
    }
    if first + AUG_ROWS < instances.len() {
        flags |= AUG_NEXT;
    }
    data[0] = flags;
    data[1] = instances.len() as i32;
    data[2] = selected as i32;
    data[3] = first as i32;
    data[4] = UPGRADE_SIGILS as i32;
    data[5] = UPGRADE_LEVELS;
    data[6] = REROLL_SIGILS as i32;
    data[7] = REROLL_LEVELS;
    for (row, instance) in instances.iter().skip(first).take(AUG_ROWS).enumerate() {
        let index = AFFIXES
            .iter()
            .position(|affix| affix.id == instance.affix.id)
            .map_or(0, |at| at as i32 + 1);
        data[8 + 2 * row] = index;
        data[9 + 2 * row] = (instance.level_at(tier) * 100.0).round() as i32;
    }
    data
}

fn take_sigils(items: &mut [Option<ItemStack>], count: u32) {
    if count == 0 {
        return;
    }
    if let Some(stack) = &mut items[AUG_SIGILS] {
        stack.count = stack.count.saturating_sub(count);
        if stack.count == 0 {
            items[AUG_SIGILS] = None;
        }
    }
}

fn augmenting_button(host: &mut dyn Host, event: &mut MenuButtonEvent<'_>) {
    if event.entity.items.len() < AUG_SLOTS {
        return;
    }
    let Some(player) = host.acting_player() else {
        return;
    };
    let creative = player.game_mode == 1;
    let Some(stack) = event.entity.items[AUG_GEAR].clone() else {
        return;
    };
    let count = affixes_of(&stack).len();
    if count == 0 {
        return;
    }
    let mut selected = augmenting_selected(&event.entity.nbt).min(count - 1);
    let first = selected / AUG_ROWS * AUG_ROWS;
    if (0..AUG_ROWS as i32).contains(&event.id) {
        selected = (first + event.id as usize).min(count - 1);
        set_augmenting_selected(&mut event.entity.nbt, selected);
        event.entity.data = augmenting_data(&event.entity.items, selected).to_vec();
        return;
    }
    match event.id {
        6 if first > 0 => {
            selected = first.saturating_sub(AUG_ROWS);
        }
        7 if first + AUG_ROWS < count => {
            selected = (first + AUG_ROWS).min(count - 1);
        }
        8 => {
            let sigils = count_of(event.entity.items.get(AUG_SIGILS), Some(AUG_SIGIL));
            let Some((picked, next, sigils, levels)) = upgrade_plan(
                &stack,
                selected,
                sigils,
                player.xp_level,
                player.xp_progress,
                creative,
            ) else {
                return;
            };
            let Some(piece) = upgraded(&stack, picked, next) else {
                return;
            };
            // Everything is checked and built: charge last, so a refused
            // payment leaves the piece and the sigils untouched.
            if levels > 0 && !host.give_levels(player.uuid, -levels) {
                return;
            }
            take_sigils(&mut event.entity.items, sigils);
            event.entity.items[AUG_GEAR] = Some(piece);
        }
        9 => {
            let sigils = count_of(event.entity.items.get(AUG_SIGILS), Some(AUG_SIGIL));
            let mut rng = Rng::new(sockets::next_roll());
            let Some(plan) = reroll_plan(
                &stack,
                selected,
                tiers::tier_of(player.uuid),
                0.0,
                sigils,
                player.xp_level,
                player.xp_progress,
                creative,
                &mut rng,
            ) else {
                return;
            };
            let Some(piece) = rerolled(&stack, selected, plan.affix, plan.level) else {
                return;
            };
            if plan.levels > 0 && !host.give_levels(player.uuid, -plan.levels) {
                return;
            }
            take_sigils(&mut event.entity.items, plan.sigils);
            event.entity.items[AUG_GEAR] = Some(piece);
        }
        _ => return,
    }
    let selected = clamp_selection(&event.entity.items, selected);
    set_augmenting_selected(&mut event.entity.nbt, selected);
    event.entity.data = augmenting_data(&event.entity.items, selected).to_vec();
}

/// Java's slots: one affixed piece, sigils of enhancement.
pub fn augmenting_slot_rule(slot: usize, stack: &ItemStack) -> Option<(bool, Option<u32>)> {
    match slot {
        AUG_GEAR => Some((sockets::has_affixes(stack), Some(1))),
        AUG_SIGILS => Some((&*stack.item == AUG_SIGIL, None)),
        _ => None,
    }
}

const AUG_TEXTURE: &str = "apotheosis:gui/augmenting";
const AUG_HEIGHT: f32 = 222.0;
const AUG_INVENTORY_Y: f32 = 140.0;
/// Java's slots: the piece (16, 16), the sigils (16, 41).
const AUG_LAYOUT: [[f32; 2]; AUG_SLOTS] = [[16.0, 16.0], [16.0, 41.0]];
/// Java's buttons, at (60, 111) and (112, 111), 29 by 13, drawn 2px larger; the
/// list takes their left-hand space.
const AUG_UPGRADE_BUTTON: [f32; 2] = [60.0, 111.0];
const AUG_REROLL_BUTTON: [f32; 2] = [108.0, 111.0];
const AUG_BUTTON_SIZE: [f32; 2] = [32.0, 14.0];
/// The page arrows, right of the list.
const AUG_PREV_BUTTON: [f32; 2] = [160.0, 39.0];
const AUG_NEXT_BUTTON: [f32; 2] = [160.0, 100.0];
const AUG_ARROW_SIZE: [f32; 2] = [12.0, 10.0];
const AUG_LIST_X: f32 = 42.0;
const AUG_LIST_WIDTH: f32 = 117.0;
const AUG_ROW_HEIGHT: f32 = 11.0;

fn aug_row_y(row: usize) -> f32 {
    39.0 + AUG_ROW_HEIGHT * row as f32
}

fn aug_row_at(at: [f32; 2]) -> Option<usize> {
    (0..AUG_ROWS).find(|&row| {
        at[0] >= AUG_LIST_X
            && at[0] < AUG_LIST_X + AUG_LIST_WIDTH
            && at[1] >= aug_row_y(row)
            && at[1] < aug_row_y(row) + AUG_ROW_HEIGHT
    })
}

fn inside(at: [f32; 2], corner: [f32; 2], size: [f32; 2]) -> bool {
    at[0] >= corner[0]
        && at[0] < corner[0] + size[0]
        && at[1] >= corner[1]
        && at[1] < corner[1] + size[1]
}

fn augmenting_screen() -> NativeScreen {
    NativeScreen {
        width: WIDTH,
        height: AUG_HEIGHT,
        slots: AUG_LAYOUT.iter().map(|slot| Some(*slot)).collect(),
        inventory_y: AUG_INVENTORY_Y,
        inventory_x: 8.0,
        own_labels: false,
        own_background: true,
    }
}

fn augmenting_draw(host: &dyn Host, event: &mut ScreenDrawEvent<'_>) {
    let [ox, oy] = event.origin;
    let java = host.sprite(
        event.batch,
        AUG_TEXTURE,
        [0.0, 0.0, WIDTH, AUG_HEIGHT],
        [ox, oy, WIDTH, AUG_HEIGHT],
    );
    if !java {
        draw_panel(host, event.batch, event.origin, AUG_HEIGHT, &AUG_LAYOUT, AUG_INVENTORY_Y);
    }
    let flags = get(event.data, 0);
    let count = get(event.data, 1).max(0) as usize;
    let selected = get(event.data, 2).max(0) as usize;
    let first = get(event.data, 3).max(0) as usize;
    for row in 0..AUG_ROWS {
        let index = first + row;
        if index >= count {
            continue;
        }
        let y = aug_row_y(row);
        if index == selected {
            host.fill(
                event.batch,
                [ox + AUG_LIST_X - 1.0, oy + y - 1.0, AUG_LIST_WIDTH + 2.0, AUG_ROW_HEIGHT],
                0x60C9_FF20,
            );
        }
        let affix = get(event.data, 8 + 2 * row) - 1;
        if let Some(affix) = usize::try_from(affix).ok().and_then(|at| AFFIXES.get(at)) {
            host.text(event.batch, affix.name, [ox + AUG_LIST_X + 3.0, oy + y + 1.0], 0xFFFF80, true);
        }
        let level = get(event.data, 9 + 2 * row).max(0) as f32 / 100.0;
        let text = format!("{level:.2}");
        let width = host.text_width(&text);
        host.text(
            event.batch,
            &text,
            [ox + AUG_LIST_X + AUG_LIST_WIDTH - width - 2.0, oy + y + 1.0],
            0xB0B0B0,
            true,
        );
    }
    if flags & AUG_PREV != 0 {
        host.text(event.batch, "<", [ox + AUG_PREV_BUTTON[0], oy + AUG_PREV_BUTTON[1]], 0xFFFFFF, true);
    }
    if flags & AUG_NEXT != 0 {
        host.text(event.batch, ">", [ox + AUG_NEXT_BUTTON[0], oy + AUG_NEXT_BUTTON[1]], 0xFFFFFF, true);
    }
    let buttons = [
        (AUG_UPGRADE_BUTTON, AUG_UPGRADE, "Up"),
        (AUG_REROLL_BUTTON, AUG_REROLL, "Re"),
    ];
    // Costs live in the button tooltip, as in Java: a label under the button would cover "Inventory".
    for (corner, flag, label) in buttons {
        let ready = flags & flag != 0;
        let [x, y] = corner;
        let src = [184.0, if ready { 133.0 } else { 150.0 }, 33.0, 17.0];
        if !(java && host.sprite(event.batch, AUG_TEXTURE, src, [ox + x - 2.0, oy + y - 2.0, 33.0, 17.0])) {
            host.fill(
                event.batch,
                [ox + x, oy + y, AUG_BUTTON_SIZE[0], AUG_BUTTON_SIZE[1]],
                if ready { 0xFF6A_6070 } else { 0xFF1A_181D },
            );
            host.text(
                event.batch,
                label,
                [ox + x + 9.0, oy + y + 3.0],
                if ready { 0xFFFFFF } else { 0x606060 },
                true,
            );
        }
    }
}

fn augmenting_click(event: &ScreenClickEvent<'_>) -> Option<i32> {
    if event.right || get(event.data, 0) & AUG_VALID == 0 {
        return None;
    }
    let flags = get(event.data, 0);
    if flags & AUG_PREV != 0 && inside(event.at, AUG_PREV_BUTTON, AUG_ARROW_SIZE) {
        return Some(6);
    }
    if flags & AUG_NEXT != 0 && inside(event.at, AUG_NEXT_BUTTON, AUG_ARROW_SIZE) {
        return Some(7);
    }
    if flags & AUG_UPGRADE != 0 && inside(event.at, AUG_UPGRADE_BUTTON, AUG_BUTTON_SIZE) {
        return Some(8);
    }
    if flags & AUG_REROLL != 0 && inside(event.at, AUG_REROLL_BUTTON, AUG_BUTTON_SIZE) {
        return Some(9);
    }
    let first = get(event.data, 3).max(0) as usize;
    let count = get(event.data, 1).max(0) as usize;
    match aug_row_at(event.at) {
        Some(row) if first + row < count => Some(row as i32),
        _ => None,
    }
}

/// What a button says: Java's inactive reasons, else its name and both costs.
fn augment_lines(data: &[i32], reroll: bool) -> Vec<Line> {
    if get(data, 0) & AUG_VALID == 0 {
        return vec![line("button.apotheosis.augmenting.no_selection", "No Affix Selected", "§c", vec![])];
    }
    let flag = if reroll { AUG_REROLL } else { AUG_UPGRADE };
    if get(data, 0) & flag == 0 {
        let (key, fallback) = if reroll {
            ("button.apotheosis.augmenting.no_alternatives", "No Alternative Affixes")
        } else {
            ("button.apotheosis.augmenting.max_level", "Already Max Level")
        };
        return vec![line(key, fallback, "§c", vec![])];
    }
    let (key, fallback, sigils, levels) = if reroll {
        (
            "button.apotheosis.augmenting.reroll",
            "Reroll Selected Affix",
            get(data, 6),
            get(data, 7),
        )
    } else {
        (
            "button.apotheosis.augmenting.upgrade",
            "Upgrade Selected Affix",
            get(data, 4),
            get(data, 5),
        )
    };
    vec![
        line(key, fallback, "§e§n", vec![]),
        line(
            "button.apotheosis.augmenting.upgrade.cost",
            "Cost: %sx %s",
            "§7",
            vec![Arg::Num(sigils), Arg::Item(AUG_SIGIL)],
        ),
        line(
            "button.apotheosis.augmenting.upgrade.exp_cost",
            "Experience Cost: %s Levels",
            "§7",
            vec![Arg::Num(levels)],
        ),
    ]
}

fn augmenting_tooltip(event: &mut ScreenTooltipEvent<'_>) {
    let reroll = inside(event.at, AUG_REROLL_BUTTON, AUG_BUTTON_SIZE);
    let upgrade = inside(event.at, AUG_UPGRADE_BUTTON, AUG_BUTTON_SIZE);
    if !reroll && !upgrade {
        return;
    }
    push_lines(event.lines, &augment_lines(event.data, reroll));
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

    // --- Augmenting table -----------------------------------------------------

    fn affix(path: &str) -> &'static Affix {
        AFFIXES
            .iter()
            .find(|affix| affix.path == path)
            .unwrap_or_else(|| panic!("missing Java affix {path}"))
    }

    /// A piece carrying exactly `levels`, by affix path, at `tier`.
    fn affixed(item: &str, tier: Tier, levels: &[(&str, f64)]) -> ItemStack {
        let mut stack = ItemStack::new(item, 1);
        sockets::after_affixes(&mut stack, tier, &mut Rng::new(11));
        let instances: Vec<Instance> = levels
            .iter()
            .map(|(path, level)| {
                let affix = affix(path);
                Instance {
                    affix,
                    level: Some(*level),
                    value: affixes::value_at(affix, tier, *level).unwrap(),
                }
            })
            .collect();
        affixes::rewrite(&mut stack, tier, &instances);
        stack
    }

    fn level_on(stack: &ItemStack, path: &str) -> f64 {
        affixes_of(stack)
            .iter()
            .find(|instance| instance.affix.path == path)
            .map(|instance| instance.level_at(sockets::tier_of(stack).unwrap()))
            .unwrap_or_else(|| panic!("no affix {path}"))
    }

    #[test]
    fn an_upgrade_is_a_quarter_level_capped_at_one() {
        assert_eq!(upgrade(0.0), Some(0.25));
        assert_eq!(upgrade(0.5), Some(0.75));
        assert_eq!(upgrade(0.9), Some(1.0));
        assert_eq!(upgrade(1.0), None);
        assert_eq!(upgrade(1.2), None, "an overcharged affix cannot be upgraded");
    }

    #[test]
    fn a_level_independent_affix_is_refused() {
        // elastic's common range is 0.5..0.5, so its level changes nothing.
        let boots = affixed("minecraft:diamond_boots", Tier::Common, &[("armor/attribute/elastic", 0.0)]);
        let instance = &affixes_of(&boots)[0];
        assert!(!instance.level_dependent(Tier::Common));
        assert!(!can_upgrade(instance, Tier::Common));
        assert_eq!(upgrade_plan(&boots, 0, 2, 225, 0.0, false), None);
        assert_eq!(upgrade_plan(&boots, 0, 2, 225, 0.0, true), None);
    }

    #[test]
    fn a_piece_already_at_or_above_the_cap_cannot_be_upgraded() {
        let sword = affixed("minecraft:diamond_sword", Tier::Rare, &[("melee/attribute/berserking", 1.0)]);
        assert_eq!(upgrade_plan(&sword, 0, 2, 225, 0.0, false), None);
        let overcharged = affixed("minecraft:diamond_sword", Tier::Rare, &[("melee/attribute/berserking", 1.2)]);
        assert_eq!(upgrade_plan(&overcharged, 0, 2, 225, 0.0, false), None);
    }

    #[test]
    fn an_upgrade_raises_the_selected_affix_and_keeps_the_rest() {
        let mut sword = affixed(
            "minecraft:diamond_sword",
            Tier::Rare,
            &[("melee/attribute/berserking", 0.5), ("melee/attribute/forceful", 0.25)],
        );
        sword.data_mut().enchantments = vec![(std::sync::Arc::from("minecraft:sharpness"), 3)];
        let (selected, next, sigils, levels) = upgrade_plan(&sword, 0, 2, 225, 0.0, false).unwrap();
        assert_eq!((sigils, levels), (2, 225));
        let upgraded = upgraded(&sword, selected, next).unwrap();
        assert_eq!(level_on(&upgraded, "melee/attribute/berserking"), 0.75);
        assert_eq!(level_on(&upgraded, "melee/attribute/forceful"), 0.25, "other affixes untouched");
        assert_eq!(upgraded.enchantments().len(), 1, "enchantments are kept");
        assert_eq!(upgraded.enchantments()[0].1, 3);
        assert_eq!(&*upgraded.item, "minecraft:diamond_sword");
        assert_eq!(affixes_of(&upgraded).len(), 2);
        assert_eq!(sockets::tier_of(&upgraded), Some(Tier::Rare));
    }

    #[test]
    fn a_reroll_only_offers_compatible_affixes_of_the_same_type() {
        let pool: Vec<&'static Affix> = vec![
            affix("melee/attribute/berserking"),
            affix("melee/attribute/vampiric"),
            affix("melee/attribute/forceful"),
            affix("melee/attribute/graceful"),
            affix("melee/mob_effect/weakening"),
        ];
        // Rerolling forceful while vampiric stays: berserking and vampiric are
        // exclusive, and the effect affix is of another type.
        let candidates = reroll_candidates(
            &pool,
            affix("melee/attribute/forceful"),
            &[affix("melee/attribute/vampiric")],
            Tier::Rare,
        );
        let ids: Vec<&str> = candidates.iter().map(|affix| affix.id).collect();
        assert_eq!(ids, ["apotheosis:melee/attribute/graceful"], "only the compatible stat affix");
        // The chosen affix is removed first, so its own exclusive partner may come back.
        let again = reroll_candidates(&pool, affix("melee/attribute/vampiric"), &[], Tier::Rare);
        assert!(again.iter().any(|affix| affix.id == "apotheosis:melee/attribute/berserking"));
    }

    #[test]
    fn a_reroll_with_no_candidate_does_nothing() {
        let pool = vec![affix("melee/attribute/berserking")];
        // Only the chosen affix is in the pool, and it is never returned.
        assert!(reroll_candidates(&pool, affix("melee/attribute/berserking"), &[], Tier::Rare).is_empty());
        assert_eq!(choose_weighted(&[], &[], &mut Rng::new(1)), None);
        let empty: Vec<&'static Affix> = Vec::new();
        assert_eq!(choose_weighted(&empty, &[], &mut Rng::new(1)), None);
    }

    #[test]
    fn zero_weights_fall_back_to_a_uniform_draw() {
        let pool: Vec<&'static Affix> = vec![
            affix("melee/attribute/berserking"),
            affix("melee/attribute/forceful"),
            affix("melee/attribute/graceful"),
        ];
        let mut seen = [false; 3];
        for seed in 0..200 {
            let index = choose_weighted(&pool, &[0, 0, 0], &mut Rng::new(seed)).unwrap();
            seen[index] = true;
        }
        assert_eq!(seen, [true, true, true], "every candidate can be drawn");
        for seed in 0..50 {
            assert_eq!(choose_weighted(&pool, &[0, 7, 0], &mut Rng::new(seed)), Some(1));
        }
        // A weight at the player's tier scales with luck, as Java's does.
        let forceful = affix("melee/attribute/forceful");
        assert_eq!(weight_at(forceful, WorldTier::Haven, 0.0), 25);
        assert!(weight_at(forceful, WorldTier::Haven, 100.0) > 25);
        // A Pinnacle-only affix weighs nothing below Pinnacle.
        let giant_slaying = affix("melee/attribute/giant_slaying");
        assert_eq!(weight_at(giant_slaying, WorldTier::Haven, 0.0), 0);
        assert_eq!(weight_at(giant_slaying, WorldTier::Pinnacle, 0.0), 15);
    }

    #[test]
    fn a_reroll_replaces_the_selected_affix_and_keeps_the_piece() {
        let sword = affixed(
            "minecraft:diamond_sword",
            Tier::Rare,
            &[("melee/attribute/berserking", 0.5), ("melee/attribute/forceful", 0.25)],
        );
        let replacement = affix("melee/attribute/graceful");
        let piece = rerolled(&sword, 0, replacement, 0.4).unwrap();
        let affixes = affixes_of(&piece);
        assert_eq!(affixes.len(), 2);
        assert!(affixes.iter().any(|instance| instance.affix.id == replacement.id));
        assert!(!affixes.iter().any(|instance| instance.affix.path == "melee/attribute/berserking"));
        assert_eq!(level_on(&piece, "melee/attribute/forceful"), 0.25);
        let sorted: Vec<&str> = affixes.iter().map(|instance| instance.affix.id).collect();
        let mut ordered = sorted.clone();
        ordered.sort();
        assert_eq!(sorted, ordered, "affixes stay sorted by id");
    }

    #[test]
    fn creative_pays_nothing_and_short_funds_pay_nothing() {
        assert_eq!(charge(2, 225, 0, 0, 0.0, true), Some((0, 0)));
        assert_eq!(charge(2, 225, 2, 225, 0.0, false), Some((2, 225)));
        assert_eq!(charge(2, 225, 1, 225, 0.0, false), None, "not enough sigils");
        assert_eq!(charge(2, 225, 2, 224, 0.0, false), None, "level below the cost");
        let sword = affixed("minecraft:diamond_sword", Tier::Rare, &[("melee/attribute/berserking", 0.0)]);
        assert_eq!(upgrade_plan(&sword, 0, 1, 225, 0.0, false), None);
        assert_eq!(upgrade_plan(&sword, 0, 2, 224, 0.0, false), None);
        assert_eq!(upgrade_plan(&sword, 0, 0, 0, 0.0, true), Some((0, 0.25, 0, 0)));
    }

    #[test]
    fn the_level_charge_mirrors_java_charging_total_experience() {
        assert_eq!(levels_charged(0, 0.0, 0), 0);
        assert_eq!(levels_charged(10, 0.0, 0), 0);
        // A player on exactly the required level loses every level.
        assert_eq!(levels_charged(225, 0.0, 225), 225);
        // A richer player keeps the levels the remaining points reach.
        let kept = 300 - levels_charged(300, 0.0, 225);
        assert!(kept > 75 && kept < 300, "left on level {kept}");
        assert_eq!(total_xp(0), 0);
        assert_eq!(total_xp(16), 352);
        assert_eq!(total_xp(30), 1395);
    }

    #[test]
    fn the_data_slots_list_six_affixes_and_their_costs() {
        let sword = affixed("minecraft:diamond_sword", Tier::Rare, &[("melee/attribute/berserking", 0.5)]);
        let items = [Some(sword), stack("apotheosis:sigil_of_enhancement", 4)];
        let data = augmenting_data(&items, 0);
        assert_eq!(data[0] & AUG_VALID, AUG_VALID);
        assert_eq!(data[1], 1);
        assert_eq!(data[2], 0);
        assert_eq!(data[3], 0);
        assert_eq!(&data[4..8], &[2, 225, 1, 175]);
        let index = AFFIXES.iter().position(|affix| affix.path == "melee/attribute/berserking").unwrap();
        assert_eq!(data[8], index as i32 + 1);
        assert_eq!(data[9], 50, "half a level in hundredths");
        assert_eq!(augmenting_data(&[None, None], 0)[0], 0);
    }

    #[test]
    fn the_augmenting_layout_and_rules_follow_java() {
        assert_eq!(menu_of("apotheosis:augmenting_table"), Some(AUGMENTING));
        let piece = affixed("minecraft:diamond_sword", Tier::Rare, &[("melee/attribute/forceful", 0.1)]);
        assert_eq!(augmenting_slot_rule(0, &piece), Some((true, Some(1))));
        assert_eq!(augmenting_slot_rule(0, &ItemStack::new("minecraft:stone", 1)), Some((false, Some(1))));
        assert_eq!(
            augmenting_slot_rule(1, &ItemStack::new("apotheosis:sigil_of_enhancement", 1)),
            Some((true, None))
        );
        assert_eq!(augmenting_slot_rule(1, &piece), Some((false, None)));
        assert_eq!(augmenting_slot_rule(2, &piece), None);
        assert_eq!(aug_row_at([60.0, 40.0]), Some(0));
        assert_eq!(aug_row_at([60.0, 50.0]), Some(1));
        assert_eq!(aug_row_at([60.0, 110.0]), None);
        assert!(inside([65.0, 115.0], AUG_UPGRADE_BUTTON, AUG_BUTTON_SIZE));
        assert!(!inside([55.0, 115.0], AUG_UPGRADE_BUTTON, AUG_BUTTON_SIZE));
    }
}
