//! Gems in sockets, gem purity, reforging affixed gear and salvaging it.
//!
//! Everything here works through the hook API at the pinned revision:
//! `use_item` hears a right click with a gem in the main hand and the gear in
//! the off hand, and `inventory_tick` hands both carried stacks out mutably
//! (`Host::carried_stack`), so a socket is filled by moving the gem into the
//! gear and taking one gem off the main hand. No helper outside the API 415
//! surface is used.
//!
//! Sockets and a socketed gem live in the item's own `minecraft:custom_data`
//! (`apotheosis:sockets`, `apotheosis:socketed`), so they survive a save, a
//! trade or an anvil repair as the item does. The gem's bonus becomes a real
//! `minecraft:attribute_modifiers` entry on the gear, beside the affixes'
//! entries; an unsocketed gem keeps the off-hand bonus the content mod gives it.

use horizon_client::mod_api::{
    Compound, EquipmentSlot, Flow, Host, InventoryTickEvent, ItemStack, LootFunctionEvent, Tag, Text,
    TooltipEvent, UseItemEvent, UseItemOnEvent, serde_json,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::{apply_affixes, gem, is_armor, is_equipment, Rng, Tier};

/// The off hand's slot in `native_inventory`'s `Player` numbering.
const OFFHAND: i32 = -106;
/// Gem dust a purity upgrade costs.
pub const POLISH_COST: u32 = 4;
/// The most sockets any piece can have.
pub const MAX_SOCKETS: u8 = 3;

const CUSTOM: &str = "minecraft:custom_data";
const PURITY_KEY: &str = "apotheosis:gem_purity";
const SOCKETS_KEY: &str = "apotheosis:sockets";
const SOCKETED_KEY: &str = "apotheosis:socketed";
const TIER_KEY: &str = "apotheosis:tier";

/// A gem's purity: the multiplier its bonus carries. Apotheosis names six
/// steps; the numbers are the ratio of each step's value to a normal gem's in
/// its published gem data (1 / 5, 2 / 5, 3.5 / 5, 5 / 5, 7 / 5, 10 / 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Purity {
    Cracked,
    Chipped,
    Flawed,
    Normal,
    Flawless,
    Perfect,
}

impl Purity {
    pub const ALL: [Purity; 6] = [
        Purity::Cracked,
        Purity::Chipped,
        Purity::Flawed,
        Purity::Normal,
        Purity::Flawless,
        Purity::Perfect,
    ];

    pub fn index(self) -> u8 {
        match self {
            Purity::Cracked => 0,
            Purity::Chipped => 1,
            Purity::Flawed => 2,
            Purity::Normal => 3,
            Purity::Flawless => 4,
            Purity::Perfect => 5,
        }
    }

    pub fn from_index(index: u8) -> Purity {
        Purity::ALL[(index as usize).min(Purity::ALL.len() - 1)]
    }

    /// What the gem's bonus is multiplied by.
    pub fn multiplier(self) -> f64 {
        [0.2, 0.4, 0.7, 1.0, 1.4, 2.0][self.index() as usize]
    }

    /// The next step up, or itself at the top.
    pub fn next(self) -> Purity {
        Purity::from_index((self.index() + 1).min(Purity::ALL.len() as u8 - 1))
    }

    pub fn name_key(self) -> &'static str {
        match self {
            Purity::Cracked => "apotheosis.purity.cracked",
            Purity::Chipped => "apotheosis.purity.chipped",
            Purity::Flawed => "apotheosis.purity.flawed",
            Purity::Normal => "apotheosis.purity.normal",
            Purity::Flawless => "apotheosis.purity.flawless",
            Purity::Perfect => "apotheosis.purity.perfect",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Purity::Cracked => "Cracked",
            Purity::Chipped => "Chipped",
            Purity::Flawed => "Flawed",
            Purity::Normal => "Normal",
            Purity::Flawless => "Flawless",
            Purity::Perfect => "Perfect",
        }
    }

    pub fn colour(self) -> u32 {
        match self {
            Purity::Cracked => 0x80_8080,
            Purity::Chipped => 0xA0_A0A0,
            Purity::Flawed => 0xC8_C8C8,
            Purity::Normal => 0x55_FF55,
            Purity::Flawless => 0x55_AAFF,
            Purity::Perfect => 0xFF_D7_00,
        }
    }

    pub fn hex(self) -> String {
        format!("#{:06X}", self.colour())
    }
}

/// A gem sitting in a socket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Socket {
    pub gem: String,
    pub purity: Purity,
}

/// The purity a fresh drop rolls. Ordinary loot is mostly poor, as in
/// Apotheosis' frontier tables; a boss draws from better tables.
const ORDINARY_PURITY: [(Purity, u64); 6] = [
    (Purity::Cracked, 30),
    (Purity::Chipped, 40),
    (Purity::Flawed, 20),
    (Purity::Normal, 8),
    (Purity::Flawless, 2),
    (Purity::Perfect, 1),
];

const BOSS_PURITY: [(Purity, u64); 6] = [
    (Purity::Cracked, 5),
    (Purity::Chipped, 20),
    (Purity::Flawed, 35),
    (Purity::Normal, 28),
    (Purity::Flawless, 10),
    (Purity::Perfect, 2),
];

fn roll_purity(purity_weights: &[(Purity, u64)], rng: &mut Rng) -> Purity {
    let total: u64 = purity_weights.iter().map(|(_, w)| *w).sum();
    let mut roll = rng.below(total.max(1));
    for (purity, weight) in purity_weights {
        if roll < *weight {
            return *purity;
        }
        roll -= *weight;
    }
    Purity::Normal
}

/// Marks a gem stack with the purity it dropped at.
pub fn set_purity(stack: &mut ItemStack, purity: Purity) {
    set_custom(stack, |map| {
        map.insert(PURITY_KEY.into(), Tag::Int(purity.index() as i32));
    });
}

/// The purity a gem stack carries; an unmarked gem (a chest gem, a crafted
/// one) is normal.
pub fn purity_of(stack: &ItemStack) -> Purity {
    let tag = custom(stack);
    match tag.get(PURITY_KEY) {
        Some(Tag::Int(index)) => Purity::from_index(*index as u8),
        _ => Purity::Normal,
    }
}

/// Gives a freshly dropped gem its purity.
pub fn roll_dropped_purity(stack: &mut ItemStack, rng: &mut Rng, boss: bool) {
    if gem(&stack.item).is_none() {
        return;
    }
    let purity = roll_purity(if boss { &BOSS_PURITY } else { &ORDINARY_PURITY }, rng);
    set_purity(stack, purity);
}

