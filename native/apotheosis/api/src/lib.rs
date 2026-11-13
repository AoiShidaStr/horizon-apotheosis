//! Apotheosis for Horizon, the native half.
//!
//! Clean-room reimplementation from Apotheosis' public behaviour (rarity tiers
//! above vanilla's, affixes on gear, higher tiers only from bosses). No
//! Apotheosis code or asset is used: see `../../README.md` and `NOTICE.md`.
//!
//! Five hooks, all on the real game's trait API:
//! - `modify_default_components`: each gem carries a real
//!   `minecraft:attribute_modifiers` entry, so a gem in the off hand grants its
//!   bonus (the game applies equipment modifiers itself).
//! - `tooltip`: a gem's tier and its off-hand effect, in the tier's colour
//!   (vanilla item rarity only goes up to epic; mythic and ancient are shown
//!   here).
//! - `loot_drops`: gear a mob drops is rolled into a rarity with affixes (real
//!   attribute modifiers plus lore), and a boss kill adds a mythic or ancient
//!   gem. Ordinary mobs never drop those two tiers.
//! - `register_commands` / `perform_command`: `/apoth reforge` rolls affixes
//!   onto plain gear, for experience levels.
use horizon_client::mod_api::{
    Flow, Hooks, Host, ItemStack, LootDropsEvent, ModifyDefaultComponentsEvent, NativeCommand,
    PerformCommandEvent, RegisterCommandsEvent, Tag, Text, TooltipEvent, serde_json,
};
use std::sync::atomic::{AtomicU64, Ordering};

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Apotheosis)
}

struct Apotheosis;

/// The tiers this slice adds. Vanilla's `minecraft:rarity` component names four
/// (`common`..`epic`); the two above it are this port's own, shown in the
/// tooltip and in an affixed item's lore.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Common,
    Uncommon,
    Rare,
    Epic,
    Mythic,
    Ancient,
}

impl Tier {
    /// How many affixes a piece of gear of this tier rolls.
    pub fn affixes(self) -> usize {
        match self {
            Tier::Common => 0,
            Tier::Uncommon => 1,
            Tier::Rare => 2,
            Tier::Epic => 3,
            Tier::Mythic => 4,
            Tier::Ancient => 5,
        }
    }

    /// What an affix's base amount is multiplied by.
    pub fn scale(self) -> f64 {
        match self {
            Tier::Common => 1.0,
            Tier::Uncommon => 1.0,
            Tier::Rare => 1.5,
            Tier::Epic => 2.0,
            Tier::Mythic => 3.0,
            Tier::Ancient => 4.0,
        }
    }

    /// `minecraft:rarity`, the component that colours the item's name. The two
    /// tiers above vanilla's take epic's colour; their name says the tier.
    pub fn vanilla_rarity(self) -> &'static str {
        match self {
            Tier::Common => "common",
            Tier::Uncommon => "uncommon",
            Tier::Rare => "rare",
            Tier::Epic | Tier::Mythic | Tier::Ancient => "epic",
        }
    }

    /// The tier's name, as a translation key the content mod's lang files carry.
    pub fn name_key(self) -> &'static str {
        match self {
            Tier::Common => "apotheosis.tier.common",
            Tier::Uncommon => "apotheosis.tier.uncommon",
            Tier::Rare => "apotheosis.tier.rare",
            Tier::Epic => "apotheosis.tier.epic",
            Tier::Mythic => "apotheosis.tier.mythic",
            Tier::Ancient => "apotheosis.tier.ancient",
        }
    }

    /// The tier's name in English, for a client without the key.
    pub fn name(self) -> &'static str {
        match self {
            Tier::Common => "Common",
            Tier::Uncommon => "Uncommon",
            Tier::Rare => "Rare",
            Tier::Epic => "Epic",
            Tier::Mythic => "Mythic",
            Tier::Ancient => "Ancient",
        }
    }

    /// The tier's colour, `0xRRGGBB`.
    pub fn colour(self) -> u32 {
        match self {
            Tier::Common => 0xFF_FFFF,
            Tier::Uncommon => 0x55_FF55,
            Tier::Rare => 0x55_55FF,
            Tier::Epic => 0xAA_00AA,
            Tier::Mythic => 0xD8_8A2C,
            Tier::Ancient => 0xB0_2A2A,
        }
    }

    /// The tier's colour as a text component's `color`.
    pub fn hex(self) -> String {
        format!("#{:06X}", self.colour())
    }
}

