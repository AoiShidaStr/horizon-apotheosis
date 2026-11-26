//! The Java gems as items.  Apotheosis lists every gem at every purity it
//! allows in its creative tab; a Horizon tab only lists item ids, so each
//! (gem, purity) pair is its own declared item, `apotheosis:gem_<name>_<purity>`
//! (`tools/gen_java_gems.py`), whose default components carry the type and
//! purity in `minecraft:custom_data` (`apotheosis:gem`, `apotheosis:gem_purity`),
//! the translated name and the model.  Default components are not part of a
//! stack's patch (`ItemStack::data` only holds the patch), so the native side
//! reads the type and purity from the item id.  The single `apotheosis:gem`
//! item of earlier versions, whose identity lives in its custom data, is still
//! recognised.
//!
//! A gem does nothing on its own, exactly as in Apotheosis: its bonuses apply
//! once it is socketed into a piece of gear, and only to the gear classes its
//! Java data names.  The catalogue itself is generated (`java_gems.rs`); this
//! module turns it into stacks, resolves the bonuses a given piece receives at
//! a given purity, and describes them for a tooltip.

use horizon_client::mod_api::{Compound, ItemStack, Tag, Text};

use crate::attributes::Attr;
use crate::java_gems::{Gem, Kind, CLASS_TEXT, GEMS};
use crate::sockets::Purity;

/// The single gem item of earlier versions, whose type and purity live in its
/// custom data; still recognised, and used for a purity below a gem's floor,
/// which has no item of its own.
pub const ITEM: &str = "apotheosis:gem";

/// The custom-data key holding the gem's type id.
pub const GEM_KEY: &str = "apotheosis:gem";
/// The custom-data key holding the gem's purity, shared with `sockets`.
pub const PURITY_KEY: &str = "apotheosis:gem_purity";
/// The model selector: a gem's type id, so each gem draws its own texture.
const MODEL_KEY: &str = "minecraft:custom_model_data";
const CUSTOM_DATA: &str = "minecraft:custom_data";

/// The gear classes a gem's bonuses name, as this port matches them by id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GemClass {
    MeleeWeapon,
    Trident,
    Bow,
    Breaker,
    Shears,
    Helmet,
    Chestplate,
    Leggings,
    Boots,
    Shield,
    Anything,
}

impl GemClass {
    pub const ALL: [GemClass; 11] = [
        GemClass::MeleeWeapon,
        GemClass::Trident,
        GemClass::Bow,
        GemClass::Breaker,
        GemClass::Shears,
        GemClass::Helmet,
        GemClass::Chestplate,
        GemClass::Leggings,
        GemClass::Boots,
        GemClass::Shield,
        GemClass::Anything,
    ];

    /// The class an item id belongs to, or none for something a gem never
    /// applies to.  Axes count as melee weapons and as breakers, as
    /// Apotheosis' classes do.
    pub fn of(item: &str) -> Option<GemClass> {
        if item == "minecraft:trident" {
            return Some(GemClass::Trident);
        }
        if item == "minecraft:shield" {
            return Some(GemClass::Shield);
        }
        if item == "minecraft:shears" {
            return Some(GemClass::Shears);
        }
        if item.ends_with("_bow") || item == "minecraft:bow" || item.ends_with("_crossbow") {
            return Some(GemClass::Bow);
        }
        if item.ends_with("_sword") || item.ends_with("_axe") || item.ends_with("_mace") {
            return Some(GemClass::MeleeWeapon);
        }
        if item.ends_with("_pickaxe") || item.ends_with("_shovel") || item.ends_with("_hoe") {
            return Some(GemClass::Breaker);
        }
        for (class, suffix) in [
            (GemClass::Helmet, "_helmet"),
            (GemClass::Chestplate, "_chestplate"),
            (GemClass::Leggings, "_leggings"),
            (GemClass::Boots, "_boots"),
        ] {
            if item.ends_with(suffix) {
                return Some(class);
            }
        }
        None
    }

    pub fn name(self) -> &'static str {
        match self {
            GemClass::MeleeWeapon => "Melee Weapon",
            GemClass::Trident => "Trident",
            GemClass::Bow => "Bow",
            GemClass::Breaker => "Tool",
            GemClass::Shears => "Shears",
            GemClass::Helmet => "Helmet",
            GemClass::Chestplate => "Chestplate",
            GemClass::Leggings => "Leggings",
            GemClass::Boots => "Boots",
            GemClass::Shield => "Shield",
            GemClass::Anything => "Anything",
        }
    }

    /// Whether `item` is the class this bonus names.  `Anything` takes any
    /// piece a gem can go into; an axe is both a melee weapon and a breaker.
    pub fn matches(self, item: &str) -> bool {
        if self == GemClass::Anything {
            return GemClass::of(item).is_some();
        }
        match (self, GemClass::of(item)) {
            (GemClass::MeleeWeapon, Some(GemClass::MeleeWeapon)) => true,
            (GemClass::Breaker, Some(GemClass::MeleeWeapon)) => item.ends_with("_axe"),
            (class, Some(other)) => class == other,
            _ => false,
        }
    }
}

/// One bonus a gem pays out at a purity: the attribute entry to write, or the
/// enchantment level to raise.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Resolved {
    Attribute {
        attribute: &'static str,
        operation: &'static str,
        amount: f64,
    },
    Enchantment {
        id: &'static str,
        level: u8,
        existing_only: bool,
    },
}

/// The gem with this type id.
pub fn get(id: &str) -> Option<&'static Gem> {
    GEMS.iter().find(|gem| gem.id == id)
}

