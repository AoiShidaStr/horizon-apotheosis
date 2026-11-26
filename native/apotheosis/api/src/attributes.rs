//! Apothic Attributes for Horizon: the combat, mining, bow and experience
//! attributes Apotheosis' affixes roll, held in an item's
//! `minecraft:custom_data` because the game has no registry for them, and
//! applied by the hooks below.
//!
//! Clean room: the names, defaults and value ranges are Apothic Attributes'
//! public data (`apothic_attributes:crit_chance` defaults to 5%, and so on);
//! no Apotheosis or Apothic Attributes code is used. See `../../README.md`.
//!
//! Where an attribute cannot be read in a hook, the value is kept in a small
//! per-entity cache filled by the equipment hooks (`on_equip`,
//! `living_equipment_change`, `player_tick`) and read from the damage and heal
//! hooks, which are given no host and no stacks.

use horizon_client::mod_api::{
    ArrowLooseEvent, BreakSpeedEvent, Compound, EquipmentSlot, ExperienceOrbPickupEvent, Host,
    ItemStack, LivingEquipmentChangeEvent, LivingHealEvent, LivingIncomingDamageEvent,
    LivingIncomingDamageV2Event, ModifyDefaultComponentsEvent, NativeEffect, NativeServerPlayer,
    OnEquipEvent, PlayerTickEvent, ProjectileLaunchEvent, Tag, Text, TooltipEvent, serde_json,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

/// The custom data key every affixed item carries its attributes under.
const CUSTOM_DATA: &str = "minecraft:custom_data";
const ATTR_KEY: &str = "apotheosis:attributes";
const ATTRIBUTE_PREFIX: &str = "apothic_attributes:";

/// The attributes this port implements, in tooltip order. The keys are the
/// attribute's path in Apothic Attributes and the component's field name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attr {
    CritChance,
    CritDamage,
    LifeSteal,
    Overheal,
    DodgeChance,
    ArmorPierce,
    ArmorShred,
    FireDamage,
    ColdDamage,
    ExperienceGained,
    HealingReceived,
    ArrowDamage,
    ArrowVelocity,
    DrawSpeed,
    MiningSpeed,
    CurrentHpDamage,
    ProtPierce,
    ProtShred,
}

impl Attr {
    pub const ALL: [Attr; 18] = [
        Attr::CritChance,
        Attr::CritDamage,
        Attr::LifeSteal,
        Attr::Overheal,
        Attr::DodgeChance,
        Attr::ArmorPierce,
        Attr::ArmorShred,
        Attr::FireDamage,
        Attr::ColdDamage,
        Attr::ExperienceGained,
        Attr::HealingReceived,
        Attr::ArrowDamage,
        Attr::ArrowVelocity,
        Attr::DrawSpeed,
        Attr::MiningSpeed,
        Attr::CurrentHpDamage,
        Attr::ProtPierce,
        Attr::ProtShred,
    ];

    /// The field name in `apotheosis:attributes`, and the lang key's suffix.
    pub fn key(self) -> &'static str {
        match self {
            Attr::CritChance => "crit_chance",
            Attr::CritDamage => "crit_damage",
            Attr::LifeSteal => "life_steal",
            Attr::Overheal => "overheal",
            Attr::DodgeChance => "dodge_chance",
            Attr::ArmorPierce => "armor_pierce",
            Attr::ArmorShred => "armor_shred",
            Attr::FireDamage => "fire_damage",
            Attr::ColdDamage => "cold_damage",
            Attr::ExperienceGained => "experience_gained",
            Attr::HealingReceived => "healing_received",
            Attr::ArrowDamage => "arrow_damage",
            Attr::ArrowVelocity => "arrow_velocity",
            Attr::DrawSpeed => "draw_speed",
            Attr::MiningSpeed => "mining_speed",
            Attr::CurrentHpDamage => "current_hp_damage",
            Attr::ProtPierce => "prot_pierce",
            Attr::ProtShred => "prot_shred",
        }
    }

    /// The attribute id the item's modifier entry names. The game has no such
    /// attribute registered; the entry is there for tooltips and resource
    /// packs, and this module does the work.
    pub fn id(self) -> String {
        format!("{ATTRIBUTE_PREFIX}{}", self.key())
    }

    /// The name shown in a tooltip without the language file.
    pub fn name(self) -> &'static str {
        match self {
            Attr::CritChance => "Crit Chance",
            Attr::CritDamage => "Crit Damage",
            Attr::LifeSteal => "Life Steal",
            Attr::Overheal => "Overheal",
            Attr::DodgeChance => "Dodge Chance",
            Attr::ArmorPierce => "Armor Pierce",
            Attr::ArmorShred => "Armor Shred",
            Attr::FireDamage => "Fire Damage",
            Attr::ColdDamage => "Cold Damage",
            Attr::ExperienceGained => "Experience Gained",
            Attr::HealingReceived => "Healing Received",
            Attr::ArrowDamage => "Arrow Damage",
            Attr::ArrowVelocity => "Arrow Velocity",
            Attr::DrawSpeed => "Draw Speed",
            Attr::MiningSpeed => "Mining Speed",
            Attr::CurrentHpDamage => "Current HP Damage",
            Attr::ProtPierce => "Protection Pierce",
            Attr::ProtShred => "Protection Shred",
        }
    }

    /// Whether the value is a fraction shown as a percentage.
    pub fn is_percent(self) -> bool {
        !matches!(
            self,
            Attr::ArmorPierce | Attr::FireDamage | Attr::ColdDamage | Attr::CurrentHpDamage
                | Attr::ProtPierce
        )
    }

    /// Apothic Attributes' default, which the stored value is a bonus over.
    pub fn default_value(self) -> f64 {
        match self {
            Attr::CritChance => 0.05,
            Attr::CritDamage => 1.5,
            Attr::HealingReceived
            | Attr::ArrowDamage
            | Attr::ArrowVelocity
            | Attr::DrawSpeed
            | Attr::MiningSpeed
            | Attr::ExperienceGained => 1.0,
            _ => 0.0,
        }
    }

    fn index(self) -> usize {
        match self {
            Attr::CritChance => 0,
            Attr::CritDamage => 1,
            Attr::LifeSteal => 2,
            Attr::Overheal => 3,
            Attr::DodgeChance => 4,
            Attr::ArmorPierce => 5,
            Attr::ArmorShred => 6,
            Attr::FireDamage => 7,
            Attr::ColdDamage => 8,
            Attr::ExperienceGained => 9,
            Attr::HealingReceived => 10,
            Attr::ArrowDamage => 11,
            Attr::ArrowVelocity => 12,
            Attr::DrawSpeed => 13,
            Attr::MiningSpeed => 14,
            Attr::CurrentHpDamage => 15,
            Attr::ProtPierce => 16,
            Attr::ProtShred => 17,
        }
    }

    /// `0.05` as `+5%`, `2.0` as `+2`.
    pub fn format(self, value: f64) -> String {
        if self.is_percent() {
            let pct = value * 100.0;
            if (pct - pct.round()).abs() < 0.05 {
                format!("+{pct:.0}%")
            } else {
                format!("+{pct:.1}%")
            }
        } else if (value - value.round()).abs() < 0.05 {
            format!("+{value:.0}")
        } else {
            format!("+{value:.1}")
        }
    }
}