/// How many sockets a piece of a given tier rolls: none at common, up to three
/// at the top, and never more than `MAX_SOCKETS`.
pub fn socket_count(tier: Tier, rng: &mut Rng) -> u8 {
    let (low, high) = match tier {
        Tier::Common => (0, 0),
        Tier::Uncommon => (0, 1),
        Tier::Rare => (0, 2),
        Tier::Epic => (1, 3),
        Tier::Mythic => (2, 3),
        Tier::Ancient => (3, 3),
    };
    low + rng.below(u64::from(high - low) + 1) as u8
}

/// The sockets a piece has.
pub fn socket_total(stack: &ItemStack) -> u8 {
    let tag = custom(stack);
    match tag.get(SOCKETS_KEY) {
        Some(Tag::Int(total)) => (*total).clamp(0, i32::from(MAX_SOCKETS)) as u8,
        _ => 0,
    }
}

/// The gems already in a piece's sockets.
pub fn socketed(stack: &ItemStack) -> Vec<Socket> {
    let tag = custom(stack);
    let Some(Tag::List(entries)) = tag.get(SOCKETED_KEY) else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|entry| {
            let gem = match entry.get("gem") {
                Some(Tag::String(id)) => id.to_string_lossy(),
                _ => return None,
            };
            let purity = match entry.get("purity") {
                Some(Tag::Int(index)) => Purity::from_index(*index as u8),
                _ => Purity::Normal,
            };
            Some(Socket { gem, purity })
        })
        .collect()
}

/// The tier an affixed piece was rolled at, when this port wrote it.
pub fn tier_of(stack: &ItemStack) -> Option<Tier> {
    let tag = custom(stack);
    let index = match tag.get(TIER_KEY) {
        Some(Tag::Int(index)) => *index as u8,
        _ => return None,
    };
    Some(match index {
        0 => Tier::Common,
        1 => Tier::Uncommon,
        2 => Tier::Rare,
        3 => Tier::Epic,
        4 => Tier::Mythic,
        _ => Tier::Ancient,
    })
}

/// Whether a piece carries this port's affixes.
pub fn has_affixes(stack: &ItemStack) -> bool {
    tier_of(stack).is_some()
}

/// Rolls a piece's sockets after `apply_affixes`, and records the tier so a
/// salvage or a reforge can read it back.
pub fn after_affixes(stack: &mut ItemStack, tier: Tier, rng: &mut Rng) {
    let total = socket_count(tier, rng);
    set_custom(stack, |map| {
        map.insert(TIER_KEY.into(), Tag::Int(tier as i32));
    });
    write_sockets(stack, total, &[]);
    refresh(stack);
}

/// Puts gem `gem` at `purity` into `gear`'s first free socket. False when the
/// gear has no free socket or already holds that gem.
pub fn insert_gem(gear: &mut ItemStack, gem_id: &str, purity: Purity) -> bool {
    if gem(gem_id).is_none() {
        return false;
    }
    let total = socket_total(gear);
    let mut sockets = socketed(gear);
    if sockets.len() >= usize::from(total) || sockets.iter().any(|s| s.gem == gem_id) {
        return false;
    }
    sockets.push(Socket {
        gem: gem_id.to_owned(),
        purity,
    });
    write_sockets(gear, total, &sockets);
    refresh(gear);
    true
}

/// A socketed gem's bonus on this piece: its base amount scaled by purity,
/// rounded to four places so a small movement-speed bonus is not lost.
pub fn socketed_amount(gem_id: &str, purity: Purity) -> Option<f64> {
    let gem = gem(gem_id)?;
    Some((gem.amount * purity.multiplier() * 10_000.0).round() / 10_000.0)
}

/// The rarity material a piece of `tier` salvages into, and how many at most.
pub fn salvage_material(tier: Tier) -> (&'static str, u64) {
    match tier {
        Tier::Common => ("apotheosis:mysterious_scrap_metal", 4),
        Tier::Uncommon => ("apotheosis:timeworn_fabric", 4),
        Tier::Rare => ("apotheosis:luminous_crystal_shard", 4),
        Tier::Epic => ("apotheosis:arcane_sands", 4),
        Tier::Mythic => ("apotheosis:godforged_pearl", 4),
        Tier::Ancient => ("apotheosis:godforged_pearl", 6),
    }
}

/// What a salvage gives: 1..=max of the tier's material, by the same roll
/// Apotheosis' salvaging recipes use, plus gem dust from the piece's tier.
pub fn salvage_yield(tier: Tier, rng: &mut Rng) -> (String, u32, u32) {
    let (material, max) = salvage_material(tier);
    let count = 1 + rng.below(max) as u32;
    let dust = match tier {
        Tier::Common | Tier::Uncommon => 0,
        Tier::Rare => 1,
        Tier::Epic => 2,
        Tier::Mythic => 3,
        Tier::Ancient => 4,
    };
    (material.to_owned(), count, dust)
}

/// The rarity component's name, for the tooltip line of a socketed gem.
fn gem_name_key(gem_id: &str) -> String {
    format!("item.{}", gem_id.replacen(':', ".", 1))
}

fn amount_text(amount: f64) -> String {
    if amount.fract().abs() < f64::EPSILON {
        format!("+{amount:.0}")
    } else {
        let mut text = format!("+{amount:.4}");
        while text.ends_with('0') {
            text.pop();
        }
        text
    }
}

/// The custom-data compound of a stack, empty when it has none.
fn custom(stack: &ItemStack) -> Tag {
    stack
        .data()
        .and_then(|data| data.other(CUSTOM))
        .filter(|tag| matches!(tag, Tag::Compound(_)))
        .cloned()
        .unwrap_or(Tag::Compound(Default::default()))
}

fn set_custom(stack: &mut ItemStack, change: impl FnOnce(&mut Compound)) {
    let mut map = match custom(stack) {
        Tag::Compound(map) => map,
        _ => Compound::default(),
    };
    change(&mut map);
    stack.data_mut().set_other(
        CUSTOM,
        if map.is_empty() {
            None
        } else {
            Some(Tag::Compound(map))
        },
    );
}

fn write_sockets(stack: &mut ItemStack, total: u8, sockets: &[Socket]) {
    let entries: Vec<Tag> = sockets
        .iter()
        .map(|socket| {
            Tag::compound([
                ("gem", Tag::String(socket.gem.as_str().into())),
                ("purity", Tag::Int(socket.purity.index() as i32)),
            ])
        })
        .collect();
    set_custom(stack, |map| {
        map.insert(SOCKETS_KEY.into(), Tag::Int(i32::from(total)));
        map.insert(SOCKETED_KEY.into(), Tag::List(entries));
    });
}

