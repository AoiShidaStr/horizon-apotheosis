# Parité des objets, blocs et recettes Java

Rapport généré par `python tools/gen_parity.py` à partir des jars de référence
(`C:/Users/yello/horizon-work/java-ref`). Les identifiants listés sont ceux du
Java ; le port les déclare sous `apotheosis:` avec le même chemin, sauf mention
contraire. Aucun fichier d'asset Java n'est copié : les textures et modèles du
port sont dessinés par les générateurs, et `content-java/` (import local) reste
ignoré par git.

## Ce qui est porté

33 blocs et 127 objets sont déclarés sous
`content/data/apotheosis/horizon/`, avec 46 recettes sous
`content/data/apotheosis/recipe/`.

Blocs : `apothic_enchanting_table`, `basic_bookshelf`, `beeshelf`, `blazing_hellshelf`, `crystal_seashelf`, `deepshelf`, `dormant_deepshelf`, `draconic_endshelf`, `echoing_deepshelf`, `echoing_sculkshelf`, `enchantment_library`, `ender_library`, `endshelf`, `filtering_shelf`, `gem_cutting_table`, `geode_shelf`, `glowing_hellshelf`, `heart_seashelf`, `hellshelf`, `infused_hellshelf`, `infused_seashelf`, `melonshelf`, `pearl_endshelf`, `reforging_table`, `salvaging_table`, `seashelf`, `sightshelf`, `sightshelf_t2`, `simple_reforging_table`, `soul_touched_deepshelf`, `soul_touched_sculkshelf`, `stoneshelf`, `treasure_shelf`.

Objets : `arcane_sands`, `diamond_upgrade_smithing_template`, `extraction_tome`, `gem`, `gem_ballast_chipped`, `gem_ballast_cracked`, `gem_ballast_flawed`, `gem_ballast_flawless`, `gem_ballast_normal`, `gem_ballast_perfect`, `gem_blood_lord_flawed`, `gem_blood_lord_flawless`, `gem_blood_lord_normal`, `gem_blood_lord_perfect`, `gem_brawlers_chipped`, `gem_brawlers_cracked`, `gem_brawlers_flawed`, `gem_brawlers_flawless`, `gem_brawlers_normal`, `gem_brawlers_perfect`, `gem_breach_chipped`, `gem_breach_cracked`, `gem_breach_flawed`, `gem_breach_flawless`, `gem_breach_normal`, `gem_breach_perfect`, `gem_combatant_chipped`, `gem_combatant_cracked`, `gem_combatant_flawed`, `gem_combatant_flawless`, `gem_combatant_normal`, `gem_combatant_perfect`, `gem_dust`, `gem_earth_flawed`, `gem_earth_flawless`, `gem_earth_normal`, `gem_earth_perfect`, `gem_endersurge_flawless`, `gem_endersurge_perfect`, `gem_fused_slate`, `gem_guardian_chipped`, `gem_guardian_cracked`, `gem_guardian_flawed`, `gem_guardian_flawless`, `gem_guardian_normal`, `gem_guardian_perfect`, `gem_inferno_flawed`, `gem_inferno_flawless`, `gem_inferno_normal`, `gem_inferno_perfect`, `gem_lightning_chipped`, `gem_lightning_cracked`, `gem_lightning_flawed`, `gem_lightning_flawless`, `gem_lightning_normal`, `gem_lightning_perfect`, `gem_lunar_chipped`, `gem_lunar_cracked`, `gem_lunar_flawed`, `gem_lunar_flawless`, `gem_lunar_normal`, `gem_lunar_perfect`, `gem_mageslayer_flawless`, `gem_mageslayer_normal`, `gem_mageslayer_perfect`, `gem_molten_breach_flawed`, `gem_molten_breach_flawless`, `gem_molten_breach_normal`, `gem_molten_breach_perfect`, `gem_royalty_flawed`, `gem_royalty_flawless`, `gem_royalty_normal`, `gem_royalty_perfect`, `gem_samurai_chipped`, `gem_samurai_cracked`, `gem_samurai_flawed`, `gem_samurai_flawless`, `gem_samurai_normal`, `gem_samurai_perfect`, `gem_slipstream_chipped`, `gem_slipstream_cracked`, `gem_slipstream_flawed`, `gem_slipstream_flawless`, `gem_slipstream_normal`, `gem_slipstream_perfect`, `gem_solar_chipped`, `gem_solar_cracked`, `gem_solar_flawed`, `gem_solar_flawless`, `gem_solar_normal`, `gem_solar_perfect`, `gem_splendor_chipped`, `gem_splendor_cracked`, `gem_splendor_flawed`, `gem_splendor_flawless`, `gem_splendor_normal`, `gem_splendor_perfect`, `gem_tyrannical_chipped`, `gem_tyrannical_cracked`, `gem_tyrannical_flawed`, `gem_tyrannical_flawless`, `gem_tyrannical_normal`, `gem_tyrannical_perfect`, `gem_verdant_ruin_flawed`, `gem_verdant_ruin_flawless`, `gem_verdant_ruin_normal`, `gem_verdant_ruin_perfect`, `gem_warlord_chipped`, `gem_warlord_cracked`, `gem_warlord_flawed`, `gem_warlord_flawless`, `gem_warlord_normal`, `gem_warlord_perfect`, `godforged_pearl`, `gold_upgrade_smithing_template`, `improved_scrap_tome`, `iron_upgrade_smithing_template`, `luminous_crystal_shard`, `mysterious_scrap_metal`, `prismatic_web`, `scrap_tome`, `sigil_of_enhancement`, `sigil_of_rebirth`, `sigil_of_socketing`, `sigil_of_unnaming`, `sigil_of_withdrawal`, `timeworn_fabric`.

