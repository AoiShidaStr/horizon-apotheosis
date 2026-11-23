//! The Java gems as items: one `apotheosis:gem` item, whose gem type and
//! purity live in its `minecraft:custom_data` (`apotheosis:gem`,
//! `apotheosis:gem_purity`), as Apotheosis stores them.
//!
//! A gem does nothing on its own, exactly as in Apotheosis: its bonuses apply
//! once it is socketed into a piece of gear, and only to the gear classes its
//! Java data names.  The catalogue itself is generated (`java_gems.rs`); this
//! module turns it into stacks, resolves the bonuses a given piece receives at
//! a given purity, and describes them for a tooltip.

use horizon_client::mod_api::{Compound, ItemStack, Tag};

use crate::attributes::Attr;
use crate::java_gems::{Gem, Kind, GEMS};
use crate::sockets::Purity;

/// The single item every gem is, as in Apotheosis 1.21.
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

/// The gem type id a stack is, if it is a gem.
pub fn id_of(stack: &ItemStack) -> Option<&'static str> {
    if &*stack.item != ITEM {
        return None;
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

/// A stack's purity, defaulting to Normal when it carries none.
pub fn purity_of(stack: &ItemStack) -> Purity {
    match stack
        .data()
        .and_then(|data| data.other(CUSTOM_DATA))
        .and_then(|data| data.get(PURITY_KEY))
    {
        Some(Tag::Int(index)) => Purity::from_index(*index as u8),
        _ => Purity::Normal,
    }
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

/// A gem stack: the item, its type, its purity, and the name Java gives it.
pub fn stack(gem: &Gem, purity: Purity) -> ItemStack {
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
        .set_other("minecraft:custom_name", Some(name));
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

/// What a gem's bonus lines say, for a tooltip: the classes it applies to and
/// the value at this purity.  Bonuses this port cannot pay out are not shown
/// here; the README lists them.
pub fn description(gem: &Gem, purity: Purity) -> Vec<(String, u32)> {
    let colour = purity.colour();
    let mut lines = Vec::new();
    let index = purity.index() as usize;
    for bonus in gem.bonuses {
        let classes = bonus
            .classes
            .iter()
            .map(|class| class.name())
            .collect::<Vec<_>>()
            .join(" or ");
        match &bonus.kind {
            Kind::Attribute {
                attribute,
                values,
                ..
            } => {
                let Some(amount) = values[index] else {
                    continue;
                };
                lines.push((
                    format!(
                        "{classes}: {} {}",
                        attribute::format(attribute, amount),
                        attribute_name(attribute)
                    ),
                    colour,
                ));
            }
            Kind::Enchantment { id, levels, .. } => {
                let Some(level) = levels[index] else {
                    continue;
                };
                lines.push((
                    format!("{classes}: {} {level}", enchantment_name(id)),
                    colour,
                ));
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

    pub fn format(attribute: &str, amount: f64) -> String {
        let sign = if amount < 0.0 { "" } else { "+" };
        if is_percent(attribute) {
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

/// What an ordinary mob or a boss rolls: a gem of the level's dimension, at a
/// purity at or above its floor.  Ordinary mobs keep to the low purities, so
/// the top steps stay a boss's prize.
pub fn roll(dimension: &str, boss: bool, rng: &mut crate::Rng) -> (&'static Gem, Purity) {
    let mut candidates: Vec<&'static Gem> = GEMS
        .iter()
        .filter(|gem| gem.dimensions.is_empty() || gem.dimensions.contains(&dimension))
        .filter(|gem| boss || gem.min_purity <= Purity::Normal)
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
    let ceiling = if boss { Purity::Perfect } else { Purity::Flawless };
    let rolled = Purity::from_index(rng.below(6) as u8);
    let purity = rolled.clamp(chosen.min_purity, ceiling.max(chosen.min_purity));
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
    fn a_gem_stack_carries_its_type_purity_and_model() {
        let gem = get("apotheosis:core/solar").unwrap();
        let stack = stack(gem, Purity::Flawless);
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
            let (gem, purity) = roll("minecraft:the_nether", true, &mut rng);
            assert!(
                gem.dimensions.is_empty() || gem.dimensions.contains(&"minecraft:the_nether")
            );
            assert!(purity >= gem.min_purity);
            let (gem, purity) = roll("minecraft:overworld", false, &mut rng);
            assert!(purity <= Purity::Flawless);
            assert!(purity >= gem.min_purity);
        }
    }

    #[test]
    fn the_nether_gems_never_roll_in_the_overworld() {
        let mut rng = crate::Rng::new(11);
        for _ in 0..500 {
            let (gem, _) = roll("minecraft:overworld", true, &mut rng);
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
    fn a_description_says_the_classes_and_the_value() {
        let gem = get("apotheosis:core/ballast").unwrap();
        let lines = description(gem, Purity::Normal);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].0.contains("Melee Weapon"));
        assert!(lines[0].0.contains("+5"));
        assert!(lines[1].0.contains("Chestplate or Leggings"));
        assert!(lines[1].0.contains("+0.4"));
    }
}