/// The attributes an item carries, as bonuses over their defaults.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AttrSet {
    values: [f64; 18],
}

impl AttrSet {
    pub fn get(self, attr: Attr) -> f64 {
        self.values[attr.index()]
    }

    pub fn set(&mut self, attr: Attr, value: f64) {
        self.values[attr.index()] = value;
    }

    pub fn add(&mut self, other: &AttrSet) {
        for attr in Attr::ALL {
            self.values[attr.index()] += other.get(attr);
        }
    }

    pub fn sub(&mut self, other: &AttrSet) {
        for attr in Attr::ALL {
            self.values[attr.index()] = (self.get(attr) - other.get(attr)).max(0.0);
        }
    }

    pub fn is_empty(self) -> bool {
        self.values.iter().all(|v| *v == 0.0)
    }

    /// The attributes an item's custom data carries, empty for a plain item.
    pub fn of_stack(stack: &ItemStack) -> AttrSet {
        let Some(Tag::Compound(compound)) = stack
            .data()
            .and_then(|data| data.other(CUSTOM_DATA))
            .and_then(|data| data.get(ATTR_KEY))
        else {
            return AttrSet::default();
        };
        let mut set = AttrSet::default();
        for attr in Attr::ALL {
            if let Some(value) = number(compound.get(&attr.key().into())) {
                set.set(attr, value);
            }
        }
        set
    }

    pub fn crit_chance(self) -> f64 {
        (Attr::CritChance.default_value() + self.get(Attr::CritChance)).clamp(0.0, 1.0)
    }

    pub fn crit_multiplier(self) -> f64 {
        (Attr::CritDamage.default_value() + self.get(Attr::CritDamage)).clamp(1.0, 100.0)
    }

    pub fn xp_multiplier(self) -> f64 {
        (1.0 + self.get(Attr::ExperienceGained)).max(0.0)
    }

    pub fn mining_multiplier(self) -> f64 {
        (1.0 + self.get(Attr::MiningSpeed)).clamp(0.0, 10.0)
    }

    pub fn healing_multiplier(self) -> f64 {
        (1.0 + self.get(Attr::HealingReceived)).max(0.0)
    }

    pub fn arrow_damage_multiplier(self) -> f64 {
        (1.0 + self.get(Attr::ArrowDamage)).max(0.0)
    }

    pub fn draw_multiplier(self) -> f64 {
        (1.0 + self.get(Attr::DrawSpeed)).clamp(0.0, 4.0)
    }

    pub fn arrow_velocity_multiplier(self) -> f64 {
        (1.0 + self.get(Attr::ArrowVelocity)).clamp(0.0, 4.0)
    }
}

/// The custom data key a piece's damage reduction shares live under.
const REDUCTION_KEY: &str = "apotheosis:reductions";

/// A piece's share of incoming damage it removes, per Java damage class.
/// `damage_reduction` affixes (Feathery, Runed, ...) land here rather than in
/// an attribute modifier, but are rolled by the same effect pass.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Reduction {
    physical: f64,
    projectile: f64,
    explosion: f64,
    fall: f64,
    lightning: f64,
    magic: f64,
}

impl Reduction {
    /// The reduction a stack's custom data carries, empty for a plain item.
    pub fn of_stack(stack: &ItemStack) -> Reduction {
        let Some(Tag::Compound(compound)) = stack
            .data()
            .and_then(|data| data.other(CUSTOM_DATA))
            .and_then(|data| data.get(REDUCTION_KEY))
        else {
            return Reduction::default();
        };
        let mut reduction = Reduction::default();
        for (key, slot) in [
            ("physical", &mut reduction.physical),
            ("projectile", &mut reduction.projectile),
            ("explosion", &mut reduction.explosion),
            ("fall", &mut reduction.fall),
            ("lightning", &mut reduction.lightning),
            ("magic", &mut reduction.magic),
        ] {
            *slot = number(compound.get(&key.into())).unwrap_or(0.0);
        }
        reduction
    }

    /// The share removed from a hit of `kind`, capped like vanilla's armour.
    pub fn get(self, kind: &str) -> f64 {
        let value = match kind {
            "projectile" => self.projectile,
            "explosion" => self.explosion,
            "fall" => self.fall,
            "lightning" => self.lightning,
            "magic" => self.magic,
            _ => self.physical,
        };
        value.clamp(0.0, 0.95)
    }

    fn add(&mut self, other: &Reduction) {
        for kind in REDUCTION_KINDS {
            *self.slot(kind) += other.get(kind);
        }
    }

    fn sub(&mut self, other: &Reduction) {
        for kind in REDUCTION_KINDS {
            *self.slot(kind) = (self.get(kind) - other.get(kind)).max(0.0);
        }
    }

    fn slot(&mut self, kind: &str) -> &mut f64 {
        match kind {
            "projectile" => &mut self.projectile,
            "explosion" => &mut self.explosion,
            "fall" => &mut self.fall,
            "lightning" => &mut self.lightning,
            "magic" => &mut self.magic,
            _ => &mut self.physical,
        }
    }

    fn is_empty(self) -> bool {
        REDUCTION_KINDS.iter().all(|kind| self.get(kind) == 0.0)
    }
}

const REDUCTION_KINDS: [&str; 6] = [
    "physical",
    "projectile",
    "explosion",
    "fall",
    "lightning",
    "magic",
];

/// Writes one damage-reduction roll into the stack's custom data, adding to
/// whatever the piece already carried.
pub fn attach_reduction(stack: &mut ItemStack, kind: &str, value: f64) {
    if !REDUCTION_KINDS.contains(&kind) || value <= 0.0 {
        return;
    }
    let mut reduction = Reduction::of_stack(stack);
    *reduction.slot(kind) += value;
    let mut custom = match stack
        .data()
        .and_then(|data| data.other(CUSTOM_DATA))
    {
        Some(Tag::Compound(compound)) => compound.clone(),
        _ => Compound::new(),
    };
    let mut inner = match custom.get(&REDUCTION_KEY.into()) {
        Some(Tag::Compound(compound)) => compound.clone(),
        _ => Compound::new(),
    };
    for kind in REDUCTION_KINDS {
        let value = reduction.get(kind);
        if value > 0.0 {
            inner.insert(kind.into(), Tag::Double(value.to_bits()));
        } else {
            inner.remove(&kind.into());
        }
    }
    custom.insert(REDUCTION_KEY.into(), Tag::Compound(inner));
    stack
        .data_mut()
        .set_other(CUSTOM_DATA, Some(Tag::Compound(custom)));
}