## Blocs Java absents

| id | statut | raison |
|---|---|---|
| apotheosis:augmenting_table | absent | bloc de table : la recette est portable, le comportement de la table ne l'est pas sans toucher au sertissage |
| apotheosis:boss_spawner | absent | invocateur de boss : la passerelle de boss (dimension, vagues, table de butin) n'est pas portée |
| apotheosis:ender_gem_case | absent | non porté dans ce lot |
| apotheosis:gem_case | hors périmètre | hors périmètre : les gemmes sont un autre chantier |
| apothic_enchanting:infused_endshelf | absent | non porté dans ce lot |
| apothic_enchanting:prismatic_altar | absent | non porté dans ce lot |
| apothic_enchanting:raven_enchanting_table | absent | non porté dans ce lot |

## Objets Java absents

| id | statut | raison |
|---|---|---|
| apotheosis:ascent_spawner_upgrade_rune | absent | rune de palier : recette portable, objet non déclaré dans ce lot |
| apotheosis:augmenting_table | absent | bloc de table : la recette est portable, le comportement de la table ne l'est pas sans toucher au sertissage |
| apotheosis:book | absent | objet utilitaire ou décoratif : recette et comportement non portés |
| apotheosis:boss_summoner | absent | invocateur de boss : la passerelle de boss (dimension, vagues, table de butin) n'est pas portée |
| apotheosis:burning_spawner_rune | absent | rune de statistique : la statistique correspondante est déjà appliquée par le catalyseur vanilla (voir `native/apotheosis/api/src/spawners.rs`) |
| apotheosis:echoing_spawner_rune | absent | rune de statistique : la statistique correspondante est déjà appliquée par le catalyseur vanilla (voir `native/apotheosis/api/src/spawners.rs`) |
| apotheosis:ender_gem_case | absent | non porté dans ce lot |
| apotheosis:ender_lead | absent | objet utilitaire ou décoratif : recette et comportement non portés |
| apotheosis:frontier_spawner_upgrade_rune | absent | rune de palier : recette portable, objet non déclaré dans ce lot |
| apotheosis:gem_case | hors périmètre | hors périmètre : les gemmes sont un autre chantier |
| apotheosis:gem_cutting_table | hors périmètre | hors périmètre : les gemmes sont un autre chantier |
| apotheosis:god_fused_pearl | absent | objet produit par infusion : `recipe/infusion` n'est pas data-driven sous Horizon |
| apotheosis:hammer | absent | objet utilitaire ou décoratif : recette et comportement non portés |
| apotheosis:ignore_conditions_spawner_rune | absent | rune de statistique : la statistique correspondante est déjà appliquée par le catalyseur vanilla (voir `native/apotheosis/api/src/spawners.rs`) |
| apotheosis:ignore_light_spawner_rune | absent | rune de statistique : la statistique correspondante est déjà appliquée par le catalyseur vanilla (voir `native/apotheosis/api/src/spawners.rs`) |
| apotheosis:ignore_players_spawner_rune | absent | rune de statistique : la statistique correspondante est déjà appliquée par le catalyseur vanilla (voir `native/apotheosis/api/src/spawners.rs`) |
| apotheosis:infused_spawner_rune | absent | objet de rune : dépend de spawner_chain, qui n'est obtenu que par la table de butin `unstable_spawner` (non portée) |
| apotheosis:initial_health_spawner_rune | absent | rune de statistique : la statistique correspondante est déjà appliquée par le catalyseur vanilla (voir `native/apotheosis/api/src/spawners.rs`) |
| apotheosis:lucky_foot | absent | objet utilitaire ou décoratif : recette et comportement non portés |
| apotheosis:music_disc_flash | absent | objet utilitaire ou décoratif : recette et comportement non portés |
| apotheosis:music_disc_glimmer | absent | objet utilitaire ou décoratif : recette et comportement non portés |
| apotheosis:music_disc_shimmer | absent | objet utilitaire ou décoratif : recette et comportement non portés |
| apotheosis:no_ai_spawner_rune | absent | rune de statistique : la statistique correspondante est déjà appliquée par le catalyseur vanilla (voir `native/apotheosis/api/src/spawners.rs`) |
| apotheosis:pinnacle_spawner_upgrade_rune | absent | rune de palier : recette portable, objet non déclaré dans ce lot |
| apotheosis:potion_charm | absent | charme de potion : le type `apotheosis:potion_charm_crafting` et son infusion ne sont pas data-driven sous Horizon |
| apotheosis:potion_charm_glow | absent | charme de potion : le type `apotheosis:potion_charm_crafting` et son infusion ne sont pas data-driven sous Horizon |
| apotheosis:redstone_control_spawner_rune | absent | rune de statistique : la statistique correspondante est déjà appliquée par le catalyseur vanilla (voir `native/apotheosis/api/src/spawners.rs`) |
| apotheosis:reforging_table | absent | non porté dans ce lot |
| apotheosis:salvaging_table | absent | non porté dans ce lot |
| apotheosis:sigil_of_malice | absent | sceau : la recette Java est un type custom (`apotheosis:malice\|supremacy\|unnaming`), non data-driven |
| apotheosis:sigil_of_supremacy | absent | sceau : la recette Java est un type custom (`apotheosis:malice\|supremacy\|unnaming`), non data-driven |
| apotheosis:silent_spawner_rune | absent | rune de statistique : la statistique correspondante est déjà appliquée par le catalyseur vanilla (voir `native/apotheosis/api/src/spawners.rs`) |
| apotheosis:simple_reforging_table | absent | non porté dans ce lot |
| apotheosis:spawn_range_spawner_rune | absent | rune de statistique : la statistique correspondante est déjà appliquée par le catalyseur vanilla (voir `native/apotheosis/api/src/spawners.rs`) |
| apotheosis:spawner_chain | absent | aucune recette en Java : butin de l'explosion d'un spawner (`unstable_spawner`), table de butin non portée |
| apotheosis:spawner_rune | absent | objet de rune : dépend de spawner_chain, qui n'est obtenu que par la table de butin `unstable_spawner` (non portée) |
| apotheosis:star_cube | absent | aucune recette en Java : butin de boss (`entity/boss_drops`), non porté |
| apotheosis:summit_spawner_upgrade_rune | absent | rune de palier : recette portable, objet non déclaré dans ce lot |
| apotheosis:youthful_spawner_rune | absent | rune de statistique : la statistique correspondante est déjà appliquée par le catalyseur vanilla (voir `native/apotheosis/api/src/spawners.rs`) |
| apothic_enchanting:apothic_enchanting_table | absent | non porté dans ce lot |
| apothic_enchanting:basic_bookshelf | absent | non porté dans ce lot |
| apothic_enchanting:beeshelf | absent | non porté dans ce lot |
| apothic_enchanting:blazing_hellshelf | absent | non porté dans ce lot |
| apothic_enchanting:boots_tome | absent | non porté dans ce lot |
| apothic_enchanting:bow_tome | absent | non porté dans ce lot |
| apothic_enchanting:chestplate_tome | absent | non porté dans ce lot |
| apothic_enchanting:crystal_seashelf | absent | non porté dans ce lot |
| apothic_enchanting:deepshelf | absent | non porté dans ce lot |
| apothic_enchanting:dormant_deepshelf | absent | non porté dans ce lot |
| apothic_enchanting:draconic_endshelf | absent | non porté dans ce lot |
| apothic_enchanting:echoing_deepshelf | absent | non porté dans ce lot |
| apothic_enchanting:echoing_sculkshelf | absent | non porté dans ce lot |
| apothic_enchanting:ender_lead | absent | objet utilitaire ou décoratif : recette et comportement non portés |
| apothic_enchanting:ender_library | absent | non porté dans ce lot |
| apothic_enchanting:endshelf | absent | non porté dans ce lot |
| apothic_enchanting:filtering_shelf | absent | non porté dans ce lot |
| apothic_enchanting:fishing_tome | absent | non porté dans ce lot |
| apothic_enchanting:flimsy_ender_lead | absent | non porté dans ce lot |
| apothic_enchanting:geode_shelf | absent | non porté dans ce lot |
| apothic_enchanting:glowing_hellshelf | absent | non porté dans ce lot |
| apothic_enchanting:heart_seashelf | absent | non porté dans ce lot |
| apothic_enchanting:hellshelf | absent | non porté dans ce lot |
| apothic_enchanting:helmet_tome | absent | non porté dans ce lot |
| apothic_enchanting:inert_trident | absent | non porté dans ce lot |
| apothic_enchanting:infused_breath | absent | objet utilitaire ou décoratif : recette et comportement non portés |
| apothic_enchanting:infused_deepshelf | absent | non porté dans ce lot |
| apothic_enchanting:infused_endshelf | absent | non porté dans ce lot |
| apothic_enchanting:infused_hellshelf | absent | non porté dans ce lot |
| apothic_enchanting:infused_seashelf | absent | non porté dans ce lot |
| apothic_enchanting:leggings_tome | absent | non porté dans ce lot |
| apothic_enchanting:library | absent | non porté dans ce lot |
| apothic_enchanting:melonshelf | absent | non porté dans ce lot |
| apothic_enchanting:music_disc_arcana | absent | objet utilitaire ou décoratif : recette et comportement non portés |
| apothic_enchanting:music_disc_eterna | absent | objet utilitaire ou décoratif : recette et comportement non portés |
| apothic_enchanting:music_disc_quanta | absent | objet utilitaire ou décoratif : recette et comportement non portés |
| apothic_enchanting:occult_ender_lead | absent | non porté dans ce lot |
| apothic_enchanting:other_tome | absent | non porté dans ce lot |
| apothic_enchanting:pearl_endshelf | absent | non porté dans ce lot |
| apothic_enchanting:pickaxe_tome | absent | non porté dans ce lot |
| apothic_enchanting:raven_enchanting_table | absent | non porté dans ce lot |
| apothic_enchanting:seashelf | absent | non porté dans ce lot |
| apothic_enchanting:sightshelf | absent | non porté dans ce lot |
| apothic_enchanting:sightshelf_t2 | absent | non porté dans ce lot |
| apothic_enchanting:soul_touched_deepshelf | absent | non porté dans ce lot |
| apothic_enchanting:soul_touched_sculkshelf | absent | non porté dans ce lot |
| apothic_enchanting:stoneshelf | absent | non porté dans ce lot |
| apothic_enchanting:treasure_shelf | absent | non porté dans ce lot |
| apothic_enchanting:warden_tendril | absent | objet utilitaire ou décoratif : recette et comportement non portés |
| apothic_enchanting:weapon_tome | absent | non porté dans ce lot |