/// A string tag (`JavaString` is built from `&str`).
fn string(value: &str) -> Tag {
    Tag::String(value.into())
}

/// A gem: the item, its tier, and the attribute it grants from the off hand.
pub struct Gem {
    pub id: &'static str,
    pub tier: Tier,
    /// `minecraft:` attribute the off-hand gem adds to.
    pub attribute: &'static str,
    pub amount: f64,
    /// The effect line's translation key, and its English text.
    pub effect_key: &'static str,
    pub effect: &'static str,
}

pub const GEMS: [Gem; 6] = [
    Gem {
        id: "apotheosis:gem_ruby",
        tier: Tier::Common,
        attribute: "minecraft:attack_damage",
        amount: 1.0,
        effect_key: "apotheosis.gem.ruby.effect",
        effect: "In the off hand: +1 Attack Damage",
    },
    Gem {
        id: "apotheosis:gem_topaz",
        tier: Tier::Uncommon,
        attribute: "minecraft:movement_speed",
        amount: 0.01,
        effect_key: "apotheosis.gem.topaz.effect",
        effect: "In the off hand: +10% Movement Speed",
    },
    Gem {
        id: "apotheosis:gem_sapphire",
        tier: Tier::Rare,
        attribute: "minecraft:max_health",
        amount: 2.0,
        effect_key: "apotheosis.gem.sapphire.effect",
        effect: "In the off hand: +2 Max Health",
    },
    Gem {
        id: "apotheosis:gem_jade",
        tier: Tier::Epic,
        attribute: "minecraft:armor",
        amount: 2.0,
        effect_key: "apotheosis.gem.jade.effect",
        effect: "In the off hand: +2 Armor",
    },
    Gem {
        id: "apotheosis:gem_onyx",
        tier: Tier::Mythic,
        attribute: "minecraft:attack_damage",
        amount: 3.0,
        effect_key: "apotheosis.gem.onyx.effect",
        effect: "In the off hand: +3 Attack Damage",
    },
    Gem {
        id: "apotheosis:gem_bloodstone",
        tier: Tier::Ancient,
        attribute: "minecraft:max_health",
        amount: 4.0,
        effect_key: "apotheosis.gem.bloodstone.effect",
        effect: "In the off hand: +4 Max Health",
    },
];

pub fn gem(item: &str) -> Option<&'static Gem> {
    GEMS.iter().find(|g| g.id == item)
}

/// Mobs whose death drops a mythic or ancient gem.
const BOSSES: [&str; 4] = [
    "minecraft:wither",
    "minecraft:ender_dragon",
    "minecraft:warden",
    "minecraft:elder_guardian",
];

/// An affix: a name (translation key + English), the attribute it moves, its
/// base amount at uncommon, and the equipment slots it works in.
pub struct Affix {
    pub id: &'static str,
    pub name_key: &'static str,
    pub name: &'static str,
    pub attribute: &'static str,
    pub attribute_name: &'static str,
    pub base: f64,
    /// `add_value` or `add_multiplied_base`, as `minecraft:attribute_modifiers`
    /// spells the operation.
    pub operation: &'static str,
    /// A slot group: `hand`, `armor` or `any` (`EquipmentSlotGroup`).
    pub slot: &'static str,
}