/// Drops the damage reduction a previous affix pass wrote.
pub fn clear_reduction(stack: &mut ItemStack) {
    let mut custom = match stack
        .data()
        .and_then(|data| data.other(CUSTOM_DATA))
    {
        Some(Tag::Compound(compound)) => compound.clone(),
        _ => return,
    };
    if custom.remove(&REDUCTION_KEY.into()).is_some() {
        stack
            .data_mut()
            .set_other(CUSTOM_DATA, Some(Tag::Compound(custom)));
    }
}

/// The damage class a vanilla damage type belongs to, as Java's
/// `damage_reduction` data names them.
pub fn damage_class(damage_type: &str) -> &'static str {
    let name = damage_type.rsplit(':').next().unwrap_or(damage_type);
    match name {
        "explosion" | "player_explosion" | "bad_respawn_point" => "explosion",
        "arrow" | "trident" | "thrown" | "fireball" | "wind_charge" | "spectral_arrow" => {
            "projectile"
        }
        "fall" | "fly_into_wall" => "fall",
        "lightning_bolt" => "lightning",
        "magic" | "indirect_magic" | "dragon_breath" | "wither" | "sonic_boom" | "thorns" => {
            "magic"
        }
        _ => "physical",
    }
}

/// Adds `attrs` to the stack's `apotheosis:attributes` and records exactly what
/// it added under the custom-data key `record`, taking back whatever that key
/// named on the previous pass first. A reroll therefore replaces its own
/// bonuses instead of stacking them, and values written by a gem or anything
/// else are left alone.
pub fn attach_recorded(stack: &mut ItemStack, attrs: &[(Attr, f64)], record: &str) {
    let mut custom = match stack
        .data()
        .and_then(|data| data.other(CUSTOM_DATA))
    {
        Some(Tag::Compound(compound)) => compound.clone(),
        _ => Compound::new(),
    };
    let previous = match custom.get(&record.into()) {
        Some(Tag::Compound(compound)) => Some(compound.clone()),
        _ => None,
    };
    if attrs.is_empty() && previous.is_none() {
        return;
    }
    let mut inner = match custom.get(&ATTR_KEY.into()) {
        Some(Tag::Compound(compound)) => compound.clone(),
        _ => Compound::new(),
    };
    // Take back the values the last pass wrote, leaving foreign ones in place.
    if let Some(previous) = previous {
        for (name, value) in previous.iter() {
            let key = name.to_string_lossy();
            if !Attr::ALL.iter().any(|attr| attr.key() == key) {
                continue;
            }
            let amount = number(Some(value)).unwrap_or(0.0);
            let left = number(inner.get(name)).unwrap_or(0.0) - amount;
            if left.abs() < 1e-9 {
                inner.remove(name);
            } else {
                inner.insert(name.clone(), Tag::Double(left.to_bits()));
            }
        }
    }
    // Then add this pass's values and record them for the next reroll.
    let mut written = Compound::new();
    for (attr, value) in attrs {
        let key = attr.key().into();
        let current = number(inner.get(&key)).unwrap_or(0.0);
        inner.insert(key.clone(), Tag::Double((current + value).to_bits()));
        let recorded = number(written.get(&key)).unwrap_or(0.0);
        written.insert(key, Tag::Double((recorded + value).to_bits()));
    }
    if inner.is_empty() {
        custom.remove(&ATTR_KEY.into());
    } else {
        custom.insert(ATTR_KEY.into(), Tag::Compound(inner));
    }
    if written.is_empty() {
        custom.remove(&record.into());
    } else {
        custom.insert(record.into(), Tag::Compound(written));
    }
    stack
        .data_mut()
        .set_other(CUSTOM_DATA, Some(Tag::Compound(custom)));
}

/// Writes `attrs` into the stack's `minecraft:custom_data`, adding to whatever
/// the item already carried. Repeated attributes are summed.
pub fn attach(stack: &mut ItemStack, attrs: &[(Attr, f64)]) {
    if attrs.is_empty() {
        return;
    }
    let mut custom = match stack
        .data()
        .and_then(|data| data.other(CUSTOM_DATA))
    {
        Some(Tag::Compound(compound)) => compound.clone(),
        _ => Compound::new(),
    };
    let mut inner = match custom.get(&ATTR_KEY.into()) {
        Some(Tag::Compound(compound)) => compound.clone(),
        _ => Compound::new(),
    };
    for (attr, value) in attrs {
        let key = attr.key().into();
        let current = number(inner.get(&key)).unwrap_or(0.0);
        inner.insert(key, Tag::Double((current + value).to_bits()));
    }
    custom.insert(ATTR_KEY.into(), Tag::Compound(inner));
    stack
        .data_mut()
        .set_other(CUSTOM_DATA, Some(Tag::Compound(custom)));
}