/// Whether a modifier is one this module wrote for a socket.
fn is_socket_modifier(entry: &Tag) -> bool {
    matches!(
        entry.get("id"),
        Some(Tag::String(id)) if id.to_string_lossy().starts_with("apotheosis:socket_")
    )
}

/// Whether a lore line is a socket line this module wrote.
fn is_socket_lore(entry: &Tag) -> bool {
    matches!(
        entry.get("translate"),
        Some(Tag::String(key))
            if key.to_string_lossy() == "apotheosis.socket.line"
                || key.to_string_lossy() == "apotheosis.socket.total"
    )
}

/// Rewrites the socket half of a piece's modifiers and lore, keeping the
/// affixes' own entries: called after socketing, after a reforge, and once when
/// the sockets are rolled.
fn socket_slot_group(item: &str) -> &'static str {
    if ["_helmet", "_chestplate", "_leggings", "_boots"]
        .iter()
        .any(|suffix| item.ends_with(suffix))
    {
        "armor"
    } else {
        // Shields are equipped in the off hand, not in an armor slot.
        "hand"
    }
}

fn refresh(stack: &mut ItemStack) {
    let total = socket_total(stack);
    let sockets = socketed(stack);
    let slots = socket_slot_group(&stack.item);

    let mut modifiers = match stack
        .data()
        .and_then(|data| data.other("minecraft:attribute_modifiers"))
    {
        Some(Tag::List(entries)) => entries
            .iter()
            .filter(|entry| !is_socket_modifier(entry))
            .cloned()
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    let mut lore = match stack.data().and_then(|data| data.other("minecraft:lore")) {
        Some(Tag::List(lines)) => lines
            .iter()
            .filter(|line| !is_socket_lore(line))
            .cloned()
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };

    for (index, socket) in sockets.iter().enumerate() {
        let Some(entry) = gem(&socket.gem) else {
            continue;
        };
        let Some(amount) = socketed_amount(&socket.gem, socket.purity) else {
            continue;
        };
        let id = format!(
            "apotheosis:socket_{index}_{}",
            socket.gem.rsplit(':').next().unwrap_or(&socket.gem)
        );
        modifiers.push(Tag::compound([
            ("type", Tag::String(entry.attribute.into())),
            ("id", Tag::String(id.as_str().into())),
            ("amount", Tag::Double(amount.to_bits())),
            ("operation", Tag::String("add_value".into())),
            ("slot", Tag::String(slots.into())),
        ]));
        lore.push(socket_lore(socket, entry, amount));
    }

    if total > 0 {
        lore.push(Tag::compound([
            (
                "translate",
                Tag::String("apotheosis.socket.total".into()),
            ),
            (
                "fallback",
                Tag::String(format!("Sockets: {}/{}", sockets.len(), total).as_str().into()),
            ),
            ("color", Tag::String("#808080".into())),
            ("italic", Tag::Byte(0)),
            (
                "with",
                Tag::List(vec![
                    Tag::String(format!("{}", sockets.len()).as_str().into()),
                    Tag::String(format!("{total}").as_str().into()),
                ]),
            ),
        ]));
    }

    let data = stack.data_mut();
    data.set_other(
        "minecraft:attribute_modifiers",
        (!modifiers.is_empty()).then_some(Tag::List(modifiers)),
    );
    data.set_other("minecraft:lore", (!lore.is_empty()).then_some(Tag::List(lore)));
}

fn socket_lore(socket: &Socket, entry: &crate::Gem, amount: f64) -> Tag {
    let fallback = format!(
        "{} {}: {} {}",
        socket.purity.name(),
        entry.id.rsplit(':').next().unwrap_or(entry.id),
        amount_text(amount),
        entry.attribute.rsplit(':').next().unwrap_or(entry.attribute)
    );
    Tag::compound([
        ("translate", Tag::String("apotheosis.socket.line".into())),
        ("fallback", Tag::String(fallback.as_str().into())),
        ("color", Tag::String(socket.purity.hex().as_str().into())),
        ("italic", Tag::Byte(0)),
        (
            "with",
            Tag::List(vec![
                Tag::compound([
                    ("translate", Tag::String(gem_name_key(&socket.gem).as_str().into())),
                    (
                        "fallback",
                        Tag::String(entry.id.rsplit(':').next().unwrap_or(entry.id).into()),
                    ),
                ]),
                Tag::compound([
                    ("translate", Tag::String(socket.purity.name_key().into())),
                    ("fallback", Tag::String(socket.purity.name().into())),
                ]),
                Tag::String(amount_text(amount).as_str().into()),
                Tag::compound([
                    (
                        "translate",
                        Tag::String(format!("attribute.name.{}", entry.attribute).as_str().into()),
                    ),
                    (
                        "fallback",
                        Tag::String(entry.attribute.rsplit(':').next().unwrap_or(entry.attribute).into()),
                    ),
                ]),
            ]),
        ),
    ])
}

/// The tooltip lines this module adds: a gem's purity and how to socket it, or
/// a piece's sockets and what sits in them.
pub fn tooltip(event: &mut TooltipEvent<'_>) {
    if let Some(seal) = seal(&event.stack.item) {
        event.lines.push((
            Text::translate_with_fallback(seal.desc_key(), seal.desc(), &[]).with_color(0x80_8080),
            0x80_8080,
        ));
        return;
    }
    if gem(&event.stack.item).is_some() {
        let purity = purity_of(event.stack);
        event.lines.push((
            Text::translate_with_fallback(
                "apotheosis.purity.line",
                &format!("Purity: {}", purity.name()),
                &[Text::translate_with_fallback(
                    purity.name_key(),
                    purity.name(),
                    &[],
                )],
            )
            .with_color(purity.colour()),
            purity.colour(),
        ));
        event.lines.push((
            Text::translate_with_fallback(
                "apotheosis.socket.hint",
                "Right-click with this gem in hand and the gear in your off hand to socket it.",
                &[],
            )
            .with_color(0x55_55FF),
            0x55_55FF,
        ));
        if purity < Purity::Perfect {
            event.lines.push((
                Text::translate_with_fallback(
                    "apotheosis.polish.hint",
                    "Right-click with gem dust in your off hand to raise its purity.",
                    &[],
                )
                .with_color(0x80_8080),
                0x80_8080,
            ));
        }
        return;
    }

    let total = socket_total(event.stack);
    if total == 0 {
        return;
    }
    let sockets = socketed(event.stack);
    event.lines.push((
        Text::translate_with_fallback(
            "apotheosis.socket.total",
            &format!("Sockets: {}/{}", sockets.len(), total),
            &[
                Text::literal(sockets.len().to_string()),
                Text::literal(total.to_string()),
            ],
        )
        .with_color(0x80_8080),
        0x80_8080,
    ));
    for socket in sockets {
        let Some(entry) = gem(&socket.gem) else {
            continue;
        };
        let Some(amount) = socketed_amount(&socket.gem, socket.purity) else {
            continue;
        };
        let fallback = format!(
            "{} {}: {} {}",
            socket.purity.name(),
            entry.id.rsplit(':').next().unwrap_or(entry.id),
            amount_text(amount),
            entry.attribute.rsplit(':').next().unwrap_or(entry.attribute)
        );
        event.lines.push((
            Text::translate_with_fallback(
                "apotheosis.socket.line",
                &fallback,
                &[
                    Text::translate_with_fallback(
                        &gem_name_key(&socket.gem),
                        entry.id.rsplit(':').next().unwrap_or(entry.id),
                        &[],
                    ),
                    Text::translate_with_fallback(
                        socket.purity.name_key(),
                        socket.purity.name(),
                        &[],
                    ),
                    Text::literal(amount_text(amount)),
                    Text::translate_with_fallback(
                        &format!("attribute.name.{}", entry.attribute),
                        entry.attribute.rsplit(':').next().unwrap_or(entry.attribute),
                        &[],
                    ),
                ],
            )
            .with_color(socket.purity.colour()),
            socket.purity.colour(),
        ));
    }
}

/// A sigil, by the item that is one. Every sigil is used the same way: in the
/// main hand, a piece in the off hand, right click.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seal {
    /// Adds one socket, up to the maximum.
    Socketing,
    /// Takes the socketed gems back out.
    Withdrawal,
    /// Rerolls the piece's rarity and affixes.
    Rebirth,
    /// Raises the piece's rarity by one step, rerolling at the new one.
    Enhancement,
}