/// The purity's word in a gem item id (`cracked`... `perfect`).
fn purity_word(purity: Purity) -> &'static str {
    match purity {
        Purity::Cracked => "cracked",
        Purity::Chipped => "chipped",
        Purity::Flawed => "flawed",
        Purity::Normal => "normal",
        Purity::Flawless => "flawless",
        Purity::Perfect => "perfect",
    }
}

/// The last segment of a gem type id: `ballast` for `apotheosis:core/ballast`.
fn short_name(gem: &Gem) -> &'static str {
    gem.id.rsplit(['/', ':']).next().unwrap_or(gem.id)
}

/// The declared item of a gem at a purity, `apotheosis:gem_ballast_cracked`;
/// none below the gem's purity floor, where Java lists no stack either.
pub fn item_id(gem: &Gem, purity: Purity) -> Option<String> {
    (purity >= gem.min_purity)
        .then(|| format!("apotheosis:gem_{}_{}", short_name(gem), purity_word(purity)))
}

/// The gem and purity a per-purity gem item id names.
pub fn from_item(item: &str) -> Option<(&'static Gem, Purity)> {
    let rest = item.strip_prefix("apotheosis:gem_")?;
    let (name, word) = rest.rsplit_once('_')?;
    let purity = Purity::ALL
        .into_iter()
        .find(|purity| purity_word(*purity) == word)?;
    let gem = GEMS.iter().find(|gem| short_name(gem) == name)?;
    (purity >= gem.min_purity).then_some((gem, purity))
}

/// The gem type id a stack is, if it is a gem.
pub fn id_of(stack: &ItemStack) -> Option<&'static str> {
    if &*stack.item != ITEM {
        return from_item(&stack.item).map(|(gem, _)| gem.id);
    }
    match stack
        .data()
        .and_then(|data| data.other(CUSTOM_DATA))
        .and_then(|data| data.get(GEM_KEY))
    {
        Some(Tag::String(id)) => GEMS
            .iter()
            .find(|gem| gem.id == id.to_string_lossy().to_string())
            .map(|gem| gem.id),
        _ => None,
    }
}

/// The gem and purity a stack carries, or none for anything else.
pub fn of_stack(stack: &ItemStack) -> Option<(&'static Gem, Purity)> {
    let gem = get(id_of(stack)?)?;
    Some((gem, purity_of(stack)))
}

/// A stack's purity: the one its custom data names, else the one its item id
/// names, defaulting to Normal when it carries none.
pub fn purity_of(stack: &ItemStack) -> Purity {
    match stack
        .data()
        .and_then(|data| data.other(CUSTOM_DATA))
        .and_then(|data| data.get(PURITY_KEY))
    {
        Some(Tag::Int(index)) => Purity::from_index(*index as u8),
        _ => from_item(&stack.item).map_or(Purity::Normal, |(_, purity)| purity),
    }
}

/// Turns a gem stack into the item of `purity`, keeping its count and any
/// component it carries besides the gem identity; false when the stack is no
/// gem, or the gem has no item at that purity.  The identity patch of an
/// `apotheosis:gem` stack is dropped: the declaration's defaults carry it, and
/// a bare stack stacks with the ones the creative tab and the loot give.
pub fn change_purity(stack: &mut ItemStack, purity: Purity) -> bool {
    let Some((gem, _)) = of_stack(stack) else {
        return false;
    };
    let Some(item) = item_id(gem, purity) else {
        return false;
    };
    stack.item = item.as_str().into();
    if stack.data().is_some() {
        let custom = match stack.data().and_then(|data| data.other(CUSTOM_DATA)) {
            Some(Tag::Compound(compound)) => {
                let mut compound = compound.clone();
                compound.remove(&GEM_KEY.into());
                compound.remove(&PURITY_KEY.into());
                (!compound.is_empty()).then_some(Tag::Compound(compound))
            }
            other => other.cloned(),
        };
        let data = stack.data_mut();
        data.set_other(CUSTOM_DATA, custom);
        for key in [MODEL_KEY, "minecraft:item_name", "minecraft:rarity"] {
            data.set_other(key, Option::<Tag>::None);
        }
        stack.tidy();
    }
    true
}

/// Writes the gem type and purity (and the model selector) into a stack.
pub fn set_identity(stack: &mut ItemStack, gem: &Gem, purity: Purity) {
    let mut custom = match stack
        .data()
        .and_then(|data| data.other(CUSTOM_DATA))
    {
        Some(Tag::Compound(compound)) => compound.clone(),
        _ => Compound::new(),
    };
    custom.insert(GEM_KEY.into(), Tag::String(gem.id.into()));
    custom.insert(PURITY_KEY.into(), Tag::Int(purity.index() as i32));
    let data = stack.data_mut();
    data.set_other(CUSTOM_DATA, Some(Tag::Compound(custom)));
    data.set_other(MODEL_KEY, Some(Tag::String(gem.id.into())));
}

/// A gem stack: the declared item of that gem and purity, bare, since its
/// default components already carry the type, purity, name and model.
///
/// Below the gem's purity floor there is no such item, and the stack is the
/// generic `apotheosis:gem` with its identity in custom data, as before.  Its
/// name is `minecraft:item_name`, not `minecraft:custom_name`: Java gives a
/// gem its normal item name (translated with the purity's colour), and only
/// `custom_name` carries the player-given italics of an anvil rename.
pub fn stack(gem: &Gem, purity: Purity) -> ItemStack {
    if let Some(item) = item_id(gem, purity) {
        return ItemStack::new(item.as_str(), 1);
    }
    let mut stack = ItemStack::new(ITEM, 1);
    set_identity(&mut stack, gem, purity);
    let name = Tag::compound([
        (
            "translate",
            Tag::String(format!("item.apotheosis.gem.{}", gem.id).as_str().into()),
        ),
        ("fallback", Tag::String(gem.name.into())),
        ("color", Tag::String(purity.hex().as_str().into())),
        ("italic", Tag::Byte(0)),
    ]);
    stack
        .data_mut()
        .set_other("minecraft:item_name", Some(name));
    let rarity = match purity {
        Purity::Cracked => "common",
        Purity::Chipped | Purity::Flawed => "uncommon",
        Purity::Normal => "rare",
        Purity::Flawless => "epic",
        Purity::Perfect => "epic",
    };
    stack
        .data_mut()
        .set_other("minecraft:rarity", Some(Tag::String(rarity.into())));
    stack
}