fn number(tag: Option<&Tag>) -> Option<f64> {
    match tag? {
        Tag::Double(bits) => Some(f64::from_bits(*bits)),
        Tag::Float(bits) => Some(f64::from(f32::from_bits(*bits))),
        Tag::Int(value) => Some(f64::from(*value)),
        Tag::Long(value) => Some(*value as f64),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Items and affix classes
// ---------------------------------------------------------------------------

/// What a piece of gear is, from its id (all a loot hook is given). An affix
/// only rolls on the classes it names, so an attack-damage affix never lands on
/// a bow and a boot-only affix never on a helmet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemClass {
    Sword,
    Axe,
    Mace,
    Spear,
    Trident,
    Bow,
    Crossbow,
    Helmet,
    Chestplate,
    Leggings,
    Boots,
    Pickaxe,
    Shovel,
    Hoe,
    Shield,
    Shears,
    Other,
}

pub const MELEE: u16 = 1 << 0;
pub const RANGED: u16 = 1 << 1;
pub const HELMET: u16 = 1 << 2;
pub const CHESTPLATE: u16 = 1 << 3;
pub const LEGGINGS: u16 = 1 << 4;
pub const BOOTS: u16 = 1 << 5;
pub const TOOL: u16 = 1 << 6;
pub const SHIELD: u16 = 1 << 7;
/// The trident is its own Java category, and also a melee weapon.
pub const TRIDENT: u16 = 1 << 8;
pub const SHEARS: u16 = 1 << 9;
/// Every slot a suit of armour covers.
pub const ARMOR: u16 = HELMET | CHESTPLATE | LEGGINGS | BOOTS;

impl ItemClass {
    pub fn mask(self) -> u16 {
        match self {
            ItemClass::Sword | ItemClass::Axe | ItemClass::Mace | ItemClass::Spear => MELEE,
            // Java lists the trident in both melee and ranged affix categories.
            ItemClass::Trident => MELEE | TRIDENT,
            ItemClass::Bow | ItemClass::Crossbow => RANGED,
            ItemClass::Helmet => HELMET,
            ItemClass::Chestplate => CHESTPLATE,
            ItemClass::Leggings => LEGGINGS,
            ItemClass::Boots => BOOTS,
            ItemClass::Pickaxe | ItemClass::Shovel | ItemClass::Hoe => TOOL,
            ItemClass::Shield => SHIELD,
            ItemClass::Shears => SHEARS,
            ItemClass::Other => 0,
        }
    }

    /// Armour a player wears (a shield is held, not worn).
    pub fn is_worn_armor(self) -> bool {
        matches!(
            self,
            ItemClass::Helmet | ItemClass::Chestplate | ItemClass::Leggings | ItemClass::Boots
        )
    }
}

pub fn class_of(item: &str) -> ItemClass {
    if item == "minecraft:bow" {
        return ItemClass::Bow;
    }
    if item == "minecraft:crossbow" {
        return ItemClass::Crossbow;
    }
    if item == "minecraft:trident" {
        return ItemClass::Trident;
    }
    if item == "minecraft:shield" {
        return ItemClass::Shield;
    }
    if item == "minecraft:shears" {
        return ItemClass::Shears;
    }
    if item.ends_with("_sword") {
        return ItemClass::Sword;
    }
    if item.ends_with("_axe") {
        return ItemClass::Axe;
    }
    if item.ends_with("_mace") {
        return ItemClass::Mace;
    }
    if item.ends_with("_spear") {
        return ItemClass::Spear;
    }
    if item.ends_with("_helmet") {
        return ItemClass::Helmet;
    }
    if item.ends_with("_chestplate") {
        return ItemClass::Chestplate;
    }
    if item.ends_with("_leggings") {
        return ItemClass::Leggings;
    }
    if item.ends_with("_boots") {
        return ItemClass::Boots;
    }
    if item.ends_with("_pickaxe") {
        return ItemClass::Pickaxe;
    }
    if item.ends_with("_shovel") {
        return ItemClass::Shovel;
    }
    if item.ends_with("_hoe") {
        return ItemClass::Hoe;
    }
    ItemClass::Other
}

/// Whether an affix with class mask `mask` may roll on `class`.
pub fn accepts(mask: u16, class: ItemClass) -> bool {
    class.mask() & mask != 0
}

// ---------------------------------------------------------------------------
// Default modifiers, merged with the affixes
// ---------------------------------------------------------------------------

/// The `minecraft:attribute_modifiers` an item has before any affix: a
/// diamond sword's `+3 attack damage`, a chestplate's armour. Horizon replaces
/// the component wholesale when a stack carries one, so an affixed item would
/// lose its base stats; this table puts them back. It is filled by
/// `modify_default_components`, which is where the item's defaults are built.
static DEFAULTS: OnceLock<Mutex<HashMap<String, Vec<Tag>>>> = OnceLock::new();

/// Caches the item's default attribute modifiers before anything (a gem) edits
/// them in the same event.
pub fn remember_default_modifiers(event: &ModifyDefaultComponentsEvent<'_>) {
    let Some(value) = event.components.get("minecraft:attribute_modifiers") else {
        return;
    };
    cache_defaults(event.item, value);
}

fn cache_defaults(item: &str, value: &serde_json::Value) {
    let Some(Tag::List(list)) = json_tag(value) else {
        return;
    };
    if list.is_empty() {
        return;
    }
    let mut map = DEFAULTS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    map.insert(item.to_owned(), list);
}

/// The cached default modifiers for `item`, empty before its defaults were
/// seen (an early loot roll, or a unit test).
pub fn base_modifiers(item: &str) -> Vec<Tag> {
    DEFAULTS
        .get()
        .and_then(|map| {
            map.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get(item)
                .cloned()
        })
        .unwrap_or_default()
}

fn json_tag(value: &serde_json::Value) -> Option<Tag> {
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::Bool(value) => Some(Tag::Byte(i8::from(*value))),
        serde_json::Value::Number(value) => Some(Tag::Double(value.as_f64()?.to_bits())),
        serde_json::Value::String(value) => Some(Tag::String(value.as_str().into())),
        serde_json::Value::Array(values) => Some(Tag::List(
            values.iter().filter_map(json_tag).collect(),
        )),
        serde_json::Value::Object(map) => Some(Tag::Compound(
            map.iter()
                .filter_map(|(key, value)| {
                    json_tag(value).map(|tag| (key.as_str().into(), tag))
                })
                .collect(),
        )),
    }
}

// ---------------------------------------------------------------------------
// Rolls
// ---------------------------------------------------------------------------

static ROLLS: AtomicU64 = AtomicU64::new(0x243F_6A88_85A3_08D3);

/// A uniform number in `0..1`, from a counter mixed with `salt`: the hook API
/// gives no random source, so rolls are this mod's own stream.
fn unit(salt: u64) -> f64 {
    let mut z = ROLLS
        .fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed)
        ^ salt;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

// ---------------------------------------------------------------------------
// Damage
// ---------------------------------------------------------------------------

/// The physical damage types the elemental and armour attributes act on.
fn is_physical(damage_type: &str) -> bool {
    let name = damage_type.rsplit(':').next().unwrap_or(damage_type);
    matches!(
        name,
        "player_attack"
            | "mob_attack"
            | "mob_attack_no_aggro"
            | "arrow"
            | "trident"
            | "spear"
            | "thrown"
            | "fireball"
            | "wind_charge"
            | "mace_smash"
    )
}

/// The outgoing amount after the attacker's attributes, and whether it crit.
fn outgoing(set: &AttrSet, amount: f64, roll: f64, physical: bool) -> (f64, bool) {
    let mut out = amount;
    let crit = roll < set.crit_chance();
    if crit {
        out *= set.crit_multiplier();
    }
    if physical {
        out += set.get(Attr::FireDamage) + set.get(Attr::ColdDamage);
    }
    // Armour shred reduces the victim's armour, and pierce ignores a flat
    // part of it: both raise the physical damage by roughly the same share.
    let shred = set.get(Attr::ArmorShred).clamp(0.0, 0.9);
    let pierce = set.get(Attr::ArmorPierce).max(0.0) * 0.02;
    out *= 1.0 + shred + pierce;
    (out.max(0.0), crit)
}

