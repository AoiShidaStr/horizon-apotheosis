use horizon_client::mod_api::{
    BlockEntityEvent, BlockEntityTickEvent, BlockEntityTicksEvent, Compound, EquipmentSlot, Flow,
    Host, InventoryTickEvent, ItemStack, PrepareItemEnchantEvent, RightClickBlockEvent, Tag, Text,
    TooltipEvent, UseItemEvent, UseItemOnEvent,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

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
    // A filtering shelf gives 0.5 Eterna per enchanted book in its six slots
    // (max 30) and 1 Arcana per book, and filters those books' enchantments
    // out of the offers. API 415 has no block-entity inventory hook, so its
    // books cannot be read: the shelf is recognised with the stats an empty
    // one has in Java (zero), and its filtering waits for that hook.
    ("filtering_shelf", ShelfStats { ..ShelfStats::DEFAULT }),
    ("geode_shelf", ShelfStats { eterna: 5.0, max_eterna: 60.0, quanta: -15.0, ..ShelfStats::DEFAULT }),
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
    ("treasure_shelf", ShelfStats { eterna: 5.0, max_eterna: 70.0, quanta: -15.0, arcana: -15.0, ..ShelfStats::DEFAULT }),
    ("wither_skull", ShelfStats { quanta: 10.0, ..ShelfStats::DEFAULT }),
];

// Vanilla checks two blocks out at the 32 perimeter positions, reading the
// outer block only when the intervening position is air.
const OFFSETS: &[(i32, i32)] = &[
    (-2, -1), (-2, 0), (-2, 1), (-1, -2), (-1, 2),
    (0, -2), (0, 2), (1, -2), (1, 2), (2, -1), (2, 0), (2, 1),
];