/// The bonuses a gem pays out on `item` at `purity`: the Java values for that
/// purity step, on the classes the piece belongs to.
pub fn resolved(gem: &Gem, purity: Purity, item: &str) -> Vec<Resolved> {
    let index = purity.index() as usize;
    let mut out = Vec::new();
    for bonus in gem.bonuses {
        if !bonus.classes.iter().any(|class| class.matches(item)) {
            continue;
        }
        match &bonus.kind {
            Kind::Attribute {
                attribute,
                operation,
                values,
            } => {
                if let Some(amount) = values[index] {
                    out.push(Resolved::Attribute {
                        attribute,
                        operation,
                        amount,
                    });
                }
            }
            Kind::Enchantment {
                id,
                existing_only,
                levels,
            } => {
                if let Some(level) = levels[index] {
                    out.push(Resolved::Enchantment {
                        id,
                        level,
                        existing_only: *existing_only,
                    });
                }
            }
        }
    }
    out
}

/// The Apothic Attributes a gem pays out on `item`: the ones this crate does
/// the work for, kept apart because they live in the item's custom data.
pub fn apothic(attributes: &[Resolved]) -> Vec<(Attr, f64)> {
    attributes
        .iter()
        .filter_map(|entry| match entry {
            Resolved::Attribute {
                attribute, amount, ..
            } => Attr::ALL
                .into_iter()
                .find(|attr| attr.id() == *attribute)
                .map(|attr| (attr, *amount)),
            Resolved::Enchantment { .. } => None,
        })
        .collect()
}

/// One translated bonus line: its key, the English fallback the generator
/// writes for a client without the key, the value substituted into the key's
/// `%s`.
pub struct DescriptionLine {
    pub key: String,
    pub fallback: String,
    pub value: String,
}

/// What a gem's bonus lines say, for a tooltip: the classes it applies to and
/// the value at this purity.  Each line has its own translation key
/// (`apotheosis.gem.bonus.<gem>.<n>`), generated into both lang files; the
/// English text stays the fallback.  Bonuses this port cannot pay out are not
/// shown here; the README lists them.
#[cfg(test)]
pub fn description(gem: &Gem, purity: Purity) -> Vec<DescriptionLine> {
    description_filtered(gem, purity, None)
}

/// The bonus lines a gem gives a piece of `item`, and only those: a sword
/// shows its melee bonus, never the chestplate's. Java's tooltip filters the
/// gem's data by the item it is socketed into, not by every class it names.
pub fn description_for(gem: &Gem, purity: Purity, item: &str) -> Vec<DescriptionLine> {
    description_filtered(gem, purity, Some(item))
}

fn description_filtered(gem: &Gem, purity: Purity, item: Option<&str>) -> Vec<DescriptionLine> {
    let mut lines = Vec::new();
    let index = purity.index() as usize;
    for (number, bonus) in gem.bonuses.iter().enumerate() {
        if let Some(item) = item
            && !bonus.classes.iter().any(|class| class.matches(item))
        {
            continue;
        }
        let classes = bonus
            .classes
            .iter()
            .map(|class| class.name())
            .collect::<Vec<_>>()
            .join(" or ");
        let key = format!("apotheosis.gem.bonus.{}.{number}", gem.id);
        match &bonus.kind {
            Kind::Attribute {
                attribute,
                operation,
                values,
            } => {
                let Some(amount) = values[index] else {
                    continue;
                };
                let value = attribute::format(attribute, operation, amount);
                lines.push(DescriptionLine {
                    key,
                    fallback: format!("{classes}: {value} {}", attribute_name(attribute)),
                    value,
                });
            }
            Kind::Enchantment { id, levels, .. } => {
                let Some(level) = levels[index] else {
                    continue;
                };
                let value = level.to_string();
                lines.push(DescriptionLine {
                    key,
                    fallback: format!("{classes}: {value} {}", enchantment_name(id)),
                    value,
                });
            }
        }
    }
    lines
}

/// The English name a gem's attribute is shown under.
fn attribute_name(attribute: &str) -> &str {
    if let Some(attr) = Attr::ALL.into_iter().find(|attr| attr.id() == attribute) {
        return attr.name();
    }
    match attribute {
        "minecraft:attack_damage" => "Attack Damage",
        "minecraft:attack_speed" => "Attack Speed",
        "minecraft:max_health" => "Max Health",
        "minecraft:armor" => "Armor",
        "minecraft:armor_toughness" => "Armor Toughness",
        "minecraft:knockback_resistance" => "Knockback Resistance",
        "minecraft:luck" => "Luck",
        "minecraft:attack_knockback" => "Attack Knockback",
        "minecraft:movement_speed" => "Movement Speed",
        other => other.rsplit(':').next().unwrap_or(other),
    }
}

/// A value as `+4`, `+35%` or `-0.05`.
pub mod attribute {
    use crate::attributes::Attr;