/// The attacker's weapon bonuses and the victim's dodge roll, in the
/// player-weapon hook (v2, which runs before v1 and carries the weapon and a
/// host). True when the hit was dodged and must be canceled.
pub fn incoming_damage_v2(host: &mut dyn Host, event: &mut LivingIncomingDamageV2Event<'_>) -> bool {
    // The dodge roll happens here, once, and v1 reuses this answer: a hit
    // that lands must not be read as dodged just because the victim wears
    // some dodge chance.
    let salt = (u64::from(event.entity as u32) << 32) ^ u64::from(event.amount.to_bits());
    let dodged = dodge_roll(
        armor_of(event.entity as u32).get(Attr::DodgeChance),
        unit(salt),
    );
    record_dodge(event.entity, dodged);
    if dodged {
        return true;
    }
    let Some(weapon) = event.weapon else {
        return false;
    };
    let set = AttrSet::of_stack(weapon);
    if set.is_empty() {
        return false;
    }
    let physical = is_physical(event.damage_type);
    let (amount, _) = outgoing(
        &set,
        f64::from(event.amount),
        unit(salt ^ 0x5DEE_CE66_D1CF_1234),
        physical,
    );
    event.amount = amount as f32;

    // Giant Slaying: a physical hit adds a share of the target's own health.
    let bonus = set.get(Attr::CurrentHpDamage).max(0.0);
    if physical && bonus > 0.0 {
        if let Some(health) = victim_health(host, event.entity as u32) {
            event.amount += (f64::from(health) * bonus) as f32;
        }
    }

    // Nullifying and Runebreaking: the victim's protection points are shaved
    // before the hit lands. Protection is 4% a point, capped at 20 points; the
    // damage is raised by the share that was removed.
    let points = protection_points(host, event.entity as u32);
    if points > 0.0 {
        let share = (points * 0.04).min(0.8);
        let shred = set.get(Attr::ProtShred).clamp(0.0, 1.0);
        let pierce = (set.get(Attr::ProtPierce).max(0.0) * 0.04).min(0.8);
        let extra = share * shred + pierce;
        if extra > 0.0 {
            event.amount = (f64::from(event.amount) * (1.0 + extra)) as f32;
        }
    }

    if let Some(player) = host.acting_player() {
        let health = amount * set.get(Attr::LifeSteal);
        if health > 0.0 {
            heal_player(host, &player, health);
        }
        let absorption = amount * set.get(Attr::Overheal);
        if absorption > 0.0 {
            absorb(host, &player, absorption);
        }
    }
    false
}

/// Whether a dodge roll of `roll` beats `chance`, once the armour cache's
/// bonus and vanilla's floor are applied.
fn dodge_roll(chance: f64, roll: f64) -> bool {
    chance > 0.0 && roll < chance.clamp(0.0, 1.0)
}

/// The dodge decision v2 already made for a victim, so v1 does not roll a
/// second time (and does not cancel a hit life steal has been paid on).
static DODGES: OnceLock<Mutex<HashMap<i32, (bool, Instant)>>> = OnceLock::new();

fn dodges() -> &'static Mutex<HashMap<i32, (bool, Instant)>> {
    DODGES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn record_dodge(entity: i32, dodged: bool) {
    dodges()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(entity, (dodged, Instant::now()));
}

fn take_dodge(entity: i32) -> Option<bool> {
    let (dodged, at) = dodges()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&entity)?;
    (at.elapsed().as_secs() < 2).then_some(dodged)
}

/// The victim-side part, in v1 (all damage routes, no host): a pending bow's
/// damage bonus and the armour's dodge chance. True when the hit was dodged
/// and must be canceled.
pub fn incoming_damage(event: &mut LivingIncomingDamageEvent<'_>) -> bool {
    if let Some(multiplier) = take_arrow(event.direct, event.damage_type) {
        event.amount = (f64::from(event.amount) * multiplier) as f32;
    }
    // Feathery, Runed, Deflective, Blast Forged, Grounded and Blockading: the
    // victim's own armour removes a share of that damage class.
    let share = reduction_of(event.entity as u32).get(damage_class(event.damage_type));
    if share > 0.0 {
        event.amount = (f64::from(event.amount) * (1.0 - share)) as f32;
    }
    if let Some(dodged) = take_dodge(event.entity) {
        return dodged;
    }
    let salt = (u64::from(event.entity as u32) << 32) ^ u64::from(event.amount.to_bits());
    dodge_roll(armor_of(event.entity as u32).get(Attr::DodgeChance), unit(salt))
}

/// The default used by `living_heal`, scaled by the healing-received armour
/// attribute from the entity cache.
pub fn living_heal(event: &mut LivingHealEvent) -> bool {
    let multiplier = armor_of(event.entity).healing_multiplier();
    if (multiplier - 1.0).abs() < f64::EPSILON {
        return false;
    }
    event.amount = (f64::from(event.amount) * multiplier) as f32;
    true
}

/// The victim's current health, read from its own NBT.
fn victim_health(host: &mut dyn Host, id: u32) -> Option<f32> {
    let entity = host.entity(id)?;
    let nbt = host.entity_nbt(entity)?;
    match nbt.get(&"Health".into()) {
        Some(Tag::Float(bits)) => Some(f32::from_bits(*bits)),
        Some(Tag::Double(bits)) => Some(f64::from_bits(*bits) as f32),
        _ => None,
    }
}

/// The victim's protection points: one per Protection level on worn armour,
/// capped at 20. Only a player's equipment is readable through the API, so a
/// mob's protection is left alone rather than guessed.
fn protection_points(host: &mut dyn Host, victim: u32) -> f64 {
    let Some(entity) = host.entity(victim) else {
        return 0.0;
    };
    if host.entity_kind(entity).as_deref() != Some("minecraft:player") {
        return 0.0;
    }
    let Some(uuid) = host.entity_uuid(entity) else {
        return 0.0;
    };
    let mut points = 0.0;
    for slot in [
        EquipmentSlot::Head,
        EquipmentSlot::Chest,
        EquipmentSlot::Legs,
        EquipmentSlot::Feet,
    ] {
        if let Some(stack) = host.player_equipment(uuid, slot) {
            points += f64::from(stack.enchantment_level("minecraft:protection"));
        }
    }
    points.min(20.0)
}

fn heal_player(host: &mut dyn Host, player: &NativeServerPlayer, health: f64) {
    if player.health <= 0.0 {
        return;
    }
    let max = host
        .attribute(player.uuid, "minecraft:max_health")
        .map_or(20.0, |(_, value)| value);
    let health = (f64::from(player.health) + health).min(max) as f32;
    host.set_player_status(player.uuid, Some(health), None, None);
}

/// Overheal: the share of the dealt damage that becomes absorption hearts.
/// The effect's amplifier is the nearest whole half-heart step.
fn absorb(host: &mut dyn Host, player: &NativeServerPlayer, amount: f64) {
    let amplifier = ((amount / 4.0).ceil().max(1.0) as u32)
        .saturating_sub(1)
        .min(u32::from(u8::MAX)) as u8;
    host.add_effect(
        player.uuid,
        &NativeEffect {
            id: "minecraft:absorption".into(),
            amplifier,
            duration: 200,
            ambient: true,
            particles: false,
        },
    );
}

