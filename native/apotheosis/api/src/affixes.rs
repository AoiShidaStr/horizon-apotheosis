//! The affix roll and its presentation.
//!
//! `java_affixes` is generated from Apotheosis 8.9.0's data: every affix keeps
//! its Java id, category list, rarity list and per-rarity range. This module
//! turns that catalogue into a roll: only `supported` affixes enter the pool,
//! an affix only lands on a category it names, and the amount is drawn from the
//! range of the tier that was rolled.
//!
//! The item's name and lore follow Java's shape: a prefix and a suffix around
//! the base item name, and one `+X Attribute` lore line per affix in the
//! rarity colour. `misc.apotheosis.affix_name.*` and `affix.*` are Java's
//! translation keys, so a client with the Java language file reads the Java
//! names; our lang files carry short fallbacks written for this port.

use crate::attributes::{self, ItemClass};
use crate::java_affixes::{Affix, Range, AFFIXES};
use crate::{string, Rng, Tier};
use horizon_client::mod_api::{Compound, ItemStack, Tag};

/// Java's `Affix.MAX_LEVEL`: 1.0 is a normal top roll, 2.0 an overcharged one.
pub const MAX_LEVEL: f64 = 2.0;

/// The level the Sigil of Supremacy raises every affix to, and the floor an
/// overcharged affix keeps.
pub const SUPREMACY_LEVEL: f64 = 1.5;

/// The chance, in 1000, that the Sigil of Malice leaves the other affixes alone
/// when only one of them can still be upgraded (Java's `FINAL_MALICE_CHANCE`).
const FINAL_MALICE_CHANCE: u64 = 33;

/// `minecraft:custom_data` key holding one level per affix id. A piece rolled
/// before levels were kept has no such key: its modifier amounts are the truth.
const LEVELS_KEY: &str = "apotheosis:affix_levels";

/// `minecraft:custom_data` key recording the Apothic attribute values the last
/// affix pass wrote, so a reroll takes them back instead of stacking them.
pub(crate) const AFFIX_ATTRS_KEY: &str = "apotheosis:affix_attrs";

const MODIFIERS: &str = "minecraft:attribute_modifiers";
const LORE: &str = "minecraft:lore";
const CUSTOM_DATA: &str = "minecraft:custom_data";

/// Java's rarity bits, in `Tier` order (common..mythic).
pub fn rarity_index(tier: Tier) -> usize {
    tier.rarity_index()
}

/// The item-class mask an affix's Java categories name.
pub fn classes(affix: &Affix) -> u16 {
    let mut mask = 0;
    for category in affix.categories {
        mask |= match *category {
            "apotheosis:melee_weapon" | "apotheosis:weapon" => attributes::MELEE,
            "apotheosis:trident" => attributes::TRIDENT,
            "apotheosis:bow" => attributes::RANGED,
            "apotheosis:breaker" => attributes::TOOL,
            "apotheosis:shears" => attributes::SHEARS,
            "apotheosis:shield" => attributes::SHIELD,
            "apotheosis:helmet" => attributes::HELMET,
            "apotheosis:chestplate" => attributes::CHESTPLATE,
            "apotheosis:leggings" => attributes::LEGGINGS,
            "apotheosis:boots" => attributes::BOOTS,
            "apotheosis:armor" => attributes::ARMOR,
            _ => 0,
        };
    }
    mask
}

/// Whether `affix` may land on a piece of `class` at `tier` as an attribute
/// line. Effect affixes (`mob_effect`, `damage_reduction`, ...) are rolled by
/// `affix_effects` instead: their payout is not an attribute modifier.
pub fn accepts(affix: &Affix, class: ItemClass, tier: Tier) -> bool {
    affix.supported
        && affix.kind == "attribute"
        && affix
            .rarities
            .contains(&(rarity_index(tier) as u8))
        && attributes::accepts(classes(affix), class)
}

/// Every affix that can land on this piece at this tier, in table order.
pub fn pool(class: ItemClass, tier: Tier) -> Vec<&'static Affix> {
    AFFIXES
        .iter()
        .filter(|affix| accepts(affix, class, tier))
        .collect()
}

/// Draw an amount from the affix's range for `tier`, snapped to Java's step.
/// `None` when the affix has no value for that rarity: the caller should not
/// have picked it, and an item is better off with one line fewer.
pub fn amount(affix: &Affix, tier: Tier, rng: &mut Rng) -> Option<f64> {
    value_at(affix, tier, roll_level(rng))
}