impl Seal {
    pub fn desc_key(self) -> &'static str {
        match self {
            Seal::Socketing => "item.apotheosis.sigil_of_socketing.desc",
            Seal::Withdrawal => "item.apotheosis.sigil_of_withdrawal.desc",
            Seal::Rebirth => "item.apotheosis.sigil_of_rebirth.desc",
            Seal::Enhancement => "item.apotheosis.sigil_of_enhancement.desc",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Seal::Socketing => "Adds one socket to an item",
            Seal::Withdrawal => "Removes all socketed gems from an item",
            Seal::Rebirth => "Rerolls an affixed item's rarity and affixes",
            Seal::Enhancement => "Raises an affixed item's rarity by one step",
        }
    }
}

/// The sigil an item id is, when it is one.
pub fn seal(item: &str) -> Option<Seal> {
    match item {
        "apotheosis:sigil_of_socketing" => Some(Seal::Socketing),
        "apotheosis:sigil_of_withdrawal" => Some(Seal::Withdrawal),
        "apotheosis:sigil_of_rebirth" => Some(Seal::Rebirth),
        "apotheosis:sigil_of_enhancement" => Some(Seal::Enhancement),
        _ => None,
    }
}

/// The working block an id is: the three tables, each doing on a right click
/// what its command does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Table {
    Reforging,
    Salvaging,
    Cutting,
}

pub fn table(block: &str) -> Option<Table> {
    match block {
        "apotheosis:reforging_table" => Some(Table::Reforging),
        "apotheosis:salvaging_table" => Some(Table::Salvaging),
        "apotheosis:gem_cutting_table" => Some(Table::Cutting),
        _ => None,
    }
}

/// What a right click with a gem or a sigil queued, applied on the next
/// inventory tick of the held stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Socket,
    Polish,
    Reforge,
    Salvage,
    Cut,
    AddSocket,
    Withdraw,
    Rebirth,
    Enhance,
}

fn pending() -> &'static Mutex<HashMap<u128, Action>> {
    static PENDING: OnceLock<Mutex<HashMap<u128, Action>>> = OnceLock::new();
    PENDING.get_or_init(|| Mutex::new(HashMap::new()))
}

fn set_pending(player: u128, action: Action) {
    if let Ok(mut pending) = pending().lock() {
        pending.insert(player, action);
    }
}

fn take_pending(player: u128) -> Option<Action> {
    pending().lock().ok()?.remove(&player)
}

/// Right click with a gem: socket it into the gear in the off hand, or spend
/// gem dust from the off hand to raise its purity. A sigil instead acts on the
/// gear in the off hand.
pub fn use_item(host: &mut dyn Host, event: &mut UseItemEvent<'_>) -> Flow {
    if let Some(seal) = seal(&event.stack.item) {
        return use_seal(host, event, seal);
    }
    if gem(&event.stack.item).is_none() {
        return Flow::Continue;
    }
    let Some(player) = host.acting_player() else {
        return Flow::Continue;
    };
    // The off hand, not the gem's own hand: a gem used from the off hand has no
    // gear to work on.
    let Some(main) = host.player_equipment(player.uuid, EquipmentSlot::MainHand) else {
        return Flow::Continue;
    };
    if main.item != event.stack.item {
        return Flow::Continue;
    }
    let Some(off) = host.player_equipment(player.uuid, EquipmentSlot::OffHand) else {
        return Flow::Continue;
    };

    if socket_total(&off) > 0 {
        set_pending(player.uuid, Action::Socket);
        return Flow::Cancel;
    }
    if can_polish(&event.stack, &off) {
        set_pending(player.uuid, Action::Polish);
        return Flow::Cancel;
    }
    Flow::Continue
}

/// Ask for the piece in the main hand to be salvaged on its next inventory
/// tick: `/apoth salvage`.
pub fn begin_salvage(host: &mut dyn Host, player: u128) {
    let affixed = host
        .held_stack(player)
        .is_some_and(|stack| has_affixes(&stack));
    if !affixed {
        host.feedback(Text::translate_with_fallback(
            "apotheosis.salvage.not_affixed",
            "Hold an affixed piece to salvage it.",
            &[],
        ));
        return;
    }
    set_pending(player, Action::Salvage);
}

/// Queue a reforge of the affixed piece in the main hand; the command has
/// already charged for it.
pub fn begin_reforge(player: u128) {
    set_pending(player, Action::Reforge);
}

/// The counter this module's rolls draw from, mixed with the tick they run on:
/// the hook API has no random source, so this is the port's own stream.
fn next_roll() -> u64 {
    static ROLLS: AtomicU64 = AtomicU64::new(0);
    let n = ROLLS.fetch_add(1, Ordering::Relaxed);
    n.wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(17)
}

/// Whether the main hand holds a piece this module can reforge: affixed gear.
pub fn can_reforge(stack: &ItemStack) -> bool {
    is_equipment(&stack.item) && has_affixes(stack)
}