// ---------------------------------------------------------------------------
// Bows
// ---------------------------------------------------------------------------

/// A shot's damage multiplier, from `arrow_loose` until `projectile_launch`
/// names the arrow it became, then on that arrow's own hit. A bow shot is
/// never spent by a sword hit, and two quick shots keep one multiplier each.
///
/// The store is plain data so the binding is testable without a host.
#[derive(Default)]
struct ArrowDamageStore {
    /// Multipliers waiting for a projectile, one queue per shooter.
    pending: HashMap<u128, std::collections::VecDeque<(f64, Instant)>>,
    /// Multipliers bound to a projectile id, with the shooter it belongs to.
    by_projectile: HashMap<i32, (u128, f64, Instant)>,
}

impl ArrowDamageStore {
    /// Every shot takes its place in the queue, a plain one as `1.0`: a shot
    /// without the affix must not let its arrow spend an earlier shot's bonus.
    fn remember_shot(&mut self, shooter: u128, multiplier: f64) {
        if shooter == 0 || !multiplier.is_finite() || multiplier < 1.0 {
            return;
        }
        let queue = self.pending.entry(shooter).or_default();
        queue.retain(|(_, at)| at.elapsed().as_secs() < 5);
        queue.push_back((multiplier, Instant::now()));
    }

    /// Binds the oldest waiting shot of `shooter` to `projectile`, for the
    /// arrow-like kinds a bow or crossbow looses.
    fn projectile_launched(&mut self, shooter: Option<u128>, projectile: u32, kind: &str) -> bool {
        if projectile == 0 || !kind.ends_with("arrow") {
            return false;
        }
        let Some(shooter) = shooter.filter(|id| *id != 0) else {
            return false;
        };
        let Some((multiplier, _)) = self
            .pending
            .get_mut(&shooter)
            .and_then(|queue| queue.pop_front())
        else {
            return false;
        };
        if multiplier <= 1.0 {
            return false;
        }
        self.by_projectile
            .insert(projectile as i32, (shooter, multiplier, Instant::now()));
        true
    }

    /// The multiplier for this hit, only when `direct` is that arrow and the
    /// damage type is an arrow's. `_shooter` is the v1 `causing` entity; the
    /// binding already carries the shooter, so a stray melee hit never takes
    /// an arrow's bonus.
    fn take_for_damage(&mut self, _shooter: i32, direct: i32, damage_type: &str) -> Option<f64> {
        if direct == 0 || !damage_type.ends_with("arrow") {
            return None;
        }
        let (_, multiplier, at) = self.by_projectile.remove(&direct)?;
        (at.elapsed().as_secs() < 5).then_some(multiplier)
    }
}

static ARROWS: OnceLock<Mutex<ArrowDamageStore>> = OnceLock::new();

fn arrows() -> &'static Mutex<ArrowDamageStore> {
    ARROWS.get_or_init(|| Mutex::new(ArrowDamageStore::default()))
}

fn remember_arrow(shooter: u128, multiplier: f64) {
    arrows()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remember_shot(shooter, multiplier);
}

/// `projectile_launch`: the shot `arrow_loose` remembered becomes this arrow.
pub fn projectile_launch(event: &mut ProjectileLaunchEvent<'_>) {
    arrows()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .projectile_launched(event.player, event.entity, event.kind);
}

fn take_arrow(direct: i32, damage_type: &str) -> Option<f64> {
    arrows()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take_for_damage(0, direct, damage_type)
}

/// Draw speed, arrow velocity and arrow damage: the first two become a higher
/// charge, the last waits for the arrow named by `projectile_launch`.
pub fn arrow_loose(host: &mut dyn Host, event: &mut ArrowLooseEvent<'_>) -> bool {
    let set = AttrSet::of_stack(event.stack);
    let faster = set.draw_multiplier() * set.arrow_velocity_multiplier();
    if !set.is_empty() && (faster - 1.0).abs() > f64::EPSILON {
        event.charge = (f64::from(event.charge) * faster).round().clamp(0.0, 100.0) as i32;
    }
    // A plain bow still queues its shot, at 1.0, so the arrow it looses gets
    // that slot rather than a bonus left by an earlier affixed shot.
    if let Some(player) = host.acting_player() {
        remember_arrow(player.uuid, set.arrow_damage_multiplier());
    }
    !set.is_empty()
}

// ---------------------------------------------------------------------------
// Mining and experience
// ---------------------------------------------------------------------------

/// The held tool's mining speed.
pub fn break_speed(host: &dyn Host, event: &mut BreakSpeedEvent) -> bool {
    let Some(stack) = host.equipment(EquipmentSlot::MainHand) else {
        return false;
    };
    let multiplier = AttrSet::of_stack(&stack).mining_multiplier();
    if (multiplier - 1.0).abs() < f64::EPSILON {
        return false;
    }
    event.speed = (f64::from(event.speed) * multiplier) as f32;
    true
}

/// The held tool's experience multiplier, on every orb the player picks up.
pub fn orb(host: &mut dyn Host, event: &mut ExperienceOrbPickupEvent<'_>) -> bool {
    let Some(stack) = host.equipment(EquipmentSlot::MainHand) else {
        return false;
    };
    let multiplier = AttrSet::of_stack(&stack).xp_multiplier();
    if (multiplier - 1.0).abs() < f64::EPSILON {
        return false;
    }
    event.value = (f64::from(event.value) * multiplier).round() as u32;
    true
}

// ---------------------------------------------------------------------------
// The per-entity armour cache
// ---------------------------------------------------------------------------

static ARMOR_CACHE: OnceLock<Mutex<HashMap<u32, AttrSet>>> = OnceLock::new();

fn armor_cache() -> &'static Mutex<HashMap<u32, AttrSet>> {
    ARMOR_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn armor_of(entity: u32) -> AttrSet {
    armor_cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&entity)
        .copied()
        .unwrap_or_default()
}

fn store_armor(entity: u32, set: AttrSet) {
    armor_cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(entity, set);
}

/// The damage reduction of every worn piece, summed per damage class.
static REDUCTION_CACHE: OnceLock<Mutex<HashMap<u32, Reduction>>> = OnceLock::new();

fn reduction_cache() -> &'static Mutex<HashMap<u32, Reduction>> {
    REDUCTION_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn reduction_of(entity: u32) -> Reduction {
    reduction_cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&entity)
        .copied()
        .unwrap_or_default()
}

fn store_reduction(entity: u32, reduction: Reduction) {
    reduction_cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(entity, reduction);
}

fn worn(stack: &ItemStack) -> Option<AttrSet> {
    class_of(&stack.item)
        .is_worn_armor()
        .then(|| AttrSet::of_stack(stack))
}

