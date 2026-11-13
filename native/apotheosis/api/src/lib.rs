//! Apotheosis for Horizon, the native half.
//!
//! Clean-room reimplementation from Apotheosis' public behaviour (gems with a
//! rarity tier above vanilla's, higher tiers dropping only from bosses). No
//! Apotheosis code or asset is used: see `../../README.md` and `NOTICE.md`.
//!
//! Two hooks, both on the real game's trait API:
//! - `tooltip`: a gem item's tier, in its tier colour, appended to its tooltip
//!   (vanilla item rarity only goes up to epic; Apotheosis' mythic and ancient
//!   tiers are shown here).
//! - `loot_drops`: a boss kill adds one mythic or ancient gem; no ordinary mob
//!   drops those tiers.
use horizon_client::mod_api::{Hooks, Host, ItemStack, LootDropsEvent, Text, TooltipEvent};
use std::sync::atomic::{AtomicU64, Ordering};

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Apotheosis)
}

struct Apotheosis;

/// The gems this slice adds: item id, tier name, tier colour (0xRRGGBBAA).
pub const GEMS: [(&str, &str, u32); 6] = [
    ("apotheosis:gem_ruby", "Common", 0xFFFFFF_FF),
    ("apotheosis:gem_topaz", "Uncommon", 0x55FF55_FF),
    ("apotheosis:gem_sapphire", "Rare", 0x5555FF_FF),
    ("apotheosis:gem_jade", "Epic", 0xAA00AA_FF),
    ("apotheosis:gem_onyx", "Mythic", 0x6B3FA0_FF),
    ("apotheosis:gem_bloodstone", "Ancient", 0x8C1F2F_FF),
];

/// Mobs whose death drops a mythic or ancient gem.
const BOSSES: [&str; 4] = [
    "minecraft:wither",
    "minecraft:ender_dragon",
    "minecraft:warden",
    "minecraft:elder_guardian",
];

/// The tooltip line a gem adds: its tier, and the tier's colour.
pub fn tier_line(item: &str) -> Option<(String, u32)> {
    GEMS.iter()
        .find(|(id, _, _)| *id == item)
        .map(|(_, tier, colour)| (format!("Tier: {tier}"), *colour))
}

/// Which gem a boss drops. Bosses alternate mythic/ancient so a session sees
/// both tiers without a random source in a hook that has none.
static BOSS_DROPS: AtomicU64 = AtomicU64::new(0);

fn boss_gem() -> &'static str {
    if BOSS_DROPS.fetch_add(1, Ordering::Relaxed) % 2 == 0 {
        "apotheosis:gem_onyx"
    } else {
        "apotheosis:gem_bloodstone"
    }
}

fn gem(item: &'static str) -> ItemStack {
    ItemStack {
        item: item.into(),
        count: 1,
        damage: 0,
        extra: None,
    }
}

impl Hooks for Apotheosis {
    fn tooltip(&self, _host: &dyn Host, event: &mut TooltipEvent<'_>) {
        if let Some((text, colour)) = tier_line(&event.stack.item) {
            event.lines.push((
                Text::literal(text).with_color(colour >> 8 & 0xFF_FFFF),
                colour,
            ));
        }
    }

    fn loot_drops(&self, _host: &mut dyn Host, event: &mut LootDropsEvent<'_>) {
        let Some((_, kind)) = event.context.entity else {
            return;
        };
        if BOSSES.contains(&kind) {
            event.drops.push(gem(boss_gem()));
        }
    }
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
        BOSS_DROPS.store(0, Ordering::Relaxed);
        assert_eq!(boss_gem(), "apotheosis:gem_onyx");
        assert_eq!(boss_gem(), "apotheosis:gem_bloodstone");
    }
}