/// The stats of the shelf block id `name` (`apotheosis:deepshelf`,
/// `minecraft:amethyst_cluster`): `Host::block` gives a qualified id, the
/// table above keeps the bare ones.
fn shelf_stats(name: &str) -> Option<ShelfStats> {
    let id = name.rsplit(':').next().unwrap_or(name);
    SHELVES.iter().find(|(shelf, _)| *shelf == id).map(|(_, stats)| *stats)
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

// ---------------------------------------------------------------------------
// Shelf tooltips
// ---------------------------------------------------------------------------
// Java's `TooltipUtil.appendBlockStats`: a block item that counts as a shelf
// lists what it gives an enchanting table, in Java's colours.

const STATS_GOLD: u32 = 0xFF_AA00;
const ETERNA_GREEN: u32 = 0x55_FF55;
const QUANTA_RED: u32 = 0xFF_5555;
const ARCANA_PURPLE: u32 = 0xAA_00AA;
const CLUES_AQUA: u32 = 0x00_AAAA;

/// One tooltip line of a shelf: its lang key, English fallback, the `%s`
/// arguments and its 0xRRGGBB colour.
#[derive(Debug, PartialEq)]
struct StatLine {
    key: &'static str,
    fallback: &'static str,
    args: Vec<String>,
    colour: u32,
}

/// The stats an item places as a shelf, with Java's two special flags
/// (allows treasure, provides stability). Vanilla blocks count as Java counts
/// them: a bookshelf through its enchanting power (1 power = 2 Eterna, max
/// 30), the skulls through Java's `basic_skulls` tag and `wither_skull` file.
fn item_shelf(item: &str) -> Option<(ShelfStats, bool, bool)> {
    let (namespace, name) = item.split_once(':')?;
    let shelf = match (namespace, name) {
        ("minecraft", "bookshelf") => "minecraft_bookshelf",
        ("minecraft", "skeleton_skull" | "creeper_head" | "zombie_head" | "piglin_head") => {
            "basic_skulls"
        }
        ("minecraft", "wither_skeleton_skull") => "wither_skull",
        ("minecraft", "amethyst_cluster") => "amethyst_cluster",
        ("apotheosis", "basic_skulls" | "wither_skull" | "amethyst_cluster") => return None,
        ("apotheosis", name) => name,
        _ => return None,
    };
    let stats = if shelf == "minecraft_bookshelf" {
        ShelfStats { eterna: 2.0, ..ShelfStats::DEFAULT }
    } else {
        shelf_stats(shelf)?
    };
    Some((stats, shelf == "treasure_shelf", shelf == "geode_shelf"))
}

/// Java's `String.format("%.2f")`, with a dot whatever the language.
fn two_places(value: f32) -> String {
    format!("{value:.2}")
}

/// The lines a shelf item's tooltip gains, as Java writes them: a header,
/// then Eterna (with its cap when positive), Quanta, Arcana and clues when
/// non-zero, then the treasure and stability flags.
fn shelf_tooltip_lines(item: &str) -> Vec<StatLine> {
    let Some((stats, treasure, stable)) = item_shelf(item) else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    let mut line = |key, fallback, args: Vec<String>, colour| {
        lines.push(StatLine { key, fallback, args, colour });
    };
    let any = stats.eterna != 0.0
        || stats.quanta != 0.0
        || stats.arcana != 0.0
        || stats.clues != 0
        || treasure
        || stable;
    if any {
        line("info.apothic_enchanting.ench_stats", "Enchanting stats:", vec![], STATS_GOLD);
    }
    if stats.eterna > 0.0 {
        line(
            "info.apothic_enchanting.eterna.p",
            " - Eterna: +%s (max %s)",
            vec![two_places(stats.eterna), two_places(stats.max_eterna)],
            ETERNA_GREEN,
        );
    } else if stats.eterna < 0.0 {
        line("info.apothic_enchanting.eterna", " - Eterna: %s", vec![two_places(stats.eterna)], ETERNA_GREEN);
    }
    if stats.quanta > 0.0 {
        line("info.apothic_enchanting.quanta.p", " - Quanta: +%s%%", vec![two_places(stats.quanta)], QUANTA_RED);
    } else if stats.quanta < 0.0 {
        line("info.apothic_enchanting.quanta", " - Quanta: %s%%", vec![two_places(stats.quanta)], QUANTA_RED);
    }
    if stats.arcana > 0.0 {
        line("info.apothic_enchanting.arcana.p", " - Arcana: +%s%%", vec![two_places(stats.arcana)], ARCANA_PURPLE);
    } else if stats.arcana < 0.0 {
        line("info.apothic_enchanting.arcana", " - Arcana: %s%%", vec![two_places(stats.arcana)], ARCANA_PURPLE);
    }
    if stats.clues > 0 {
        line("info.apothic_enchanting.clues.p", " - Enchanting clues: +%s", vec![stats.clues.to_string()], CLUES_AQUA);
    } else if stats.clues < 0 {
        line("info.apothic_enchanting.clues", " - Enchanting clues: %s", vec![stats.clues.to_string()], CLUES_AQUA);
    }
    if treasure {
        line("info.apothic_enchanting.allows_treasure", " - Allows treasure enchantments", vec![], STATS_GOLD);
    }
    if stable {
        line("info.apothic_enchanting.provides_stability", " - Provides quantic stability", vec![], STATS_GOLD);
    }
    lines
}

/// An item tooltip, on the client: a shelf block item lists its stats.
pub(super) fn tooltip(event: &mut TooltipEvent<'_>) {
    for line in shelf_tooltip_lines(event.stack.item.as_ref()) {
        let args: Vec<Text> = line.args.into_iter().map(Text::literal).collect();
        let text = Text::translate_with_fallback(line.key, line.fallback, &args).with_color(line.colour);
        event.lines.push((text, crate::tip_colour(line.colour)));
    }
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

/// The off hand's slot in `native_inventory`'s `Player` numbering.
const OFFHAND: i32 = -106;

/// A refusal or a result, shown where a player is sure to see it: over the
/// hotbar (`spawners.rs` does the same) and as a chat line.
fn say(host: &mut dyn Host, player: u128, line: Text) {
    let _ = host.action_bar(player, line.clone());
    let _ = host.tell(player, line);
}

/// What a click asked for, applied on the next inventory tick.
#[derive(Clone, Debug, PartialEq)]
enum Job {
    Tome(Tome),
    Infuse { pos: [i32; 3] },
    Library { dimension: String, pos: [i32; 3], max_level: u32 },
}

/// A click waiting for the next tick: the slot it came from and the stacks it
/// saw. The tick applies it only if both hands are still exactly those.
#[derive(Clone, Debug, PartialEq)]
struct Pending {
    job: Job,
    slot: i32,
    main: ItemStack,
    off: Option<ItemStack>,
}

fn pending() -> &'static Mutex<HashMap<u128, Pending>> {
    static PENDING: OnceLock<Mutex<HashMap<u128, Pending>>> = OnceLock::new();
    PENDING.get_or_init(|| Mutex::new(HashMap::new()))
}

fn set_pending(player: u128, action: Pending) {
    if let Ok(mut pending) = pending().lock() {
        pending.insert(player, action);
    }
}

/// Takes the pending click only when the tick is the slot it started from.
fn take_pending(player: u128, slot: i32) -> Option<Pending> {
    let mut pending = pending().lock().ok()?;
    if pending.get(&player).is_some_and(|action| action.slot == slot) {
        pending.remove(&player)
    } else {
        None
    }
}

/// An empty slot reads as nothing.
fn present(stack: Option<ItemStack>) -> Option<ItemStack> {
    stack.filter(|stack| stack.count > 0)
}

/// Writes a stack into a carried slot; a count of 0 empties it.
fn write_slot(host: &mut dyn Host, slot: i32, stack: ItemStack) -> bool {
    match host.carried_stack(slot) {
        Some(current) => {
            *current = stack;
            true
        }
        None => false,
    }
}

/// One item fewer in the stack (an empty stack when it was the last).
fn one_fewer(stack: &ItemStack) -> ItemStack {
    let mut left = stack.clone();
    left.count = left.count.saturating_sub(1);
    left
}

/// The player a queued click belongs to, as the tick sees them.
#[derive(Clone, Copy, Debug)]
struct Who {
    uuid: u128,
    xp_level: i32,
    creative: bool,
}

impl Who {
    fn can_pay(&self, cost: i32) -> bool {
        self.creative || self.xp_level >= cost
    }
}

/// A tome in the main hand, the item it acts on in the off hand: queued here,
/// applied on the next inventory tick like the sockets' sigils.
pub(super) fn on_use_item(host: &mut dyn Host, event: &mut UseItemEvent<'_>) -> Flow {
    let Some(tome) = tome_kind(&event.stack.item) else {
        return Flow::Continue;
    };
    let Some(player) = host.acting_player() else {
        return Flow::Continue;
    };
    let Some(main) = host.player_equipment(player.uuid, EquipmentSlot::MainHand) else {
        return Flow::Continue;
    };
    if main.item != event.stack.item {
        return Flow::Continue;
    }
    let off = host.player_equipment(player.uuid, EquipmentSlot::OffHand);
    set_pending(
        player.uuid,
        Pending {
            job: Job::Tome(tome),
            slot: i32::from(player.selected_slot),
            main,
            off,
        },
    );
    Flow::Cancel
}

/// Applies a queued click on the stacks it was validated on. `inventory_tick`
/// runs for every carried stack, so only the selected slot works, and only when
/// it is the slot the click came from.
pub(super) fn inventory_tick(host: &mut dyn Host, event: &mut InventoryTickEvent) {
    if !event.selected {
        return;
    }
    let Some(player) = host.acting_player() else {
        return;
    };
    let Some(pending) = take_pending(player.uuid, event.slot) else {
        return;
    };
    let Some(main) = present(host.carried_stack(pending.slot).cloned()) else {
        return;
    };
    let off = present(host.carried_stack(OFFHAND).cloned());
    if main != pending.main || off != pending.off {
        // The hand changed since the click: nothing is spent or destroyed.
        return;
    }
    let who = Who {
        uuid: player.uuid,
        xp_level: player.xp_level,
        creative: player.game_mode == 1,
    };
    match pending.job {
        Job::Tome(tome) => apply_tome(host, who, tome, pending.slot, &main, off),
        Job::Infuse { pos } => apply_infusion(host, who, pending.slot, pos, &main),
        Job::Library { dimension, pos, max_level } => {
            apply_library(host, who, &dimension, pos, max_level, pending.slot, &main)
        }
    }
}

fn apply_tome(
    host: &mut dyn Host,
    who: Who,
    tome: Tome,
    slot: i32,
    tome_stack: &ItemStack,
    target: Option<ItemStack>,
) {
    let no_target = || {
        Text::translate_with_fallback(
            "apotheosis.tome.no_target",
            "Offhand must hold an enchanted item or a cursed item.",
            &[],
        )
    };
    let Some(target) = target else {
        say(host, who.uuid, no_target());
        return;
    };
    if tome_kind(&target.item).is_some() {
        say(host, who.uuid, no_target());
        return;
    }

    let (kept, removed) = if tome == Tome::Web {
        strip_curses(target.crafting_enchantments())
    } else {
        (target.crafting_enchantments().to_vec(), 0)
    };
    if tome == Tome::Web && removed == 0 {
        say(
            host,
            who.uuid,
            Text::translate_with_fallback("apotheosis.tome.no_curse", "That item carries no curse.", &[]),
        );
        return;
    }
    if tome != Tome::Web && kept.is_empty() {
        say(
            host,
            who.uuid,
            Text::translate_with_fallback(
                "apotheosis.tome.not_enchanted",
                "That item has no enchantments.",
                &[],
            ),
        );
        return;
    }

    let kept = if tome == Tome::Scrap {
        scrap_selection(&kept, dice(who.uuid))
    } else {
        kept
    };
    let cost = tome_cost(tome, kept.len());
    if !who.can_pay(cost) {
        say(
            host,
            who.uuid,
            Text::translate_with_fallback(
                "apotheosis.tome.levels",
                "You need %s experience levels.",
                &[Text::literal(cost.to_string())],
            ),
        );
        return;
    }

    let replacement = output_stack(tome, &target, kept.clone());
    let no_room = Text::translate_with_fallback(
        "apotheosis.tome.no_room",
        "No room for the extracted book; nothing was changed.",
        &[],
    );
    // A book is handed out before anything is taken: a full inventory leaves
    // both hands untouched, and nothing is ever given without being taken.
    if tome == Tome::Web {
        if !write_slot(host, OFFHAND, replacement) {
            return;
        }
    } else {
        if host.give_stack(who.uuid, replacement).is_some() {
            say(host, who.uuid, no_room);
            return;
        }
        let rest = if tome == Tome::Extraction {
            let mut stripped = target.clone();
            stripped.set_crafting_enchantments(Vec::new());
            stripped
        } else {
            // Scrapping consumes the item.
            let mut gone = target.clone();
            gone.count = 0;
            gone
        };
        write_slot(host, OFFHAND, rest);
    }
    write_slot(host, slot, one_fewer(tome_stack));

    if !who.creative {
        let _ = host.give_levels(who.uuid, -cost);
    }
    let message = match tome {
        Tome::Scrap | Tome::Improved => ("apotheosis.tome.scrapped", "Scrapped %s enchantments."),
        Tome::Extraction => ("apotheosis.tome.extracted", "Extracted %s enchantments."),
        Tome::Web => ("apotheosis.tome.cleansed", "Curses removed."),
    };
    let count = Text::literal(kept.len().to_string());
    let args: &[Text] = if tome == Tome::Web {
        &[]
    } else {
        std::slice::from_ref(&count)
    };
    say(host, who.uuid, Text::translate_with_fallback(message.0, message.1, args));
}

// ---------------------------------------------------------------------------
// Infusion
// ---------------------------------------------------------------------------
//
// Apothic Enchanting's infusion lives in its enchanting table's menu, which
// API 415 cannot reach. The mod's own table block does it on a right click
// instead: the item in hand is infused when the shelves around the table meet
// the recipe's stats, and the click charges the levels Java charges for it.

const INFUSION_TABLE: &str = "apotheosis:apothic_enchanting_table";

#[derive(Clone, Copy, Debug, PartialEq)]
struct StackStats {
    eterna: f32,
    quanta: f32,
    arcana: f32,
}

/// One `recipe/infusion` entry: the item that goes in, the stats the table must
/// reach (`min`) and the most it may reach (`max`, negative for no bound).
#[derive(Clone, Copy, Debug)]
struct InfusionRecipe {
    input: &'static str,
    output: &'static str,
    count: u32,
    min: StackStats,
    max: StackStats,
}

const UNBOUNDED: StackStats = StackStats {
    eterna: -1.0,
    quanta: -1.0,
    arcana: -1.0,
};

// The updatable infusions whose input and result both exist in this port, with
// the requirements of their Java data files. Recipes needing an item the port
// does not carry (infused breath, ender leads, the inert trident, the music
// discs) are left out: nothing in hand could ever match them.
const INFUSIONS: &[InfusionRecipe] = &[
    InfusionRecipe {
        input: "minecraft:amethyst_block",
        output: "minecraft:budding_amethyst",
        count: 1,
        min: StackStats { eterna: 60.0, quanta: 30.0, arcana: 50.0 },
        max: StackStats { quanta: 50.0, ..UNBOUNDED },
    },
    InfusionRecipe {
        input: "minecraft:carrot",
        output: "minecraft:golden_carrot",
        count: 1,
        min: StackStats { eterna: 20.0, quanta: 10.0, arcana: 0.0 },
        max: StackStats { eterna: 20.0, quanta: 30.0, arcana: -1.0 },
    },
    InfusionRecipe {
        input: "apotheosis:dormant_deepshelf",
        output: "apotheosis:deepshelf",
        count: 1,
        min: StackStats { eterna: 60.0, quanta: 40.0, arcana: 40.0 },
        max: UNBOUNDED,
    },
    InfusionRecipe {
        input: "minecraft:echo_shard",
        output: "minecraft:echo_shard",
        count: 4,
        min: StackStats { eterna: 70.0, quanta: 50.0, arcana: 50.0 },
        max: UNBOUNDED,
    },
    InfusionRecipe {
        input: "apotheosis:enchantment_library",
        output: "apotheosis:ender_library",
        count: 1,
        min: StackStats { eterna: 100.0, quanta: 45.0, arcana: 100.0 },
        max: StackStats { eterna: 100.0, quanta: 50.0, arcana: 100.0 },
    },
    InfusionRecipe {
        input: "apotheosis:hellshelf",
        output: "apotheosis:infused_hellshelf",
        count: 1,
        min: StackStats { eterna: 45.0, quanta: 30.0, arcana: 0.0 },
        max: UNBOUNDED,
    },
    InfusionRecipe {
        input: "minecraft:honey_bottle",
        output: "minecraft:experience_bottle",
        count: 1,
        min: StackStats { eterna: 20.0, quanta: 25.0, arcana: 25.0 },
        max: UNBOUNDED,
    },
    InfusionRecipe {
        input: "minecraft:honey_bottle",
        output: "minecraft:experience_bottle",
        count: 8,
        min: StackStats { eterna: 60.0, quanta: 25.0, arcana: 25.0 },
        max: UNBOUNDED,
    },
    InfusionRecipe {
        input: "minecraft:honey_bottle",
        output: "minecraft:experience_bottle",
        count: 32,
        min: StackStats { eterna: 100.0, quanta: 25.0, arcana: 25.0 },
        max: UNBOUNDED,
    },
    InfusionRecipe {
        input: "apotheosis:improved_scrap_tome",
        output: "apotheosis:extraction_tome",
        count: 4,
        min: StackStats { eterna: 60.0, quanta: 25.0, arcana: 45.0 },
        max: StackStats { quanta: 75.0, ..UNBOUNDED },
    },
    InfusionRecipe {
        input: "apotheosis:scrap_tome",
        output: "apotheosis:improved_scrap_tome",
        count: 4,
        min: StackStats { eterna: 45.0, quanta: 25.0, arcana: 35.0 },
        max: StackStats { quanta: 50.0, ..UNBOUNDED },
    },
    InfusionRecipe {
        input: "apotheosis:seashelf",
        output: "apotheosis:infused_seashelf",
        count: 1,
        min: StackStats { eterna: 45.0, quanta: 15.0, arcana: 10.0 },
        max: UNBOUNDED,
    },
];

/// Floating point sums of shelf stats never land exactly on a requirement.
const STAT_EPSILON: f32 = 1.0e-4;

fn within(value: f32, min: f32, max: f32) -> bool {
    value + STAT_EPSILON >= min && (max < 0.0 || value <= max + STAT_EPSILON)
}

impl InfusionRecipe {
    fn accepts(&self, stats: StackStats) -> bool {
        within(stats.eterna, self.min.eterna, self.max.eterna)
            && within(stats.quanta, self.min.quanta, self.max.quanta)
            && within(stats.arcana, self.min.arcana, self.max.arcana)
    }
}

/// The easiest infusion `item` can go through, for the message that tells a
/// player what the table would need.
fn easiest_infusion(item: &str) -> Option<&'static InfusionRecipe> {
    INFUSIONS
        .iter()
        .filter(|recipe| recipe.input == item)
        .min_by(|a, b| a.min.eterna.total_cmp(&b.min.eterna))
}