pub const AFFIXES: [Affix; 8] = [
    Affix {
        id: "keen",
        name_key: "apotheosis.affix.keen",
        name: "Keen",
        attribute: "minecraft:attack_damage",
        attribute_name: "Attack Damage",
        base: 1.0,
        operation: "add_value",
        slot: "hand",
    },
    Affix {
        id: "brutal",
        name_key: "apotheosis.affix.brutal",
        name: "Brutal",
        attribute: "minecraft:attack_damage",
        attribute_name: "Attack Damage",
        base: 2.0,
        operation: "add_value",
        slot: "hand",
    },
    Affix {
        id: "swift",
        name_key: "apotheosis.affix.swift",
        name: "Swift",
        attribute: "minecraft:movement_speed",
        attribute_name: "Movement Speed",
        base: 0.01,
        operation: "add_value",
        slot: "armor",
    },
    Affix {
        id: "stalwart",
        name_key: "apotheosis.affix.stalwart",
        name: "Stalwart",
        attribute: "minecraft:max_health",
        attribute_name: "Max Health",
        base: 2.0,
        operation: "add_value",
        slot: "armor",
    },
    Affix {
        id: "bulwarked",
        name_key: "apotheosis.affix.bulwarked",
        name: "Bulwarked",
        attribute: "minecraft:armor",
        attribute_name: "Armor",
        base: 1.0,
        operation: "add_value",
        slot: "armor",
    },
    Affix {
        id: "hasty",
        name_key: "apotheosis.affix.hasty",
        name: "Hasty",
        attribute: "minecraft:mining_efficiency",
        attribute_name: "Mining Efficiency",
        base: 0.15,
        operation: "add_value",
        slot: "hand",
    },
    Affix {
        id: "steadfast",
        name_key: "apotheosis.affix.steadfast",
        name: "Steadfast",
        attribute: "minecraft:knockback_resistance",
        attribute_name: "Knockback Resistance",
        base: 0.1,
        operation: "add_value",
        slot: "armor",
    },
    Affix {
        id: "tough",
        name_key: "apotheosis.affix.tough",
        name: "Tough",
        attribute: "minecraft:armor_toughness",
        attribute_name: "Armor Toughness",
        base: 1.0,
        operation: "add_value",
        slot: "armor",
    },
];

/// The tooltip line a gem adds: its tier, and the tier's colour.
pub fn tier_line(item: &str) -> Option<(String, u32)> {
    gem(item).map(|g| (format!("Tier: {}", g.tier.name()), g.tier.colour()))
}

/// Whether a dropped item can carry affixes: a weapon, a tool or a piece of
/// armour. Recognised from the item id, which is all a `loot_drops` hook is
/// given about an item.
pub fn is_equipment(item: &str) -> bool {
    const SUFFIXES: [&str; 14] = [
        "_sword",
        "_axe",
        "_pickaxe",
        "_shovel",
        "_hoe",
        "_helmet",
        "_chestplate",
        "_leggings",
        "_boots",
        "_bow",
        "_crossbow",
        "_spear",
        "_trident",
        "_mace",
    ];
    item == "minecraft:bow"
        || item == "minecraft:crossbow"
        || item == "minecraft:trident"
        || item == "minecraft:shield"
        || SUFFIXES.iter().any(|s| item.ends_with(s))
}

/// Whether a piece of equipment is worn as armour (the `armor` slot group),
/// as opposed to held in a hand (`hand`). Shields are worn in the off hand and
/// carry armour-group modifiers.
pub fn is_armor(item: &str) -> bool {
    item == "minecraft:shield"
        || ["_helmet", "_chestplate", "_leggings", "_boots"]
            .iter()
            .any(|s| item.ends_with(s))
}

/// A stand-in for a random source: the hook API has none, so rolls come from a
/// counter mixed with what dropped (splitmix64). Two kills of one mob at one
/// place in different ticks roll differently; the stream is this mod's own, not
/// the level's.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// A number in `0..n`, uniformly enough for a loot roll.
    fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next() % n }
    }
    /// Whether a roll of `chance` in 100 comes up.
    fn chance(&mut self, chance: u64) -> bool {
        self.below(100) < chance
    }
    /// One of `weights`' tiers, by weight.
    fn tier(&mut self, weights: &[(Tier, u64)]) -> Tier {
        let total: u64 = weights.iter().map(|(_, w)| *w).sum();
        let mut roll = self.below(total.max(1));
        for (tier, weight) in weights {
            if roll < *weight {
                return *tier;
            }
            roll -= *weight;
        }
        weights.last().map_or(Tier::Uncommon, |(tier, _)| *tier)
    }
}

/// What an ordinary mob killed by a player can roll: never above epic, so the
/// two high tiers stay boss-only.
const ORDINARY_TIERS: [(Tier, u64); 3] = [
    (Tier::Uncommon, 55),
    (Tier::Rare, 30),
    (Tier::Epic, 15),
];

/// What a boss's gear rolls.
const BOSS_TIERS: [(Tier, u64); 4] = [
    (Tier::Rare, 25),
    (Tier::Epic, 40),
    (Tier::Mythic, 25),
    (Tier::Ancient, 10),
];