fn worn_reduction(stack: &ItemStack) -> Reduction {
    if class_of(&stack.item).is_worn_armor() {
        Reduction::of_stack(stack)
    } else {
        Reduction::default()
    }
}

/// An armour slot changed: take the old piece's attributes off and put the new
/// piece's on. `from` is none when the slot was empty.
pub fn equipment_changed(event: &mut LivingEquipmentChangeEvent<'_>) {
    let mut set = armor_of(event.entity);
    let mut reduction = reduction_of(event.entity);
    if let Some(from) = event.from.as_ref() {
        if let Some(attrs) = worn(from) {
            set.sub(&attrs);
        }
        reduction.sub(&worn_reduction(from));
    }
    if let Some(to) = event.to.as_ref() {
        if let Some(attrs) = worn(to) {
            set.add(&attrs);
        }
        reduction.add(&worn_reduction(to));
    }
    store_armor(event.entity, set);
    store_reduction(event.entity, reduction);
}

/// A player put a piece on: `on_equip` names no entity, so the acting player
/// is the one whose cache changes.
pub fn equipped(host: &mut dyn Host, event: &mut OnEquipEvent<'_>) {
    let Some(new) = event.new.as_ref() else {
        return;
    };
    let Some(stack) = worn(new) else {
        return;
    };
    let Some(player) = host.acting_player() else {
        return;
    };
    if player.entity_id == 0 {
        return;
    }
    let id = player.entity_id as u32;
    let mut set = armor_of(id);
    set.add(&stack);
    store_armor(id, set);
    let mut reduction = reduction_of(id);
    reduction.add(&worn_reduction(new));
    store_reduction(id, reduction);
}

/// Refreshes the acting player's whole armour from the equipment slots, which
/// catches armour already worn when the world opened.
pub fn tick(host: &mut dyn Host, _event: &mut PlayerTickEvent) {
    let Some(player) = host.acting_player() else {
        return;
    };
    if player.entity_id == 0 {
        return;
    }
    let mut set = AttrSet::default();
    let mut reduction = Reduction::default();
    for slot in [
        EquipmentSlot::Head,
        EquipmentSlot::Chest,
        EquipmentSlot::Legs,
        EquipmentSlot::Feet,
    ] {
        if let Some(stack) = host.equipment(slot) {
            if let Some(attrs) = worn(&stack) {
                set.add(&attrs);
            }
            reduction.add(&worn_reduction(&stack));
        }
    }
    store_armor(player.entity_id as u32, set);
    store_reduction(player.entity_id as u32, reduction);
}

// ---------------------------------------------------------------------------
// Tooltip
// ---------------------------------------------------------------------------