    /// Whether an attribute's value is a fraction shown as a percentage.
    pub fn is_percent(attribute: &str) -> bool {
        if let Some(attr) = Attr::ALL.into_iter().find(|attr| attr.id() == attribute) {
            return attr.is_percent();
        }
        !matches!(
            attribute,
            "minecraft:attack_damage"
                | "minecraft:armor"
                | "minecraft:armor_toughness"
                | "minecraft:max_health"
                | "minecraft:luck"
                | "minecraft:knockback_resistance"
                | "minecraft:attack_knockback"
        )
    }

    /// `operation` is the modifier's: a multiplier (`add_multiplied_base`,
    /// `add_multiplied_total`) always reads as a percentage, as in Java.
    pub fn format(attribute: &str, operation: &str, amount: f64) -> String {
        let sign = if amount < 0.0 { "" } else { "+" };
        if is_percent(attribute) || operation != "add_value" {
            let pct = amount * 100.0;
            if (pct - pct.round()).abs() < 0.05 {
                format!("{sign}{pct:.0}%")
            } else {
                format!("{sign}{pct:.1}%")
            }
        } else if (amount - amount.round()).abs() < 0.05 {
            format!("{sign}{amount:.0}")
        } else {
            format!("{sign}{amount:.2}")
        }
    }
}

fn enchantment_name(id: &str) -> &str {
    match id {
        "minecraft:sharpness" => "Sharpness",
        "minecraft:protection" => "Protection",
        "minecraft:fortune" => "Fortune",
        "minecraft:efficiency" => "Efficiency",
        other => other.rsplit(':').next().unwrap_or(other),
    }
}

/// A piece of tooltip text, kept apart from `Text` so the lines can be checked
/// without a language loaded: a translation key with its English fallback and
/// arguments, a literal, or pieces one after another.
#[derive(Clone, Debug, PartialEq)]
pub enum Piece {
    Translate {
        key: String,
        fallback: String,
        args: Vec<Piece>,
    },
    Literal(String),
    Concat(Vec<Piece>),
}

impl Piece {
    fn translate(key: impl Into<String>, fallback: impl Into<String>, args: Vec<Piece>) -> Piece {
        Piece::Translate {
            key: key.into(),
            fallback: fallback.into(),
            args,
        }
    }

    /// The text in the player's language: each key resolved against the
    /// client's current lang, its fallback where the lang has none.
    pub fn text(&self) -> Text {
        match self {
            Piece::Translate {
                key,
                fallback,
                args,
            } => Text::translate_with_fallback(
                key,
                fallback,
                &args.iter().map(Piece::text).collect::<Vec<_>>(),
            ),
            Piece::Literal(text) => Text::literal(text.as_str()),
            Piece::Concat(pieces) => pieces
                .iter()
                .fold(Text::empty(), |text, piece| text.append(piece.text())),
        }
    }
}

/// One line a gem's tooltip adds under its name, and its colour.
#[derive(Clone, Debug, PartialEq)]
pub struct TooltipLine {
    pub piece: Piece,
    pub colour: u32,
}

/// The colour of Java's "socketable into" heading and gear lines.
const SOCKETABLE_COLOUR: u32 = 0x0A_FF0A;
/// The colour of Java's "when socketed in" heading and bonus lines.
const SOCKETED_COLOUR: u32 = 0xFF_AA00;

/// The English text of a `gem_class.<key>`, its fallback.
fn class_text(key: &str) -> &'static str {
    CLASS_TEXT
        .iter()
        .find(|(k, _)| *k == key)
        .map_or("", |(_, text)| text)
}

/// The translated name of a bonus's attribute or enchantment.
fn bonus_name(kind: &Kind) -> Piece {
    match kind {
        Kind::Attribute { attribute, .. } => {
            let (namespace, path) = attribute.split_once(':').unwrap_or(("", attribute));
            let key = if namespace == "minecraft" {
                format!("attribute.name.{path}")
            } else {
                format!("apotheosis.attribute.{path}")
            };
            Piece::translate(key, attribute_name(attribute), Vec::new())
        }
        Kind::Enchantment { id, .. } => Piece::translate(
            format!("enchantment.{}", id.replace(':', ".")),
            enchantment_name(id),
            Vec::new(),
        ),
    }
}

/// The lines Java Apotheosis' gem tooltip shows under the name, at `purity`:
/// the gear the gem can be set into, an empty line, then each bonus it pays
/// out at that purity as `• <gear>: <value> <name>`.  Only the bonuses this
/// port pays out, as `resolved` applies them; the purity itself is the name's
/// colour and word, as in Java.
pub fn tooltip_lines(gem: &Gem, purity: Purity) -> Vec<TooltipLine> {
    let index = purity.index() as usize;
    let paid: Vec<(&str, String, &Kind)> = gem
        .bonuses
        .iter()
        .filter_map(|bonus| {
            let value = match &bonus.kind {
                Kind::Attribute {
                    attribute,
                    operation,
                    values,
                } => attribute::format(attribute, operation, values[index]?),
                Kind::Enchantment { levels, .. } => format!("+{}", levels[index]?),
            };
            Some((bonus.key, value, &bonus.kind))
        })
        .collect();
    let class = |key: &str| {
        Piece::translate(format!("gem_class.{key}"), class_text(key), Vec::new())
    };
    let mut lines = vec![TooltipLine {
        piece: Piece::translate("text.apotheosis.socketable_into", "Can be set into:", Vec::new()),
        colour: SOCKETABLE_COLOUR,
    }];
    let mut keys: Vec<&str> = Vec::new();
    for (key, _, _) in &paid {
        if !keys.contains(key) {
            keys.push(key);
            lines.push(TooltipLine {
                piece: class(key),
                colour: SOCKETABLE_COLOUR,
            });
        }
    }
    lines.push(TooltipLine {
        piece: Piece::Literal(String::new()),
        colour: SOCKETED_COLOUR,
    });
    lines.push(TooltipLine {
        piece: Piece::translate("text.apotheosis.when_socketed_in", "Once set into:", Vec::new()),
        colour: SOCKETED_COLOUR,
    });
    for (key, value, kind) in paid {
        let value = Piece::Concat(vec![
            Piece::Literal(format!("{value} ")),
            bonus_name(kind),
        ]);
        lines.push(TooltipLine {
            piece: Piece::translate(
                "text.apotheosis.dot_prefix",
                "• %s",
                vec![Piece::translate(
                    "text.apotheosis.gem_bonus",
                    "%s: %s",
                    vec![class(key), value],
                )],
            ),
            colour: SOCKETED_COLOUR,
        });
    }
    lines
}