/// Applies a queued action on the held stack. Runs once per player per tick:
/// `inventory_tick` is called for every carried stack, so only the selected
/// slot does the work.
pub fn inventory_tick(host: &mut dyn Host, event: &mut InventoryTickEvent) {
    if !event.selected {
        return;
    }
    let Some(player) = host.acting_player() else {
        return;
    };
    let Some(action) = take_pending(player.uuid) else {
        return;
    };
    let Some(main) = host.carried_stack(event.slot).cloned() else {
        return;
    };

    match action {
        Action::Reforge => reforge(host, event.slot, &main, player.uuid),
        Action::Salvage => salvage(host, event.slot, &main, player.uuid),
        Action::Cut => cut(host, event.slot, &main),
        other => {
            let Some(off) = host.carried_stack(OFFHAND).cloned() else {
                return;
            };
            match other {
                Action::Socket => socket(host, event.slot, &main, &off),
                Action::Polish => polish(host, event.slot, &main, &off),
                Action::AddSocket => add_socket(host, event.slot, &main, &off),
                Action::Withdraw => withdraw(host, event.slot, &main, &off, player.uuid),
                Action::Rebirth => rebirth(host, event.slot, &main, &off),
                Action::Enhance => enhance(host, event.slot, &main, &off),
                _ => {}
            }
        }
    }
}

/// A sigil in the main hand, a piece in the off hand: queue what the sigil
/// does, and take it off the main hand on the next inventory tick.
pub fn use_seal(host: &mut dyn Host, event: &UseItemEvent<'_>, _seal: Seal) -> Flow {
    let Some(player) = host.acting_player() else {
        return Flow::Continue;
    };
    let Some(main) = host.player_equipment(player.uuid, EquipmentSlot::MainHand) else {
        return Flow::Continue;
    };
    if main.item != event.stack.item {
        return Flow::Continue;
    }
    let Some(off) = host.player_equipment(player.uuid, EquipmentSlot::OffHand) else {
        return Flow::Continue;
    };
    if !can_reforge(&off) {
        host.feedback(Text::translate_with_fallback(
            "apotheosis.seal.no_piece",
            "Hold an affixed piece in your off hand to use a sigil on it.",
            &[],
        ));
        return Flow::Cancel;
    }
    set_pending(
        player.uuid,
        match _seal {
            Seal::Socketing => Action::AddSocket,
            Seal::Withdrawal => Action::Withdraw,
            Seal::Rebirth => Action::Rebirth,
            Seal::Enhancement => Action::Enhance,
        },
    );
    Flow::Cancel
}

/// A right click on one of the working tables: the block does what the command
/// of the same name does, on the stack in the main hand.
pub fn use_item_on(host: &mut dyn Host, event: &mut UseItemOnEvent<'_>) -> Flow {
    let Some(table) = table(&host.block(event.pos)) else {
        return Flow::Continue;
    };
    let Some(player) = host.acting_player() else {
        return Flow::Continue;
    };
    // The click belongs to the table, never to the block under it: a table
    // that opened a crafting grid on a misclick would hide its own use.
    match table {
        Table::Reforging => {
            // Plain gear too: the command rolls affixes onto it, and so does
            // the table; `reforge` itself says what it cannot work on.
            if !is_equipment(&event.stack.item) {
                host.feedback(Text::translate_with_fallback(
                    "apotheosis.table.reforge.hint",
                    "The reforging table works on the affixed piece in your hand.",
                    &[],
                ));
                return Flow::Cancel;
            }
            crate::reforge(host, player.uuid, player.xp_level);
            Flow::Cancel
        }
        Table::Salvaging => {
            if !has_affixes(&event.stack) {
                host.feedback(Text::translate_with_fallback(
                    "apotheosis.table.salvage.hint",
                    "The salvaging table breaks down the affixed piece in your hand.",
                    &[],
                ));
                return Flow::Cancel;
            }
            begin_salvage(host, player.uuid);
            Flow::Cancel
        }
        Table::Cutting => {
            if gem(&event.stack.item).is_none() || purity_of(&event.stack) >= Purity::Perfect {
                host.feedback(Text::translate_with_fallback(
                    "apotheosis.table.cut.hint",
                    "The gem cutting table sharpens the gem in your hand.",
                    &[],
                ));
                return Flow::Cancel;
            }
            set_pending(player.uuid, Action::Cut);
            Flow::Cancel
        }
    }
}

/// Takes one sigil off the main hand; the slot empties when that was the last.
fn consume_seal(host: &mut dyn Host, slot: i32, seal_stack: &ItemStack) {
    let mut left = seal_stack.clone();
    left.count = left.count.saturating_sub(1);
    write_slot(host, slot, left);
}

fn add_socket(host: &mut dyn Host, slot: i32, seal_stack: &ItemStack, gear: &ItemStack) {
    if !can_reforge(gear) {
        return;
    }
    let total = socket_total(gear);
    if total >= MAX_SOCKETS {
        host.feedback(Text::translate_with_fallback(
            "apotheosis.socket.max",
            "That piece already has all %s sockets.",
            &[Text::literal(total.to_string())],
        ));
        return;
    }
    let mut piece = gear.clone();
    write_sockets(&mut piece, total + 1, &socketed(gear));
    refresh(&mut piece);
    write_slot(host, OFFHAND, piece);
    consume_seal(host, slot, seal_stack);
    host.feedback(Text::translate_with_fallback(
        "apotheosis.socket.added",
        "Socket added: %s/%s.",
        &[
            Text::literal((total + 1).to_string()),
            Text::literal(MAX_SOCKETS.to_string()),
        ],
    ));
}

fn withdraw(host: &mut dyn Host, slot: i32, seal_stack: &ItemStack, gear: &ItemStack, player: u128) {
    let gems = socketed(gear);
    if gems.is_empty() {
        host.feedback(Text::translate_with_fallback(
            "apotheosis.socket.none",
            "That piece has no gem in its sockets.",
            &[],
        ));
        return;
    }
    let mut piece = gear.clone();
    write_sockets(&mut piece, socket_total(gear), &[]);
    refresh(&mut piece);
    write_slot(host, OFFHAND, piece);
    let count = gems.len();
    for socket in gems {
        let mut gem = ItemStack::new(&socket.gem, 1);
        set_purity(&mut gem, socket.purity);
        let _ = host.give_stack(player, gem);
    }
    consume_seal(host, slot, seal_stack);
    host.feedback(Text::translate_with_fallback(
        "apotheosis.socket.withdrawn",
        "%s gem(s) taken back out.",
        &[Text::literal(count.to_string())],
    ));
}