pub(crate) fn roll_range(range: Range, rng: &mut Rng) -> f64 {
    snap(range, range.min + (range.max - range.min) * (rng.below(10_001) as f64 / 10_000.0))
}

/// The amount a level names, as Java's `StepFunction` interpolates it: level 0
/// is the range's low end, 1.0 its normal top, and above 1.0 overcharges the
/// affix past it (up to twice the range at `MAX_LEVEL`).
pub fn value_at(affix: &Affix, tier: Tier, level: f64) -> Option<f64> {
    let range = affix.values[rarity_index(tier)]?;
    Some(snap(range, range.min + (range.max - range.min) * level))
}

/// The level a stored amount implies, for a piece rolled before levels were
/// kept. A constant range has no meaningful level.
pub fn level_of(affix: &Affix, tier: Tier, value: f64) -> f64 {
    let Some(range) = affix.values[rarity_index(tier)] else {
        return 0.0;
    };
    let span = range.max - range.min;
    if span.abs() < f64::EPSILON {
        return 0.0;
    }
    ((value - range.min) / span).clamp(0.0, MAX_LEVEL)
}

/// A level a loot or reforge roll writes: in `[0, 1)`, so a fresh piece is
/// never overcharged.
pub fn roll_level(rng: &mut Rng) -> f64 {
    rng.below(10_000) as f64 / 10_000.0
}

fn snap(range: Range, value: f64) -> f64 {
    let mut value = value;
    if range.step != 0.0 {
        // Negative steps are real (fireproof goes -0.15 to -0.35 by -0.05).
        let steps = ((value - range.min) / range.step).round();
        value = range.min + steps * range.step;
    }
    (value * 1_000_000.0).round() / 1_000_000.0
}

/// One affix on a piece, with the level it was rolled at and the amount that
/// level gives. `level` is absent on a piece from before levels were stored:
/// its `value` is then whatever the old roll wrote, kept as it is.
#[derive(Clone, Copy, Debug)]
pub struct Instance {
    pub affix: &'static Affix,
    pub level: Option<f64>,
    pub value: f64,
}

impl Instance {
    /// Whether the affix's payout follows its level. A constant range (min ==
    /// max) does not, and Java's Sigil of Malice skips such an affix.
    pub fn level_dependent(&self, tier: Tier) -> bool {
        self.affix.values[rarity_index(tier)].is_some_and(|range| range.min != range.max)
    }

    /// The stored level, or the one the old amount implies.
    pub fn level_at(&self, tier: Tier) -> f64 {
        match self.level {
            Some(level) => level,
            None => level_of(self.affix, tier, self.value),
        }
    }

    /// Sets the level and recomputes the amount from it, as Java does after an
    /// upgrade or a reset.
    pub fn set_level(&mut self, tier: Tier, level: f64) {
        let level = level.clamp(0.0, MAX_LEVEL);
        self.level = Some(level);
        if let Some(value) = value_at(self.affix, tier, level) {
            self.value = value;
        }
    }
}

/// The affixes a piece carries, in modifier order. Empty for a piece without
/// this port's tier or affix modifiers.
pub fn instances(stack: &ItemStack) -> Vec<Instance> {
    // A piece this port never rolled has no tier and no affix modifiers.
    if crate::sockets::tier_of(stack).is_none() {
        return Vec::new();
    }
    let levels = stored_levels(stack);
    let Some(Tag::List(entries)) = stack.data().and_then(|data| data.other(MODIFIERS)) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries {
        if !is_affix_modifier(entry) {
            continue;
        }
        let Some(Tag::String(id)) = entry.get("id") else {
            continue;
        };
        let id = id.to_string_lossy();
        let Some(affix) = AFFIXES.iter().find(|affix| affix.id == id.as_str()) else {
            continue;
        };
        let Some(value) = number(entry.get("amount")) else {
            continue;
        };
        let level = levels
            .as_ref()
            .and_then(|map| number(map.get(&id.as_str().into())));
        found.push(Instance {
            affix,
            level,
            value,
        });
    }
    found
}

/// The indices of the level-dependent affixes in `instances`, in order.
pub fn level_targets(instances: &[Instance], tier: Tier) -> Vec<usize> {
    instances
        .iter()
        .enumerate()
        .filter(|(_, instance)| instance.level_dependent(tier))
        .map(|(index, _)| index)
        .collect()
}

