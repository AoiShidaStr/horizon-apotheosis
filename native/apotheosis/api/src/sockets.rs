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
    Compound, EquipmentSlot, Flow, Host, InventoryTickEvent, ItemStack, LootFunctionEvent,
    LootStack, NativeLootContext, Tag, Text, TooltipEvent, UseItemEvent, UseItemOnEvent, serde_json,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::affixes;
use crate::attributes::{self, Attr};
use crate::gems::{self, Resolved};
use crate::{apply_affixes, is_equipment, Rng, Tier};

/// The off hand's slot in `native_inventory`'s `Player` numbering.
const OFFHAND: i32 = -106;
/// The Java `apotheosis:purity_upgrade` gem-cutting recipes, by the purity
/// they reach: the gem dust count first, then the rarity materials
/// (`data/apotheosis/recipe/gem_cutting/<purity>.json`).  Java defines no
/// recipe for `perfect`, so a gem stops at `flawless` at the table.
pub const UPGRADE_COSTS: &[(Purity, u32, &[(&str, u32)])] = &[
    (
        Purity::Cracked,
        1,
        &[
            ("apotheosis:mysterious_scrap_metal", 3),
            ("apotheosis:timeworn_fabric", 1),
        ],
    ),
    (
        Purity::Chipped,
        3,
        &[
            ("apotheosis:mysterious_scrap_metal", 9),
            ("apotheosis:timeworn_fabric", 3),
            ("apotheosis:luminous_crystal_shard", 1),
        ],
    ),
    (
        Purity::Flawed,
        5,
        &[
            ("apotheosis:mysterious_scrap_metal", 27),
            ("apotheosis:timeworn_fabric", 9),
            ("apotheosis:luminous_crystal_shard", 3),
            ("apotheosis:arcane_sands", 1),
        ],
    ),
    (
        Purity::Normal,
        7,
        &[
            ("apotheosis:timeworn_fabric", 27),
            ("apotheosis:luminous_crystal_shard", 9),
            ("apotheosis:arcane_sands", 3),
            ("apotheosis:godforged_pearl", 1),
        ],
    ),
    (
        Purity::Flawless,
        9,
        &[
            ("apotheosis:luminous_crystal_shard", 27),
            ("apotheosis:arcane_sands", 9),
            ("apotheosis:godforged_pearl", 3),
        ],
    ),
];

/// What raising a gem to `target` costs, or none where Java has no recipe.
pub fn upgrade_cost(target: Purity) -> Option<(u32, &'static [(&'static str, u32)])> {
    UPGRADE_COSTS
        .iter()
        .find(|(purity, _, _)| *purity == target)
        .map(|(_, dust, materials)| (*dust, *materials))
}

/// Java's `data/apotheosis/recipe/reforging/*.json`, by the rarity reforged:
/// experience levels, rarity material and sigils of rebirth. Ancient is this
/// port's extension and reuses the mythic row, as it does for affix ranges.
pub const REFORGE_COSTS: &[(Tier, i32, u32, u32)] = &[
    (Tier::Common, 2, 1, 0),
    (Tier::Uncommon, 5, 2, 1),
    (Tier::Rare, 15, 2, 2),
    (Tier::Epic, 30, 2, 4),
    (Tier::Mythic, 50, 3, 5),
];

/// What reforging a piece of `tier` at `table` costs, or none where Java's
/// recipe does not name that table. The simple table serves common, uncommon
/// and rare only; the reforging table serves every rarity.
pub fn reforge_cost(tier: Tier, table: Table) -> Option<(i32, u32, u32)> {
    let row = REFORGE_COSTS
        .iter()
        .find(|(rarity, ..)| *rarity == tier || (tier == Tier::Ancient && *rarity == Tier::Mythic))?;
    let (_, levels, material, sigils) = *row;
    if table == Table::SimpleReforging && tier > Tier::Rare {
        return None;
    }
    Some((levels, material, sigils))
}

/// Java's `recipe/salvaging/gem/*.json`: the gem dust a gem of each purity
/// yields, inclusive at both ends.
pub const GEM_SALVAGE: &[(Purity, u32, u32)] = &[
    (Purity::Cracked, 1, 2),
    (Purity::Chipped, 1, 3),
    (Purity::Flawed, 2, 4),
    (Purity::Normal, 3, 5),
    (Purity::Flawless, 4, 7),
    (Purity::Perfect, 5, 10),
];

/// How much gem dust salvaging a gem of `purity` gives.
pub fn gem_salvage_yield(purity: Purity, rng: &mut Rng) -> u32 {
    let (_, low, high) = GEM_SALVAGE
        .iter()
        .find(|(step, ..)| *step == purity)
        .copied()
        .unwrap_or((Purity::Normal, 3, 5));
    low + rng.below(u64::from(high - low) + 1) as u32
}

/// The most sockets any piece can have.
pub const MAX_SOCKETS: u8 = 3;

const CUSTOM: &str = "minecraft:custom_data";
const SOCKETS_KEY: &str = "apotheosis:sockets";
const SOCKETED_KEY: &str = "apotheosis:socketed";
const TIER_KEY: &str = "apotheosis:tier";
/// What the last `refresh` wrote into the item, so a reforge can take it back.
const SOCKET_ATTRS_KEY: &str = "apotheosis:socket_attrs";
const SOCKET_ENCHANTS_KEY: &str = "apotheosis:socket_enchants";

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

/// Marks a gem stack with the purity it dropped at: a gem becomes the item of
/// that purity; one below its floor, which has no item, keeps the purity in
/// its custom data.
pub fn set_purity(stack: &mut ItemStack, purity: Purity) {
    if gems::change_purity(stack, purity) {
        return;
    }
    set_custom(stack, |map| {
        map.insert(gems::PURITY_KEY.into(), Tag::Int(purity.index() as i32));
    });
}

/// The purity a gem stack carries: its custom data's, else its item id's; an
/// unmarked generic gem is normal.
pub fn purity_of(stack: &ItemStack) -> Purity {
    gems::purity_of(stack)
}

/// Gives a freshly dropped gem its purity, never below the floor its Java data
/// sets for it.
pub fn roll_dropped_purity(stack: &mut ItemStack, rng: &mut Rng, boss: bool) {
    let Some(gem) = gems::of_stack(stack).map(|(gem, _)| gem) else {
        return;
    };
    let rolled = roll_purity(if boss { &BOSS_PURITY } else { &ORDINARY_PURITY }, rng);
    set_purity(stack, rolled.max(gem.min_purity));
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

/// Puts gem `gem_id` at `purity` into `gear`'s first free socket. False when
/// the gear has no free socket, already holds that gem, or the gem has no bonus
/// for a piece of this kind.
pub fn insert_gem(gear: &mut ItemStack, gem_id: &str, purity: Purity) -> bool {
    let Some(entry) = gems::get(gem_id) else {
        return false;
    };
    if gems::resolved(entry, purity, &gear.item).is_empty() {
        return false;
    }
    let total = socket_total(gear);
    let mut sockets = socketed(gear);
    if sockets.len() >= usize::from(total) || sockets.iter().any(|s| s.gem == entry.id) {
        return false;
    }
    sockets.push(Socket {
        gem: entry.id.to_owned(),
        purity,
    });
    write_sockets(gear, total, &sockets);
    refresh(gear);
    true
}

/// The rarity material a piece of `tier` salvages into, and how many at most.
pub fn salvage_material(tier: Tier) -> (&'static str, u64) {
    match tier {
        Tier::Common => ("apotheosis:mysterious_scrap_metal", 4),
        Tier::Uncommon => ("apotheosis:timeworn_fabric", 4),
        Tier::Rare => ("apotheosis:luminous_crystal_shard", 4),
        Tier::Epic => ("apotheosis:arcane_sands", 4),
        Tier::Mythic => ("apotheosis:godforged_pearl", 4),
        // Ancient is this port's top step; Java's mythic salvaging recipe is
        // the closest one, so it gives the same 1..=4 pearls.
        Tier::Ancient => ("apotheosis:godforged_pearl", 4),
    }
}

/// What a salvage gives: 1..=max of the tier's material, by the same roll
/// Apotheosis' `recipe/salvaging/*.json` use. Those recipes name the rarity
/// material alone, so no gem dust is added here.
pub fn salvage_yield(tier: Tier, rng: &mut Rng) -> (String, u32) {
    let (material, max) = salvage_material(tier);
    let count = 1 + rng.below(max) as u32;
    (material.to_owned(), count)
}

/// The rarity component's name, for the tooltip line of a socketed gem.
fn gem_name_key(gem_id: &str) -> String {
    format!("item.apotheosis.gem.{gem_id}")
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
            if [
                "apotheosis.socket.line",
                "apotheosis.socket.header",
                "apotheosis.socket.bonus",
                "apotheosis.socket.total",
            ]
            .contains(&key.to_string_lossy().as_ref())
    )
}