## Recettes Java absentes

| recette | statut | raison |
|---|---|---|
| ascent | absent | recette vanilla portable : résultat ou ingrédient absent |
| ascent_spawner_upgrade_rune | absent | recette vanilla portable : résultat ou ingrédient absent |
| augmenting_table | absent | recette vanilla portable : résultat ou ingrédient absent |
| book | absent | recette vanilla portable : résultat ou ingrédient absent |
| burning | absent | type non porté : apothic_spawners:spawner_modifier |
| burning_spawner_rune | absent | recette vanilla portable : résultat ou ingrédient absent |
| chain_armor | absent | recyclage par groupe d'objets : logique native, pas de type data-driven |
| chipped | absent | type non porté : apotheosis:purity_upgrade |
| common | absent | reforge (niveau, matériau, sceau) : pas de type de recette data-driven sous Horizon |
| compressed_iron_armor | absent | recyclage par groupe d'objets : logique native, pas de type data-driven |
| cracked | absent | type non porté : apotheosis:purity_upgrade |
| diamond_armor | absent | recyclage par groupe d'objets : logique native, pas de type data-driven |
| diamond_horse_armor | absent | recyclage par groupe d'objets : logique native, pas de type data-driven |
| diamond_tools | absent | recyclage par groupe d'objets : logique native, pas de type data-driven |
| echoing | absent | type non porté : apothic_spawners:spawner_modifier |
| echoing_spawner_rune | absent | recette vanilla portable : résultat ou ingrédient absent |
| ender_gem_case | absent | type non porté : apothic_enchanting:keep_nbt_infusion |
| epic | absent | reforge (niveau, matériau, sceau) : pas de type de recette data-driven sous Horizon |
| flawed | absent | type non porté : apotheosis:purity_upgrade |
| flawless | absent | type non porté : apotheosis:purity_upgrade |
| frontier | absent | recette vanilla portable : résultat ou ingrédient absent |
| frontier_spawner_upgrade_rune | absent | recette vanilla portable : résultat ou ingrédient absent |
| gem_case | absent | recette vanilla portable : résultat ou ingrédient absent |
| god_fused_pearl | absent | infusion d'enchantement : requires Eterna/Quanta/Arcana, pas de type data-driven |
| gold_armor | absent | recyclage par groupe d'objets : logique native, pas de type data-driven |
| gold_tools | absent | recyclage par groupe d'objets : logique native, pas de type data-driven |
| golden_horse_armor | absent | recyclage par groupe d'objets : logique native, pas de type data-driven |
| ignore_conditions | absent | type non porté : apothic_spawners:spawner_modifier |
| ignore_conditions_spawner_rune | absent | recette vanilla portable : résultat ou ingrédient absent |
| ignore_light | absent | type non porté : apothic_spawners:spawner_modifier |
| ignore_light_spawner_rune | absent | recette vanilla portable : résultat ou ingrédient absent |
| ignore_players | absent | type non porté : apothic_spawners:spawner_modifier |
| ignore_players_spawner_rune | absent | recette vanilla portable : résultat ou ingrédient absent |
| infused_spawner_rune | absent | infusion d'enchantement : requires Eterna/Quanta/Arcana, pas de type data-driven |
| initial_health | absent | type non porté : apothic_spawners:spawner_modifier |
| initial_health_spawner_rune | absent | recette vanilla portable : résultat ou ingrédient absent |
| iron_armor | absent | recyclage par groupe d'objets : logique native, pas de type data-driven |
| iron_horse_armor | absent | recyclage par groupe d'objets : logique native, pas de type data-driven |
| iron_tools | absent | recyclage par groupe d'objets : logique native, pas de type data-driven |
| leather_armor | absent | recyclage par groupe d'objets : logique native, pas de type data-driven |
| leather_horse_armor | absent | recyclage par groupe d'objets : logique native, pas de type data-driven |
| malice | absent | sceau de malveillance : type custom |
| mythic | absent | reforge (niveau, matériau, sceau) : pas de type de recette data-driven sous Horizon |
| netherite_armor | absent | recyclage par groupe d'objets : logique native, pas de type data-driven |
| netherite_tools | absent | recyclage par groupe d'objets : logique native, pas de type data-driven |
| no_ai | absent | type non porté : apothic_spawners:spawner_modifier |
| no_ai_spawner_rune | absent | recette vanilla portable : résultat ou ingrédient absent |
| normal | absent | type non porté : apotheosis:purity_upgrade |
| perfect | absent | recyclage par groupe d'objets : logique native, pas de type data-driven |
| pinnacle | absent | recette vanilla portable : résultat ou ingrédient absent |
| pinnacle_spawner_upgrade_rune | absent | recette vanilla portable : résultat ou ingrédient absent |
| potion_charm | absent | charme de potion : potion en ingrédient non exprimable en recette vanilla |
| rare | absent | reforge (niveau, matériau, sceau) : pas de type de recette data-driven sous Horizon |
| raven_enchanting_table | absent | recette vanilla portable : résultat ou ingrédient absent |
| redstone_control | absent | type non porté : apothic_spawners:spawner_modifier |
| redstone_control_spawner_rune | absent | recette vanilla portable : résultat ou ingrédient absent |
| sigil_add_sockets | absent | ajout de châsses : autre chantier |
| silent | absent | type non porté : apothic_spawners:spawner_modifier |
| silent_spawner_rune | absent | recette vanilla portable : résultat ou ingrédient absent |
| socketing | absent | sertissage : autre chantier |
| spawn_range | absent | type non porté : apothic_spawners:spawner_modifier |
| spawn_range_spawner_rune | absent | recette vanilla portable : résultat ou ingrédient absent |
| spawner_rune | absent | recette vanilla portable : résultat ou ingrédient absent |
| stone_tools | absent | recyclage par groupe d'objets : logique native, pas de type data-driven |
| summit | absent | recette vanilla portable : résultat ou ingrédient absent |
| summit_spawner_upgrade_rune | absent | recette vanilla portable : résultat ou ingrédient absent |
| supremacy | absent | sceau de suprématie : type custom |
| uncommon | absent | reforge (niveau, matériau, sceau) : pas de type de recette data-driven sous Horizon |
| unnaming | absent | sceau de dénomination : type custom |
| upgrade_chainmail_boots_to_iron_boots | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_chainmail_chestplate_to_iron_chestplate | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_chainmail_helmet_to_iron_helmet | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_chainmail_leggings_to_iron_leggings | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_golden_axe_to_diamond_axe | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_golden_boots_to_diamond_boots | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_golden_chestplate_to_diamond_chestplate | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_golden_helmet_to_diamond_helmet | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_golden_hoe_to_diamond_hoe | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_golden_leggings_to_diamond_leggings | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_golden_pickaxe_to_diamond_pickaxe | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_golden_shovel_to_diamond_shovel | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_golden_sword_to_diamond_sword | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_iron_axe_to_golden_axe | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_iron_boots_to_golden_boots | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_iron_chestplate_to_golden_chestplate | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_iron_helmet_to_golden_helmet | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_iron_hoe_to_golden_hoe | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_iron_leggings_to_golden_leggings | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_iron_pickaxe_to_golden_pickaxe | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_iron_shovel_to_golden_shovel | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_iron_sword_to_golden_sword | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_stone_axe_to_iron_axe | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_stone_hoe_to_iron_hoe | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_stone_pickaxe_to_iron_pickaxe | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_stone_shovel_to_iron_shovel | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| upgrade_stone_sword_to_iron_sword | absent | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| widthdrawal | absent | type non porté : apotheosis:withdrawal |
| wolf_armor | absent | recyclage par groupe d'objets : logique native, pas de type data-driven |
| wooden_tools | absent | recyclage par groupe d'objets : logique native, pas de type data-driven |
| youthful | absent | type non porté : apothic_spawners:spawner_modifier |
| youthful_spawner_rune | absent | recette vanilla portable : résultat ou ingrédient absent |
| boots_tome | absent | recette vanilla portable : résultat ou ingrédient absent |
| bow_tome | absent | recette vanilla portable : résultat ou ingrédient absent |
| budding_amethyst | absent | infusion d'enchantement : requires Eterna/Quanta/Arcana, pas de type data-driven |
| carrot | absent | infusion d'enchantement : requires Eterna/Quanta/Arcana, pas de type data-driven |
| chestplate_tome | absent | recette vanilla portable : résultat ou ingrédient absent |
| cobweb | absent | recette vanilla portable : résultat ou ingrédient absent |
| echo_shard | absent | infusion d'enchantement : requires Eterna/Quanta/Arcana, pas de type data-driven |
| ender_lead | absent | infusion d'enchantement : requires Eterna/Quanta/Arcana, pas de type data-driven |
| fishing_tome | absent | recette vanilla portable : résultat ou ingrédient absent |
| flimsy_ender_lead | absent | recette vanilla portable : résultat ou ingrédient absent |
| helmet_tome | absent | recette vanilla portable : résultat ou ingrédient absent |
| inert_trident | absent | recette vanilla portable : résultat ou ingrédient absent |
| infused_breath | absent | infusion d'enchantement : requires Eterna/Quanta/Arcana, pas de type data-driven |
| leggings_tome | absent | recette vanilla portable : résultat ou ingrédient absent |
| library | absent | recette vanilla portable : résultat ou ingrédient absent |
| music_disc_arcana | absent | infusion d'enchantement : requires Eterna/Quanta/Arcana, pas de type data-driven |
| music_disc_eterna | absent | infusion d'enchantement : requires Eterna/Quanta/Arcana, pas de type data-driven |
| music_disc_quanta | absent | infusion d'enchantement : requires Eterna/Quanta/Arcana, pas de type data-driven |
| occult_ender_lead | absent | infusion d'enchantement : requires Eterna/Quanta/Arcana, pas de type data-driven |
| other_tome | absent | recette vanilla portable : résultat ou ingrédient absent |
| pickaxe_tome | absent | recette vanilla portable : résultat ou ingrédient absent |
| pufferfish | absent | recette vanilla portable : résultat ou ingrédient absent |
| trident | absent | infusion d'enchantement : requires Eterna/Quanta/Arcana, pas de type data-driven |
| weapon_tome | absent | recette vanilla portable : résultat ou ingrédient absent |
| xp_bottle | absent | infusion d'enchantement : requires Eterna/Quanta/Arcana, pas de type data-driven |
| xp_bottle_2 | absent | infusion d'enchantement : requires Eterna/Quanta/Arcana, pas de type data-driven |
| xp_bottle_3 | absent | infusion d'enchantement : requires Eterna/Quanta/Arcana, pas de type data-driven |
| burning | absent | type non porté : apothic_spawners:spawner_modifier |
| echoing | absent | type non porté : apothic_spawners:spawner_modifier |
| ignore_conditions | absent | type non porté : apothic_spawners:spawner_modifier |
| ignore_light | absent | type non porté : apothic_spawners:spawner_modifier |
| ignore_players | absent | type non porté : apothic_spawners:spawner_modifier |
| initial_health | absent | type non porté : apothic_spawners:spawner_modifier |
| max_delay | absent | type non porté : apothic_spawners:spawner_modifier |
| max_nearby | absent | type non porté : apothic_spawners:spawner_modifier |
| min_delay | absent | type non porté : apothic_spawners:spawner_modifier |
| no_ai | absent | type non porté : apothic_spawners:spawner_modifier |
| player_range | absent | type non porté : apothic_spawners:spawner_modifier |
| redstone_control | absent | type non porté : apothic_spawners:spawner_modifier |
| silent | absent | type non porté : apothic_spawners:spawner_modifier |
| spawn_count | absent | type non porté : apothic_spawners:spawner_modifier |
| spawn_range | absent | type non porté : apothic_spawners:spawner_modifier |
| youthful | absent | type non porté : apothic_spawners:spawner_modifier |