/// The levels of `targets`' affixes, in order: the pure input of `malice` and
/// `supremacy`.
pub fn levels_of(instances: &[Instance], targets: &[usize], tier: Tier) -> Vec<f64> {
    targets
        .iter()
        .map(|&index| instances[index].level_at(tier))
        .collect()
}

/// What a Sigil of Malice does to a piece: which target it overcharges and,
/// unless nothing is reset, which one it drops back to level 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Malice {
    pub up: usize,
    pub reset: Option<usize>,
}

/// Java's `AffixHelper.applyMalice` choice, without the item: `levels` are the
/// level-dependent affixes' levels. `None` means the sigil cannot apply — fewer
/// than two such affixes, or all of them already at `MAX_LEVEL`.
///
/// With two or more upgradable affixes, two distinct indices are chosen among
/// every target (those already at the maximum included). With exactly one, it
/// is always the one raised, and only on `FINAL_MALICE_CHANCE` is nothing
/// reset; otherwise one of the others is reset.
pub fn can_apply_malice(levels: &[f64]) -> bool {
    levels.len() >= 2 && !levels.iter().all(|level| *level >= MAX_LEVEL)
}

pub fn malice(levels: &[f64], rng: &mut Rng) -> Option<Malice> {
    if !can_apply_malice(levels) {
        return None;
    }
    let upgradable: Vec<usize> = levels
        .iter()
        .enumerate()
        .filter(|(_, level)| **level < MAX_LEVEL)
        .map(|(index, _)| index)
        .collect();
    if upgradable.len() == 1 {
        let up = upgradable[0];
        if rng.below(1_000) < FINAL_MALICE_CHANCE {
            return Some(Malice { up, reset: None });
        }
        // The others' list is the targets minus the buffed one, in order.
        let mut reset = rng.below(levels.len() as u64 - 1) as usize;
        if reset >= up {
            reset += 1;
        }
        return Some(Malice {
            up,
            reset: Some(reset),
        });
    }
    let up = rng.below(levels.len() as u64) as usize;
    let reset = loop {
        let pick = rng.below(levels.len() as u64) as usize;
        if pick != up {
            break pick;
        }
    };
    Some(Malice {
        up,
        reset: Some(reset),
    })
}

/// Java's `AffixHelper.applySupremacy`: every level rises to at least
/// `SUPREMACY_LEVEL`, and one already above it keeps its own level. `None` for
/// a piece with no affix at all.
pub fn supremacy(levels: &[f64]) -> Option<Vec<f64>> {
    if levels.is_empty() {
        return None;
    }
    Some(
        levels
            .iter()
            .map(|level| level.max(SUPREMACY_LEVEL).min(MAX_LEVEL))
            .collect(),
    )
}