fn rebirth(host: &mut dyn Host, slot: i32, seal_stack: &ItemStack, gear: &ItemStack) {
    if !can_reforge(gear) {
        return;
    }
    let tier = reroll(host, OFFHAND, gear);
    consume_seal(host, slot, seal_stack);
    host.feedback(Text::translate_with_fallback(
        "apotheosis.reforge.done",
        "Reforged: %s.",
        &[Text::translate_with_fallback(
            tier.name_key(),
            tier.name(),
            &[],
        )],
    ));
}

fn enhance(host: &mut dyn Host, slot: i32, seal_stack: &ItemStack, gear: &ItemStack) {
    let Some(tier) = tier_of(gear) else {
        return;
    };
    if tier == Tier::Ancient {
        host.feedback(Text::translate_with_fallback(
            "apotheosis.enhance.max",
            "That piece is already ancient.",
            &[],
        ));
        return;
    }
    let next = next_tier(tier);
    let mut piece = gear.clone();
    let mut rng = Rng::new(next_roll());
    apply_affixes(&mut piece, next, &mut rng);
    set_custom(&mut piece, |map| {
        map.insert(TIER_KEY.into(), Tag::Int(next as i32));
    });
    write_sockets(&mut piece, socket_total(gear), &socketed(gear));
    refresh(&mut piece);
    write_slot(host, OFFHAND, piece);
    consume_seal(host, slot, seal_stack);
    host.feedback(Text::translate_with_fallback(
        "apotheosis.enhance.done",
        "Enhanced: %s.",
        &[Text::translate_with_fallback(
            next.name_key(),
            next.name(),
            &[],
        )],
    ));
}

/// One rarity up, stopping at the top.
pub fn next_tier(tier: Tier) -> Tier {
    match tier {
        Tier::Common => Tier::Uncommon,
        Tier::Uncommon => Tier::Rare,
        Tier::Rare => Tier::Epic,
        Tier::Epic => Tier::Mythic,
        Tier::Mythic | Tier::Ancient => Tier::Ancient,
    }
}

/// The gem cutting table: 4 gem dust out of the inventory raise the held gem's
/// purity a step.
fn cut(host: &mut dyn Host, slot: i32, main: &ItemStack) {
    if gem(&main.item).is_none() {
        return;
    }
    let purity = purity_of(main);
    if purity == Purity::Perfect {
        return;
    }
    let taken = host.take_items("apotheosis:gem_dust", POLISH_COST);
    if taken < POLISH_COST {
        if taken > 0 {
            let _ = host.give_items("apotheosis:gem_dust", taken);
        }
        host.feedback(Text::translate_with_fallback(
            "apotheosis.cut.no_dust",
            "The gem cutting table needs %s gem dust.",
            &[Text::literal(POLISH_COST.to_string())],
        ));
        return;
    }
    let mut improved = main.clone();
    set_purity(&mut improved, purity.next());
    write_slot(host, slot, improved);
    host.feedback(Text::translate_with_fallback(
        "apotheosis.polish.done",
        "Purity raised to %s.",
        &[Text::translate_with_fallback(
            purity.next().name_key(),
            purity.next().name(),
            &[],
        )],
    ));
}

/// The purity a gem from a chest rolls: the ordinary weights, drawn from the
/// loot roll's own seed so the table stays deterministic.
pub fn chest_gem_purity(seed: i64) -> Purity {
    let mut rng = Rng::new(seed as u64);
    roll_purity(&ORDINARY_PURITY, &mut rng)
}

/// A gem in a chest rolls its purity from the loot roll's own seed.
pub fn loot_function(event: &mut LootFunctionEvent<'_>) {
    if event.kind != "apotheosis:gem_purity" {
        return;
    }
    if gem(&event.stack.item).is_none() {
        return;
    }
    let purity = chest_gem_purity(event.ctx.seed);
    let custom = serde_json::json!({ PURITY_KEY: purity.index() });
    event.stack.components.retain(|(id, _)| id != CUSTOM);
    event
        .stack
        .components
        .push((CUSTOM.to_owned(), custom));
}

/// Rerolls a piece's rarity and affixes in place, keeping its sockets and the
/// gems in them; returns the rarity it rolled.
fn reroll(host: &mut dyn Host, slot: i32, gear: &ItemStack) -> Tier {
    let mut rng = Rng::new(next_roll());
    let tier = rng.tier(&crate::ORDINARY_TIERS);
    let mut piece = gear.clone();
    apply_affixes(&mut piece, tier, &mut rng);
    set_custom(&mut piece, |map| {
        map.insert(TIER_KEY.into(), Tag::Int(tier as i32));
    });
    write_sockets(&mut piece, socket_total(gear), &socketed(gear));
    refresh(&mut piece);
    write_slot(host, slot, piece);
    tier
}

/// A stack with nothing left in it: `carried_stack` empties the slot when the
/// count reaches 0.
fn emptied(stack: &ItemStack) -> ItemStack {
    let mut stack = stack.clone();
    stack.count = 0;
    stack
}

fn write_slot(host: &mut dyn Host, slot: i32, stack: ItemStack) {
    if let Some(current) = host.carried_stack(slot) {
        *current = stack;
    }
}

fn socket(host: &mut dyn Host, slot: i32, main: &ItemStack, off: &ItemStack) {
    let Some(entry) = gem(&main.item) else {
        return;
    };
    let purity = purity_of(main);
    let mut gear = off.clone();
    if !insert_gem(&mut gear, &main.item, purity) {
        host.feedback(Text::translate_with_fallback(
            "apotheosis.socket.full",
            "No free socket there, or that gem is already in it.",
            &[],
        ));
        return;
    }
    write_slot(host, OFFHAND, gear);
    let mut left = main.clone();
    left.count = left.count.saturating_sub(1);
    write_slot(host, slot, left);
    host.feedback(Text::translate_with_fallback(
        "apotheosis.socket.done",
        "Socketed %s (%s).",
        &[
            Text::translate_with_fallback(
                &gem_name_key(&main.item),
                entry.id.rsplit(':').next().unwrap_or(entry.id),
                &[],
            ),
            Text::translate_with_fallback(purity.name_key(), purity.name(), &[]),
        ],
    ));
}

fn can_polish(main: &ItemStack, off: &ItemStack) -> bool {
    &*off.item == "apotheosis:gem_dust"
        && off.count >= POLISH_COST
        && purity_of(main) < Purity::Perfect
}

fn polish(host: &mut dyn Host, slot: i32, main: &ItemStack, off: &ItemStack) {
    if !can_polish(main, off) {
        return;
    }
    let purity = purity_of(main);
    let mut improved = main.clone();
    set_purity(&mut improved, purity.next());
    write_slot(host, slot, improved);
    let mut dust = off.clone();
    dust.count -= POLISH_COST;
    write_slot(host, OFFHAND, dust);
    host.feedback(Text::translate_with_fallback(
        "apotheosis.polish.done",
        "Purity raised to %s.",
        &[Text::translate_with_fallback(
            purity.next().name_key(),
            purity.next().name(),
            &[],
        )],
    ));
}