/// The infusion of `item` the table's stats allow, the hardest one when
/// several fit: `InfusionRecipe.findMatch` sorts by required Eterna first.
fn find_infusion(item: &str, stats: StackStats) -> Option<&'static InfusionRecipe> {
    INFUSIONS
        .iter()
        .filter(|recipe| recipe.input == item && recipe.accepts(stats))
        .max_by(|a, b| a.min.eterna.total_cmp(&b.min.eterna))
}

/// The levels the click charges: the table's cost for the slot the infusion
/// fills, clamped to the recipe's own Eterna requirement as Java does.
fn infusion_cost(recipe: &InfusionRecipe, eterna: f32) -> i32 {
    let table = (eterna.round() as i32).max(3);
    let required = (recipe.min.eterna.ceil() as i32).max(3);
    table.min(required)
}

fn item_name(item: &str) -> String {
    format!("item.{}", item.replace(':', "."))
}

fn stat(text: f32) -> Text {
    Text::literal(text.to_string())
}

/// An item used on the apothic enchanting table: the shelves two blocks out
/// decide which infusion it can undergo, and the click applies it at once.
pub(super) fn on_use_item_on(host: &mut dyn Host, event: &mut UseItemOnEvent<'_>) -> Flow {
    if host.block(event.pos) != INFUSION_TABLE {
        return Flow::Continue;
    }
    let Some(player) = host.acting_player() else {
        return Flow::Continue;
    };
    let Some(main) = host.player_equipment(player.uuid, EquipmentSlot::MainHand) else {
        return Flow::Continue;
    };
    if main.item != event.stack.item {
        return Flow::Continue;
    }
    set_pending(
        player.uuid,
        Pending {
            job: Job::Infuse { pos: event.pos },
            slot: i32::from(player.selected_slot),
            main,
            off: None,
        },
    );
    // The click belongs to the table, so vanilla never opens a block GUI.
    Flow::Cancel
}