/// Which gem a boss drops. Bosses alternate mythic/ancient so a session sees
/// both tiers.
static DROPS: AtomicU64 = AtomicU64::new(0);

fn boss_gem() -> &'static str {
    if DROPS.fetch_add(1, Ordering::Relaxed) % 2 == 0 {
        "apotheosis:gem_onyx"
    } else {
        "apotheosis:gem_bloodstone"
    }
}

/// A stack of one item.
fn one(item: &str) -> ItemStack {
    ItemStack::new(item, 1)
}

/// The gear an affixed drop can be.
const ORDINARY_GEAR: [&str; 8] = [
    "minecraft:iron_sword",
    "minecraft:iron_axe",
    "minecraft:bow",
    "minecraft:iron_helmet",
    "minecraft:iron_chestplate",
    "minecraft:iron_leggings",
    "minecraft:iron_boots",
    "minecraft:shield",
];

const BOSS_GEAR: [&str; 5] = [
    "minecraft:diamond_sword",
    "minecraft:diamond_axe",
    "minecraft:diamond_chestplate",
    "minecraft:diamond_leggings",
    "minecraft:bow",
];

/// The amount an affix adds at `tier`, rounded to what the tooltip shows.
fn amount_at(affix: &Affix, tier: Tier) -> f64 {
    let raw = affix.base * tier.scale();
    (raw * 100.0).round() / 100.0
}

/// A tier's own lore line, at the top of an affixed item's lore.
fn tier_tag(tier: Tier) -> Tag {
    Tag::compound([
        ("text", string(tier.name())),
        ("color", string(&tier.hex())),
        ("italic", Tag::Byte(0)),
    ])
}

/// An affix's lore line: its name and what it adds, translated when the client
/// has the keys and readable when it does not.
fn affix_tag(affix: &Affix, amount: f64) -> Tag {
    let value = if amount.fract() == 0.0 {
        format!("+{amount:.0}")
    } else {
        format!("+{amount}")
    };
    let fallback = format!("{} {} {}", affix.name, value, affix.attribute_name);
    let attribute_key = format!("attribute.name.{}", affix.attribute);
    Tag::compound([
        ("translate", string("apotheosis.affix.line")),
        ("fallback", string(&fallback)),
        ("color", string("#5555FF")),
        ("italic", Tag::Byte(0)),
        (
            "with",
            Tag::List(vec![
                Tag::compound([
                    ("translate", string(affix.name_key)),
                    ("fallback", string(affix.name)),
                ]),
                string(&value),
                Tag::compound([
                    ("translate", string(&attribute_key)),
                    ("fallback", string(affix.attribute_name)),
                ]),
            ]),
        ),
    ])
}

/// Rolls `tier`'s affixes onto `stack`: real `minecraft:attribute_modifiers`, a
/// lore that says them, and the vanilla rarity that colours the name.
pub fn apply_affixes(stack: &mut ItemStack, tier: Tier, rng: &mut Rng) {
    let count = tier.affixes();
    if count == 0 {
        return;
    }
    // Only affixes whose slot group matches the piece: an armour affix on a
    // sword would show a lore line that never applies, and the other way round.
    let armor = is_armor(&stack.item);
    let mut pool: Vec<usize> = (0..AFFIXES.len())
        .filter(|i| (AFFIXES[*i].slot == "armor") == armor)
        .collect();
    if pool.is_empty() {
        pool = (0..AFFIXES.len()).collect();
    }
    let mut modifiers = Vec::with_capacity(count);
    let mut lore = Vec::with_capacity(count + 1);
    lore.push(tier_tag(tier));
    for _ in 0..count.min(pool.len()) {
        let pick = rng.below(pool.len() as u64) as usize;
        let affix = &AFFIXES[pool.swap_remove(pick)];
        let amount = amount_at(affix, tier);
        let id = format!("apotheosis:{}", affix.id);
        modifiers.push(Tag::compound([
            ("type", string(affix.attribute)),
            ("id", string(&id)),
            ("amount", Tag::Double(amount.to_bits())),
            ("operation", string(affix.operation)),
            ("slot", string(affix.slot)),
        ]));
        lore.push(affix_tag(affix, amount));
    }
    let data = stack.data_mut();
    data.set_other("minecraft:attribute_modifiers", Some(Tag::List(modifiers)));
    data.set_other("minecraft:lore", Some(Tag::List(lore)));
    data.set_other(
        "minecraft:rarity",
        Some(string(tier.vanilla_rarity())),
    );
}