fn reforge(host: &mut dyn Host, slot: i32, main: &ItemStack, player: u128) {
    // The command charged for this tick; if the hand changed in between, the
    // levels go back rather than reroll something the player did not ask for.
    if !can_reforge(main) {
        let _ = host.give_levels(player, crate::REFORGE_COST);
        host.feedback(Text::translate_with_fallback(
            "apotheosis.reforge.not_affixed",
            "Hold an affixed piece to reforge it.",
            &[],
        ));
        return;
    }
    let tier = reroll(host, slot, main);
    host.feedback(Text::translate_with_fallback(
        "apotheosis.reforge.done",
        "Reforged: %s.",
        &[Text::translate_with_fallback(
            tier.name_key(),
            tier.name(),
            &[],
        )],
    ));
}

fn salvage(host: &mut dyn Host, slot: i32, main: &ItemStack, player: u128) {
    if !has_affixes(main) {
        host.feedback(Text::translate_with_fallback(
            "apotheosis.salvage.not_affixed",
            "Hold an affixed piece to salvage it.",
            &[],
        ));
        return;
    }
    let tier = tier_of(main).unwrap_or(Tier::Common);
    let mut rng = Rng::new(next_roll());
    let (material, count, dust) = salvage_yield(tier, &mut rng);
    let gems = socketed(main);
    write_slot(host, slot, emptied(main));
    let given = host.give_items(&material, count);
    if dust > 0 {
        let _ = host.give_items("apotheosis:gem_dust", dust);
    }
    // The gems come back at the purity they were socketed with.
    for socket in gems {
        let mut gem = ItemStack::new(&socket.gem, 1);
        set_purity(&mut gem, socket.purity);
        let _ = host.give_stack(player, gem);
    }
    host.feedback(Text::translate_with_fallback(
        "apotheosis.salvage.done",
        "Salvaged into %s.",
        &[Text::translate_with_fallback(
            &format!("item.{}", material.replacen(':', ".", 1)),
            material.rsplit(':').next().unwrap_or(&material),
            &[],
        )],
    ));
    if given < count {
        host.feedback(Text::translate_with_fallback(
            "apotheosis.salvage.no_room",
            "Some of it did not fit in your inventory.",
            &[],
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn purity_steps_multiply_the_bonus_and_top_out_at_double() {
        let mut last = 0.0;
        for purity in Purity::ALL {
            assert!(purity.multiplier() > last, "{purity:?} did not grow");
            last = purity.multiplier();
        }
        assert_eq!(Purity::Normal.multiplier(), 1.0);
        assert_eq!(Purity::Perfect.multiplier(), 2.0);
        assert_eq!(Purity::Perfect.next(), Purity::Perfect);
        assert_eq!(Purity::Cracked.next(), Purity::Chipped);
    }

    #[test]
    fn sockets_never_exceed_three_and_rise_with_the_tier() {
        let mut rng = Rng::new(3);
        for tier in [
            Tier::Common,
            Tier::Uncommon,
            Tier::Rare,
            Tier::Epic,
            Tier::Mythic,
            Tier::Ancient,
        ] {
            for _ in 0..64 {
                let count = socket_count(tier, &mut rng);
                assert!(count <= MAX_SOCKETS, "{tier:?} rolled {count} sockets");
                if tier == Tier::Ancient {
                    assert_eq!(count, MAX_SOCKETS);
                }
                if tier == Tier::Common {
                    assert_eq!(count, 0);
                }
            }
        }
        let mut rng = Rng::new(9);
        assert_eq!(socket_count(Tier::Ancient, &mut rng), 3);
    }

    #[test]
    fn a_gem_fills_a_socket_once_and_scales_with_purity() {
        let mut gear = ItemStack::new("minecraft:diamond_sword", 1);
        let mut rng = Rng::new(5);
        after_affixes(&mut gear, Tier::Epic, &mut rng);
        assert_eq!(socket_total(&gear), 3);
        assert!(insert_gem(&mut gear, "apotheosis:gem_ruby", Purity::Flawless));
        assert!(!insert_gem(&mut gear, "apotheosis:gem_ruby", Purity::Perfect));
        assert_eq!(socketed(&gear).len(), 1);

        let Some(Tag::List(modifiers)) = gear
            .data()
            .and_then(|data| data.other("minecraft:attribute_modifiers"))
        else {
            panic!("no modifiers");
        };
        let socket_modifier = modifiers
            .iter()
            .find(|entry| is_socket_modifier(entry))
            .expect("a socket modifier");
        assert_eq!(
            socket_modifier.get("amount"),
            Some(&Tag::Double(
                socketed_amount("apotheosis:gem_ruby", Purity::Flawless)
                    .unwrap()
                    .to_bits()
            ))
        );
        assert_eq!(
            socketed_amount("apotheosis:gem_ruby", Purity::Perfect),
            Some(2.0)
        );
    }

    #[test]
    fn a_socketed_gem_on_a_shield_targets_the_hand_slot_group() {
        let mut shield = ItemStack::new("minecraft:shield", 1);
        write_sockets(&mut shield, 1, &[]);
        assert!(insert_gem(
            &mut shield,
            "apotheosis:gem_ruby",
            Purity::Normal
        ));

        let Some(Tag::List(modifiers)) = shield
            .data()
            .and_then(|data| data.other("minecraft:attribute_modifiers"))
        else {
            panic!("no modifiers");
        };
        let socket_modifier = modifiers
            .iter()
            .find(|entry| is_socket_modifier(entry))
            .expect("a socket modifier");
        assert_eq!(socket_modifier.get("slot"), Some(&Tag::String("hand".into())));
        assert_eq!(socket_slot_group("minecraft:shield"), "hand");
        assert_eq!(socket_slot_group("minecraft:diamond_chestplate"), "armor");
        assert_eq!(socket_slot_group("minecraft:diamond_sword"), "hand");
    }

    #[test]
    fn polishing_rejects_a_non_gem_in_the_main_hand() {
        let sword = ItemStack::new("minecraft:diamond_sword", 1);
        let dust = ItemStack::new("apotheosis:gem_dust", POLISH_COST);
        assert!(!can_polish(&sword, &dust));
    }

    #[test]
    fn a_socketed_gem_keeps_the_affix_modifiers() {
        let mut gear = ItemStack::new("minecraft:diamond_chestplate", 1);
        let mut rng = Rng::new(11);
        apply_affixes(&mut gear, Tier::Ancient, &mut rng);
        after_affixes(&mut gear, Tier::Ancient, &mut rng);
        let before = match gear
            .data()
            .and_then(|data| data.other("minecraft:attribute_modifiers"))
        {
            Some(Tag::List(entries)) => entries.len(),
            _ => panic!("no modifiers"),
        };
        assert!(insert_gem(&mut gear, "apotheosis:gem_jade", Purity::Normal));
        let after = match gear
            .data()
            .and_then(|data| data.other("minecraft:attribute_modifiers"))
        {
            Some(Tag::List(entries)) => entries.len(),
            _ => panic!("no modifiers"),
        };
        assert_eq!(after, before + 1, "the affix modifiers were dropped");
    }

    #[test]
    fn a_piece_without_sockets_cannot_take_a_gem() {
        let mut gear = ItemStack::new("minecraft:iron_sword", 1);
        assert_eq!(socket_total(&gear), 0);
        assert!(!insert_gem(&mut gear, "apotheosis:gem_onyx", Purity::Normal));
    }

    #[test]
    fn reforging_keeps_the_socketed_gems() {
        let mut gear = ItemStack::new("minecraft:diamond_sword", 1);
        let mut rng = Rng::new(21);
        after_affixes(&mut gear, Tier::Epic, &mut rng);
        assert!(insert_gem(&mut gear, "apotheosis:gem_topaz", Purity::Chipped));
        let sockets = socket_total(&gear);

        // What `reforge` does to the stack.
        let mut rerolled = gear.clone();
        let kept_gems = socketed(&gear);
        let mut roll = Rng::new(77);
        apply_affixes(&mut rerolled, Tier::Ancient, &mut roll);
        set_custom(&mut rerolled, |map| {
            map.insert(TIER_KEY.into(), Tag::Int(Tier::Ancient as i32));
        });
        write_sockets(&mut rerolled, sockets, &kept_gems);
        refresh(&mut rerolled);

        assert_eq!(socket_total(&rerolled), sockets);
        assert_eq!(socketed(&rerolled), kept_gems);
        assert!(rerolled
            .data()
            .and_then(|data| data.other("minecraft:attribute_modifiers"))
            .is_some_and(|tag| match tag {
                Tag::List(entries) => entries.iter().any(is_socket_modifier),
                _ => false,
            }));
    }

    #[test]
    fn every_tier_salvages_into_its_own_material() {
        let mut rng = Rng::new(4);
        assert_eq!(salvage_material(Tier::Common).0, "apotheosis:mysterious_scrap_metal");
        assert_eq!(salvage_material(Tier::Uncommon).0, "apotheosis:timeworn_fabric");
        assert_eq!(salvage_material(Tier::Rare).0, "apotheosis:luminous_crystal_shard");
        assert_eq!(salvage_material(Tier::Epic).0, "apotheosis:arcane_sands");
        assert_eq!(salvage_material(Tier::Mythic).0, "apotheosis:godforged_pearl");
        for tier in [Tier::Common, Tier::Rare, Tier::Ancient] {
            let (material, count, _) = salvage_yield(tier, &mut rng);
            assert_eq!(material, salvage_material(tier).0);
            assert!((1..=salvage_material(tier).1).contains(&u64::from(count)));
        }
    }

    #[test]
    fn a_dropped_gem_says_its_purity_and_crafted_ones_are_normal() {
        let mut stack = ItemStack::new("apotheosis:gem_opal", 1);
        assert_eq!(purity_of(&stack), Purity::Normal);
        let mut rng = Rng::new(13);
        roll_dropped_purity(&mut stack, &mut rng, true);
        assert!(purity_of(&stack) >= Purity::Cracked);
        set_purity(&mut stack, Purity::Perfect);
        assert_eq!(purity_of(&stack), Purity::Perfect);

        let mut sword = ItemStack::new("minecraft:iron_sword", 1);
        roll_dropped_purity(&mut sword, &mut rng, false);
        assert!(!has_affixes(&sword), "a plain item got gem data");
    }

    #[test]
    fn a_piece_records_its_tier_for_a_later_salvage() {
        let mut gear = ItemStack::new("minecraft:bow", 1);
        let mut rng = Rng::new(2);
        after_affixes(&mut gear, Tier::Mythic, &mut rng);
        assert_eq!(tier_of(&gear), Some(Tier::Mythic));
        assert!(can_reforge(&gear));
        assert!(!can_reforge(&ItemStack::new("minecraft:bow", 1)));
    }

    #[test]
    fn amounts_read_cleanly_in_a_tooltip() {
        assert_eq!(amount_text(2.0), "+2");
        assert_eq!(amount_text(0.5), "+0.5");
        assert_eq!(amount_text(0.002), "+0.002");
    }

    #[test]
    fn the_four_sigils_and_three_tables_are_recognised() {
        assert_eq!(seal("apotheosis:sigil_of_socketing"), Some(Seal::Socketing));
        assert_eq!(seal("apotheosis:sigil_of_withdrawal"), Some(Seal::Withdrawal));
        assert_eq!(seal("apotheosis:sigil_of_rebirth"), Some(Seal::Rebirth));
        assert_eq!(
            seal("apotheosis:sigil_of_enhancement"),
            Some(Seal::Enhancement)
        );
        assert_eq!(seal("apotheosis:gem_opal"), None);

        assert_eq!(table("apotheosis:reforging_table"), Some(Table::Reforging));
        assert_eq!(table("apotheosis:salvaging_table"), Some(Table::Salvaging));
        assert_eq!(table("apotheosis:gem_cutting_table"), Some(Table::Cutting));
        assert_eq!(table("minecraft:crafting_table"), None);
    }

    #[test]
    fn enhancing_rises_one_rarity_and_stops_at_ancient() {
        assert_eq!(next_tier(Tier::Common), Tier::Uncommon);
        assert_eq!(next_tier(Tier::Mythic), Tier::Ancient);
        assert_eq!(next_tier(Tier::Ancient), Tier::Ancient);
    }

    #[test]
    fn a_chest_gem_rolls_a_purity_from_its_seed_alone() {
        assert_eq!(chest_gem_purity(42), chest_gem_purity(42));
        // The weights are ordinary ones, so the seed range fills the low end.
        let mut seen: Vec<Purity> = Vec::new();
        for seed in 0..200 {
            let purity = chest_gem_purity(seed);
            if !seen.contains(&purity) {
                seen.push(purity);
            }
        }
        assert!(seen.len() >= 4, "the roll is too narrow: {seen:?}");
        assert!(!seen.contains(&Purity::Perfect) || seen.len() >= 4);
    }
}