## Recettes de modificateur de spawner

Les 16 recettes de catalyseur du namespace `apothic_spawners`, les 11 recettes
de rune du namespace `apotheosis` et les 4 runes de palier sont reproduites dans
`native/apotheosis/api/src/spawners.rs` (table `MODIFIERS`, 31 entrées), avec
leurs valeurs exactes et, pour les runes seulement, leur recette inverse :
l'inverseur est le quartz en main secondaire, que le port lit comme l'objet en
main (limite documentée dans le module). Les inverses de catalyseur sont
désactivés en Java (`neoforge:false`) et le port ne les applique donc pas.


## Écarts notables

- `apothic_enchanting:infused_endshelf` n'a **ni blockstate ni recette** en Java
  (seulement un modèle d'objet) : rien à porter fidèlement, il est absent.
- Les quatre étagères demandées se répartissent ainsi : `filtering_shelf`,
  `geode_shelf` et `treasure_shelf` ont un blockstate et une recette, mais
  **aucun fichier `enchanting_stats`** : leurs statistiques et leurs effets
  (filtration, trésor, géode) sont calculés dans le code d'Apothic Enchanting,
  pas en data. Les porter sans inventer de valeurs demande un module natif ;
  ils restent absents et ne sont pas devinés.
- `spawner_chain` n'a aucune recette : en Java il tombe de la table de butin
  `apothic_spawners:loot_table/gameplay/unstable_spawner` (explosion d'un
  spawner). Cette table n'est pas portée, donc les runes restent hors de
  portée en survie ; c'est une raison de plus de ne pas déclarer les objets
  de rune seuls.
- Les 28 recettes `recipe/smithing` utilisent le type
  `apotheosis:sized_upgrade_recipe`, qui consomme **4** lingots ; le format
  vanilla `minecraft:smithing_transform` n'en consomme qu'un. Les porter
  telles quelles changerait l'équilibrage : elles sont absentes.
- Les 32 (en fait 31 actives : 16 catalyseurs + 11 runes + 4 paliers)
  recettes `spawner_modifiers` sont, elles, entièrement reproduites dans la
  table `MODIFIERS` du module natif, inverses compris.


## Types de recette non data-driven

| id | statut | raison |
|---|---|---|
| apotheosis:add_sockets | ajout de châsses : autre chantier |
| apotheosis:malice | sceau de malveillance : type custom |
| apotheosis:potion_charm_crafting | charme de potion : potion en ingrédient non exprimable en recette vanilla |
| apotheosis:potion_charm_infusion | charme de potion infusé : dépend de l'infusion |
| apotheosis:reforging | reforge (niveau, matériau, sceau) : pas de type de recette data-driven sous Horizon |
| apotheosis:salvaging | recyclage par groupe d'objets : logique native, pas de type data-driven |
| apotheosis:sized_upgrade_recipe | forge dimensionnée : le format vanilla ne consomme pas 4 lingots |
| apotheosis:socketing | sertissage : autre chantier |
| apotheosis:spawn_egg | œuf de spawn en ingrédient : pas d'équivalent data-driven |
| apotheosis:supremacy | sceau de suprématie : type custom |
| apotheosis:unnaming | sceau de dénomination : type custom |
| apothic_enchanting:infusion | infusion d'enchantement : requires Eterna/Quanta/Arcana, pas de type data-driven |

Ces types ne peuvent pas être exprimés par un fichier JSON sous Horizon :
leur comportement vit dans le code du mod. Ils sont donc absents en tant que
recettes ; les objets qu'ils produisent sont soit déjà portés, soit listés
ci-dessus avec leur raison.


## Hors périmètre

Les gemmes (objets, blocs, sertissage, `native/apotheosis/api/src/sockets.rs`)
et les affixes (`affix_effects.rs`) appartiennent à d'autres chantiers et ne
sont ni modifiés ni comptés comme manquants ici.


## Volumes relevés

| jar | blocs / objets / recettes |
|---|---|
| Apotheosis | 8 blocs / 55 objets / 114 recettes |
| Apothic Enchanting | 34 blocs / 54 objets / 59 recettes |
| Apothic Spawners | 0 blocs / 0 objets / 16 recettes |
| Apothic Attributes | 0 blocs / 0 objets / 0 recettes |