fn apply_infusion(host: &mut dyn Host, who: Who, slot: i32, pos: [i32; 3], held: &ItemStack) {
    if host.block(pos) != INFUSION_TABLE {
        return;
    }
    let stats = read_table_stats(host, pos);
    let available = StackStats {
        eterna: stats.eterna,
        quanta: stats.quanta,
        arcana: stats.arcana,
    };
    let Some(recipe) = find_infusion(&held.item, available) else {
        say(
            host,
            who.uuid,
            match easiest_infusion(&held.item) {
                Some(recipe) => Text::translate_with_fallback(
                    "apotheosis.infusion.requirements",
                    "Needs Eterna %s, Quanta %s and Arcana %s; the shelves give %s, %s and %s.",
                    &[
                        stat(recipe.min.eterna),
                        stat(recipe.min.quanta),
                        stat(recipe.min.arcana),
                        stat(available.eterna),
                        stat(available.quanta),
                        stat(available.arcana),
                    ],
                ),
                None => Text::translate_with_fallback(
                    "apotheosis.infusion.no_match",
                    "Nothing in your hand can be infused here.",
                    &[],
                ),
            },
        );
        return;
    };
    if held.count != 1 {
        say(
            host,
            who.uuid,
            Text::translate_with_fallback(
                "apotheosis.infusion.single",
                "The table infuses one item at a time.",
                &[],
            ),
        );
        return;
    }
    let cost = infusion_cost(recipe, available.eterna);
    if !who.can_pay(cost) {
        say(
            host,
            who.uuid,
            Text::translate_with_fallback(
                "apotheosis.infusion.levels",
                "You need %s experience levels.",
                &[Text::literal(cost.to_string())],
            ),
        );
        return;
    }
    // The hand's stack is the only copy: replacing it in place consumes the
    // input and hands out the result in one write, so nothing can duplicate.
    if !write_slot(host, slot, ItemStack::new(recipe.output, recipe.count)) {
        return;
    }
    if !who.creative {
        let _ = host.give_levels(who.uuid, -cost);
    }
    say(
        host,
        who.uuid,
        Text::translate_with_fallback(
            "apotheosis.infusion.done",
            "Infused %s.",
            &[Text::translate_with_fallback(
                &item_name(recipe.output),
                recipe.output,
                &[],
            )],
        ),
    );
}

// ---------------------------------------------------------------------------
// Enchantment Library
// ---------------------------------------------------------------------------
//
// API 415 has no menu hook that reaches the player hosting a world, and no
// vanilla-menu click hook, so the library is driven by right clicks and keeps
// its points in the saved compound of its block entity (API 15/17). No item
// ever sits in a slot the library shows: a click is applied at once on the
// server thread, which leaves nothing to duplicate with a menu, a hopper or a
// second player.

/// The library blocks and the ids their block entities save under.
pub(super) const LIBRARY: &str = "apotheosis:enchantment_library";
pub(super) const ENDER_LIBRARY: &str = "apotheosis:ender_library";
const LIBRARY_ENTITY: &str = "apotheosis:enchantment_library";
const ENDER_LIBRARY_ENTITY: &str = "apotheosis:ender_library";
/// The saved compound's key holding the stored points.
const LIBRARY_POINTS: &str = "points";
/// The highest level the ordinary library holds and hands out: one point a
/// level.
pub(super) const LIBRARY_MAX_LEVEL: u32 = 30;
/// Apothic Enchanting builds its Ender Library with max level 31, against the
/// ordinary library's 16; this port's ordinary library already stores a point
/// per level up to 30, so the Ender one is the same store one level higher.
pub(super) const ENDER_LIBRARY_MAX_LEVEL: u32 = 31;

/// The block entity id and the level cap of the library block `block`.
fn library_kind(block: &str) -> Option<(&'static str, u32)> {
    match block {
        LIBRARY => Some((LIBRARY_ENTITY, LIBRARY_MAX_LEVEL)),
        ENDER_LIBRARY => Some((ENDER_LIBRARY_ENTITY, ENDER_LIBRARY_MAX_LEVEL)),
        _ => None,
    }
}
/// Libraries whose points are mirrored in memory, and how many offers a player
/// may have selected.
const LIBRARY_CACHE: usize = 64;
const SELECTION_CACHE: usize = 256;

/// Stored points of one library, by enchantment. `dirty` marks a change the
/// block entity's next tick has to save; `max_level` is the cap its block
/// gives it (30 for the library, 31 for the Ender Library).
#[derive(Clone, Debug, PartialEq)]
struct Library {
    points: Vec<(Arc<str>, u32)>,
    max_level: u32,
    dirty: bool,
}

impl Default for Library {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            max_level: LIBRARY_MAX_LEVEL,
            dirty: false,
        }
    }
}

impl Library {
    /// Adds `levels` points to `enchantment`, never past the library's cap.
    fn absorb(&mut self, enchantment: &Arc<str>, levels: u32) {
        if levels == 0 {
            return;
        }
        let cap = self.max_level;
        let current = self
            .points
            .iter_mut()
            .find(|(id, _)| id == enchantment)
            .map(|(_, points)| points);
        match current {
            Some(points) => *points = (*points + levels).min(cap),
            None => {
                self.points.push((enchantment.clone(), levels.min(cap)));
                self.points.sort_by(|a, b| a.0.cmp(&b.0));
            }
        }
    }

    fn stored(&self, enchantment: &str) -> u32 {
        self.points
            .iter()
            .find(|(id, _)| id.as_ref() == enchantment)
            .map_or(0, |(_, points)| *points)
    }

    /// Takes `level` points out, when the library holds them.
    fn withdraw(&mut self, enchantment: &str, level: u32) -> bool {
        if level == 0 || self.stored(enchantment) < level {
            return false;
        }
        for (id, points) in &mut self.points {
            if id.as_ref() == enchantment {
                *points -= level;
            }
        }
        self.points.retain(|(_, points)| *points > 0);
        true
    }

    /// Every book the library can hand out, cheapest first, then by name.
    fn offers(&self) -> Vec<(Arc<str>, u32)> {
        let mut offers = Vec::new();
        for (id, points) in &self.points {
            for level in 1..=(*points).min(self.max_level) {
                offers.push((id.clone(), level));
            }
        }
        offers.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        offers
    }
}

fn libraries() -> &'static Mutex<Vec<((String, [i32; 3]), Library)>> {
    static LIBRARIES: Mutex<Vec<((String, [i32; 3]), Library)>> = Mutex::new(Vec::new());
    &LIBRARIES
}

fn selections() -> &'static Mutex<Vec<((u128, String, [i32; 3]), (Arc<str>, u32))>> {
    static SELECTIONS: Mutex<Vec<((u128, String, [i32; 3]), (Arc<str>, u32))>> = Mutex::new(Vec::new());
    &SELECTIONS
}

/// The mirror of the library at `pos`, empty when none is cached yet, with
/// the cap of the block standing there.
fn library_at(dimension: &str, pos: [i32; 3], max_level: u32) -> Library {
    let key = (dimension.to_owned(), pos);
    let mut library = libraries()
        .lock()
        .map(|all| {
            all.iter()
                .find(|(at, _)| *at == key)
                .map_or_else(Library::default, |(_, library)| library.clone())
        })
        .unwrap_or_default();
    library.max_level = max_level;
    library
}

fn store_library(dimension: &str, pos: [i32; 3], library: Library) {
    let key = (dimension.to_owned(), pos);
    let Ok(mut all) = libraries().lock() else {
        return;
    };
    if let Some((_, entry)) = all.iter_mut().find(|(at, _)| *at == key) {
        *entry = library;
    } else {
        if all.len() >= LIBRARY_CACHE {
            all.remove(0);
        }
        all.push((key, library));
    }
}

