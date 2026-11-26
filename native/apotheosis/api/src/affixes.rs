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
use horizon_client::mod_api::Tag;

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
    let range = affix.values[rarity_index(tier)]?;
    Some(roll_range(range, rng))
}

pub(crate) fn roll_range(range: Range, rng: &mut Rng) -> f64 {
    let fraction = rng.below(10_001) as f64 / 10_000.0;
    let mut value = range.min + (range.max - range.min) * fraction;
    if range.step != 0.0 {
        // Negative steps are real (fireproof goes -0.15 to -0.35 by -0.05).
        let steps = ((value - range.min) / range.step).round();
        value = range.min + steps * range.step;
    }
    (value * 1_000_000.0).round() / 1_000_000.0
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