/// The equipment slot group a socketed gem's modifier belongs to, from the
/// item itself: Java's `EquipmentSlotGroup` follows the piece, so a sword
/// works in the main hand only, a shield in the off hand, and each piece of
/// armour in its own armour slot. `hand` would let a socketed sword pay out
/// its bonus from the off hand too.
fn socket_slot_group(item: &str) -> &'static str {
    if item == "minecraft:shield" {
        return "offhand";
    }
    for (suffix, group) in [
        ("_helmet", "head"),
        ("_chestplate", "chest"),
        ("_leggings", "legs"),
        ("_boots", "feet"),
    ] {
        if item.ends_with(suffix) {
            return group;
        }
    }
    "mainhand"
}

/// Rewrites the socket half of a piece's modifiers and lore, keeping the
/// affixes' own entries: called after socketing, after a reforge, and once when
/// the sockets are rolled.
fn refresh(stack: &mut ItemStack) {
    undo_socket_extras(stack);
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

    let mut apothic: Vec<(Attr, f64)> = Vec::new();
    let mut plan: Vec<(String, u8, bool)> = Vec::new();
    for (index, socket) in sockets.iter().enumerate() {
        let Some(entry) = gems::get(&socket.gem) else {
            continue;
        };
        let resolved = gems::resolved(entry, socket.purity, &stack.item);
        for (n, bonus) in resolved.iter().enumerate() {
            match bonus {
                Resolved::Attribute {
                    attribute,
                    operation,
                    amount,
                } => {
                    let id = format!("apotheosis:socket_{index}_{n}");
                    modifiers.push(Tag::compound([
                        ("type", Tag::String((*attribute).into())),
                        ("id", Tag::String(id.as_str().into())),
                        ("amount", Tag::Double(amount.to_bits())),
                        ("operation", Tag::String((*operation).into())),
                        ("slot", Tag::String(slots.into())),
                    ]));
                }
                Resolved::Enchantment {
                    id,
                    level,
                    existing_only,
                } => plan.push(((*id).to_owned(), *level, *existing_only)),
            }
        }
        apothic.extend(gems::apothic(&resolved));
        lore.extend(socket_lore(socket, entry, &stack.item));
    }

    if total > 0 {
        lore.push(Tag::compound([
            (
                "translate",
                Tag::String("apotheosis.socket.total".into()),
            ),
            (
                "fallback",
                Tag::String(format!("Sockets: {}/{total}", sockets.len()).as_str().into()),
            ),
            ("color", Tag::String("#808080".into())),
            ("italic", Tag::Byte(0)),
            (
                "with",
                Tag::List(vec![
                    Tag::String(sockets.len().to_string().as_str().into()),
                    Tag::String(total.to_string().as_str().into()),
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

    // The Apothic Attributes live in the item's custom data, not in the
    // modifier list; what this refresh wrote is recorded so the next one can
    // take it back instead of stacking it.
    if !apothic.is_empty() {
        attributes::attach(stack, &apothic);
        let mut entries = Compound::new();
        for (attr, amount) in &apothic {
            entries.insert(attr.key().to_string().as_str().into(), Tag::Double(amount.to_bits()));
        }
        set_custom(stack, |map| {
            map.insert(SOCKET_ATTRS_KEY.into(), Tag::Compound(entries));
        });
    }
    let added = apply_enchantments(stack, &plan);
    if !added.is_empty() {
        let mut entries = Compound::new();
        for (id, level) in &added {
            entries.insert(id.as_str().into(), Tag::Int(i32::from(*level)));
        }
        set_custom(stack, |map| {
            map.insert(SOCKET_ENCHANTS_KEY.into(), Tag::Compound(entries));
        });
    }
}

/// Takes back the attributes and enchantment levels the last refresh wrote, so
/// a reforge or a withdrawn gem leaves no bonus behind.
fn undo_socket_extras(stack: &mut ItemStack) {
    let previous = custom(stack);
    if let Some(Tag::Compound(attrs)) = previous.get(SOCKET_ATTRS_KEY) {
        let pairs: Vec<(Attr, f64)> = attrs
            .iter()
            .filter_map(|(key, value)| {
                let attr = Attr::ALL
                    .into_iter()
                    .find(|attr| attr.key() == key.to_string_lossy().to_string())?;
                let amount = match value {
                    Tag::Double(bits) => f64::from_bits(*bits),
                    _ => return None,
                };
                Some((attr, -amount))
            })
            .collect();
        if !pairs.is_empty() {
            attributes::attach(stack, &pairs);
        }
    }
    if let Some(Tag::Compound(levels)) = previous.get(SOCKET_ENCHANTS_KEY) {
        let mut current = enchantment_levels(stack);
        for (id, value) in levels.iter() {
            let Tag::Int(level) = value else {
                continue;
            };
            let key = id.to_string_lossy();
            let before = current.get(&key.to_string()).copied().unwrap_or(0);
            let left = before.saturating_sub(u8::try_from(*level).unwrap_or(0));
            if left == 0 {
                current.remove(&key.to_string());
            } else {
                current.insert(key.to_string(), left);
            }
        }
        write_enchantment_levels(stack, &current);
    }
    set_custom(stack, |map| {
        let attrs_key = SOCKET_ATTRS_KEY.into();
        map.remove(&attrs_key);
        let enchants_key = SOCKET_ENCHANTS_KEY.into();
        map.remove(&enchants_key);
    });
}

/// A piece's enchantment levels, by id.
fn enchantment_levels(stack: &ItemStack) -> std::collections::BTreeMap<String, u8> {
    stack
        .enchantments()
        .iter()
        .map(|(id, level)| (id.to_string(), (*level).min(255) as u8))
        .collect()
}

/// Writes `levels` as the item's `minecraft:enchantments` component.
fn write_enchantment_levels(stack: &mut ItemStack, levels: &std::collections::BTreeMap<String, u8>) {
    if levels.is_empty() {
        stack
            .data_mut()
            .set_loot_component("minecraft:enchantments", &serde_json::Value::Null);
        return;
    }
    let mut map = serde_json::Map::new();
    for (id, level) in levels {
        map.insert(id.clone(), serde_json::json!(*level));
    }
    stack
        .data_mut()
        .set_loot_component("minecraft:enchantments", &serde_json::json!({ "levels": map }));
}

/// Raises the planned enchantments, returning the levels it actually added so
/// the next refresh can take them back.
fn apply_enchantments(stack: &mut ItemStack, plan: &[(String, u8, bool)]) -> Vec<(String, u8)> {
    let mut levels = enchantment_levels(stack);
    let mut added = Vec::new();
    for (id, level, existing_only) in plan {
        let current = levels.get(id).copied().unwrap_or(0);
        if *existing_only && current == 0 {
            continue;
        }
        if *level <= current {
            continue;
        }
        levels.insert(id.clone(), *level);
        added.push((id.clone(), *level - current));
    }
    if !added.is_empty() {
        write_enchantment_levels(stack, &levels);
    }
    added
}

fn socket_lore(socket: &Socket, entry: &crate::java_gems::Gem, item: &str) -> Vec<Tag> {
    let colour = socket.purity.hex();
    let gem = || {
        Tag::compound([
            (
                "translate",
                Tag::String(format!("item.apotheosis.gem.{}", entry.id).as_str().into()),
            ),
            ("fallback", Tag::String(entry.name.into())),
        ])
    };
    let purity = || {
        Tag::compound([
            ("translate", Tag::String(socket.purity.name_key().into())),
            ("fallback", Tag::String(socket.purity.name().into())),
        ])
    };
    let mut lines = vec![Tag::compound([
        (
            "translate",
            Tag::String("apotheosis.socket.header".into()),
        ),
        (
            "fallback",
            Tag::String(
                format!("{} {}", socket.purity.name(), entry.name)
                    .as_str()
                    .into(),
            ),
        ),
        ("color", Tag::String(colour.as_str().into())),
        ("italic", Tag::Byte(0)),
        ("with", Tag::List(vec![gem(), purity()])),
    ])];
    for line in gems::description_for(entry, socket.purity, item) {
        let fallback = format!("{} {}: {}", socket.purity.name(), entry.name, line.fallback);
        let bonus = Tag::compound([
            ("translate", Tag::String(line.key.as_str().into())),
            (
                "fallback",
                Tag::String(line.fallback.as_str().into()),
            ),
            (
                "with",
                Tag::List(vec![Tag::String(line.value.as_str().into())]),
            ),
        ]);
        lines.push(Tag::compound([
            (
                "translate",
                Tag::String("apotheosis.socket.bonus".into()),
            ),
            ("fallback", Tag::String(fallback.as_str().into())),
            ("color", Tag::String(colour.as_str().into())),
            ("italic", Tag::Byte(0)),
            ("with", Tag::List(vec![gem(), purity(), bonus])),
        ]));
    }
    lines
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
    if gems::of_stack(event.stack).is_some() {
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
        let Some(entry) = gems::get(&socket.gem) else {
            continue;
        };
        let colour = socket.purity.colour();
        let gem_text = || {
            Text::translate_with_fallback(
                &format!("item.apotheosis.gem.{}", entry.id),
                entry.name,
                &[],
            )
        };
        let purity_text = || {
            Text::translate_with_fallback(
                socket.purity.name_key(),
                socket.purity.name(),
                &[],
            )
        };
        event.lines.push((
            Text::translate_with_fallback(
                "apotheosis.socket.header",
                &format!("{} {}", socket.purity.name(), entry.name),
                &[gem_text(), purity_text()],
            )
            .with_color(colour),
            colour,
        ));
        for line in gems::description_for(entry, socket.purity, &event.stack.item) {
            event.lines.push((
                Text::translate_with_fallback(
                    &line.key,
                    &line.fallback,
                    &[Text::literal(line.value)],
                )
                .with_color(colour),
                colour,
            ));
        }
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
    /// Removes the affix name this port wrote, never a player's own name.
    Unnaming,
}

impl Seal {
    pub fn desc_key(self) -> &'static str {
        match self {
            Seal::Socketing => "item.apotheosis.sigil_of_socketing.desc",
            Seal::Withdrawal => "item.apotheosis.sigil_of_withdrawal.desc",
            Seal::Rebirth => "item.apotheosis.sigil_of_rebirth.desc",
            Seal::Enhancement => "item.apotheosis.sigil_of_enhancement.desc",
            Seal::Unnaming => "item.apotheosis.sigil_of_unnaming.desc",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Seal::Socketing => "Adds one socket to an item",
            Seal::Withdrawal => "Removes all socketed gems from an item",
            Seal::Rebirth => "Rerolls an affixed item's rarity and affixes",
            Seal::Enhancement => "Raises an affixed item's rarity by one step",
            Seal::Unnaming => "Removes the affix name from an item",
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
        "apotheosis:sigil_of_unnaming" => Some(Seal::Unnaming),
        _ => None,
    }
}

/// The working block an id is: the three tables, each doing on a right click
/// what its command does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Table {
    SimpleReforging,
    Reforging,
    Salvaging,
    Cutting,
}

pub fn table(block: &str) -> Option<Table> {
    match block {
        "apotheosis:simple_reforging_table" => Some(Table::SimpleReforging),
        "apotheosis:reforging_table" => Some(Table::Reforging),
        "apotheosis:salvaging_table" => Some(Table::Salvaging),
        "apotheosis:gem_cutting_table" => Some(Table::Cutting),
        _ => None,
    }
}

/// What a right click with a gem or a sigil queued, applied on the next
/// inventory tick of the stack it started from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Socket,
    Polish,
    Salvage,
    Cut,
    AddSocket,
    Withdraw,
    Rebirth,
    Enhance,
    Unnaming,
}

/// What a delayed action was validated on: the stacks' item ids and counts,
/// the gear's tier and sockets, a gem's purity and the ids of the modifiers a
/// piece carries. Two pieces of the same item differ by these, so the tick can
/// tell the piece the player clicked from another one that took its place.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Fingerprint {
    item: String,
    count: u32,
    tier: Option<Tier>,
    sockets: u8,
    socketed: Vec<Socket>,
    purity: Purity,
    modifiers: Vec<String>,
}

fn fingerprint(stack: &ItemStack) -> Fingerprint {
    let mut modifiers: Vec<String> = match stack
        .data()
        .and_then(|data| data.other("minecraft:attribute_modifiers"))
    {
        Some(Tag::List(entries)) => entries
            .iter()
            .filter_map(|entry| match entry.get("id") {
                Some(Tag::String(id)) => Some(id.to_string_lossy()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    modifiers.sort();
    Fingerprint {
        item: stack.item.to_string(),
        count: stack.count,
        tier: tier_of(stack),
        sockets: socket_total(stack),
        socketed: socketed(stack),
        purity: purity_of(stack),
        modifiers,
    }
}

/// An action waiting for the next tick, with the slot it came from and the
/// stacks it was validated on.
#[derive(Clone, Debug, PartialEq)]
struct Pending {
    action: Action,
    /// The inventory slot the click started from (the main hand's).
    slot: i32,
    /// The stack in that slot as the click saw it.
    main: Fingerprint,
    /// The off-hand piece the click saw, for the actions that work on one.
    target: Option<Fingerprint>,
}

impl Pending {
    fn new(action: Action, slot: i32, main: &ItemStack, target: Option<&ItemStack>) -> Self {
        Self {
            action,
            slot,
            main: fingerprint(main),
            target: target.map(fingerprint),
        }
    }

    fn accepts(&self, slot: i32, main: &ItemStack, target: Option<&ItemStack>) -> bool {
        self.slot == slot
            && self.main == fingerprint(main)
            && match (&self.target, target) {
                (None, None) => true,
                (Some(expected), Some(target)) => *expected == fingerprint(target),
                _ => false,
            }
    }
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

/// Takes the pending action only when the tick is the slot it started from:
/// another slot's tick leaves it waiting instead of consuming it.
fn take_pending(player: u128, slot: i32) -> Option<Pending> {
    let mut pending = pending().lock().ok()?;
    if pending.get(&player).is_some_and(|action| action.slot == slot) {
        pending.remove(&player)
    } else {
        None
    }
}

/// Right click with a gem: socket it into the gear in the off hand, or spend
/// gem dust from the off hand to raise its purity. A sigil instead acts on the
/// gear in the off hand.
pub fn use_item(host: &mut dyn Host, event: &mut UseItemEvent<'_>) -> Flow {
    if let Some(seal) = seal(&event.stack.item) {
        return use_seal(host, event, seal);
    }
    if gems::of_stack(&event.stack).is_none() {
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

    let slot = i32::from(player.selected_slot);
    if socket_total(&off) > 0 {
        set_pending(
            player.uuid,
            Pending::new(Action::Socket, slot, &main, Some(&off)),
        );
        return Flow::Cancel;
    }
    if can_polish(&event.stack, &off) {
        set_pending(
            player.uuid,
            Pending::new(Action::Polish, slot, &event.stack, None),
        );
        return Flow::Cancel;
    }
    Flow::Continue
}

/// Ask for the piece in the main hand (`slot`) to be salvaged on its next
/// inventory tick: `/apoth salvage`.
pub fn begin_salvage(host: &mut dyn Host, player: u128, slot: i32) {
    let Some(stack) = host.held_stack(player) else {
        say(host, player, Text::translate_with_fallback(
            "apotheosis.salvage.not_affixed",
            "Hold an affixed piece to salvage it.",
            &[],
        ));
        return;
    };
    if !has_affixes(&stack) {
        say(host, player, Text::translate_with_fallback(
            "apotheosis.salvage.not_affixed",
            "Hold an affixed piece to salvage it.",
            &[],
        ));
        return;
    }
    set_pending(player, Pending::new(Action::Salvage, slot, &stack, None));
}

/// The counter this module's rolls draw from, mixed with the tick they run on:
/// the hook API has no random source, so this is the port's own stream.
pub(crate) fn next_roll() -> u64 {
    static ROLLS: AtomicU64 = AtomicU64::new(0);
    let n = ROLLS.fetch_add(1, Ordering::Relaxed) + 1;
    seed_mix(n, &[])
}

/// The seed of one reforge, mixed from a counter that advances, the wall
/// clock, the player and the item: Java renews a per-player reforge seed after
/// every reforge, so two reforges never land on the same roll.
pub(crate) fn reforge_seed(player: u128, item: &str) -> u64 {
    static REFORGES: AtomicU64 = AtomicU64::new(0);
    let n = REFORGES.fetch_add(1, Ordering::Relaxed) + 1;
    let clock = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos() as u64)
        .unwrap_or(n);
    seed_mix(
        n ^ clock.rotate_left(23),
        &[player as u64, (player >> 64) as u64, item_id(item)],
    )
}

/// A stable number for an item id, so the same piece reforge always mixes its
/// own id in.
pub(crate) fn item_id(item: &str) -> u64 {
    let mut hash = 0xCBF2_9CE4_8422_2325u64;
    for byte in item.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

/// splitmix64's finalizer over a counter and any number of extra values.
fn seed_mix(counter: u64, extras: &[u64]) -> u64 {
    let mut seed = counter.wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(17);
    for extra in extras {
        seed ^= extra.rotate_left(31);
    }
    seed ^= seed >> 30;
    seed = seed.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    seed ^= seed >> 27;
    seed = seed.wrapping_mul(0x94D0_49BB_1331_11EB);
    seed ^ (seed >> 31)
}

/// Whether the main hand holds a piece this module can reforge: affixed gear.
pub fn can_reforge(stack: &ItemStack) -> bool {
    is_equipment(&stack.item) && has_affixes(stack)
}

/// Applies a queued action on the stack it was validated on. Runs once per
/// player per tick: `inventory_tick` is called for every carried stack, so
/// only the selected slot does the work, and only when it is the slot the
/// click started from.
pub fn inventory_tick(host: &mut dyn Host, event: &mut InventoryTickEvent) {
    if !event.selected {
        return;
    }
    let Some(player) = host.acting_player() else {
        return;
    };
    let Some(pending) = take_pending(player.uuid, event.slot) else {
        return;
    };
    // The clicked slot, never whichever slot is selected now.
    let Some(main) = host.carried_stack(pending.slot).cloned() else {
        return;
    };
    let target = match pending.target {
        Some(_) => host.carried_stack(OFFHAND).cloned(),
        None => None,
    };
    if !pending.accepts(pending.slot, &main, target.as_ref()) {
        // The hand changed since the click: nothing is spent and nothing is
        // destroyed.
        return;
    }

    match pending.action {
        Action::Salvage => salvage(host, pending.slot, &main, player.uuid, &pending.main),
        Action::Cut => cut(host, pending.slot, &main, player.uuid),
        other => {
            let Some(off) = target else {
                return;
            };
            match other {
                Action::Socket => socket(host, pending.slot, &main, &off, player.uuid),
                Action::Polish => polish(host, pending.slot, &main, &off, player.uuid),
                Action::AddSocket => add_socket(host, pending.slot, &main, &off, player.uuid),
                Action::Withdraw => withdraw(host, pending.slot, &main, &off, player.uuid),
                Action::Rebirth => rebirth(host, pending.slot, &main, &off, player.uuid),
                Action::Enhance => enhance(host, pending.slot, &main, &off, player.uuid),
                Action::Unnaming => unname(host, pending.slot, &main, &off, player.uuid),
                _ => {}
            }
        }
    }
}

/// A sigil in the main hand, a piece in the off hand: queue what the sigil
/// does, and take it off the main hand on the next inventory tick.
pub fn use_seal(host: &mut dyn Host, event: &UseItemEvent<'_>, kind: Seal) -> Flow {
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
        say(host, player.uuid, Text::translate_with_fallback(
            "apotheosis.seal.no_piece",
            "Hold an affixed piece in your off hand to use a sigil on it.",
            &[],
        ));
        return Flow::Cancel;
    }
    set_pending(
        player.uuid,
        Pending::new(
            match kind {
                Seal::Socketing => Action::AddSocket,
                Seal::Withdrawal => Action::Withdraw,
                Seal::Rebirth => Action::Rebirth,
                Seal::Enhancement => Action::Enhance,
                Seal::Unnaming => Action::Unnaming,
            },
            i32::from(player.selected_slot),
            &main,
            Some(&off),
        ),
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
        // The reforging and salvaging tables open their own menu (`menus`).
        Table::SimpleReforging | Table::Reforging | Table::Salvaging => Flow::Continue,
        Table::Cutting => {
            if gems::of_stack(&event.stack).is_none() || purity_of(&event.stack) >= Purity::Perfect {
                say(host, player.uuid, Text::translate_with_fallback(
                    "apotheosis.table.cut.hint",
                    "The gem cutting table sharpens the gem in your hand.",
                    &[],
                ));
                return Flow::Cancel;
            }
            set_pending(
                player.uuid,
                Pending::new(
                    Action::Cut,
                    i32::from(player.selected_slot),
                    &event.stack,
                    None,
                ),
            );
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

fn add_socket(host: &mut dyn Host, slot: i32, seal_stack: &ItemStack, gear: &ItemStack, player: u128) {
    if !can_reforge(gear) {
        return;
    }
    let total = socket_total(gear);
    if total >= MAX_SOCKETS {
        say(host, player, Text::translate_with_fallback(
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
    say(host, player, Text::translate_with_fallback(
        "apotheosis.socket.added",
        "Socket added: %s/%s.",
        &[
            Text::literal((total + 1).to_string()),
            Text::literal(MAX_SOCKETS.to_string()),
        ],
    ));
}

/// Hands each socketed gem to the inventory, one stack per socket, and
/// returns the indices of the ones that did not fit. A gem the inventory
/// refuses is reported instead of being thrown away.
fn hand_out_gems(gems: &[Socket], mut give: impl FnMut(&ItemStack) -> bool) -> Vec<usize> {
    let mut unplaced = Vec::new();
    for (index, socket) in gems.iter().enumerate() {
        // The socket names a gem *type* (`apotheosis:core/ballast`), not an
        // item: rebuilding a stack from it gave an unknown item with only a
        // purity tag.  A real gem keeps the single `apotheosis:gem` item and
        // its type, model and purity in custom data.
        let Some(entry) = gems::get(&socket.gem) else {
            unplaced.push(index);
            continue;
        };
        let gem = gems::stack(entry, socket.purity);
        if !give(&gem) {
            unplaced.push(index);
        }
    }
    unplaced
}

fn withdraw(host: &mut dyn Host, slot: i32, seal_stack: &ItemStack, gear: &ItemStack, player: u128) {
    let gems = socketed(gear);
    if gems.is_empty() {
        say(host, player, Text::translate_with_fallback(
            "apotheosis.socket.none",
            "That piece has no gem in its sockets.",
            &[],
        ));
        return;
    }
    // The gems go back to the inventory first: only the ones that fit leave
    // their sockets, so a full inventory loses nothing and the sigil is only
    // spent when at least one gem came out.
    let unplaced = hand_out_gems(&gems, |gem| {
        host.give_stack(player, gem.clone()).is_none()
    });
    let count = gems.len() - unplaced.len();
    if count == 0 {
        say(host, player, Text::translate_with_fallback(
            "apotheosis.socket.no_room",
            "No room for the gems; they stayed in their sockets.",
            &[],
        ));
        return;
    }
    let kept: Vec<Socket> = unplaced.iter().map(|&index| gems[index].clone()).collect();
    let mut piece = gear.clone();
    write_sockets(&mut piece, socket_total(gear), &kept);
    refresh(&mut piece);
    write_slot(host, OFFHAND, piece);
    consume_seal(host, slot, seal_stack);
    say(host, player, Text::translate_with_fallback(
        "apotheosis.socket.withdrawn",
        "%s gem(s) taken back out.",
        &[Text::literal(count.to_string())],
    ));
    if !kept.is_empty() {
        say(host, player, Text::translate_with_fallback(
            "apotheosis.socket.no_room",
            "No room for every gem; the rest stayed in their sockets.",
            &[],
        ));
    }
}

fn rebirth(host: &mut dyn Host, slot: i32, seal_stack: &ItemStack, gear: &ItemStack, player: u128) {
    if !can_reforge(gear) {
        return;
    }
    let seed = reforge_seed(player, &gear.item);
    let tier = reroll(host, OFFHAND, gear, seed);
    consume_seal(host, slot, seal_stack);
    say(host, player, Text::translate_with_fallback(
        "apotheosis.reforge.done",
        "Reforged: %s.",
        &[Text::translate_with_fallback(
            tier.name_key(),
            tier.name(),
            &[],
        )],
    ));
}

fn enhance(host: &mut dyn Host, slot: i32, seal_stack: &ItemStack, gear: &ItemStack, player: u128) {
    let Some(tier) = tier_of(gear) else {
        return;
    };
    if tier == Tier::Ancient {
        say(host, player, Text::translate_with_fallback(
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
    say(host, player, Text::translate_with_fallback(
        "apotheosis.enhance.done",
        "Enhanced: %s.",
        &[Text::translate_with_fallback(
            next.name_key(),
            next.name(),
            &[],
        )],
    ));
}

/// Removes only the name this port generated from a piece's affixes. A name a
/// player set themselves is left alone and the sigil is not spent.
fn unname(host: &mut dyn Host, slot: i32, seal_stack: &ItemStack, gear: &ItemStack, player: u128) {
    const CUSTOM_NAME: &str = "minecraft:custom_name";
    let name = gear
        .data()
        .and_then(|data| data.other(CUSTOM_NAME))
        .cloned();
    let Some(name) = name else {
        say(host, player, Text::translate_with_fallback(
            "apotheosis.unnaming.none",
            "That piece has no name to remove.",
            &[],
        ));
        return;
    };
    if !affixes::is_generated_name(&name) {
        say(host, player, Text::translate_with_fallback(
            "apotheosis.unnaming.custom",
            "That name was not written by affixes; it is left alone.",
            &[],
        ));
        return;
    }
    let mut piece = gear.clone();
    piece
        .data_mut()
        .set_other(CUSTOM_NAME, Option::<Tag>::None);
    write_slot(host, OFFHAND, piece);
    consume_seal(host, slot, seal_stack);
    say(host, player, Text::translate_with_fallback(
        "apotheosis.unnaming.done",
        "The affix name was removed.",
        &[],
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

/// The gem cutting table: the held gem rises one purity, at the Java
/// `purity_upgrade` cost for the purity it reaches.
fn cut(host: &mut dyn Host, slot: i32, main: &ItemStack, player: u128) {
    if gems::of_stack(main).is_none() {
        return;
    }
    let purity = purity_of(main);
    let Some((dust, materials)) = upgrade_cost(purity.next()) else {
        return;
    };
    let taken = host.take_items("apotheosis:gem_dust", dust);
    let mut taken_materials = Vec::new();
    let mut short = taken < dust;
    for (item, count) in materials {
        let got = host.take_items(item, *count);
        taken_materials.push((*item, got, *count));
        short |= got < *count;
    }
    if short {
        if taken > 0 {
            let _ = host.give_items("apotheosis:gem_dust", taken);
        }
        for (item, got, _) in taken_materials {
            if got > 0 {
                let _ = host.give_items(item, got);
            }
        }
        say(host, player, Text::translate_with_fallback(
            "apotheosis.cut.no_dust",
            "The gem cutting table needs %s gem dust and the purity materials.",
            &[Text::literal(dust.to_string())],
        ));
        return;
    }
    let mut improved = main.clone();
    set_purity(&mut improved, purity.next());
    write_slot(host, slot, improved);
    say(host, player, Text::translate_with_fallback(
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

/// The dimension a loot roll happens in, from the level the game set around
/// it (API 156).  A roll the game made without a place falls back to the
/// overworld, as the old hard-coded draw did.
fn loot_dimension<'a>(ctx: &'a NativeLootContext<'_>) -> &'a str {
    ctx.place
        .as_deref()
        .and_then(|place| place.level.as_ref())
        .map(|level| level.dimension.as_str())
        .unwrap_or("minecraft:overworld")
}

/// A gem in a chest becomes a real gem of the table's dimension, at its Java
/// weight, and rolls its purity from the loot roll's own seed.  No type is
/// filtered by its purity floor: the floor clamps the rolled purity, as
/// `Gem.toStack` does in Java.
pub fn loot_function(event: &mut LootFunctionEvent<'_>) {
    if event.kind != "apotheosis:gem_purity" || event.stack.item != gems::ITEM {
        return;
    }
    fill_chest_gem(loot_dimension(event.ctx), event.ctx.seed, event.stack);
}

/// Fills a chest's generic gem entry for a roll in `dimension` with `seed`.
/// Split from [`loot_function`] so a test can roll a real End chest without
/// the game handing it a level.
fn fill_chest_gem(dimension: &str, seed: i64, stack: &mut LootStack) {
    let mut rng = Rng::new(seed as u64);
    let (gem, _) = gems::roll(dimension, false, &mut rng);
    let purity = chest_gem_purity(seed).max(gem.min_purity);
    // The gem's own item: its declaration carries the type, purity, name and
    // model, so the generic entry's patch is dropped.
    if let Some(item) = gems::item_id(gem, purity) {
        stack.item = item;
        stack.components.retain(|(id, _)| {
            id != CUSTOM && id != "minecraft:item_name" && id != "minecraft:custom_model_data"
        });
        return;
    }
    let custom_name = serde_json::json!({
        "translate": format!("item.apotheosis.gem.{}", gem.id),
        "fallback": gem.name,
        "color": purity.hex(),
        "italic": false,
    });
    stack.components.retain(|(id, _)| {
        id != CUSTOM && id != "minecraft:item_name" && id != "minecraft:custom_model_data"
    });
    stack.components.push((
        CUSTOM.to_owned(),
        serde_json::json!({
            gems::GEM_KEY: gem.id,
            gems::PURITY_KEY: purity.index(),
        }),
    ));
    stack
        .components
        .push(("minecraft:custom_model_data".to_owned(), serde_json::json!(gem.id)));
    stack
        .components
        .push(("minecraft:item_name".to_owned(), custom_name));
}

/// What reforging `gear` at `tier` with `seed` gives, the piece itself left
/// untouched: Java's `LootController.createLootItem(input.copy(), rarity)`.
/// An affixed piece keeps its sockets and the gems in them; plain gear rolls
/// its sockets as a drop does.
pub(crate) fn reforged(gear: &ItemStack, tier: Tier, seed: u64) -> ItemStack {
    let mut rng = Rng::new(seed);
    let mut piece = gear.clone();
    let affixed = has_affixes(gear);
    apply_affixes(&mut piece, tier, &mut rng);
    if affixed {
        set_custom(&mut piece, |map| {
            map.insert(TIER_KEY.into(), Tag::Int(tier as i32));
        });
        write_sockets(&mut piece, socket_total(gear), &socketed(gear));
        refresh(&mut piece);
    } else {
        after_affixes(&mut piece, tier, &mut rng);
    }
    piece
}

/// Rerolls the piece in `slot` in place at a rarity drawn from `seed`: the
/// sigil of rebirth.
fn reroll(host: &mut dyn Host, slot: i32, gear: &ItemStack, seed: u64) -> Tier {
    let tier = Rng::new(seed ^ 0x5EED).tier(&crate::ORDINARY_TIERS);
    write_slot(host, slot, reforged(gear, tier, seed));
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

/// A message to the player whose action this is. `Host::feedback` is only
/// delivered for a command's answer, so a message from a table, a sigil or a
/// tick goes to the player's own chat instead, as the tomes already do.
fn say(host: &mut dyn Host, player: u128, line: Text) {
    let _ = host.tell(player, line);
}

fn socket(host: &mut dyn Host, slot: i32, main: &ItemStack, off: &ItemStack, player: u128) {
    let Some(entry) = gems::of_stack(main).map(|(gem, _)| gem) else {
        return;
    };
    let purity = purity_of(main);
    let mut gear = off.clone();
    if !insert_gem(&mut gear, entry.id, purity) {
        say(host, player, Text::translate_with_fallback(
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
    say(host, player, Text::translate_with_fallback(
        "apotheosis.socket.done",
        "Socketed %s (%s).",
        &[
            Text::translate_with_fallback(
                &format!("item.apotheosis.gem.{}", entry.id),
                entry.name,
                &[],
            ),
            Text::translate_with_fallback(purity.name_key(), purity.name(), &[]),
        ],
    ));
}

fn can_polish(main: &ItemStack, off: &ItemStack) -> bool {
    if gems::of_stack(main).is_none() || &*off.item != "apotheosis:gem_dust" {
        return false;
    }
    // No Java recipe reaches `perfect`, and the dust cost rises with the step.
    match upgrade_cost(purity_of(main).next()) {
        Some((dust, _)) => off.count >= dust,
        None => false,
    }
}

fn polish(host: &mut dyn Host, slot: i32, main: &ItemStack, off: &ItemStack, player: u128) {
    if !can_polish(main, off) {
        return;
    }
    let purity = purity_of(main);
    let Some((dust_cost, materials)) = upgrade_cost(purity.next()) else {
        return;
    };
    let mut taken = Vec::new();
    let mut short = false;
    for (item, count) in materials {
        let got = host.take_items(item, *count);
        taken.push((*item, got, *count));
        short |= got < *count;
    }
    if short {
        for (item, got, _) in taken {
            if got > 0 {
                let _ = host.give_items(item, got);
            }
        }
        say(host, player, Text::translate_with_fallback(
            "apotheosis.cut.no_dust",
            "The gem cutting table needs %s gem dust and the purity materials.",
            &[Text::literal(dust_cost.to_string())],
        ));
        return;
    }
    let mut improved = main.clone();
    set_purity(&mut improved, purity.next());
    write_slot(host, slot, improved);
    let mut dust = off.clone();
    dust.count -= dust_cost;
    write_slot(host, OFFHAND, dust);
    say(host, player, Text::translate_with_fallback(
        "apotheosis.polish.done",
        "Purity raised to %s.",
        &[Text::translate_with_fallback(
            purity.next().name_key(),
            purity.next().name(),
            &[],
        )],
    ));
}

fn salvage(
    host: &mut dyn Host,
    slot: i32,
    main: &ItemStack,
    player: u128,
    expected: &Fingerprint,
) {
    if fingerprint(main) != *expected || !has_affixes(main) {
        say(host, player, Text::translate_with_fallback(
            "apotheosis.salvage.changed",
            "The piece in your hand changed; nothing was salvaged.",
            &[],
        ));
        return;
    }
    let tier = tier_of(main).unwrap_or(Tier::Common);
    let mut rng = Rng::new(next_roll());
    let (material, count) = salvage_yield(tier, &mut rng);
    // The gems come back at the purity they were socketed with, before the
    // piece is destroyed: if one of them cannot fit, nothing is salvaged. The
    // ones already handed out are taken back so the player gains nothing.
    let gems = socketed(main);
    let unplaced = hand_out_gems(&gems, |gem| {
        host.give_stack(player, gem.clone()).is_none()
    });
    if !unplaced.is_empty() {
        // Take back the gems that fitted by their real item id: the type id
        // `socket.gem` is not an item and could never be found in a slot.
        for (index, socket) in gems.iter().enumerate() {
            if unplaced.contains(&index) {
                continue;
            }
            if let Some(entry) = gems::get(&socket.gem) {
                let _ = host.take_items(&gems::stack(entry, socket.purity).item, 1);
            }
        }
        say(host, player, Text::translate_with_fallback(
            "apotheosis.salvage.no_room",
            "No room for the gems in it; nothing was salvaged.",
            &[],
        ));
        return;
    }
    write_slot(host, slot, emptied(main));
    let given = host.give_items(&material, count);
    say(host, player, Text::translate_with_fallback(
        "apotheosis.salvage.done",
        "Salvaged into %s.",
        &[Text::translate_with_fallback(
            &format!("item.{}", material.replacen(':', ".", 1)),
            material.rsplit(':').next().unwrap_or(&material),
            &[],
        )],
    ));
    if given < count {
        say(host, player, Text::translate_with_fallback(
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
    fn a_gem_fills_a_socket_once_and_takes_its_java_value() {
        let mut gear = ItemStack::new("minecraft:diamond_sword", 1);
        let mut rng = Rng::new(5);
        after_affixes(&mut gear, Tier::Epic, &mut rng);
        assert_eq!(socket_total(&gear), 3);
        assert!(insert_gem(&mut gear, "apotheosis:core/ballast", Purity::Flawless));
        assert!(!insert_gem(&mut gear, "apotheosis:core/ballast", Purity::Perfect));
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
        // Java's ballast gem gives a flawless melee weapon +7 attack damage.
        assert_eq!(
            socket_modifier.get("amount"),
            Some(&Tag::Double(7.0f64.to_bits()))
        );
        assert_eq!(socket_modifier.get("type"), Some(&Tag::String("minecraft:attack_damage".into())));
    }

    /// The reported bug: a socketed sword's bonus paid out from the off hand
    /// too, because its modifier used the `hand` group. Java follows the item:
    /// main hand for a weapon or tool, its own slot for armour, off hand for a
    /// shield.
    #[test]
    fn a_socketed_gem_uses_the_slot_group_the_item_names() {
        let mut shield = ItemStack::new("minecraft:shield", 1);
        write_sockets(&mut shield, 1, &[]);
        assert!(insert_gem(
            &mut shield,
            "apotheosis:core/brawlers",
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
        assert_eq!(
            socket_modifier.get("slot"),
            Some(&Tag::String("offhand".into()))
        );
        assert_eq!(socket_slot_group("minecraft:shield"), "offhand");
        assert_eq!(socket_slot_group("minecraft:diamond_helmet"), "head");
        assert_eq!(socket_slot_group("minecraft:diamond_chestplate"), "chest");
        assert_eq!(socket_slot_group("minecraft:diamond_leggings"), "legs");
        assert_eq!(socket_slot_group("minecraft:diamond_boots"), "feet");
        assert_eq!(socket_slot_group("minecraft:diamond_sword"), "mainhand");
        assert_eq!(socket_slot_group("minecraft:iron_pickaxe"), "mainhand");
    }

    #[test]
    fn a_weapons_socketed_bonus_is_main_hand_only() {
        let mut sword = ItemStack::new("minecraft:diamond_sword", 1);
        write_sockets(&mut sword, 1, &[]);
        assert!(insert_gem(
            &mut sword,
            "apotheosis:core/ballast",
            Purity::Normal
        ));
        let Some(Tag::List(modifiers)) = sword
            .data()
            .and_then(|data| data.other("minecraft:attribute_modifiers"))
        else {
            panic!("no modifiers");
        };
        for entry in modifiers.iter().filter(|entry| is_socket_modifier(entry)) {
            assert_eq!(entry.get("slot"), Some(&Tag::String("mainhand".into())));
        }
    }

    #[test]
    fn a_socketed_gems_lore_only_names_the_pieces_own_bonus() {
        let mut sword = ItemStack::new("minecraft:diamond_sword", 1);
        write_sockets(&mut sword, 1, &[]);
        assert!(insert_gem(
            &mut sword,
            "apotheosis:core/ballast",
            Purity::Normal
        ));
        let Some(Tag::List(lines)) = sword.data().and_then(|data| data.other("minecraft:lore"))
        else {
            panic!("no lore");
        };
        let text: Vec<String> = lines
            .iter()
            .filter_map(|line| match line.get("fallback") {
                Some(Tag::String(value)) => Some(value.to_string_lossy()),
                _ => None,
            })
            .collect();
        assert!(
            text.iter().any(|line| line.contains("+5 Attack Damage")),
            "the melee bonus is missing: {text:?}"
        );
        assert!(
            !text.iter().any(|line| line.contains("Knockback Resistance")),
            "a sword's lore listed the chestplate bonus: {text:?}"
        );
    }

    #[test]
    fn affix_modifiers_follow_their_own_equipment_slot() {
        let sword = affixed("minecraft:diamond_sword", Tier::Mythic, 3);
        let Some(Tag::List(modifiers)) = sword
            .data()
            .and_then(|data| data.other("minecraft:attribute_modifiers"))
        else {
            panic!("no modifiers");
        };
        for entry in modifiers.iter().filter(|entry| crate::is_affix_modifier(entry)) {
            assert_eq!(entry.get("slot"), Some(&Tag::String("mainhand".into())));
        }
        let chest = affixed("minecraft:diamond_chestplate", Tier::Mythic, 4);
        let Some(Tag::List(modifiers)) = chest
            .data()
            .and_then(|data| data.other("minecraft:attribute_modifiers"))
        else {
            panic!("no modifiers");
        };
        for entry in modifiers.iter().filter(|entry| crate::is_affix_modifier(entry)) {
            assert_eq!(entry.get("slot"), Some(&Tag::String("chest".into())));
        }
    }

    #[test]
    fn polishing_rejects_a_non_gem_in_the_main_hand() {
        let sword = ItemStack::new("minecraft:diamond_sword", 1);
        let dust = ItemStack::new("apotheosis:gem_dust", 4);
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
        assert!(insert_gem(&mut gear, "apotheosis:core/ballast", Purity::Normal));
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
        assert!(!insert_gem(&mut gear, "apotheosis:core/guardian", Purity::Normal));
    }

    #[test]
    fn reforging_keeps_the_socketed_gems() {
        let mut gear = ItemStack::new("minecraft:diamond_sword", 1);
        let mut rng = Rng::new(21);
        after_affixes(&mut gear, Tier::Epic, &mut rng);
        assert!(insert_gem(&mut gear, "apotheosis:core/brawlers", Purity::Chipped));
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
            let (material, count) = salvage_yield(tier, &mut rng);
            assert_eq!(material, salvage_material(tier).0);
            assert!((1..=salvage_material(tier).1).contains(&u64::from(count)));
        }
    }

    #[test]
    fn reforge_costs_are_the_five_java_recipes_and_the_simple_table_stops_at_rare() {
        for (index, tier) in [
            Tier::Common,
            Tier::Uncommon,
            Tier::Rare,
            Tier::Epic,
            Tier::Mythic,
        ]
        .into_iter()
        .enumerate()
        {
            let expected = [(2, 1, 0), (5, 2, 1), (15, 2, 2), (30, 2, 4), (50, 3, 5)][index];
            assert_eq!(reforge_cost(tier, Table::Reforging), Some(expected));
            if tier <= Tier::Rare {
                assert_eq!(reforge_cost(tier, Table::SimpleReforging), Some(expected));
            } else {
                assert_eq!(reforge_cost(tier, Table::SimpleReforging), None);
            }
        }
        // Ancient reuses the mythic row, and only the full table serves it.
        assert_eq!(reforge_cost(Tier::Ancient, Table::Reforging), Some((50, 3, 5)));
        assert_eq!(reforge_cost(Tier::Ancient, Table::SimpleReforging), None);
    }

    #[test]
    fn gem_salvaging_matches_the_six_java_ranges() {
        for (purity, low, high) in [
            (Purity::Cracked, 1, 2),
            (Purity::Chipped, 1, 3),
            (Purity::Flawed, 2, 4),
            (Purity::Normal, 3, 5),
            (Purity::Flawless, 4, 7),
            (Purity::Perfect, 5, 10),
        ] {
            let mut seen = Vec::new();
            for seed in 0..256 {
                let mut rng = Rng::new(seed);
                let dust = gem_salvage_yield(purity, &mut rng);
                assert!((low..=high).contains(&dust), "{purity:?} gave {dust}");
                if !seen.contains(&dust) {
                    seen.push(dust);
                }
            }
            assert_eq!(seen.len() as u32, high - low + 1, "{purity:?} is too narrow");
        }
    }

    #[test]
    fn the_unnaming_sigil_removes_generated_names_only() {
        let gear = affixed("minecraft:diamond_sword", Tier::Rare, 71);
        assert!(gear
            .data()
            .and_then(|data| data.other("minecraft:custom_name"))
            .is_some());
        let sigil = ItemStack::new("apotheosis:sigil_of_unnaming", 2);
        let mut host = Fake::default();
        host.slots.insert(1, sigil.clone());
        host.slots.insert(OFFHAND, gear.clone());
        unname(&mut host, 1, &sigil, &gear, PLAYER);

        let named = host.slots.get(&OFFHAND).unwrap();
        assert_eq!(named.data().and_then(|data| data.other("minecraft:custom_name")), None);
        assert_eq!(host.slots.get(&1).unwrap().count, 1, "the sigil was not spent");

        // A name the player typed is not an affix name: it stays and so does
        // the sigil.
        let mut mine = affixed("minecraft:diamond_sword", Tier::Rare, 72);
        mine.data_mut()
            .set_other("minecraft:custom_name", Some(Tag::String("Excalibur".into())));
        let sigil = ItemStack::new("apotheosis:sigil_of_unnaming", 2);
        let mut host = Fake::default();
        host.slots.insert(1, sigil.clone());
        host.slots.insert(OFFHAND, mine.clone());
        unname(&mut host, 1, &sigil, &mine, PLAYER);

        assert_eq!(
            host.slots
                .get(&OFFHAND)
                .unwrap()
                .data()
                .and_then(|data| data.other("minecraft:custom_name"))
                .cloned(),
            Some(Tag::String("Excalibur".into()))
        );
        assert_eq!(host.slots.get(&1).unwrap().count, 2);
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
        assert_eq!(
            table("apotheosis:simple_reforging_table"),
            Some(Table::SimpleReforging)
        );
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

    #[test]
    fn a_chest_gem_follows_the_chest_dimension() {
        let mut found = false;
        for seed in 0..512 {
            let mut stack = LootStack {
                item: gems::ITEM.to_owned(),
                count: 1,
                damage: 0,
                enchantments: Vec::new(),
                potion: None,
                map: false,
                exploration: None,
                ominous_amplifier: None,
                stew_effects: Vec::new(),
                names: Vec::new(),
                components: Vec::new(),
            };
            fill_chest_gem("minecraft:the_end", seed, &mut stack);
            // The entry becomes the gem's own item, whose declaration carries
            // its type, purity, name and model: no patch is left on it.
            let (gem, purity) =
                gems::from_item(&stack.item).expect("the chest gem is no gem item");
            assert!(stack.components.is_empty(), "{:?}", stack.components);
            assert!(purity >= gem.min_purity);
            if gem.id == "apotheosis:the_end/endersurge" {
                found = true;
                break;
            }
        }
        assert!(found, "an End chest never drew an End gem");
    }

    const PLAYER: u128 = 7;

    /// A host for the functions that act on carried stacks. `free_stacks` is
    /// how many whole stacks the inventory still takes; 0 is a full one.
    #[derive(Default)]
    struct Fake {
        slots: HashMap<i32, ItemStack>,
        free_stacks: u32,
        given: Vec<(String, u32)>,
        items: Vec<(String, u32)>,
        taken: Vec<(String, u32)>,
        levels: Vec<i32>,
        /// Messages sent to a player's chat, and command feedback: a table or
        /// sigil must use the first, only a command the second.
        told: Vec<String>,
        feedback: Vec<String>,
    }

    impl Host for Fake {
        fn carried_stack(&mut self, slot: i32) -> Option<&mut ItemStack> {
            self.slots.get_mut(&slot)
        }

        fn give_stack(&mut self, _uuid: u128, stack: ItemStack) -> Option<ItemStack> {
            if self.free_stacks == 0 {
                return Some(stack);
            }
            self.free_stacks -= 1;
            self.given.push((stack.item.to_string(), stack.count));
            None
        }

        fn give_items(&mut self, item: &str, count: u32) -> u32 {
            if self.free_stacks == 0 {
                return 0;
            }
            self.items.push((item.to_owned(), count));
            count
        }

        fn take_items(&mut self, item: &str, count: u32) -> u32 {
            self.taken.push((item.to_owned(), count));
            count
        }

        fn give_levels(&mut self, _uuid: u128, levels: i32) -> bool {
            self.levels.push(levels);
            true
        }

        fn tell(&mut self, _uuid: u128, line: Text) -> bool {
            self.told.push(line.plain());
            true
        }

        fn feedback(&mut self, line: Text) {
            self.feedback.push(line.plain());
        }
    }

    fn affixed(item: &str, tier: Tier, seed: u64) -> ItemStack {
        let mut stack = ItemStack::new(item, 1);
        let mut rng = Rng::new(seed);
        apply_affixes(&mut stack, tier, &mut rng);
        after_affixes(&mut stack, tier, &mut rng);
        stack
    }

    fn three_gems() -> ItemStack {
        let mut gear = affixed("minecraft:diamond_chestplate", Tier::Ancient, 31);
        assert_eq!(socket_total(&gear), MAX_SOCKETS);
        assert!(insert_gem(&mut gear, "apotheosis:core/ballast", Purity::Normal));
        assert!(insert_gem(&mut gear, "apotheosis:core/guardian", Purity::Flawless));
        assert!(insert_gem(&mut gear, "apotheosis:core/brawlers", Purity::Perfect));
        gear
    }

    #[test]
    fn a_delayed_action_only_runs_on_the_slot_and_stack_it_was_validated_on() {
        let sigil = ItemStack::new("apotheosis:sigil_of_rebirth", 1);
        let rare = affixed("minecraft:diamond_chestplate", Tier::Rare, 3);
        let mythic = affixed("minecraft:diamond_chestplate", Tier::Mythic, 4);
        let pending = Pending::new(Action::Rebirth, 1, &sigil, Some(&rare));

        assert!(pending.accepts(1, &sigil, Some(&rare)));
        assert!(
            !pending.accepts(2, &sigil, Some(&rare)),
            "the tick of another slot ran the action"
        );
        assert!(
            !pending.accepts(1, &sigil, Some(&mythic)),
            "another piece was acted on"
        );
        assert_ne!(
            fingerprint(&rare),
            fingerprint(&mythic),
            "two pieces of one item are the same fingerprint"
        );

        let gem = ItemStack::new("apotheosis:gem_ruby", 1);
        let mut polished = gem.clone();
        set_purity(&mut polished, Purity::Perfect);
        let pending = Pending::new(Action::Polish, 0, &gem, None);
        assert!(pending.accepts(0, &gem, None));
        assert!(
            !pending.accepts(0, &polished, None),
            "a gem whose purity changed was polished again"
        );
    }

    #[test]
    fn a_sigil_is_taken_from_the_slot_it_was_used_from() {
        let sigil = ItemStack::new("apotheosis:sigil_of_socketing", 4);
        let mut host = Fake::default();
        host.slots.insert(1, sigil.clone());
        host.slots
            .insert(2, ItemStack::new("minecraft:diamond", 64));
        consume_seal(&mut host, 1, &sigil);
        assert_eq!(host.slots.get(&1).unwrap().count, 3);
        assert_eq!(host.slots.get(&2).unwrap().count, 64);
    }

    #[test]
    fn a_purity_step_costs_what_the_java_gem_cutting_recipe_says() {
        assert_eq!(
            upgrade_cost(Purity::Chipped),
            Some((
                3,
                &[
                    ("apotheosis:mysterious_scrap_metal", 9),
                    ("apotheosis:timeworn_fabric", 3),
                    ("apotheosis:luminous_crystal_shard", 1),
                ][..]
            ))
        );
        assert_eq!(upgrade_cost(Purity::Flawless).unwrap().0, 9);
        // Java has no perfect purity recipe: a gem stops at flawless.
        assert_eq!(upgrade_cost(Purity::Perfect), None);
    }

    #[test]
    fn polishing_never_touches_a_swapped_in_sword() {
        let sword = ItemStack::new("minecraft:diamond_sword", 1);
        let dust = ItemStack::new("apotheosis:gem_dust", 4);
        let mut host = Fake::default();
        host.slots.insert(0, sword.clone());
        host.slots.insert(OFFHAND, dust.clone());
        polish(&mut host, 0, &sword, &dust, PLAYER);
        assert_eq!(custom(host.slots.get(&0).unwrap()).get(gems::PURITY_KEY), None);
        assert_eq!(host.slots.get(&OFFHAND).unwrap().count, 4);
        assert!(!can_polish(&sword, &dust));
    }

    #[test]
    fn a_full_inventory_keeps_withdrawn_gems_in_their_sockets() {
        let gear = three_gems();
        let sigil = ItemStack::new("apotheosis:sigil_of_withdrawal", 2);
        let mut host = Fake::default();
        host.slots.insert(4, sigil.clone());
        host.slots.insert(OFFHAND, gear.clone());
        withdraw(&mut host, 4, &sigil, &gear, PLAYER);

        let kept = host.slots.get(&OFFHAND).expect("the piece was destroyed");
        assert_eq!(socketed(kept), socketed(&gear));
        assert_eq!(host.slots.get(&4).unwrap().count, 2, "the sigil was spent");
        assert!(host.given.is_empty());
    }

    #[test]
    fn withdrawing_with_room_returns_every_gem_and_spends_the_sigil() {
        let gear = three_gems();
        let sigil = ItemStack::new("apotheosis:sigil_of_withdrawal", 1);
        let mut host = Fake {
            free_stacks: 8,
            ..Default::default()
        };
        host.slots.insert(4, sigil.clone());
        host.slots.insert(OFFHAND, gear.clone());
        withdraw(&mut host, 4, &sigil, &gear, PLAYER);

        assert_eq!(host.given.len(), 3);
        assert!(
            host.given.iter().all(|(item, _)| gems::from_item(item).is_some()),
            "a gem came back as its type id, not as its gem item"
        );
        assert_eq!(socketed(host.slots.get(&OFFHAND).unwrap()), Vec::new());
        assert_eq!(host.slots.get(&4).unwrap().count, 0);
    }

    #[test]
    fn a_withdrawn_gem_keeps_its_type_purity_and_model() {
        let sockets = vec![Socket {
            gem: "apotheosis:core/ballast".into(),
            purity: Purity::Flawless,
        }];
        let mut given = Vec::new();
        let unplaced = hand_out_gems(&sockets, |gem| {
            given.push(gem.clone());
            true
        });
        assert!(unplaced.is_empty());
        assert_eq!(&*given[0].item, "apotheosis:gem_ballast_flawless");
        assert!(given[0].data().is_none(), "the declaration carries the identity");
        assert_eq!(gems::id_of(&given[0]), Some("apotheosis:core/ballast"));
        assert_eq!(gems::purity_of(&given[0]), Purity::Flawless);
        assert!(gems::of_stack(&given[0]).is_some());
    }

    #[test]
    fn a_full_inventory_keeps_a_salvaged_piece_whole() {
        let gear = three_gems();
        let mut host = Fake::default();
        host.slots.insert(3, gear.clone());
        salvage(&mut host, 3, &gear, PLAYER, &fingerprint(&gear));

        assert_eq!(
            *host.slots.get(&3).expect("the piece was destroyed"),
            gear
        );
        assert!(host.items.is_empty(), "material was given for nothing");
        assert!(host.taken.is_empty());
        assert!(host.given.is_empty());
    }

    #[test]
    fn a_partly_full_inventory_salvages_nothing_and_takes_the_gems_back() {
        let gear = three_gems();
        let mut host = Fake {
            free_stacks: 2,
            ..Default::default()
        };
        host.slots.insert(3, gear.clone());
        salvage(&mut host, 3, &gear, PLAYER, &fingerprint(&gear));

        assert_eq!(*host.slots.get(&3).unwrap(), gear);
        assert_eq!(host.given.len(), 2);
        assert_eq!(host.taken.len(), 2, "the gems handed out were not taken back");
        assert!(
            host.taken.iter().all(|(item, _)| gems::from_item(item).is_some()),
            "the rollback tried to take back a gem type id, not the gem item"
        );
        let ids = |list: &[(String, u32)]| {
            let mut ids: Vec<String> = list.iter().map(|(item, _)| item.clone()).collect();
            ids.sort();
            ids
        };
        assert_eq!(ids(&host.taken), ids(&host.given), "the rollback took other gems back");
        assert!(host.items.is_empty());
    }

    #[test]
    fn a_salvage_only_destroys_the_piece_it_was_validated_on() {
        let designated = affixed("minecraft:diamond_sword", Tier::Mythic, 71);
        let swapped_in = affixed("minecraft:diamond_chestplate", Tier::Ancient, 72);
        let mut host = Fake {
            free_stacks: 8,
            ..Default::default()
        };
        host.slots.insert(3, swapped_in.clone());
        salvage(&mut host, 3, &swapped_in, PLAYER, &fingerprint(&designated));

        assert_eq!(*host.slots.get(&3).expect("the piece vanished"), swapped_in);
        assert!(host.items.is_empty());
        assert!(host.given.is_empty());
    }

    #[test]
    fn every_reforge_draws_a_new_seed() {
        let first = reforge_seed(PLAYER, "minecraft:diamond_sword");
        let second = reforge_seed(PLAYER, "minecraft:diamond_sword");
        assert_ne!(first, second, "two reforges drew the same seed");
        assert_ne!(
            first,
            reforge_seed(PLAYER + 1, "minecraft:diamond_sword"),
            "the player is not part of the seed"
        );
        assert_ne!(
            first,
            reforge_seed(PLAYER, "minecraft:diamond_chestplate"),
            "the item is not part of the seed"
        );
    }
}