/// The attribute lines an affixed item shows when it has no affix lore. New
/// items carry Java's one-line-per-affix tooltip, so this fallback only covers
/// pieces rolled before that lore existed.
pub fn tooltip(event: &mut TooltipEvent<'_>) {
    if event
        .stack
        .data()
        .and_then(|data| data.other("minecraft:lore"))
        .is_some()
    {
        return;
    }
    let set = AttrSet::of_stack(event.stack);
    if set.is_empty() {
        return;
    }
    for attr in Attr::ALL {
        let value = set.get(attr);
        if value == 0.0 {
            continue;
        }
        let key = format!("apotheosis.attribute.{}", attr.key());
        let line = Text::translate_with_fallback(&key, attr.name(), &[])
            .append(Text::literal(format!(" {}", attr.format(value))))
            .with_color(0x55_55FF);
        event.lines.push((line, crate::tip_colour(0x55_55FF)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::one;

    #[test]
    fn a_bow_never_takes_melee_affixes() {
        assert_eq!(class_of("minecraft:bow"), ItemClass::Bow);
        assert_eq!(class_of("minecraft:crossbow"), ItemClass::Crossbow);
        assert_eq!(class_of("minecraft:netherite_sword"), ItemClass::Sword);
        assert!(accepts(MELEE, ItemClass::Sword));
        assert!(!accepts(MELEE, ItemClass::Bow));
        assert!(!accepts(MELEE, ItemClass::Pickaxe));
        assert!(accepts(RANGED, ItemClass::Bow));
        assert!(!accepts(RANGED, ItemClass::Axe));
    }

    #[test]
    fn armor_affixes_follow_the_slot() {
        assert!(accepts(BOOTS, ItemClass::Boots));
        assert!(!accepts(BOOTS, ItemClass::Helmet));
        assert!(accepts(HELMET, ItemClass::Helmet));
        assert!(accepts(ARMOR, ItemClass::Chestplate));
        assert!(accepts(ARMOR, ItemClass::Leggings));
        assert!(!accepts(TOOL, ItemClass::Shield));
        assert!(accepts(SHIELD, ItemClass::Shield));
        assert!(!accepts(ARMOR, ItemClass::Bow));
    }

    #[test]
    fn attributes_round_trip_through_custom_data() {
        let mut stack = one("minecraft:iron_sword");
        attach(
            &mut stack,
            &[(Attr::LifeSteal, 0.05), (Attr::CritChance, 0.1)],
        );
        attach(&mut stack, &[(Attr::LifeSteal, 0.05)]);
        let set = AttrSet::of_stack(&stack);
        assert!((set.get(Attr::LifeSteal) - 0.1).abs() < 1e-9);
        assert!((set.get(Attr::CritChance) - 0.1).abs() < 1e-9);
        assert_eq!(set.get(Attr::Overheal), 0.0);
        // A plain item has none.
        assert!(AttrSet::of_stack(&one("minecraft:dirt")).is_empty());
    }

    #[test]
    fn attaching_keeps_other_custom_data() {
        let mut stack = one("minecraft:iron_sword");
        stack.data_mut().set_other(
            CUSTOM_DATA,
            Some(Tag::compound([("other_mod:mark", Tag::Byte(1))])),
        );
        attach(&mut stack, &[(Attr::Overheal, 0.05)]);
        let data = stack.data().expect("components");
        let Some(Tag::Compound(custom)) = data.other(CUSTOM_DATA) else {
            panic!("custom data was replaced");
        };
        assert!(custom.get(&"other_mod:mark".into()).is_some());
        assert!((AttrSet::of_stack(&stack).get(Attr::Overheal) - 0.05).abs() < 1e-9);
    }

    #[test]
    fn crit_raises_damage_and_elemental_damage_only_hits_the_physical() {
        let mut set = AttrSet::default();
        set.set(Attr::CritDamage, 0.5);
        let (plain, crit) = outgoing(&set, 10.0, 0.99, true);
        assert!(!crit);
        assert_eq!(plain, 10.0);
        let (hit, crit) = outgoing(&set, 10.0, 0.0, true);
        assert!(crit);
        assert!((hit - 20.0).abs() < 1e-9, "1.5 + 0.5 is a 2x crit");

        let mut set = AttrSet::default();
        set.set(Attr::FireDamage, 2.0);
        set.set(Attr::ColdDamage, 1.0);
        assert_eq!(outgoing(&set, 10.0, 0.99, true).0, 13.0);
        assert_eq!(outgoing(&set, 10.0, 0.99, false).0, 10.0);
    }

    #[test]
    fn pierce_and_shred_raise_the_physical_hit() {
        let mut set = AttrSet::default();
        set.set(Attr::ArmorPierce, 4.0);
        set.set(Attr::ArmorShred, 0.2);
        let (hit, _) = outgoing(&set, 10.0, 0.99, true);
        // 4 pierce points are 8%, plus 20% shred.
        assert!((hit - 12.8).abs() < 1e-9);
    }

    #[test]
    fn dodge_uses_the_roll_not_just_a_nonzero_chance() {
        assert!(dodge_roll(0.05, 0.049));
        assert!(!dodge_roll(0.05, 0.05));
        assert!(!dodge_roll(0.05, 0.95));
        assert!(!dodge_roll(0.0, 0.0));
    }

    #[test]
    fn arrow_damage_is_bound_to_its_projectile_and_arrow_damage_type() {
        let mut arrows = ArrowDamageStore::default();
        arrows.remember_shot(77, 1.6);
        arrows.remember_shot(77, 1.25);
        arrows.projectile_launched(Some(77), 101, "minecraft:arrow");
        arrows.projectile_launched(Some(77), 102, "minecraft:arrow");

        assert_eq!(arrows.take_for_damage(999, 101, "minecraft:player_attack"), None);
        assert_eq!(arrows.take_for_damage(999, 101, "minecraft:arrow"), Some(1.6));
        assert_eq!(arrows.take_for_damage(999, 102, "minecraft:arrow"), Some(1.25));
        assert_eq!(arrows.take_for_damage(77, 77, "minecraft:player_attack"), None);

        arrows.remember_shot(77, 1.4);
        arrows.projectile_launched(Some(77), 103, "minecraft:arrow");
        assert_eq!(arrows.take_for_damage(77, 103, "minecraft:mob_attack"), None);
        assert_eq!(arrows.take_for_damage(77, 103, "minecraft:arrow"), Some(1.4));
    }

    #[test]
    fn non_arrow_projectiles_do_not_consume_a_bow_shot_bonus() {
        let mut arrows = ArrowDamageStore::default();
        arrows.remember_shot(88, 1.6);
        arrows.projectile_launched(Some(88), 201, "minecraft:trident");
        assert_eq!(arrows.take_for_damage(88, 201, "minecraft:trident"), None);
        assert_eq!(arrows.take_for_damage(88, 201, "minecraft:arrow"), None);

        // A plain shot queues 1.0: it holds its place in the queue, so the
        // affixed shot before it keeps its own arrow and the plain one none.
        let mut mixed = ArrowDamageStore::default();
        mixed.remember_shot(88, 1.6);
        mixed.remember_shot(88, 1.0);
        mixed.projectile_launched(Some(88), 301, "minecraft:arrow");
        mixed.projectile_launched(Some(88), 302, "minecraft:arrow");
        assert_eq!(mixed.take_for_damage(88, 301, "minecraft:arrow"), Some(1.6));
        assert_eq!(mixed.take_for_damage(88, 302, "minecraft:arrow"), None);
    }

    #[test]
    fn healing_and_experience_multipliers_start_at_one() {
        let mut set = AttrSet::default();
        assert_eq!(set.healing_multiplier(), 1.0);
        assert_eq!(set.xp_multiplier(), 1.0);
        assert_eq!(set.mining_multiplier(), 1.0);
        set.set(Attr::HealingReceived, 0.25);
        set.set(Attr::ExperienceGained, 0.5);
        set.set(Attr::MiningSpeed, 0.15);
        assert_eq!(set.healing_multiplier(), 1.25);
        assert_eq!(set.xp_multiplier(), 1.5);
        assert_eq!(set.mining_multiplier(), 1.15);
    }

    #[test]
    fn default_modifiers_are_merged_when_known() {
        let value = serde_json::json!([{
            "type": "minecraft:attack_damage",
            "id": "minecraft:base_attack_damage",
            "amount": 3.0,
            "operation": "add_value",
            "slot": "mainhand",
        }]);
        cache_defaults("minecraft:test_sword", &value);
        let base = base_modifiers("minecraft:test_sword");
        assert_eq!(base.len(), 1);
        assert_eq!(
            base[0].get("type"),
            Some(&Tag::String("minecraft:attack_damage".into()))
        );
        assert_eq!(
            number(base[0].get("amount")),
            Some(3.0),
            "the amount survives as a double"
        );
        assert!(base_modifiers("minecraft:test_unknown_pickaxe").is_empty());
    }

    #[test]
    fn an_attribute_is_stored_under_its_apothic_id() {
        assert_eq!(Attr::LifeSteal.id(), "apothic_attributes:life_steal");
        assert_eq!(Attr::CritChance.default_value(), 0.05);
        assert_eq!(Attr::CritDamage.default_value(), 1.5);
        assert!(Attr::ArmorPierce.format(2.0) == "+2");
        assert!(Attr::CritChance.format(0.05) == "+5%");
        assert_eq!(Attr::ProtShred.format(0.2), "+20%");
        assert_eq!(Attr::ProtPierce.format(2.0), "+2");
        assert_eq!(Attr::CurrentHpDamage.format(0.15), "+0.1");
    }

    #[test]
    fn damage_types_fall_in_java_s_classes() {
        assert_eq!(damage_class("minecraft:arrow"), "projectile");
        assert_eq!(damage_class("minecraft:trident"), "projectile");
        assert_eq!(damage_class("minecraft:explosion"), "explosion");
        assert_eq!(damage_class("minecraft:fall"), "fall");
        assert_eq!(damage_class("minecraft:lightning_bolt"), "lightning");
        assert_eq!(damage_class("minecraft:magic"), "magic");
        assert_eq!(damage_class("minecraft:player_attack"), "physical");
        assert_eq!(damage_class("minecraft:mob_attack"), "physical");
    }

    #[test]
    fn a_reduction_round_trips_and_sums_per_class() {
        let mut stack = one("minecraft:diamond_boots");
        assert!(Reduction::of_stack(&stack).is_empty());
        attach_reduction(&mut stack, "fall", 0.2);
        attach_reduction(&mut stack, "fall", 0.1);
        attach_reduction(&mut stack, "magic", 0.05);
        let reduction = Reduction::of_stack(&stack);
        assert!((reduction.get("fall") - 0.3).abs() < 1.0e-9);
        assert!((reduction.get("magic") - 0.05).abs() < 1.0e-9);
        assert_eq!(reduction.get("projectile"), 0.0);
        clear_reduction(&mut stack);
        assert!(Reduction::of_stack(&stack).is_empty());
    }
}