/// `"minecraft:sharpness=3;minecraft:unbreaking=1"`, the compound's readable
/// form: the ids hold `:`, so `=` separates a level and `;` entries.
fn encode_points(points: &[(Arc<str>, u32)]) -> String {
    let mut text = String::new();
    for (id, level) in points {
        if *level == 0 {
            continue;
        }
        if !text.is_empty() {
            text.push(';');
        }
        text.push_str(id);
        text.push('=');
        text.push_str(&level.to_string());
    }
    text
}

fn decode_points(text: &str, max_level: u32) -> Vec<(Arc<str>, u32)> {
    let mut points: Vec<(Arc<str>, u32)> = Vec::new();
    for entry in text.split(';') {
        let Some((id, level)) = entry.rsplit_once('=') else {
            continue;
        };
        let Ok(level) = level.parse::<u32>() else {
            continue;
        };
        if id.is_empty() || level == 0 {
            continue;
        }
        let level = level.min(max_level);
        match points.iter_mut().find(|(known, _)| known.as_ref() == id) {
            Some((_, known)) => *known = (*known).saturating_add(level).min(max_level),
            None => points.push((Arc::from(id), level)),
        }
    }
    points.sort_by(|a, b| a.0.cmp(&b.0));
    points
}

fn read_library(nbt: &Compound, max_level: u32) -> Library {
    let points = match nbt.get(&LIBRARY_POINTS.into()) {
        Some(Tag::String(text)) => decode_points(&text.to_string_lossy(), max_level),
        _ => Vec::new(),
    };
    Library {
        points,
        max_level,
        dirty: false,
    }
}

fn write_library(nbt: &mut Compound, library: &Library) {
    nbt.insert(
        LIBRARY_POINTS.into(),
        Tag::String(encode_points(&library.points).as_str().into()),
    );
}

fn remember_selection(uuid: u128, dimension: &str, pos: [i32; 3], offer: (Arc<str>, u32)) {
    let key = (uuid, dimension.to_owned(), pos);
    let Ok(mut all) = selections().lock() else {
        return;
    };
    if let Some((_, selected)) = all.iter_mut().find(|(at, _)| *at == key) {
        *selected = offer;
    } else {
        if all.len() >= SELECTION_CACHE {
            all.remove(0);
        }
        all.push((key, offer));
    }
}

fn selected_offer(uuid: u128, dimension: &str, pos: [i32; 3]) -> Option<(Arc<str>, u32)> {
    let key = (uuid, dimension.to_owned(), pos);
    selections()
        .lock()
        .ok()?
        .iter()
        .find(|(at, _)| *at == key)
        .map(|(_, offer)| offer.clone())
}

fn enchantment_name(id: &str) -> String {
    format!("enchantment.{}", id.replace(':', "."))
}

/// A right click on a library. An empty hand cycles the offer and a stray item
/// explains itself at once (no stack changes); a book or an enchanted book is
/// queued and applied on the next inventory tick.
pub(super) fn on_right_click_block(host: &mut dyn Host, event: &mut RightClickBlockEvent) -> Flow {
    let Some((_, max_level)) = library_kind(&host.block(event.pos)) else {
        return Flow::Continue;
    };
    let Some(player) = host.acting_player() else {
        return Flow::Continue;
    };
    let dimension = player.dimension.clone();
    let held = host.player_equipment(player.uuid, EquipmentSlot::MainHand);

    match held {
        Some(main) if matches!(&*main.item, "minecraft:enchanted_book" | "minecraft:book") => {
            set_pending(
                player.uuid,
                Pending {
                    job: Job::Library { dimension, pos: event.pos, max_level },
                    slot: i32::from(player.selected_slot),
                    main,
                    off: None,
                },
            );
        }
        None => {
            let library = library_at(&dimension, event.pos, max_level);
            let offers = library.offers();
            if offers.is_empty() {
                say(host, player.uuid, library_empty());
                return Flow::Continue;
            }
            let next = match selected_offer(player.uuid, &dimension, event.pos) {
                Some(ref offer) => offers
                    .iter()
                    .position(|candidate| candidate == offer)
                    .map_or(0, |index| (index + 1) % offers.len()),
                None => 0,
            };
            let offer = offers[next].clone();
            remember_selection(player.uuid, &dimension, event.pos, offer.clone());
            say(
                host,
                player.uuid,
                Text::translate_with_fallback(
                    "apotheosis.library.offer",
                    "Offer: %s %s (%s points stored). Right-click with a book to take it.",
                    &[
                        Text::translate_with_fallback(&enchantment_name(&offer.0), &offer.0, &[]),
                        Text::literal(offer.1.to_string()),
                        Text::literal(library.stored(&offer.0).to_string()),
                    ],
                ),
            );
        }
        Some(_) => {
            say(
                host,
                player.uuid,
                Text::translate_with_fallback(
                    "apotheosis.library.usage",
                    "Hold an enchanted book to store it, or a book to take an enchantment out.",
                    &[],
                ),
            );
        }
    }
    Flow::Continue
}

fn library_empty() -> Text {
    Text::translate_with_fallback(
        "apotheosis.library.empty",
        "The library holds no points yet: store enchanted books first.",
        &[],
    )
}

/// A book stored into the library: each of its enchantments adds its level in
/// points, past the cap the extra levels are lost. The book is consumed only
/// once the points are recorded. A plain book takes the selected offer out.
fn apply_library(
    host: &mut dyn Host,
    who: Who,
    dimension: &str,
    pos: [i32; 3],
    max_level: u32,
    slot: i32,
    held: &ItemStack,
) {
    let mut library = library_at(dimension, pos, max_level);
    if &*held.item == "minecraft:enchanted_book" {
        let enchantments = held.crafting_enchantments().to_vec();
        if enchantments.is_empty() {
            say(
                host,
                who.uuid,
                Text::translate_with_fallback(
                    "apotheosis.library.not_enchanted",
                    "That book carries no enchantment.",
                    &[],
                ),
            );
            return;
        }
        if !write_slot(host, slot, one_fewer(held)) {
            return;
        }
        for (enchantment, level) in &enchantments {
            library.absorb(enchantment, *level);
        }
        library.dirty = true;
        let levels: u32 = enchantments.iter().map(|(_, level)| *level).sum();
        let total: u32 = library.points.iter().map(|(_, points)| *points).sum();
        store_library(dimension, pos, library);
        say(
            host,
            who.uuid,
            Text::translate_with_fallback(
                "apotheosis.library.absorbed",
                "Stored %s levels; the library holds %s points.",
                &[Text::literal(levels.to_string()), Text::literal(total.to_string())],
            ),
        );
        return;
    }

    let offers = library.offers();
    if offers.is_empty() {
        say(host, who.uuid, library_empty());
        return;
    }
    let offer = selected_offer(who.uuid, dimension, pos)
        .filter(|(id, level)| library.stored(id) >= *level)
        .unwrap_or_else(|| offers[0].clone());
    let mut book = ItemStack::new("minecraft:enchanted_book", 1);
    book.set_crafting_enchantments(vec![offer.clone()]);
    // The book is handed out first: a full inventory leaves the hand and the
    // points as they were, and the points are spent only once it is out.
    if host.give_stack(who.uuid, book).is_some() {
        say(
            host,
            who.uuid,
            Text::translate_with_fallback(
                "apotheosis.library.no_room",
                "No room for the book; nothing was taken.",
                &[],
            ),
        );
        return;
    }
    write_slot(host, slot, one_fewer(held));
    if !library.withdraw(&offer.0, offer.1) {
        return;
    }
    library.dirty = true;
    store_library(dimension, pos, library);
    remember_selection(who.uuid, dimension, pos, offer.clone());
    say(
        host,
        who.uuid,
        Text::translate_with_fallback(
            "apotheosis.library.withdrawn",
            "Took %s %s for %s points.",
            &[
                Text::translate_with_fallback(&enchantment_name(&offer.0), &offer.0, &[]),
                Text::literal(offer.1.to_string()),
                Text::literal(offer.1.to_string()),
            ],
        ),
    );
}