/// Rewrites a piece's affix modifiers, stat lines, stored levels and recorded
/// Apothic attributes from `instances`, keeping its sockets' entries, its effect
/// affixes and anything another mod or the player wrote. The affix name and the
/// rarity are left alone: a level change never rerolls them.
pub fn rewrite(stack: &mut ItemStack, tier: Tier, instances: &[Instance]) {
    let class = attributes::class_of(&stack.item);
    let mut modifiers = match stack.data().and_then(|data| data.other(MODIFIERS)) {
        Some(Tag::List(entries)) => entries
            .iter()
            .filter(|entry| !is_affix_modifier(entry))
            .cloned()
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    let mut lore = match stack.data().and_then(|data| data.other(LORE)) {
        Some(Tag::List(lines)) => lines
            .iter()
            .filter(|line| !is_stat_line(line))
            .cloned()
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    // The item's own modifiers come first, as in `apply_affixes`: writing the
    // component replaces the defaults.
    for entry in attributes::base_modifiers(&stack.item) {
        let id = modifier_id(&entry);
        if id.is_none() || !modifiers.iter().any(|kept| modifier_id(kept) == id) {
            modifiers.push(entry);
        }
    }
    let mut custom: Vec<(attributes::Attr, f64)> = Vec::new();
    let mut levels = Compound::new();
    for instance in instances {
        let (Some(attribute), Some(operation)) =
            (instance.affix.attribute, instance.affix.operation)
        else {
            continue;
        };
        modifiers.push(Tag::compound([
            ("type", string(attribute)),
            ("id", string(instance.affix.id)),
            ("amount", Tag::Double(instance.value.to_bits())),
            ("operation", string(operation)),
            ("slot", string(slot(class))),
        ]));
        lore.push(stat_line(instance.affix, tier, instance.value));
        if let Some(attr) = instance.affix.attr {
            custom.push((attr, instance.value));
        }
        if let Some(level) = instance.level {
            levels.insert(instance.affix.id.into(), Tag::Double(level.to_bits()));
        }
    }
    let data = stack.data_mut();
    data.set_other(
        MODIFIERS,
        (!modifiers.is_empty()).then_some(Tag::List(modifiers)),
    );
    data.set_other(LORE, (!lore.is_empty()).then_some(Tag::List(lore)));
    let mut map = match data.other(CUSTOM_DATA) {
        Some(Tag::Compound(compound)) => compound.clone(),
        _ => Compound::new(),
    };
    if levels.is_empty() {
        map.remove(&LEVELS_KEY.into());
    } else {
        map.insert(LEVELS_KEY.into(), Tag::Compound(levels));
    }
    data.set_other(CUSTOM_DATA, Some(Tag::Compound(map)));
    attributes::attach_recorded(stack, &custom, AFFIX_ATTRS_KEY);
}

fn stored_levels(stack: &ItemStack) -> Option<Compound> {
    let Some(Tag::Compound(custom)) = stack.data().and_then(|data| data.other(CUSTOM_DATA)) else {
        return None;
    };
    match custom.get(&LEVELS_KEY.into()) {
        Some(Tag::Compound(map)) => Some(map.clone()),
        _ => None,
    }
}

pub(crate) fn modifier_id(entry: &Tag) -> Option<String> {
    match entry.get("id")? {
        Tag::String(id) => Some(id.to_string_lossy()),
        _ => None,
    }
}

/// Whether a modifier entry was written by an affix pass of this module. The
/// ids are the Java affix ids; socket modifiers use `apotheosis:socket_*` and
/// are left to `sockets`.
pub(crate) fn is_affix_modifier(entry: &Tag) -> bool {
    modifier_id(entry)
        .is_some_and(|id| AFFIXES.iter().any(|affix| affix.id == id))
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

/// Whether a lore line is one of this module's `+X Attribute` lines, as opposed
/// to an effect affix's line or someone else's.
fn is_stat_line(entry: &Tag) -> bool {
    matches!(
        entry.get("translate"),
        Some(Tag::String(key)) if key.to_string_lossy() == "apotheosis.affix.stat"
    )
}

/// The slot group the real modifier belongs to, from the piece it landed on.
/// Java derives it from the item, not from the hand it happens to be in: a
/// weapon or tool works in the main hand, a shield in the off hand, and a
/// piece of armour in its own armour slot. `hand` (both hands) would let a
/// socketed sword's bonus pay out from the off hand as well.
pub fn slot(class: ItemClass) -> &'static str {
    match class {
        ItemClass::Helmet => "head",
        ItemClass::Chestplate => "chest",
        ItemClass::Leggings => "legs",
        ItemClass::Boots => "feet",
        ItemClass::Shield => "offhand",
        _ => "mainhand",
    }
}

/// The attribute's tooltip name: our Apothic key for an Apothic attribute, the
/// game's own `attribute.name.*` key for a real one.
fn attribute_label(affix: &Affix) -> (String, String) {
    if let Some(attr) = affix.attr {
        return (
            format!("apotheosis.attribute.{}", attr.key()),
            attr.name().to_string(),
        );
    }
    let id = affix.attribute.unwrap_or("minecraft:unknown");
    let path = id.rsplit(':').next().unwrap_or(id).rsplit('.').next().unwrap_or(id);
    let words: Vec<&str> = path.split('_').collect();
    let fallback = words
        .iter()
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ");
    (format!("attribute.name.{path}"), fallback)
}

/// `+5%` for a multiplier, `+2` for an additive value, `-0.15` for a negative.
pub fn format_amount(affix: &Affix, value: f64) -> String {
    if let Some(attr) = affix.attr {
        return attr.format(value);
    }
    let percent = affix
        .operation
        .map(|operation| operation.contains("multiplied"))
        .unwrap_or(false);
    let shown = if percent { value * 100.0 } else { value };
    let sign = if shown < 0.0 { "-" } else { "+" };
    let magnitude = shown.abs();
    let body = if (magnitude - magnitude.round()).abs() < 0.005 {
        format!("{magnitude:.0}")
    } else if (magnitude * 10.0 - (magnitude * 10.0).round()).abs() < 0.005 {
        format!("{magnitude:.1}")
    } else {
        format!("{magnitude:.2}")
    };
    format!("{sign}{body}{}", if percent { "%" } else { "" })
}

/// One lore line, `+X Attribute`, in the rolled rarity's colour.
pub fn stat_line(affix: &Affix, tier: Tier, value: f64) -> Tag {
    let (key, fallback) = attribute_label(affix);
    let amount = format_amount(affix, value);
    let plain = format!("{amount} {fallback}");
    Tag::compound([
        ("translate", string("apotheosis.affix.stat")),
        ("fallback", string(&plain)),
        ("color", string(&tier.hex())),
        ("italic", Tag::Byte(0)),
        (
            "with",
            Tag::List(vec![
                string(&amount),
                Tag::compound([
                    ("translate", string(&key)),
                    ("fallback", string(&fallback)),
                ]),
            ]),
        ),
    ])
}

/// `diamond_sword` -> `Diamond Sword`, for the name without a language file.
fn humanize(path: &str) -> String {
    path.split('_')
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn item_name(item: &str) -> Tag {
    let (namespace, path) = item.split_once(':').unwrap_or(("minecraft", item));
    let key = if namespace == "minecraft" {
        format!("item.minecraft.{path}")
    } else {
        format!("item.{namespace}.{path}")
    };
    Tag::compound([
        ("translate", string(&key)),
        ("fallback", string(&humanize(path))),
    ])
}

fn affix_name(affix: &Affix) -> Tag {
    Tag::compound([
        ("translate", string(affix.name_key)),
        ("fallback", string(affix.name)),
    ])
}

fn affix_suffix(affix: &Affix) -> Tag {
    Tag::compound([
        ("translate", string(affix.suffix_key)),
        ("fallback", string(affix.suffix)),
    ])
}

/// The `minecraft:custom_name` component for the rolled affixes, unless the
/// item already had a player-given name. Java composes prefix + item + suffix
/// with its own translation keys, so both languages read naturally.
pub fn item_name_component(
    item: &str,
    tier: Tier,
    chosen: &[&Affix],
) -> Option<Tag> {
    let prefix = chosen.first()?;
    let suffix = chosen.last().unwrap_or(prefix);
    let key = "misc.apotheosis.affix_name.three";
    let fallback = format!("{} {} {}", prefix.name, humanize(item.rsplit(':').next()?), suffix.suffix);
    Some(Tag::compound([
        ("translate", string(key)),
        ("fallback", string(&fallback)),
        ("color", string(&tier.hex())),
        ("italic", Tag::Byte(0)),
        (
            "with",
            Tag::List(vec![
                affix_name(prefix),
                item_name(item),
                affix_suffix(suffix),
            ]),
        ),
    ]))
}

/// Colours a kept `minecraft:custom_name` with the tier's Java colour. Java has
/// a rarity component that colours a renamed piece; Horizon only knows
/// vanilla's four rarities, so the name itself carries the colour and a mythic
/// piece stays orange after an anvil rename. The name's own text and styling
/// (a gradient, extra spans) are kept.
pub fn recolour_name(name: &Tag, tier: Tier) -> Tag {
    let colour = string(&tier.hex());
    match name {
        Tag::String(text) => Tag::compound([
            ("text", Tag::String(text.clone())),
            ("color", colour),
        ]),
        Tag::Compound(fields) => {
            let mut fields = fields.clone();
            fields.insert("color".into(), colour);
            Tag::Compound(fields)
        }
        other => other.clone(),
    }
}

/// Whether a `minecraft:custom_name` was written by an earlier roll of ours,
/// in which case a reroll may replace it; a player's name is left alone.
pub fn is_generated_name(name: &Tag) -> bool {
    let Tag::Compound(compound) = name else {
        return false;
    };
    matches!(
        compound.get(&"translate".into()),
        Some(Tag::String(key)) if key.to_string_lossy().starts_with("misc.apotheosis.affix_name.")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(path: &str) -> &'static Affix {
        AFFIXES
            .iter()
            .find(|affix| affix.path == path)
            .unwrap_or_else(|| panic!("missing Java affix {path}"))
    }

    /// The affix amounts a piece carries, by id.
    fn amounts(stack: &ItemStack) -> std::collections::BTreeMap<String, f64> {
        let Some(Tag::List(entries)) = stack
            .data()
            .and_then(|data| data.other("minecraft:attribute_modifiers"))
        else {
            return Default::default();
        };
        entries
            .iter()
            .filter(|entry| is_affix_modifier(entry))
            .filter_map(|entry| {
                let Some(Tag::String(id)) = entry.get("id") else {
                    return None;
                };
                Some((
                    id.to_string_lossy(),
                    number(entry.get("amount")).unwrap_or(0.0),
                ))
            })
            .collect()
    }

    fn affixed(item: &str, tier: Tier, seed: u64) -> ItemStack {
        let mut stack = ItemStack::new(item, 1);
        let mut rng = Rng::new(seed);
        crate::apply_affixes(&mut stack, tier, &mut rng);
        crate::sockets::after_affixes(&mut stack, tier, &mut rng);
        stack
    }

    #[test]
    fn a_level_names_the_amount_up_to_double_the_top() {
        let blessed = find("armor/attribute/blessed");
        let range = blessed.values[rarity_index(Tier::Epic)].unwrap();
        assert_eq!(value_at(blessed, Tier::Epic, 0.0), Some(range.min));
        assert_eq!(value_at(blessed, Tier::Epic, 1.0), Some(range.max));
        // Above 1.0 overcharges past the normal top, as Java's level does.
        assert_eq!(
            value_at(blessed, Tier::Epic, MAX_LEVEL),
            Some(range.max + (range.max - range.min))
        );
        // A stored amount reads back as the level that produced it, within the
        // range's step.
        for level in [0.0, 0.3, 0.7, 1.0, 1.6] {
            let value = value_at(blessed, Tier::Epic, level).unwrap();
            assert!(
                (level_of(blessed, Tier::Epic, value) - level).abs() <= 0.25 / (range.max - range.min),
                "level {level} read back off"
            );
        }
    }

    #[test]
    fn a_fresh_roll_is_never_overcharged_and_keeps_its_levels() {
        let stack = affixed("minecraft:diamond_sword", Tier::Mythic, 31);
        let carried = instances(&stack);
        assert_eq!(carried.len(), Tier::Mythic.affixes());
        let Some(Tag::Compound(custom)) = stack
            .data()
            .and_then(|data| data.other("minecraft:custom_data"))
        else {
            panic!("no custom data");
        };
        let Some(Tag::Compound(stored)) = custom.get(&LEVELS_KEY.into()) else {
            panic!("a fresh roll wrote no levels");
        };
        assert_eq!(stored.len(), carried.len());
        for instance in &carried {
            let level = instance.level.expect("a new-format piece has levels");
            assert!((0.0..1.0).contains(&level), "{level} is out of [0, 1)");
            assert_eq!(instance.value, value_at(instance.affix, Tier::Mythic, level).unwrap());
        }
    }

    #[test]
    fn an_old_piece_without_levels_keeps_its_exact_amounts() {
        let mut stack = affixed("minecraft:diamond_chestplate", Tier::Mythic, 77);
        let before = amounts(&stack);
        assert!(!before.is_empty());
        // Strip the levels key: the piece is now in the format rolled before
        // the model kept a level per affix.
        {
            let mut custom = match stack
                .data()
                .and_then(|data| data.other("minecraft:custom_data"))
            {
                Some(Tag::Compound(map)) => map.clone(),
                _ => panic!("no custom data"),
            };
            custom.remove(&LEVELS_KEY.into());
            stack
                .data_mut()
                .set_other("minecraft:custom_data", Some(Tag::Compound(custom)));
        }
        let old = instances(&stack);
        assert_eq!(old.len(), before.len());
        for instance in &old {
            assert_eq!(instance.level, None, "the old piece gained a level");
            assert_eq!(instance.value, before[instance.affix.id]);
        }
        // The levels deduced from those amounts rewrite the very same amounts,
        // so a piece is never silently re-rolled when levels arrive.
        let tier = crate::sockets::tier_of(&stack).expect("a tier");
        let mut upgraded = old.clone();
        for instance in &mut upgraded {
            let level = instance.level_at(tier);
            instance.set_level(tier, level);
        }
        let mut copy = stack.clone();
        rewrite(&mut copy, tier, &upgraded);
        assert_eq!(amounts(&copy), before);
    }

    #[test]
    fn malice_needs_two_level_dependent_affixes_and_one_below_the_max() {
        assert!(!can_apply_malice(&[]));
        assert!(!can_apply_malice(&[0.4]));
        assert!(!can_apply_malice(&[MAX_LEVEL, MAX_LEVEL]));
        assert!(can_apply_malice(&[MAX_LEVEL, MAX_LEVEL, 1.2]));
        let mut rng = Rng::new(3);
        assert_eq!(malice(&[], &mut rng), None);
        assert_eq!(malice(&[0.9], &mut rng), None);
        assert_eq!(malice(&[MAX_LEVEL, MAX_LEVEL], &mut rng), None);
    }

    #[test]
    fn malice_picks_two_distinct_indices_among_all_the_targets() {
        let levels = [0.2, 0.7, MAX_LEVEL, 1.4];
        let mut seen_buffed_at_max = false;
        for seed in 0..512 {
            let mut rng = Rng::new(seed);
            let outcome = malice(&levels, &mut rng).unwrap_or_else(|| panic!("seed {seed}"));
            let reset = outcome.reset.expect("two or more upgradable affixes reset one");
            assert!(outcome.up < levels.len() && reset < levels.len());
            assert_ne!(reset, outcome.up, "the same affix was upgraded and reset");
            if levels[outcome.up] >= MAX_LEVEL {
                seen_buffed_at_max = true;
            }
        }
        assert!(
            seen_buffed_at_max,
            "a target already at the maximum was never chosen"
        );
    }

    #[test]
    fn the_last_upgradable_affix_is_always_taken_and_rarely_spares_the_rest() {
        let levels = [MAX_LEVEL, 0.4, MAX_LEVEL, MAX_LEVEL];
        let mut spared = 0;
        const RUNS: u32 = 4_000;
        for seed in 0..u64::from(RUNS) {
            let mut rng = Rng::new(seed);
            let outcome = malice(&levels, &mut rng).unwrap();
            assert_eq!(outcome.up, 1, "the only upgradable affix was not the one taken");
            match outcome.reset {
                None => spared += 1,
                Some(index) => assert!(index == 0 || index == 2 || index == 3),
            }
        }
        let ratio = f64::from(spared) / f64::from(RUNS);
        assert!(
            (ratio - 0.033).abs() < 0.02,
            "the final chance was {ratio}, not 0.033"
        );
    }

    #[test]
    fn supremacy_raises_to_one_and_a_half_without_lowering_what_is_above() {
        assert_eq!(supremacy(&[]), None);
        assert_eq!(
            supremacy(&[0.0, 0.4, 1.5, 1.7, MAX_LEVEL]),
            Some(vec![1.5, 1.5, 1.5, 1.7, MAX_LEVEL])
        );
    }

    #[test]
    fn the_whole_java_catalogue_is_present() {
        assert_eq!(AFFIXES.len(), 99);
        for affix in &AFFIXES {
            assert_eq!(affix.id, format!("apotheosis:{}", affix.path));
            assert_eq!(affix.name_key, format!("affix.{}", affix.id));
            assert_eq!(affix.suffix_key, format!("{}.suffix", affix.name_key));
            assert!(!affix.name.is_empty() && !affix.suffix.is_empty());
            assert!(!affix.categories.is_empty(), "{} has no category", affix.id);
            if affix.supported && affix.kind == "attribute" {
                assert!(affix.attribute.is_some(), "{}", affix.id);
                assert!(affix.operation.is_some(), "{}", affix.id);
                assert!(
                    affix.values.iter().any(Option::is_some),
                    "{} has no value",
                    affix.id
                );
            }
        }
    }

    #[test]
    fn java_values_and_rarities_are_the_source() {
        let blessed = find("armor/attribute/blessed");
        assert_eq!(blessed.attribute, Some("minecraft:max_health"));
        assert_eq!(blessed.operation, Some("add_value"));
        assert_eq!(blessed.rarities, &[0, 1, 2, 3, 4]);
        assert_eq!(
            blessed.values[4],
            Some(Range {
                min: 5.0,
                max: 8.0,
                step: 0.25
            })
        );
        let violent = find("melee/attribute/violent");
        assert_eq!(violent.attribute, Some("minecraft:attack_damage"));
        assert!(attributes::accepts(classes(violent), ItemClass::Sword));
        assert!(attributes::accepts(classes(violent), ItemClass::Trident));
        assert!(!attributes::accepts(classes(violent), ItemClass::Bow));
        let lucky = find("generic/attribute/lucky");
        assert_eq!(lucky.rarities, &[1, 2, 3, 4], "Java has no common Lucky");
    }

    #[test]
    fn unsupported_java_attributes_never_enter_the_pool() {
        for path in [
            "armor/attribute/aquatic",
            "armor/attribute/timekeeping",
            "armor/attribute/unbound",
            "armor/attribute/winged",
        ] {
            let affix = find(path);
            assert!(!affix.supported, "{path} should stay catalogue-only");
            assert!(pool(ItemClass::Boots, Tier::Mythic)
                .iter()
                .all(|candidate| candidate.id != affix.id));
        }
    }

    #[test]
    fn shears_and_tridents_use_their_java_categories() {
        assert_eq!(attributes::class_of("minecraft:shears"), ItemClass::Shears);
        assert!(attributes::accepts(attributes::SHEARS, ItemClass::Shears));
        let violent = find("melee/attribute/violent");
        assert!(accepts(violent, ItemClass::Trident, Tier::Rare));
        assert!(pool(ItemClass::Shears, Tier::Mythic)
            .iter()
            .all(|affix| affix.categories.contains(&"apotheosis:shears")));
    }

    #[test]
    fn rolls_stay_inside_the_tier_range_and_on_java_steps() {
        let blessed = find("armor/attribute/blessed");
        let mut rng = Rng::new(0x1234);
        for tier in [Tier::Common, Tier::Rare, Tier::Mythic, Tier::Ancient] {
            let range = blessed.values[rarity_index(tier)].unwrap();
            for _ in 0..200 {
                let value = amount(blessed, tier, &mut rng).unwrap();
                let low = range.min.min(range.max);
                let high = range.min.max(range.max);
                assert!(
                    value >= low - 1e-9 && value <= high + 1e-9,
                    "{tier:?} rolled {value} outside {low}..={high}"
                );
                let steps = (value - range.min) / range.step;
                assert!((steps - steps.round()).abs() < 1e-6, "{value} off step");
            }
        }
    }

    #[test]
    fn negative_ranges_snap_like_java() {
        let fireproof = find("armor/attribute/fireproof");
        let range = fireproof.values[rarity_index(Tier::Rare)].unwrap();
        assert!(range.min > range.max, "Java's fireproof is a negative range");
        let mut rng = Rng::new(7);
        for _ in 0..100 {
            let value = amount(fireproof, Tier::Rare, &mut rng).unwrap();
            assert!(value <= range.min + 1e-9 && value >= range.max - 1e-9);
        }
    }

    #[test]
    fn a_lower_rarity_never_gets_a_higher_rarity_affix() {
        let adamantine = find("armor/attribute/adamantine");
        assert!(!accepts(adamantine, ItemClass::Chestplate, Tier::Uncommon));
        assert!(accepts(adamantine, ItemClass::Chestplate, Tier::Rare));
        assert!(pool(ItemClass::Sword, Tier::Mythic)
            .iter()
            .all(|affix| affix.supported));
    }

    #[test]
    fn the_tooltip_is_java_shaped_and_rarity_coloured() {
        let blessed = find("armor/attribute/blessed");
        let line = stat_line(blessed, Tier::Epic, 5.25);
        let Tag::Compound(compound) = line else {
            panic!("lore line is a component");
        };
        assert_eq!(
            compound.get(&"translate".into()),
            Some(&Tag::String("apotheosis.affix.stat".into()))
        );
        assert_eq!(
            compound.get(&"color".into()),
            Some(&Tag::String(Tier::Epic.hex().as_str().into()))
        );
        assert_eq!(
            compound.get(&"fallback".into()),
            Some(&Tag::String("+5.25 Max Health".into()))
        );
    }

    #[test]
    fn the_name_is_prefix_item_and_suffix() {
        let blessed = find("armor/attribute/blessed");
        let violent = find("melee/attribute/violent");
        let name = item_name_component(
            "minecraft:diamond_sword",
            Tier::Rare,
            &[blessed, violent],
        )
        .unwrap();
        let Tag::Compound(compound) = name else {
            panic!("name is a component");
        };
        assert_eq!(
            compound.get(&"translate".into()),
            Some(&Tag::String(
                "misc.apotheosis.affix_name.three".into()
            ))
        );
        let Some(Tag::List(with)) = compound.get(&"with".into()) else {
            panic!("name has its three components");
        };
        assert_eq!(with.len(), 3);
        assert_eq!(
            with[0],
            Tag::compound([
                ("translate", string(&blessed.name_key)),
                ("fallback", string(blessed.name)),
            ])
        );
        assert_eq!(
            with[2],
            Tag::compound([
                ("translate", string(&violent.suffix_key)),
                ("fallback", string(violent.suffix)),
            ])
        );
    }
}