/// A seed for one drop: the counter, the position and the mob, so two kills
/// never roll the same loot.
fn seed_of(context: &horizon_client::mod_api::NativeLootDrops<'_>, n: u64) -> u64 {
    let (x, y, z) = (context.origin[0], context.origin[1], context.origin[2]);
    let mut seed = n.wrapping_add(0x2545_F491_4F6C_DD1D);
    for part in [x.to_bits(), y.to_bits(), z.to_bits()] {
        seed ^= part.rotate_left(17);
    }
    if let Some((id, kind)) = context.entity {
        seed ^= u64::from(id) << 8;
        for byte in kind.bytes() {
            seed = seed.rotate_left(5) ^ u64::from(byte);
        }
    }
    seed
}

impl Hooks for Apotheosis {
    fn modify_default_components(&self, event: &mut ModifyDefaultComponentsEvent<'_>) {
        let Some(gem) = gem(event.item) else {
            return;
        };
        let modifiers = serde_json::json!([{
            "type": gem.attribute,
            "id": gem.id,
            "amount": gem.amount,
            "operation": "add_value",
            "slot": "offhand",
        }]);
        event
            .components
            .insert("minecraft:attribute_modifiers".into(), modifiers);
    }

    fn tooltip(&self, _host: &dyn Host, event: &mut TooltipEvent<'_>) {
        let Some(gem) = gem(&event.stack.item) else {
            return;
        };
        let (text, colour) = (format!("Tier: {}", gem.tier.name()), gem.tier.colour());
        event.lines.push((
            Text::translate_with_fallback(
                "apotheosis.tooltip.tier",
                &text,
                &[Text::translate_with_fallback(
                    gem.tier.name_key(),
                    gem.tier.name(),
                    &[],
                )],
            )
            .with_color(colour),
            colour,
        ));
        event.lines.push((
            Text::translate_with_fallback(gem.effect_key, gem.effect, &[])
                .with_color(0x55_55FF),
            0x55_55FF,
        ));
    }

    fn loot_drops(&self, _host: &mut dyn Host, event: &mut LootDropsEvent<'_>) {
        let Some((_, kind)) = event.context.entity else {
            return;
        };
        // A player's use (shears, a brush) is not a kill.
        if event.context.used {
            return;
        }
        let boss = BOSSES.contains(&kind);
        let killed_by_player = event.context.player.is_some();
        if !boss && !killed_by_player {
            return;
        }
        let n = DROPS.fetch_add(1, Ordering::Relaxed);
        let mut rng = Rng::new(seed_of(&event.context, n));

        if boss {
            event.drops.push(one(boss_gem()));
        }
        // The gear this mob dropped, upgraded in place: a zombie's sword, a
        // skeleton's bow.
        for stack in event.drops.iter_mut() {
            if !is_equipment(&stack.item) || stack.extra.is_some() || stack.damage != 0 {
                continue;
            }
            if boss {
                let tier = rng.tier(&BOSS_TIERS);
                apply_affixes(stack, tier, &mut rng);
            } else if rng.chance(35) {
                let tier = rng.tier(&ORDINARY_TIERS);
                apply_affixes(stack, tier, &mut rng);
            }
        }
        // And one of its own: a boss always, an ordinary mob rarely.
        let gear = |rng: &mut Rng| -> &'static str {
            if boss {
                BOSS_GEAR[rng.below(BOSS_GEAR.len() as u64) as usize]
            } else {
                ORDINARY_GEAR[rng.below(ORDINARY_GEAR.len() as u64) as usize]
            }
        };
        let drop = if boss {
            true
        } else {
            rng.chance(4)
        };
        if drop {
            let tier = if boss {
                rng.tier(&BOSS_TIERS)
            } else {
                rng.tier(&ORDINARY_TIERS)
            };
            let item = gear(&mut rng);
            let mut stack = one(item);
            apply_affixes(&mut stack, tier, &mut rng);
            event.drops.push(stack);
        }
    }

    fn register_commands(&self, _host: &mut dyn Host, event: &mut RegisterCommandsEvent<'_>) {
        event.commands.push(NativeCommand::new("apoth"));
        event.commands.push(NativeCommand::new("apoth reforge"));
    }

    fn perform_command(
        &self,
        host: &mut dyn Host,
        event: &mut PerformCommandEvent<'_>,
    ) -> Flow {
        let command = event.command.trim();
        let words: Vec<&str> = command.split_whitespace().collect();
        if words.first() != Some(&"apoth") {
            return Flow::Continue;
        }
        if words.len() < 2 {
            host.feedback(
                Text::translate_with_fallback(
                    "apotheosis.command.help",
                    "Apotheosis: /apoth reforge — roll affixes onto plain gear you hold (3 levels).",
                    &[],
                ),
            );
            return Flow::Cancel;
        }
        if words[1] != "reforge" {
            host.feedback(Text::translate_with_fallback(
                "apotheosis.command.unknown",
                "Unknown Apotheosis command.",
                &[],
            ));
            return Flow::Cancel;
        }
        reforge(host, event.player.uuid, event.player.xp_level);
        Flow::Cancel
    }
}