/// Every library block gets a block entity, saved with its chunk.
pub(super) fn on_block_entity(event: &BlockEntityEvent<'_>) -> Option<(String, Compound)> {
    library_kind(event.block).map(|(entity, _)| (entity.to_owned(), Compound::new()))
}

/// A library's block entity ticks, so pending points reach the save.
pub(super) fn on_block_entity_ticks(event: &mut BlockEntityTicksEvent<'_>) {
    if library_kind(event.block).is_some() {
        event.ticks = true;
    }
}

/// Loads the saved points the first time the entity ticks, and writes them back
/// after a click changed them. A library broken before its next tick keeps its
/// cached points until the cache forgets it, never a duplicated book.
pub(super) fn on_block_entity_tick(host: &mut dyn Host, event: &mut BlockEntityTickEvent<'_>) {
    let Some((_, max_level)) = library_kind(&host.block(event.pos)) else {
        return;
    };
    let dimension = host.dimension().to_owned();
    let key = (dimension.clone(), event.pos);
    let Ok(mut all) = libraries().lock() else {
        return;
    };
    if let Some((_, library)) = all.iter_mut().find(|(at, _)| *at == key) {
        library.max_level = max_level;
        if library.dirty {
            write_library(&mut event.entity.nbt, library);
            library.dirty = false;
        } else {
            // A library placed where another stood starts empty again: the
            // saved compound, not the cache, is what the chunk holds.
            let saved = read_library(&event.entity.nbt, max_level);
            if saved.points != library.points {
                library.points = saved.points;
            }
        }
        return;
    }
    let library = read_library(&event.entity.nbt, max_level);
    if all.len() >= LIBRARY_CACHE {
        all.remove(0);
    }
    all.push((key, library));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualified_block_ids_find_their_shelf() {
        // `host.block` answers `namespace:path`, the table lists bare paths.
        assert_eq!(shelf_stats("apotheosis:deepshelf").unwrap().eterna, 5.0);
        assert_eq!(shelf_stats("minecraft:amethyst_cluster").unwrap().eterna, 1.0);
        assert_eq!(shelf_stats("apotheosis:stoneshelf").unwrap().eterna, -3.0);
        assert_eq!(shelf_stats("minecraft:air"), None);
    }

    fn shelf_line(key: &'static str, args: &[&str], colour: u32) -> (&'static str, Vec<String>, u32) {
        (key, args.iter().map(|arg| arg.to_string()).collect(), colour)
    }

    fn shelf_lines(item: &str) -> Vec<(&'static str, Vec<String>, u32)> {
        shelf_tooltip_lines(item)
            .into_iter()
            .map(|line| (line.key, line.args, line.colour))
            .collect()
    }

    #[test]
    fn shelf_tooltips_list_java_stats_and_colours() {
        assert_eq!(
            shelf_lines("apotheosis:deepshelf"),
            vec![
                shelf_line("info.apothic_enchanting.ench_stats", &[], 0xFFAA00),
                shelf_line("info.apothic_enchanting.eterna.p", &["5.00", "70.00"], 0x55FF55),
                shelf_line("info.apothic_enchanting.quanta.p", &["5.00"], 0xFF5555),
                shelf_line("info.apothic_enchanting.arcana.p", &["5.00"], 0xAA00AA),
            ]
        );
        assert_eq!(
            shelf_lines("apotheosis:blazing_hellshelf"),
            vec![
                shelf_line("info.apothic_enchanting.ench_stats", &[], 0xFFAA00),
                shelf_line("info.apothic_enchanting.eterna.p", &["10.00", "65.00"], 0x55FF55),
                shelf_line("info.apothic_enchanting.quanta.p", &["10.00"], 0xFF5555),
                shelf_line("info.apothic_enchanting.clues", &["-1"], 0x00AAAA),
            ]
        );
        assert_eq!(
            shelf_lines("apotheosis:treasure_shelf"),
            vec![
                shelf_line("info.apothic_enchanting.ench_stats", &[], 0xFFAA00),
                shelf_line("info.apothic_enchanting.eterna.p", &["5.00", "70.00"], 0x55FF55),
                shelf_line("info.apothic_enchanting.quanta", &["-15.00"], 0xFF5555),
                shelf_line("info.apothic_enchanting.arcana", &["-15.00"], 0xAA00AA),
                shelf_line("info.apothic_enchanting.allows_treasure", &[], 0xFFAA00),
            ]
        );
    }

    #[test]
    fn vanilla_shelf_blocks_show_their_stats() {
        assert_eq!(
            shelf_lines("minecraft:bookshelf"),
            vec![
                shelf_line("info.apothic_enchanting.ench_stats", &[], 0xFFAA00),
                shelf_line("info.apothic_enchanting.eterna.p", &["2.00", "30.00"], 0x55FF55),
            ]
        );
        assert_eq!(shelf_lines("minecraft:zombie_head")[1], shelf_line("info.apothic_enchanting.quanta.p", &["5.00"], 0xFF5555));
        assert_eq!(
            shelf_lines("minecraft:wither_skeleton_skull")[1],
            shelf_line("info.apothic_enchanting.quanta.p", &["10.00"], 0xFF5555)
        );
        assert_eq!(shelf_lines("minecraft:amethyst_cluster")[1].1, vec!["1.00", "40.00"]);
        assert_eq!(shelf_lines("apotheosis:geode_shelf").last().unwrap().0, "info.apothic_enchanting.provides_stability");
        // No stats, no header: a plain block, an empty filtering shelf, a table id.
        assert!(shelf_lines("minecraft:dirt").is_empty());
        assert!(shelf_lines("apotheosis:filtering_shelf").is_empty());
        assert!(shelf_lines("apotheosis:wither_skull").is_empty());
    }

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

    #[test]
    fn library_points_cap_at_its_top_level_and_are_spent_by_level() {
        let mut library = Library::default();
        library.absorb(&Arc::from("minecraft:sharpness"), 5);
        library.absorb(&Arc::from("minecraft:shadow"), 40);
        assert_eq!(library.stored("minecraft:sharpness"), 5);
        assert_eq!(library.stored("minecraft:shadow"), LIBRARY_MAX_LEVEL);
        assert!(library.withdraw("minecraft:sharpness", 3));
        assert_eq!(library.stored("minecraft:sharpness"), 2);
        assert!(!library.withdraw("minecraft:sharpness", 3));
        assert!(!library.withdraw("minecraft:unbreaking", 1));
        assert!(!library.withdraw("minecraft:sharpness", 0));
    }

    #[test]
    fn library_offers_every_level_it_can_pay_for() {
        let mut library = Library::default();
        library.absorb(&Arc::from("minecraft:unbreaking"), 2);
        library.absorb(&Arc::from("minecraft:sharpness"), 3);
        assert_eq!(
            library.offers(),
            vec![
                (Arc::from("minecraft:sharpness"), 1),
                (Arc::from("minecraft:unbreaking"), 1),
                (Arc::from("minecraft:sharpness"), 2),
                (Arc::from("minecraft:unbreaking"), 2),
                (Arc::from("minecraft:sharpness"), 3),
            ]
        );
        assert!(Library::default().offers().is_empty());
    }

    #[test]
    fn library_points_round_trip_through_the_saved_compound() {
        let mut library = Library::default();
        library.absorb(&Arc::from("minecraft:sharpness"), 3);
        library.absorb(&Arc::from("minecraft:protection"), 2);
        let mut nbt = Compound::new();
        write_library(&mut nbt, &library);
        let read = read_library(&nbt, LIBRARY_MAX_LEVEL);
        assert_eq!(read.points, library.points);
        assert!(!read.dirty);
        // Malformed entries are skipped rather than guessed.
        assert!(decode_points(";;=2;minecraft:sharpness=x;=1;=0", LIBRARY_MAX_LEVEL).is_empty());
        assert_eq!(
            decode_points("a=1;a=2", LIBRARY_MAX_LEVEL),
            vec![(Arc::from("a"), 3)]
        );
    }

    #[test]
    fn the_three_shelves_keep_their_java_stats() {
        let geode = shelf_stats("apotheosis:geode_shelf").unwrap();
        assert_eq!((geode.eterna, geode.max_eterna, geode.quanta), (5.0, 60.0, -15.0));
        let treasure = shelf_stats("apotheosis:treasure_shelf").unwrap();
        assert_eq!(
            (treasure.eterna, treasure.max_eterna, treasure.quanta, treasure.arcana),
            (5.0, 70.0, -15.0, -15.0)
        );
        // Without a block-entity inventory hook an empty filtering shelf is
        // all the port can read, and Java's empty shelf gives nothing.
        let filtering = shelf_stats("apotheosis:filtering_shelf").unwrap();
        assert_eq!((filtering.eterna, filtering.arcana), (0.0, 0.0));
    }

    #[test]
    fn both_libraries_carry_their_own_top_level() {
        assert_eq!(library_kind(LIBRARY), Some((LIBRARY_ENTITY, 30)));
        assert_eq!(library_kind(ENDER_LIBRARY), Some((ENDER_LIBRARY_ENTITY, 31)));
        assert_eq!(library_kind("minecraft:bookshelf"), None);

        let mut ender = read_library(&Compound::new(), ENDER_LIBRARY_MAX_LEVEL);
        ender.absorb(&Arc::from("minecraft:sharpness"), 99);
        assert_eq!(ender.stored("minecraft:sharpness"), 31);
        // A point saved by the Ender Library is capped when read back into the
        // ordinary one, and the reverse keeps every point it can pay for.
        assert_eq!(
            decode_points("minecraft:sharpness=31", LIBRARY_MAX_LEVEL),
            vec![(Arc::from("minecraft:sharpness"), 30)]
        );
        assert_eq!(
            decode_points("minecraft:sharpness=30", ENDER_LIBRARY_MAX_LEVEL),
            vec![(Arc::from("minecraft:sharpness"), 30)]
        );
    }

    #[test]
    fn infusion_takes_the_hardest_recipe_the_table_reaches() {
        let honey = StackStats { eterna: 100.0, quanta: 25.0, arcana: 25.0 };
        assert_eq!(find_infusion("minecraft:honey_bottle", honey).unwrap().count, 32);
        let honey = StackStats { eterna: 60.0, quanta: 25.0, arcana: 25.0 };
        assert_eq!(find_infusion("minecraft:honey_bottle", honey).unwrap().count, 8);
        let honey = StackStats { eterna: 20.0, quanta: 25.0, arcana: 25.0 };
        assert_eq!(find_infusion("minecraft:honey_bottle", honey).unwrap().count, 1);
        // Below the easiest recipe, or on an item no recipe takes, nothing fits.
        let honey = StackStats { eterna: 19.0, quanta: 25.0, arcana: 25.0 };
        assert!(find_infusion("minecraft:honey_bottle", honey).is_none());
        assert!(find_infusion("minecraft:stone", honey).is_none());
        assert_eq!(easiest_infusion("minecraft:honey_bottle").unwrap().count, 1);

        // The Ender Library keeps the library item and asks for a perfect table.
        let perfect = StackStats { eterna: 100.0, quanta: 45.0, arcana: 100.0 };
        let recipe = find_infusion(LIBRARY, perfect).unwrap();
        assert_eq!(recipe.output, ENDER_LIBRARY);
        assert_eq!(
            find_infusion(LIBRARY, StackStats { quanta: 50.0, ..perfect }).map(|recipe| recipe.output),
            Some(ENDER_LIBRARY)
        );
        assert!(find_infusion(LIBRARY, StackStats { quanta: 51.0, ..perfect }).is_none());
    }

    #[test]
    fn infusion_honours_the_upper_bounds_of_its_recipes() {
        // A carrot infuses up to 30 Quanta; the golden carrot recipe is the
        // only one that fits, and its Eterna bound stops a stronger table.
        let fits = StackStats { eterna: 20.0, quanta: 30.0, arcana: 0.0 };
        assert_eq!(find_infusion("minecraft:carrot", fits).unwrap().output, "minecraft:golden_carrot");
        assert!(find_infusion("minecraft:carrot", StackStats { quanta: 31.0, ..fits }).is_none());
        assert!(find_infusion("minecraft:carrot", StackStats { eterna: 21.0, ..fits }).is_none());
        // Quanta -1 in a data file means no bound at all.
        let echo = StackStats { eterna: 70.0, quanta: 100.0, arcana: 100.0 };
        assert_eq!(find_infusion("minecraft:echo_shard", echo).unwrap().count, 4);
    }

    #[test]
    fn infusion_charges_the_recipe_level_not_the_whole_table() {
        let carrot = easiest_infusion("minecraft:carrot").unwrap();
        assert_eq!(infusion_cost(carrot, 20.0), 20);
        assert_eq!(infusion_cost(carrot, 100.0), 20);
        // A table too weak for the recipe still never charges less than 3.
        assert_eq!(infusion_cost(carrot, 1.5), 3);
    }

    #[test]
    fn a_stored_enchantment_lists_its_levels_and_nothing_else() {
        let mut library = Library::default();
        library.absorb(&Arc::from("minecraft:sharpness"), 0);
        assert!(library.points.is_empty());
        assert_eq!(encode_points(&library.points), "");
    }

    const PLAYER: u128 = 11;
    const TEST_DIM: &str = "test:ingame";

    /// A host holding carried slots; `free` is how many whole stacks the
    /// inventory still takes (0 is a full one).
    #[derive(Default)]
    struct Fake {
        slots: HashMap<i32, ItemStack>,
        free: u32,
        given: Vec<ItemStack>,
        levels: Vec<i32>,
        lines: Vec<Text>,
        chat: Vec<Text>,
    }

    impl Host for Fake {
        fn block(&self, _pos: [i32; 3]) -> String {
            INFUSION_TABLE.to_owned()
        }
        fn carried_stack(&mut self, slot: i32) -> Option<&mut ItemStack> {
            // Like the real host, a slot written with a count of 0 is emptied.
            if self.slots.get(&slot).is_some_and(|stack| stack.count == 0) {
                self.slots.remove(&slot);
            }
            self.slots.get_mut(&slot)
        }
        fn give_stack(&mut self, _uuid: u128, stack: ItemStack) -> Option<ItemStack> {
            if self.free == 0 {
                return Some(stack);
            }
            self.free -= 1;
            self.given.push(stack);
            None
        }
        fn give_levels(&mut self, _uuid: u128, levels: i32) -> bool {
            self.levels.push(levels);
            true
        }
        fn action_bar(&mut self, _uuid: u128, line: Text) -> bool {
            self.lines.push(line);
            true
        }
        fn tell(&mut self, _uuid: u128, line: Text) -> bool {
            self.chat.push(line);
            true
        }
    }

    fn who(levels: i32) -> Who {
        Who { uuid: PLAYER, xp_level: levels, creative: false }
    }

    fn enchanted(item: &str, ids: &[(&str, u32)]) -> ItemStack {
        let mut stack = ItemStack::new(item, 1);
        stack.set_crafting_enchantments(levels(ids));
        stack
    }

    fn hands(main: ItemStack, off: Option<ItemStack>) -> Fake {
        let mut host = Fake { free: 1, ..Fake::default() };
        host.slots.insert(2, main);
        if let Some(off) = off {
            host.slots.insert(OFFHAND, off);
        }
        host
    }

    #[test]
    fn a_tome_scraps_the_off_hand_item_into_a_book_and_charges_levels() {
        let pick = enchanted("minecraft:diamond_pickaxe", &[("minecraft:efficiency", 4)]);
        let tome = ItemStack::new(IMPROVED_TOME, 2);
        let mut host = hands(tome.clone(), Some(pick));
        let target = host.slots.get(&OFFHAND).cloned();
        apply_tome(&mut host, who(30), Tome::Improved, 2, &tome, target);
        assert_eq!(host.given.len(), 1);
        assert_eq!(&*host.given[0].item, "minecraft:enchanted_book");
        assert!(host.carried_stack(OFFHAND).is_none(), "the scrapped item is gone");
        assert_eq!(host.slots[&2].count, 1, "one tome is used");
        assert_eq!(host.levels, vec![-IMPROVED_COST]);
        assert_eq!((host.lines.len(), host.chat.len()), (1, 1), "the result is shown");
    }

    #[test]
    fn a_full_inventory_changes_nothing_and_says_so() {
        let pick = enchanted("minecraft:diamond_pickaxe", &[("minecraft:efficiency", 4)]);
        let tome = ItemStack::new(EXTRACTION_TOME, 1);
        let mut host = hands(tome.clone(), Some(pick.clone()));
        host.free = 0;
        apply_tome(&mut host, who(30), Tome::Extraction, 2, &tome, Some(pick.clone()));
        assert!(host.given.is_empty() && host.levels.is_empty());
        assert_eq!(host.slots[&OFFHAND], pick);
        assert_eq!(host.slots[&2], tome);
        assert_eq!(host.lines.len(), 1);
    }

    #[test]
    fn extraction_strips_the_item_and_keeps_it() {
        let pick = enchanted("minecraft:diamond_pickaxe", &[("minecraft:fortune", 2)]);
        let tome = ItemStack::new(EXTRACTION_TOME, 1);
        let mut host = hands(tome.clone(), Some(pick.clone()));
        apply_tome(&mut host, who(30), Tome::Extraction, 2, &tome, Some(pick));
        assert!(host.slots[&OFFHAND].crafting_enchantments().is_empty());
        assert_eq!(host.given.len(), 1);
        assert!(host.carried_stack(2).is_none(), "the last tome is used up");
    }

    #[test]
    fn every_tome_refusal_is_shown_to_the_player() {
        let tome = ItemStack::new(SCRAP_TOME, 1);
        let plain = ItemStack::new("minecraft:diamond_pickaxe", 1);
        let pick = enchanted("minecraft:diamond_pickaxe", &[("minecraft:efficiency", 4)]);
        let web = ItemStack::new(PRISMATIC_WEB, 1);
        let cases = [
            (Tome::Scrap, None, 30),
            (Tome::Scrap, Some(plain.clone()), 30),
            (Tome::Scrap, Some(pick), 0),
            (Tome::Web, Some(plain), 30),
        ];
        for (kind, target, xp) in cases {
            let held = if kind == Tome::Web { &web } else { &tome };
            let mut host = hands(held.clone(), target.clone());
            apply_tome(&mut host, who(xp), kind, 2, held, target);
            assert_eq!(host.lines.len(), 1, "{kind:?} refusal shows on the action bar");
            assert_eq!(host.chat.len(), 1, "{kind:?} refusal shows in chat");
            assert!(host.given.is_empty() && host.levels.is_empty());
        }
    }

    #[test]
    fn the_web_cleanses_curses_in_the_hand_slot() {
        let helm = enchanted(
            "minecraft:diamond_helmet",
            &[("minecraft:binding_curse", 1), ("minecraft:protection", 3)],
        );
        let web = ItemStack::new(PRISMATIC_WEB, 1);
        let mut host = hands(web.clone(), Some(helm.clone()));
        apply_tome(&mut host, who(40), Tome::Web, 2, &web, Some(helm));
        assert_eq!(host.slots[&OFFHAND].crafting_enchantments().len(), 1);
        assert_eq!(host.levels, vec![-WEB_COST]);
    }

    #[test]
    fn an_infusion_with_nothing_to_infuse_explains_itself() {
        let stick = ItemStack::new("minecraft:stick", 1);
        let mut host = hands(stick.clone(), None);
        apply_infusion(&mut host, who(50), 2, [0, 64, 0], &stick);
        assert_eq!((host.lines.len(), host.chat.len()), (1, 1));
        assert_eq!(host.slots[&2], stick);
    }

    #[test]
    fn a_library_stores_a_book_then_gives_one_back_for_a_plain_book() {
        let pos = [101, 5, 101];
        let book = enchanted("minecraft:enchanted_book", &[("minecraft:sharpness", 3)]);
        let mut host = hands(book.clone(), None);
        apply_library(&mut host, who(0), TEST_DIM, pos, 10, 2, &book);
        assert!(!host.slots.contains_key(&2) || host.slots[&2].count == 0);
        assert_eq!(library_at(TEST_DIM, pos, 10).stored("minecraft:sharpness"), 3);
        assert_eq!((host.lines.len(), host.chat.len()), (1, 1));

        let blank = ItemStack::new("minecraft:book", 1);
        let mut host = hands(blank.clone(), None);
        apply_library(&mut host, who(0), TEST_DIM, pos, 10, 2, &blank);
        assert_eq!(host.given.len(), 1);
        assert_eq!(&*host.given[0].item, "minecraft:enchanted_book");
        assert!(library_at(TEST_DIM, pos, 10).stored("minecraft:sharpness") < 3);
    }

    #[test]
    fn a_library_refuses_visibly_and_keeps_the_book_when_inventory_is_full() {
        let pos = [102, 5, 102];
        let blank = ItemStack::new("minecraft:book", 1);
        let mut host = hands(blank.clone(), None);
        apply_library(&mut host, who(0), TEST_DIM, pos, 10, 2, &blank);
        assert_eq!((host.lines.len(), host.chat.len()), (1, 1), "empty library says so");

        let book = enchanted("minecraft:enchanted_book", &[("minecraft:power", 2)]);
        let mut host = hands(book.clone(), None);
        apply_library(&mut host, who(0), TEST_DIM, pos, 10, 2, &book);
        let mut host = hands(blank.clone(), None);
        host.free = 0;
        apply_library(&mut host, who(0), TEST_DIM, pos, 10, 2, &blank);
        assert_eq!(host.slots[&2], blank, "the book stays in the hand");
        assert_eq!(library_at(TEST_DIM, pos, 10).stored("minecraft:power"), 2);
        assert_eq!(host.lines.len(), 1);

        let bare = ItemStack::new("minecraft:enchanted_book", 1);
        let mut host = hands(bare.clone(), None);
        apply_library(&mut host, who(0), TEST_DIM, pos, 10, 2, &bare);
        assert_eq!(host.lines.len(), 1, "an unenchanted book is refused aloud");
    }

    #[test]
    fn a_pending_click_is_taken_only_on_its_own_slot() {
        let held = ItemStack::new(SCRAP_TOME, 1);
        set_pending(
            909,
            Pending { job: Job::Tome(Tome::Scrap), slot: 4, main: held, off: None },
        );
        assert!(take_pending(909, 3).is_none());
        assert!(take_pending(909, 4).is_some());
        assert!(take_pending(909, 4).is_none());
    }
}