/// The gem type a drop of `dimension` is, by Java weight.  Every type in the
/// dimension is eligible, a type that names no dimension anywhere.
pub fn roll_type(dimension: &str, rng: &mut crate::Rng) -> &'static Gem {
    let mut candidates: Vec<&'static Gem> = GEMS
        .iter()
        .filter(|gem| gem.dimensions.is_empty() || gem.dimensions.contains(&dimension))
        .collect();
    if candidates.is_empty() {
        candidates = GEMS
            .iter()
            .filter(|gem| gem.dimensions.is_empty())
            .collect();
    }
    let total: u64 = candidates.iter().map(|gem| gem.weight).sum();
    let mut pick = rng.below(total.max(1));
    let mut chosen = candidates[candidates.len() - 1];
    for gem in &candidates {
        if pick < gem.weight {
            chosen = gem;
            break;
        }
        pick -= gem.weight;
    }
    chosen
}

/// What an ordinary mob or a boss rolls: a gem of the level's dimension, at a
/// purity drawn from the world `tier`'s table, between the gem's own floor and
/// the drop's ceiling.  The floor clamps the roll afterwards, as Java's
/// `Gem.toStack` does, so a Flawless-only End gem can still drop in the End.
/// Horizon's Host reads no attribute, so the player's luck is 0 here.
pub fn roll(
    dimension: &str,
    tier: crate::tiers::WorldTier,
    boss: bool,
    rng: &mut crate::Rng,
) -> (&'static Gem, Purity) {
    let chosen = roll_type(dimension, rng);
    let ceiling = if boss { Purity::Perfect } else { Purity::Flawless };
    let purity = crate::tiers::roll_gem_purity(tier, 0.0, chosen.min_purity, ceiling, rng);
    (chosen, purity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalogue_is_the_java_gems_outside_twilight() {
        assert_eq!(GEMS.len(), 21);
        for gem in &GEMS {
            assert!(gem.id.starts_with("apotheosis:"), "{}", gem.id);
            assert!(
                !gem.id.starts_with("apotheosis:twilight/"),
                "{} is a Twilight Forest gem",
                gem.id
            );
            assert!(!gem.name.is_empty());
        }
        assert!(get("apotheosis:core/ballast").is_some());
        assert!(get("apotheosis:twilight/queen").is_none());
    }

    #[test]
    fn a_gem_stack_is_the_bare_item_of_its_gem_and_purity() {
        let gem = get("apotheosis:core/solar").unwrap();
        let stack = stack(gem, Purity::Flawless);
        assert_eq!(&*stack.item, "apotheosis:gem_solar_flawless");
        assert!(stack.data().is_none(), "the declaration's defaults carry it");
        assert_eq!(id_of(&stack), Some("apotheosis:core/solar"));
        assert_eq!(purity_of(&stack), Purity::Flawless);
        assert_eq!(of_stack(&stack).map(|(gem, _)| gem.id), Some(gem.id));
    }

    #[test]
    fn every_gem_item_is_recognised_by_its_id_alone() {
        let mut seen = std::collections::HashSet::new();
        for gem in &GEMS {
            for purity in Purity::ALL {
                let Some(item) = item_id(gem, purity) else {
                    assert!(purity < gem.min_purity, "{} has no {purity:?} item", gem.id);
                    continue;
                };
                assert!(seen.insert(item.clone()), "{item} names two gems");
                let (found, read) = from_item(&item).expect(&item);
                assert_eq!((found.id, read), (gem.id, purity), "{item}");
                let bare = ItemStack::new(item.as_str(), 1);
                assert_eq!(of_stack(&bare).map(|(g, p)| (g.id, p)), Some((gem.id, purity)));
            }
        }
        assert_eq!(seen.len(), 107, "the Java tab's gem count changed");
        // Below its Flawless floor, Endersurge has no item.
        assert!(from_item("apotheosis:gem_endersurge_cracked").is_none());
        let end = get("apotheosis:the_end/endersurge").unwrap();
        assert_eq!(item_id(end, Purity::Normal), None);
        for other in [
            "apotheosis:gem_dust",
            "apotheosis:gem_cutting_table",
            "apotheosis:gem_fused_slate",
            "apotheosis:gem_ballast",
            "apotheosis:gem_ballast_shiny",
            "minecraft:gem_ballast_cracked",
        ] {
            assert!(from_item(other).is_none(), "{other}");
            assert!(of_stack(&ItemStack::new(other, 1)).is_none(), "{other}");
        }
    }

    #[test]
    fn a_generic_gem_is_still_recognised_by_its_data() {
        let gem = get("apotheosis:core/solar").unwrap();
        let mut stack = ItemStack::new(ITEM, 1);
        set_identity(&mut stack, gem, Purity::Flawless);
        assert_eq!(&*stack.item, ITEM);
        assert_eq!(id_of(&stack), Some("apotheosis:core/solar"));
        assert_eq!(purity_of(&stack), Purity::Flawless);
        assert_eq!(
            stack.data().and_then(|d| d.other(MODEL_KEY)),
            Some(&Tag::String("apotheosis:core/solar".into()))
        );
        assert!(of_stack(&stack).is_some());
        assert!(of_stack(&ItemStack::new("minecraft:stick", 1)).is_none());
    }

    #[test]
    fn a_purity_upgrade_gives_the_item_of_the_next_purity() {
        let gem = get("apotheosis:core/ballast").unwrap();
        let mut stack = stack(gem, Purity::Cracked);
        stack.count = 3;
        assert!(change_purity(&mut stack, Purity::Cracked.next()));
        assert_eq!(&*stack.item, "apotheosis:gem_ballast_chipped");
        assert_eq!(stack.count, 3);
        assert!(stack.data().is_none());
        assert_eq!(purity_of(&stack), Purity::Chipped);

        // A generic gem becomes its gem item, and loses only its identity patch.
        let mut legacy = ItemStack::new(ITEM, 1);
        set_identity(&mut legacy, gem, Purity::Normal);
        let mut custom = match legacy.data().and_then(|d| d.other(CUSTOM_DATA)) {
            Some(Tag::Compound(c)) => c.clone(),
            _ => panic!("no custom data"),
        };
        custom.insert("other:key".into(), Tag::Int(1));
        legacy
            .data_mut()
            .set_other(CUSTOM_DATA, Some(Tag::Compound(custom)));
        assert!(change_purity(&mut legacy, Purity::Flawless));
        assert_eq!(&*legacy.item, "apotheosis:gem_ballast_flawless");
        assert_eq!(legacy.data().and_then(|d| d.other(MODEL_KEY)), None);
        match legacy.data().and_then(|d| d.other(CUSTOM_DATA)) {
            Some(Tag::Compound(c)) => {
                assert!(c.get(&GEM_KEY.into()).is_none() && c.get(&PURITY_KEY.into()).is_none());
                assert_eq!(c.get(&"other:key".into()), Some(&Tag::Int(1)));
            }
            other => panic!("the other custom data was lost: {other:?}"),
        }
        assert_eq!(of_stack(&legacy).map(|(_, p)| p), Some(Purity::Flawless));

        // No item below the floor, and nothing for a non-gem.
        let end = get("apotheosis:the_end/endersurge").unwrap();
        let mut low = ItemStack::new(ITEM, 1);
        set_identity(&mut low, end, Purity::Cracked);
        assert!(!change_purity(&mut low, Purity::Chipped));
        assert_eq!(&*low.item, ITEM);
        assert!(!change_purity(&mut ItemStack::new("minecraft:stick", 1), Purity::Normal));
    }

    #[test]
    fn a_rolled_drop_is_the_item_of_its_gem_and_purity() {
        let mut rng = crate::Rng::new(3);
        for dimension in ["minecraft:overworld", "minecraft:the_nether", "minecraft:the_end"] {
            for boss in [false, true] {
                for _ in 0..100 {
                    let (gem, purity) =
                        roll(dimension, crate::tiers::WorldTier::Frontier, boss, &mut rng);
                    let drop = stack(gem, purity);
                    assert_eq!(Some(drop.item.to_string()), item_id(gem, purity));
                    assert_eq!(of_stack(&drop).map(|(g, p)| (g.id, p)), Some((gem.id, purity)));
                }
            }
        }
    }

    #[test]
    fn bonuses_apply_only_to_the_classes_java_names() {
        let gem = get("apotheosis:core/ballast").unwrap();
        let chest = resolved(gem, Purity::Normal, "minecraft:diamond_chestplate");
        assert_eq!(
            chest,
            vec![Resolved::Attribute {
                attribute: "minecraft:knockback_resistance",
                operation: "add_value",
                amount: 0.4,
            }]
        );
        // Leggings share the core_armor class.
        assert_eq!(resolved(gem, Purity::Normal, "minecraft:diamond_leggings").len(), 1);
        // A sword takes the attack damage, not the knockback resistance.
        assert_eq!(
            resolved(gem, Purity::Normal, "minecraft:diamond_sword"),
            vec![Resolved::Attribute {
                attribute: "minecraft:attack_damage",
                operation: "add_value",
                amount: 5.0,
            }]
        );
        // A piece of armour no class of its data names takes nothing.
        assert!(resolved(gem, Purity::Normal, "minecraft:diamond_helmet").is_empty());
        assert!(resolved(gem, Purity::Perfect, "minecraft:iron_ingot").is_empty());
    }

    #[test]
    fn purity_picks_the_value_java_stores_for_that_step() {
        let gem = get("apotheosis:core/guardian").unwrap();
        let value = |purity| match resolved(gem, purity, "minecraft:diamond_sword")[0] {
            Resolved::Attribute { amount, .. } => amount,
            _ => panic!("an attribute"),
        };
        assert_eq!(value(Purity::Cracked), 0.05);
        assert_eq!(value(Purity::Normal), 0.2);
        assert_eq!(value(Purity::Perfect), 0.3);
    }

    #[test]
    fn an_unsupported_attribute_is_recorded_not_replaced() {
        let gem = get("apotheosis:core/breach").unwrap();
        assert!(gem.unsupported_attributes.contains(&"apothic_attributes:prot_pierce"));
        // Only the two attributes this port implements are paid out.
        let sword = resolved(gem, Purity::Normal, "minecraft:diamond_sword");
        assert_eq!(sword.len(), 1);
        assert_eq!(
            sword[0],
            Resolved::Attribute {
                attribute: "apothic_attributes:armor_pierce",
                operation: "add_value",
                amount: 7.0,
            }
        );
    }

    #[test]
    fn a_modded_bonus_type_is_listed_as_unsupported() {
        let gem = get("apotheosis:the_nether/molten_breach").unwrap();
        assert!(gem.unsupported.contains(&"omnetic"), "omnetic missing");
        assert!(gem
            .unsupported_attributes
            .contains(&"minecraft:generic.scale"));
        assert!(gem
            .unsupported_attributes
            .contains(&"apothic_attributes:prot_pierce"));
    }

    #[test]
    fn only_vanilla_enchantments_are_paid_out() {
        let gem = get("apotheosis:overworld/earth").unwrap();
        let lines = resolved(gem, Purity::Normal, "minecraft:diamond_chestplate");
        assert!(lines.contains(&Resolved::Enchantment {
            id: "minecraft:protection",
            level: 2,
            existing_only: true,
        }));
        let inferno = get("apotheosis:the_nether/inferno").unwrap();
        assert!(
            inferno
                .unsupported
                .iter()
                .any(|kind| kind.contains("berserkers_fury")),
            "a modded enchantment must be recorded, not applied"
        );
    }

    #[test]
    fn a_roll_stays_in_the_levels_dimension_and_above_the_purity_floor() {
        let mut rng = crate::Rng::new(7);
        for _ in 0..200 {
            let (gem, purity) =
                roll("minecraft:the_nether", crate::tiers::WorldTier::Ascent, true, &mut rng);
            assert!(
                gem.dimensions.is_empty() || gem.dimensions.contains(&"minecraft:the_nether")
            );
            assert!(purity >= gem.min_purity);
            let (gem, purity) =
                roll("minecraft:overworld", crate::tiers::WorldTier::Frontier, false, &mut rng);
            assert!(purity <= Purity::Flawless);
            assert!(purity >= gem.min_purity);
        }
    }

    #[test]
    fn the_nether_gems_never_roll_in_the_overworld() {
        let mut rng = crate::Rng::new(11);
        for _ in 0..500 {
            let (gem, _) =
                roll("minecraft:overworld", crate::tiers::WorldTier::Frontier, true, &mut rng);
            assert_ne!(gem.id, "apotheosis:the_nether/inferno");
        }
    }

    #[test]
    fn an_axe_belongs_to_both_melee_weapon_and_breaker() {
        assert!(GemClass::MeleeWeapon.matches("minecraft:iron_axe"));
        assert!(GemClass::Breaker.matches("minecraft:iron_axe"));
        assert!(!GemClass::Breaker.matches("minecraft:iron_sword"));
        assert!(GemClass::Anything.matches("minecraft:iron_pickaxe"));
        assert!(!GemClass::Anything.matches("minecraft:iron_ingot"));
        assert_eq!(
            GemClass::of("minecraft:netherite_chestplate"),
            Some(GemClass::Chestplate)
        );
        assert_eq!(GemClass::of("minecraft:bow"), Some(GemClass::Bow));
    }

    #[test]
    fn the_apothic_bonuses_are_the_ones_this_crate_implements() {
        let gem = get("apotheosis:core/guardian").unwrap();
        let lines = resolved(gem, Purity::Normal, "minecraft:diamond_sword");
        assert_eq!(apothic(&lines), vec![(Attr::LifeSteal, 0.2)]);
        let gem = get("apotheosis:core/ballast").unwrap();
        let lines = resolved(gem, Purity::Normal, "minecraft:diamond_sword");
        assert!(apothic(&lines).is_empty());
    }

    #[test]
    fn a_pieces_description_only_names_the_bonus_it_gets() {
        let gem = get("apotheosis:core/ballast").unwrap();
        let sword = description_for(gem, Purity::Normal, "minecraft:diamond_sword");
        assert_eq!(sword.len(), 1, "a sword showed another class's bonus");
        assert!(sword[0].fallback.contains("+5"));
        assert!(
            !sword[0].fallback.contains("Chestplate"),
            "a sword's lore listed the chestplate bonus: {}",
            sword[0].fallback
        );
        let chest = description_for(gem, Purity::Normal, "minecraft:diamond_chestplate");
        assert_eq!(chest.len(), 1);
        assert!(chest[0].fallback.contains("Knockback Resistance"));
        assert!(
            !chest[0].fallback.contains("Attack Damage"),
            "a chestplate's lore listed the melee bonus"
        );
        // The gem item's own tooltip still lists every class it applies to.
        assert_eq!(description(gem, Purity::Normal).len(), 2);
    }

    #[test]
    fn a_description_says_the_classes_and_the_value() {
        let gem = get("apotheosis:core/ballast").unwrap();
        let lines = description(gem, Purity::Normal);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].fallback.contains("Melee Weapon or Trident"));
        assert!(lines[0].fallback.contains("+5"));
        assert_eq!(lines[0].value, "+5");
        assert!(lines[1].fallback.contains("Chestplate or Leggings"));
        assert!(lines[1].fallback.contains("+0.4"));
    }

    #[test]
    fn an_ordinary_end_roll_can_pick_a_flawless_only_gem() {
        let mut found = false;
        for seed in 0..512 {
            let (gem, purity) = roll(
                "minecraft:the_end",
                crate::tiers::WorldTier::Frontier,
                false,
                &mut crate::Rng::new(seed),
            );
            if gem.id == "apotheosis:the_end/endersurge" {
                found = true;
                assert!(
                    purity >= Purity::Flawless,
                    "endersurge dropped below its Flawless floor: {purity:?}"
                );
                break;
            }
        }
        assert!(found, "a chest in the End can never roll its End gem");
    }

    #[test]
    fn every_bonus_has_a_per_gem_translation_key() {
        let gem = get("apotheosis:core/ballast").unwrap();
        let lines = description(gem, Purity::Normal);
        assert_eq!(
            lines[0].key,
            "apotheosis.gem.bonus.apotheosis:core/ballast.0"
        );
        assert_eq!(
            lines[1].key,
            "apotheosis.gem.bonus.apotheosis:core/ballast.1"
        );
        for line in description(gem, Purity::Perfect) {
            assert!(
                line.key.starts_with(&format!("apotheosis.gem.bonus.{}.", gem.id)),
                "{} is not this gem's key",
                line.key
            );
        }
    }

    /// A piece as text that names its keys: `key(arg, arg)`, literals as they are.
    fn spelled(piece: &Piece) -> String {
        match piece {
            Piece::Translate { key, args, .. } if args.is_empty() => key.clone(),
            Piece::Translate { key, args, .. } => format!(
                "{key}({})",
                args.iter().map(spelled).collect::<Vec<_>>().join(", ")
            ),
            Piece::Literal(text) => text.clone(),
            Piece::Concat(pieces) => pieces.iter().map(spelled).collect(),
        }
    }

    /// The lines as (key, args as strings, colour); a literal line has no key.
    fn rows(lines: &[TooltipLine]) -> Vec<(String, Vec<String>, u32)> {
        lines
            .iter()
            .map(|line| match &line.piece {
                Piece::Translate { key, args, .. } => {
                    (key.clone(), args.iter().map(spelled).collect(), line.colour)
                }
                other => (String::new(), vec![spelled(other)], line.colour),
            })
            .collect()
    }

    /// The value a gem pays out on `item`, as the socket writes it.
    fn paid_value(gem: &Gem, purity: Purity, item: &str, attribute: &str) -> String {
        resolved(gem, purity, item)
            .into_iter()
            .find_map(|entry| match entry {
                Resolved::Attribute {
                    attribute: a,
                    operation,
                    amount,
                } if a == attribute => Some(attribute::format(a, operation, amount)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{} pays no {attribute} on {item}", gem.id))
    }

    fn bonus(class: &str, value: &str, name_key: &str) -> (String, Vec<String>, u32) {
        (
            "text.apotheosis.dot_prefix".into(),
            vec![format!(
                "text.apotheosis.gem_bonus(gem_class.{class}, {value} {name_key})"
            )],
            0xFF_AA00,
        )
    }

    fn heading(key: &str, colour: u32) -> (String, Vec<String>, u32) {
        (key.into(), Vec::new(), colour)
    }

    #[test]
    fn a_solar_gems_tooltip_is_javas_at_normal() {
        let gem = get("apotheosis:core/solar").unwrap();
        let fire = paid_value(
            gem,
            Purity::Normal,
            "minecraft:diamond_sword",
            "apothic_attributes:fire_damage",
        );
        assert_eq!(fire, "+4");
        assert_eq!(
            rows(&tooltip_lines(gem, Purity::Normal)),
            vec![
                heading("text.apotheosis.socketable_into", 0x0A_FF0A),
                heading("gem_class.light_weapon", 0x0A_FF0A),
                (String::new(), vec![String::new()], 0xFF_AA00),
                heading("text.apotheosis.when_socketed_in", 0xFF_AA00),
                bonus("light_weapon", &fire, "apotheosis.attribute.fire_damage"),
            ]
        );
    }

    #[test]
    fn a_tyrannical_gems_tooltip_names_each_gear_once() {
        let gem = get("apotheosis:core/tyrannical").unwrap();
        let p = Purity::Normal;
        let value = |item: &str, attribute: &str| paid_value(gem, p, item, attribute);
        assert_eq!(
            rows(&tooltip_lines(gem, p)),
            vec![
                heading("text.apotheosis.socketable_into", 0x0A_FF0A),
                heading("gem_class.light_weapon", 0x0A_FF0A),
                heading("gem_class.core_armor", 0x0A_FF0A),
                heading("gem_class.shield", 0x0A_FF0A),
                heading("gem_class.boots", 0x0A_FF0A),
                (String::new(), vec![String::new()], 0xFF_AA00),
                heading("text.apotheosis.when_socketed_in", 0xFF_AA00),
                bonus(
                    "light_weapon",
                    &value("minecraft:diamond_sword", "minecraft:attack_knockback"),
                    "attribute.name.attack_knockback"
                ),
                bonus(
                    "core_armor",
                    &value("minecraft:diamond_chestplate", "minecraft:armor_toughness"),
                    "attribute.name.armor_toughness"
                ),
                bonus(
                    "shield",
                    &value("minecraft:shield", "minecraft:armor_toughness"),
                    "attribute.name.armor_toughness"
                ),
                bonus(
                    "boots",
                    &value("minecraft:diamond_boots", "minecraft:max_health"),
                    "attribute.name.max_health"
                ),
                bonus(
                    "boots",
                    &value("minecraft:diamond_boots", "apothic_attributes:life_steal"),
                    "apotheosis.attribute.life_steal"
                ),
            ]
        );
    }

}

#[cfg(test)]
mod format_tests {
    use super::attribute::format;

    #[test]
    fn a_multiplier_bonus_reads_as_a_percentage() {
        // Tyrannical on a shield: armor toughness x1.30, not "+0.30".
        assert_eq!(format("minecraft:armor_toughness", "add_multiplied_base", 0.3), "+30%");
        assert_eq!(format("minecraft:max_health", "add_multiplied_total", 0.17), "+17%");
        assert_eq!(format("minecraft:armor", "add_value", 3.0), "+3");
    }
}