/// How many levels a reforge costs.
pub const REFORGE_COST: i32 = 3;

/// `/apoth reforge`: takes one plain piece of gear the player holds, gives it
/// back with a rolled rarity, and charges `REFORGE_COST` levels.
fn reforge(host: &mut dyn Host, player: u128, xp_level: i32) {
    let Some((item, count, plain)) = host.held() else {
        host.feedback(Text::translate_with_fallback(
            "apotheosis.reforge.empty",
            "Hold the piece of gear you want reforged.",
            &[],
        ));
        return;
    };
    if !is_equipment(&item) {
        host.feedback(Text::translate_with_fallback(
            "apotheosis.reforge.not_gear",
            "That is not a weapon, a tool or a piece of armour.",
            &[],
        ));
        return;
    }
    if !plain || count == 0 {
        host.feedback(Text::translate_with_fallback(
            "apotheosis.reforge.already",
            "That piece is already affixed; reforging takes plain gear.",
            &[],
        ));
        return;
    }
    if xp_level < REFORGE_COST {
        host.feedback(Text::translate_with_fallback(
            "apotheosis.reforge.levels",
            "Reforging costs 3 experience levels.",
            &[],
        ));
        return;
    }
    if host.take_items(&item, 1) != 1 {
        host.feedback(Text::translate_with_fallback(
            "apotheosis.reforge.empty",
            "Hold the piece of gear you want reforged.",
            &[],
        ));
        return;
    }
    let n = DROPS.fetch_add(1, Ordering::Relaxed);
    let mut rng = Rng::new(n);
    let tier = rng.tier(&ORDINARY_TIERS);
    let mut stack = one(&item);
    apply_affixes(&mut stack, tier, &mut rng);
    if host.give_stack(player, stack).is_some() {
        // No room: hand the plain piece back rather than lose it.
        let _ = host.give_stack(player, one(&item));
        host.feedback(Text::translate_with_fallback(
            "apotheosis.reforge.no_room",
            "No room for the reforged piece; nothing was charged.",
            &[],
        ));
        return;
    }
    let _ = host.give_levels(player, -REFORGE_COST);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tier_names_itself() {
        assert_eq!(tier_line("apotheosis:gem_onyx").unwrap().0, "Tier: Mythic");
        assert_eq!(tier_line("apotheosis:gem_bloodstone").unwrap().0, "Tier: Ancient");
    }

    #[test]
    fn vanilla_items_get_no_line() {
        assert!(tier_line("minecraft:diamond_sword").is_none());
    }

    #[test]
    fn bosses_alternate_the_high_tiers() {
        DROPS.store(0, Ordering::Relaxed);
        assert_eq!(boss_gem(), "apotheosis:gem_onyx");
        assert_eq!(boss_gem(), "apotheosis:gem_bloodstone");
    }

    #[test]
    fn every_gem_grants_a_bonus() {
        for gem in &GEMS {
            assert!(gem.amount > 0.0, "{} grants nothing", gem.id);
            assert!(
                gem.attribute.starts_with("minecraft:"),
                "{} names no attribute",
                gem.id
            );
        }
    }

    #[test]
    fn gear_is_recognised_by_its_id() {
        assert!(is_equipment("minecraft:diamond_sword"));
        assert!(is_equipment("minecraft:iron_chestplate"));
        assert!(is_equipment("minecraft:bow"));
        assert!(is_equipment("minecraft:netherite_pickaxe"));
        assert!(!is_equipment("minecraft:dirt"));
        assert!(!is_equipment("apotheosis:gem_ruby"));
    }

    #[test]
    fn higher_tiers_carry_more_and_bigger_affixes() {
        assert_eq!(Tier::Common.affixes(), 0);
        let mut last = 0;
        for tier in [
            Tier::Common,
            Tier::Uncommon,
            Tier::Rare,
            Tier::Epic,
            Tier::Mythic,
            Tier::Ancient,
        ] {
            assert!(tier.affixes() >= last);
            last = tier.affixes();
        }
        assert!(Tier::Ancient.scale() > Tier::Epic.scale());
    }

    #[test]
    fn ordinary_loot_never_reaches_the_boss_tiers() {
        for (tier, _) in ORDINARY_TIERS {
            assert!(tier <= Tier::Epic, "{tier:?} is boss-only");
        }
    }

    #[test]
    fn affixing_writes_modifiers_lore_and_rarity() {
        let mut stack = one("minecraft:diamond_sword");
        let mut rng = Rng::new(7);
        apply_affixes(&mut stack, Tier::Rare, &mut rng);
        let data = stack.data().expect("components");
        let Some(Tag::List(modifiers)) = data.other("minecraft:attribute_modifiers") else {
            panic!("no attribute modifiers");
        };
        assert_eq!(modifiers.len(), 2);
        for entry in modifiers {
            for key in ["type", "id", "amount", "operation", "slot"] {
                assert!(entry.get(key).is_some(), "modifier without {key}");
            }
        }
        let Some(Tag::List(lore)) = data.other("minecraft:lore") else {
            panic!("no lore");
        };
        assert_eq!(lore.len(), 3, "a tier line and one line per affix");
        assert_eq!(
            data.other("minecraft:rarity"),
            Some(&Tag::String("rare".into()))
        );
    }

    #[test]
    fn affixes_of_one_item_are_distinct() {
        let mut stack = one("minecraft:diamond_chestplate");
        let mut rng = Rng::new(11);
        apply_affixes(&mut stack, Tier::Ancient, &mut rng);
        let data = stack.data().expect("components");
        let Some(Tag::List(modifiers)) = data.other("minecraft:attribute_modifiers") else {
            panic!("no attribute modifiers");
        };
        assert_eq!(modifiers.len(), 5);
        let ids: Vec<String> = modifiers
            .iter()
            .filter_map(|m| match m.get("id") {
                Some(Tag::String(s)) => Some(s.to_string_lossy()),
                _ => None,
            })
            .collect();
        let unique: std::collections::HashSet<&String> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len(), "an affix was rolled twice");
    }

    #[test]
    fn amounts_round_to_what_the_tooltip_shows() {
        let swift = AFFIXES.iter().find(|a| a.id == "swift").unwrap();
        assert_eq!(amount_at(swift, Tier::Uncommon), 0.01);
        assert_eq!(amount_at(swift, Tier::Rare), 0.02);
        let keen = AFFIXES.iter().find(|a| a.id == "keen").unwrap();
        assert_eq!(amount_at(keen, Tier::Ancient), 4.0);
    }

    #[test]
    fn a_seed_follows_its_drop() {
        let a = 1u64;
        assert_ne!(seed_of_fake(a, [0.0, 64.0, 0.0]), seed_of_fake(a, [2.0, 64.0, 0.0]));
    }

    /// `seed_of` over a bare context, for the test above.
    fn seed_of_fake(n: u64, origin: [f64; 3]) -> u64 {
        let (x, y, z) = (origin[0], origin[1], origin[2]);
        let mut seed = n.wrapping_add(0x2545_F491_4F6C_DD1D);
        for part in [x.to_bits(), y.to_bits(), z.to_bits()] {
            seed ^= part.rotate_left(17);
        }
        seed
    }
}
