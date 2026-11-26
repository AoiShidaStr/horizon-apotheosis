//! `apotheosis:is_monster`: the mobs whose vanilla `MobCategory` is `MONSTER`.
//!
//! A loot condition only receives the entity id, not its category, so the
//! 1.21.1 monster set is pinned here. Passive and neutral categories
//! (`CREATURE`, `AMBIENT`, `WATER_CREATURE`, `MISC`…) stay out of it, as in
//! Java's `MobCategory.MONSTER` test.

/// The entity types `MobCategory.MONSTER` names in Minecraft 1.21.1.
const MONSTERS: [&str; 39] = [
    "minecraft:blaze",
    "minecraft:bogged",
    "minecraft:breeze",
    "minecraft:cave_spider",
    "minecraft:creeper",
    "minecraft:drowned",
    "minecraft:elder_guardian",
    "minecraft:ender_dragon",
    "minecraft:enderman",
    "minecraft:endermite",
    "minecraft:evoker",
    "minecraft:ghast",
    "minecraft:giant",
    "minecraft:guardian",
    "minecraft:hoglin",
    "minecraft:husk",
    "minecraft:illusioner",
    "minecraft:magma_cube",
    "minecraft:phantom",
    "minecraft:piglin",
    "minecraft:piglin_brute",
    "minecraft:pillager",
    "minecraft:ravager",
    "minecraft:shulker",
    "minecraft:silverfish",
    "minecraft:skeleton",
    "minecraft:slime",
    "minecraft:spider",
    "minecraft:stray",
    "minecraft:vex",
    "minecraft:vindicator",
    "minecraft:warden",
    "minecraft:witch",
    "minecraft:wither",
    "minecraft:wither_skeleton",
    "minecraft:zoglin",
    "minecraft:zombie",
    "minecraft:zombie_villager",
    "minecraft:zombified_piglin",
];

/// Whether `entity` is a monster, as Java's `apotheosis:is_monster` decides.
pub fn is_monster(entity: &str) -> bool {
    MONSTERS.contains(&entity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monsters_are_the_vanilla_hostile_mobs() {
        for id in [
            "minecraft:zombie",
            "minecraft:creeper",
            "minecraft:skeleton",
            "minecraft:enderman",
            "minecraft:wither",
            "minecraft:ender_dragon",
            "minecraft:warden",
            "minecraft:piglin",
        ] {
            assert!(is_monster(id), "{id} is a monster");
        }
        for id in [
            "minecraft:cow",
            "minecraft:pig",
            "minecraft:sheep",
            "minecraft:villager",
            "minecraft:iron_golem",
            "minecraft:bat",
            "minecraft:cod",
        ] {
            assert!(!is_monster(id), "{id} is not a monster");
        }
    }
}
