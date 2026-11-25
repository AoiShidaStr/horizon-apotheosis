# Carte des assets Java — Apotheosis pour Horizon

Rapport généré par `tools/import_java_assets.py`. **Aucun asset Java n'est dans ce dépôt** : les textures, modèles et textes importés vivent dans `content-java/`, ignoré par Git, et sont ajoutés au `.hmod` par `tools/pack.py` (sauf avec `--no-java`).

Source inspectée : `java-ref`

## Résumé

- blocs avec blockstate Java : 33
- objets avec modèle Java : 52
- modèles copiés : 119
- textures copiées : 129
- clés lang en_us : 1742
- clés lang fr_fr : 1621

Les ids sans équivalent Java gardent nos définitions et nos textures originales de `content/`. Les gemmes viennent de Java : une seule gemme (`apotheosis:gem`) dont le modèle choisit la texture du type qu'elle porte, ci-dessous en liste à part.

## Blocs

| Id du port | Source Java | Fichiers importés |
| --- | --- | --- |
| `apothic_enchanting_table` | `apothic_enchanting:apothic_enchanting_table` | `blockstates/apothic_enchanting_table.json` → `apothic_enchanting:block/apothic_enchanting_table` |
| `basic_bookshelf` | `apothic_enchanting:basic_bookshelf` | `blockstates/basic_bookshelf.json` → `apothic_enchanting:block/basic_bookshelf` |
| `beeshelf` | `apothic_enchanting:beeshelf` | `blockstates/beeshelf.json` → `apothic_enchanting:block/beeshelf` |
| `blazing_hellshelf` | `apothic_enchanting:blazing_hellshelf` | `blockstates/blazing_hellshelf.json` → `apothic_enchanting:block/blazing_hellshelf` |
| `crystal_seashelf` | `apothic_enchanting:crystal_seashelf` | `blockstates/crystal_seashelf.json` → `apothic_enchanting:block/crystal_seashelf` |
| `deepshelf` | `apothic_enchanting:deepshelf` | `blockstates/deepshelf.json` → `apothic_enchanting:block/deepshelf` |
| `dormant_deepshelf` | `apothic_enchanting:dormant_deepshelf` | `blockstates/dormant_deepshelf.json` → `apothic_enchanting:block/dormant_deepshelf` |
| `draconic_endshelf` | `apothic_enchanting:draconic_endshelf` | `blockstates/draconic_endshelf.json` → `apothic_enchanting:block/draconic_endshelf` |
| `echoing_deepshelf` | `apothic_enchanting:echoing_deepshelf` | `blockstates/echoing_deepshelf.json` → `apothic_enchanting:block/echoing_deepshelf` |
| `echoing_sculkshelf` | `apothic_enchanting:echoing_sculkshelf` | `blockstates/echoing_sculkshelf.json` → `apothic_enchanting:block/echoing_sculkshelf` |
| `enchantment_library` | `apothic_enchanting:library` | `blockstates/enchantment_library.json` → `apothic_enchanting:block/library`, variant `""` ajouté (notre bloc n'a pas la propriété Java) |
| `ender_library` | `apothic_enchanting:ender_library` | `blockstates/ender_library.json` → `apothic_enchanting:block/ender_library`, variant `""` ajouté (notre bloc n'a pas la propriété Java) |
| `endshelf` | `apothic_enchanting:endshelf` | `blockstates/endshelf.json` → `apothic_enchanting:block/endshelf` |
| `filtering_shelf` | `apothic_enchanting:filtering_shelf` | `blockstates/filtering_shelf.json` → `apothic_enchanting:block/filtering_shelf`, variant `""` ajouté (notre bloc n'a pas la propriété Java) |
| `gem_cutting_table` | `apotheosis:gem_cutting_table` | `blockstates/gem_cutting_table.json` → `apotheosis:block/gem_cutting_table`, variant `""` ajouté (notre bloc n'a pas la propriété Java) |
| `geode_shelf` | `apothic_enchanting:geode_shelf` | `blockstates/geode_shelf.json` → `apothic_enchanting:block/geode_shelf`, variant `""` ajouté (notre bloc n'a pas la propriété Java) |
| `glowing_hellshelf` | `apothic_enchanting:glowing_hellshelf` | `blockstates/glowing_hellshelf.json` → `apothic_enchanting:block/glowing_hellshelf` |
| `heart_seashelf` | `apothic_enchanting:heart_seashelf` | `blockstates/heart_seashelf.json` → `apothic_enchanting:block/heart_seashelf` |
| `hellshelf` | `apothic_enchanting:hellshelf` | `blockstates/hellshelf.json` → `apothic_enchanting:block/hellshelf` |
| `infused_hellshelf` | `apothic_enchanting:infused_hellshelf` | `blockstates/infused_hellshelf.json` → `apothic_enchanting:block/infused_hellshelf` |
| `infused_seashelf` | `apothic_enchanting:infused_seashelf` | `blockstates/infused_seashelf.json` → `apothic_enchanting:block/infused_seashelf` |
| `melonshelf` | `apothic_enchanting:melonshelf` | `blockstates/melonshelf.json` → `apothic_enchanting:block/melonshelf` |
| `pearl_endshelf` | `apothic_enchanting:pearl_endshelf` | `blockstates/pearl_endshelf.json` → `apothic_enchanting:block/pearl_endshelf` |
| `reforging_table` | `apotheosis:reforging_table` | `blockstates/reforging_table.json` → `apotheosis:block/reforging_table` |
| `salvaging_table` | `apotheosis:salvaging_table` | `blockstates/salvaging_table.json` → `apotheosis:block/salvaging_table` |
| `seashelf` | `apothic_enchanting:seashelf` | `blockstates/seashelf.json` → `apothic_enchanting:block/seashelf` |
| `sightshelf` | `apothic_enchanting:sightshelf` | `blockstates/sightshelf.json` → `apothic_enchanting:block/sightshelf` |
| `sightshelf_t2` | `apothic_enchanting:sightshelf_t2` | `blockstates/sightshelf_t2.json` → `apothic_enchanting:block/sightshelf_t2` |
| `simple_reforging_table` | `apotheosis:simple_reforging_table` | `blockstates/simple_reforging_table.json` → `apotheosis:block/simple_reforging_table` |
| `soul_touched_deepshelf` | `apothic_enchanting:soul_touched_deepshelf` | `blockstates/soul_touched_deepshelf.json` → `apothic_enchanting:block/soul_touched_deepshelf` |
| `soul_touched_sculkshelf` | `apothic_enchanting:soul_touched_sculkshelf` | `blockstates/soul_touched_sculkshelf.json` → `apothic_enchanting:block/soul_touched_sculkshelf` |
| `stoneshelf` | `apothic_enchanting:stoneshelf` | `blockstates/stoneshelf.json` → `apothic_enchanting:block/stoneshelf` |
| `treasure_shelf` | `apothic_enchanting:treasure_shelf` | `blockstates/treasure_shelf.json` → `apothic_enchanting:block/treasure_shelf` |

## Objets

| Id du port | Source Java | Fichiers importés |
| --- | --- | --- |
| `apothic_enchanting_table` | `apothic_enchanting:apothic_enchanting_table` | `items/apothic_enchanting_table.json` → `apothic_enchanting:item/apothic_enchanting_table` |
| `arcane_sands` | `apotheosis:arcane_sands` | `items/arcane_sands.json` → `apotheosis:item/arcane_sands` |
| `basic_bookshelf` | `apothic_enchanting:basic_bookshelf` | `items/basic_bookshelf.json` → `apothic_enchanting:item/basic_bookshelf` |
| `beeshelf` | `apothic_enchanting:beeshelf` | `items/beeshelf.json` → `apothic_enchanting:item/beeshelf` |
| `blazing_hellshelf` | `apothic_enchanting:blazing_hellshelf` | `items/blazing_hellshelf.json` → `apothic_enchanting:item/blazing_hellshelf` |
| `crystal_seashelf` | `apothic_enchanting:crystal_seashelf` | `items/crystal_seashelf.json` → `apothic_enchanting:item/crystal_seashelf` |
| `deepshelf` | `apothic_enchanting:deepshelf` | `items/deepshelf.json` → `apothic_enchanting:item/deepshelf` |
| `diamond_upgrade_smithing_template` | `apotheosis:diamond_upgrade_smithing_template` | `items/diamond_upgrade_smithing_template.json` → `apotheosis:item/diamond_upgrade_smithing_template` |
| `dormant_deepshelf` | `apothic_enchanting:dormant_deepshelf` | `items/dormant_deepshelf.json` → `apothic_enchanting:item/dormant_deepshelf` |
| `draconic_endshelf` | `apothic_enchanting:draconic_endshelf` | `items/draconic_endshelf.json` → `apothic_enchanting:item/draconic_endshelf` |
| `echoing_deepshelf` | `apothic_enchanting:echoing_deepshelf` | `items/echoing_deepshelf.json` → `apothic_enchanting:item/echoing_deepshelf` |
| `echoing_sculkshelf` | `apothic_enchanting:echoing_sculkshelf` | `items/echoing_sculkshelf.json` → `apothic_enchanting:item/echoing_sculkshelf` |
| `enchantment_library` | `apothic_enchanting:library` | `items/enchantment_library.json` → `apothic_enchanting:item/library` |
| `ender_library` | `apothic_enchanting:ender_library` | `items/ender_library.json` → `apothic_enchanting:item/ender_library` |
| `endshelf` | `apothic_enchanting:endshelf` | `items/endshelf.json` → `apothic_enchanting:item/endshelf` |
| `extraction_tome` | `apothic_enchanting:extraction_tome` | `items/extraction_tome.json` → `apothic_enchanting:item/extraction_tome` |
| `filtering_shelf` | `apothic_enchanting:filtering_shelf` | `items/filtering_shelf.json` → `apothic_enchanting:item/filtering_shelf` |
| `gem` | original — pas d'équivalent Java | — |
| `gem_ballast_chipped` | gemme du port, texture Java `apotheosis:item/gems/ballast` | `apotheosis:item/gems/ballast` |
| `gem_ballast_cracked` | gemme du port, texture Java `apotheosis:item/gems/ballast` | `apotheosis:item/gems/ballast` |
| `gem_ballast_flawed` | gemme du port, texture Java `apotheosis:item/gems/ballast` | `apotheosis:item/gems/ballast` |
| `gem_ballast_flawless` | gemme du port, texture Java `apotheosis:item/gems/ballast` | `apotheosis:item/gems/ballast` |
| `gem_ballast_normal` | gemme du port, texture Java `apotheosis:item/gems/ballast` | `apotheosis:item/gems/ballast` |
| `gem_ballast_perfect` | gemme du port, texture Java `apotheosis:item/gems/ballast` | `apotheosis:item/gems/ballast` |
| `gem_blood_lord_flawed` | gemme du port, texture Java `apotheosis:item/gems/blood_lord` | `apotheosis:item/gems/blood_lord` |
| `gem_blood_lord_flawless` | gemme du port, texture Java `apotheosis:item/gems/blood_lord` | `apotheosis:item/gems/blood_lord` |
| `gem_blood_lord_normal` | gemme du port, texture Java `apotheosis:item/gems/blood_lord` | `apotheosis:item/gems/blood_lord` |
| `gem_blood_lord_perfect` | gemme du port, texture Java `apotheosis:item/gems/blood_lord` | `apotheosis:item/gems/blood_lord` |
| `gem_brawlers_chipped` | gemme du port, texture Java `apotheosis:item/gems/brawlers` | `apotheosis:item/gems/brawlers` |
| `gem_brawlers_cracked` | gemme du port, texture Java `apotheosis:item/gems/brawlers` | `apotheosis:item/gems/brawlers` |
| `gem_brawlers_flawed` | gemme du port, texture Java `apotheosis:item/gems/brawlers` | `apotheosis:item/gems/brawlers` |
| `gem_brawlers_flawless` | gemme du port, texture Java `apotheosis:item/gems/brawlers` | `apotheosis:item/gems/brawlers` |
| `gem_brawlers_normal` | gemme du port, texture Java `apotheosis:item/gems/brawlers` | `apotheosis:item/gems/brawlers` |
| `gem_brawlers_perfect` | gemme du port, texture Java `apotheosis:item/gems/brawlers` | `apotheosis:item/gems/brawlers` |
| `gem_breach_chipped` | gemme du port, texture Java `apotheosis:item/gems/breach` | `apotheosis:item/gems/breach` |
| `gem_breach_cracked` | gemme du port, texture Java `apotheosis:item/gems/breach` | `apotheosis:item/gems/breach` |
| `gem_breach_flawed` | gemme du port, texture Java `apotheosis:item/gems/breach` | `apotheosis:item/gems/breach` |
| `gem_breach_flawless` | gemme du port, texture Java `apotheosis:item/gems/breach` | `apotheosis:item/gems/breach` |
| `gem_breach_normal` | gemme du port, texture Java `apotheosis:item/gems/breach` | `apotheosis:item/gems/breach` |
| `gem_breach_perfect` | gemme du port, texture Java `apotheosis:item/gems/breach` | `apotheosis:item/gems/breach` |
| `gem_combatant_chipped` | gemme du port, texture Java `apotheosis:item/gems/combatant` | `apotheosis:item/gems/combatant` |
| `gem_combatant_cracked` | gemme du port, texture Java `apotheosis:item/gems/combatant` | `apotheosis:item/gems/combatant` |
| `gem_combatant_flawed` | gemme du port, texture Java `apotheosis:item/gems/combatant` | `apotheosis:item/gems/combatant` |
| `gem_combatant_flawless` | gemme du port, texture Java `apotheosis:item/gems/combatant` | `apotheosis:item/gems/combatant` |
| `gem_combatant_normal` | gemme du port, texture Java `apotheosis:item/gems/combatant` | `apotheosis:item/gems/combatant` |
| `gem_combatant_perfect` | gemme du port, texture Java `apotheosis:item/gems/combatant` | `apotheosis:item/gems/combatant` |
| `gem_cutting_table` | `apotheosis:gem_cutting_table` | `items/gem_cutting_table.json` → `apotheosis:item/gem_cutting_table` |
| `gem_dust` | `apotheosis:gem_dust` | `items/gem_dust.json` → `apotheosis:item/gem_dust` |
| `gem_earth_flawed` | gemme du port, texture Java `apotheosis:item/gems/earth` | `apotheosis:item/gems/earth` |
| `gem_earth_flawless` | gemme du port, texture Java `apotheosis:item/gems/earth` | `apotheosis:item/gems/earth` |
| `gem_earth_normal` | gemme du port, texture Java `apotheosis:item/gems/earth` | `apotheosis:item/gems/earth` |
| `gem_earth_perfect` | gemme du port, texture Java `apotheosis:item/gems/earth` | `apotheosis:item/gems/earth` |
| `gem_endersurge_flawless` | gemme du port, texture Java `apotheosis:item/gems/endersurge` | `apotheosis:item/gems/endersurge` |
| `gem_endersurge_perfect` | gemme du port, texture Java `apotheosis:item/gems/endersurge` | `apotheosis:item/gems/endersurge` |
| `gem_fused_slate` | `apotheosis:gem_fused_slate` | `items/gem_fused_slate.json` → `apotheosis:item/gem_fused_slate` |
| `gem_guardian_chipped` | gemme du port, texture Java `apotheosis:item/gems/guardian` | `apotheosis:item/gems/guardian` |
| `gem_guardian_cracked` | gemme du port, texture Java `apotheosis:item/gems/guardian` | `apotheosis:item/gems/guardian` |
| `gem_guardian_flawed` | gemme du port, texture Java `apotheosis:item/gems/guardian` | `apotheosis:item/gems/guardian` |
| `gem_guardian_flawless` | gemme du port, texture Java `apotheosis:item/gems/guardian` | `apotheosis:item/gems/guardian` |
| `gem_guardian_normal` | gemme du port, texture Java `apotheosis:item/gems/guardian` | `apotheosis:item/gems/guardian` |
| `gem_guardian_perfect` | gemme du port, texture Java `apotheosis:item/gems/guardian` | `apotheosis:item/gems/guardian` |
| `gem_inferno_flawed` | gemme du port, texture Java `apotheosis:item/gems/inferno` | `apotheosis:item/gems/inferno` |
| `gem_inferno_flawless` | gemme du port, texture Java `apotheosis:item/gems/inferno` | `apotheosis:item/gems/inferno` |
| `gem_inferno_normal` | gemme du port, texture Java `apotheosis:item/gems/inferno` | `apotheosis:item/gems/inferno` |
| `gem_inferno_perfect` | gemme du port, texture Java `apotheosis:item/gems/inferno` | `apotheosis:item/gems/inferno` |
| `gem_lightning_chipped` | gemme du port, texture Java `apotheosis:item/gems/lightning` | `apotheosis:item/gems/lightning` |
| `gem_lightning_cracked` | gemme du port, texture Java `apotheosis:item/gems/lightning` | `apotheosis:item/gems/lightning` |
| `gem_lightning_flawed` | gemme du port, texture Java `apotheosis:item/gems/lightning` | `apotheosis:item/gems/lightning` |
| `gem_lightning_flawless` | gemme du port, texture Java `apotheosis:item/gems/lightning` | `apotheosis:item/gems/lightning` |
| `gem_lightning_normal` | gemme du port, texture Java `apotheosis:item/gems/lightning` | `apotheosis:item/gems/lightning` |
| `gem_lightning_perfect` | gemme du port, texture Java `apotheosis:item/gems/lightning` | `apotheosis:item/gems/lightning` |
| `gem_lunar_chipped` | gemme du port, texture Java `apotheosis:item/gems/lunar` | `apotheosis:item/gems/lunar` |
| `gem_lunar_cracked` | gemme du port, texture Java `apotheosis:item/gems/lunar` | `apotheosis:item/gems/lunar` |
| `gem_lunar_flawed` | gemme du port, texture Java `apotheosis:item/gems/lunar` | `apotheosis:item/gems/lunar` |
| `gem_lunar_flawless` | gemme du port, texture Java `apotheosis:item/gems/lunar` | `apotheosis:item/gems/lunar` |
| `gem_lunar_normal` | gemme du port, texture Java `apotheosis:item/gems/lunar` | `apotheosis:item/gems/lunar` |
| `gem_lunar_perfect` | gemme du port, texture Java `apotheosis:item/gems/lunar` | `apotheosis:item/gems/lunar` |
| `gem_mageslayer_flawless` | gemme du port, texture Java `apotheosis:item/gems/mageslayer` | `apotheosis:item/gems/mageslayer` |
| `gem_mageslayer_normal` | gemme du port, texture Java `apotheosis:item/gems/mageslayer` | `apotheosis:item/gems/mageslayer` |
| `gem_mageslayer_perfect` | gemme du port, texture Java `apotheosis:item/gems/mageslayer` | `apotheosis:item/gems/mageslayer` |
| `gem_molten_breach_flawed` | gemme du port, texture Java `apotheosis:item/gems/molten_breach` | `apotheosis:item/gems/molten_breach` |
| `gem_molten_breach_flawless` | gemme du port, texture Java `apotheosis:item/gems/molten_breach` | `apotheosis:item/gems/molten_breach` |
| `gem_molten_breach_normal` | gemme du port, texture Java `apotheosis:item/gems/molten_breach` | `apotheosis:item/gems/molten_breach` |
| `gem_molten_breach_perfect` | gemme du port, texture Java `apotheosis:item/gems/molten_breach` | `apotheosis:item/gems/molten_breach` |
| `gem_royalty_flawed` | gemme du port, texture Java `apotheosis:item/gems/royalty` | `apotheosis:item/gems/royalty` |
| `gem_royalty_flawless` | gemme du port, texture Java `apotheosis:item/gems/royalty` | `apotheosis:item/gems/royalty` |
| `gem_royalty_normal` | gemme du port, texture Java `apotheosis:item/gems/royalty` | `apotheosis:item/gems/royalty` |
| `gem_royalty_perfect` | gemme du port, texture Java `apotheosis:item/gems/royalty` | `apotheosis:item/gems/royalty` |
| `gem_samurai_chipped` | gemme du port, texture Java `apotheosis:item/gems/samurai` | `apotheosis:item/gems/samurai` |
| `gem_samurai_cracked` | gemme du port, texture Java `apotheosis:item/gems/samurai` | `apotheosis:item/gems/samurai` |
| `gem_samurai_flawed` | gemme du port, texture Java `apotheosis:item/gems/samurai` | `apotheosis:item/gems/samurai` |
| `gem_samurai_flawless` | gemme du port, texture Java `apotheosis:item/gems/samurai` | `apotheosis:item/gems/samurai` |
| `gem_samurai_normal` | gemme du port, texture Java `apotheosis:item/gems/samurai` | `apotheosis:item/gems/samurai` |
| `gem_samurai_perfect` | gemme du port, texture Java `apotheosis:item/gems/samurai` | `apotheosis:item/gems/samurai` |
| `gem_slipstream_chipped` | gemme du port, texture Java `apotheosis:item/gems/slipstream` | `apotheosis:item/gems/slipstream` |
| `gem_slipstream_cracked` | gemme du port, texture Java `apotheosis:item/gems/slipstream` | `apotheosis:item/gems/slipstream` |
| `gem_slipstream_flawed` | gemme du port, texture Java `apotheosis:item/gems/slipstream` | `apotheosis:item/gems/slipstream` |
| `gem_slipstream_flawless` | gemme du port, texture Java `apotheosis:item/gems/slipstream` | `apotheosis:item/gems/slipstream` |
| `gem_slipstream_normal` | gemme du port, texture Java `apotheosis:item/gems/slipstream` | `apotheosis:item/gems/slipstream` |
| `gem_slipstream_perfect` | gemme du port, texture Java `apotheosis:item/gems/slipstream` | `apotheosis:item/gems/slipstream` |
| `gem_solar_chipped` | gemme du port, texture Java `apotheosis:item/gems/solar` | `apotheosis:item/gems/solar` |
| `gem_solar_cracked` | gemme du port, texture Java `apotheosis:item/gems/solar` | `apotheosis:item/gems/solar` |
| `gem_solar_flawed` | gemme du port, texture Java `apotheosis:item/gems/solar` | `apotheosis:item/gems/solar` |
| `gem_solar_flawless` | gemme du port, texture Java `apotheosis:item/gems/solar` | `apotheosis:item/gems/solar` |
| `gem_solar_normal` | gemme du port, texture Java `apotheosis:item/gems/solar` | `apotheosis:item/gems/solar` |
| `gem_solar_perfect` | gemme du port, texture Java `apotheosis:item/gems/solar` | `apotheosis:item/gems/solar` |
| `gem_splendor_chipped` | gemme du port, texture Java `apotheosis:item/gems/splendor` | `apotheosis:item/gems/splendor` |
| `gem_splendor_cracked` | gemme du port, texture Java `apotheosis:item/gems/splendor` | `apotheosis:item/gems/splendor` |
| `gem_splendor_flawed` | gemme du port, texture Java `apotheosis:item/gems/splendor` | `apotheosis:item/gems/splendor` |
| `gem_splendor_flawless` | gemme du port, texture Java `apotheosis:item/gems/splendor` | `apotheosis:item/gems/splendor` |
| `gem_splendor_normal` | gemme du port, texture Java `apotheosis:item/gems/splendor` | `apotheosis:item/gems/splendor` |
| `gem_splendor_perfect` | gemme du port, texture Java `apotheosis:item/gems/splendor` | `apotheosis:item/gems/splendor` |
| `gem_tyrannical_chipped` | gemme du port, texture Java `apotheosis:item/gems/tyrannical` | `apotheosis:item/gems/tyrannical` |
| `gem_tyrannical_cracked` | gemme du port, texture Java `apotheosis:item/gems/tyrannical` | `apotheosis:item/gems/tyrannical` |
| `gem_tyrannical_flawed` | gemme du port, texture Java `apotheosis:item/gems/tyrannical` | `apotheosis:item/gems/tyrannical` |
| `gem_tyrannical_flawless` | gemme du port, texture Java `apotheosis:item/gems/tyrannical` | `apotheosis:item/gems/tyrannical` |
| `gem_tyrannical_normal` | gemme du port, texture Java `apotheosis:item/gems/tyrannical` | `apotheosis:item/gems/tyrannical` |
| `gem_tyrannical_perfect` | gemme du port, texture Java `apotheosis:item/gems/tyrannical` | `apotheosis:item/gems/tyrannical` |
| `gem_verdant_ruin_flawed` | gemme du port, texture Java `apotheosis:item/gems/verdant_ruin` | `apotheosis:item/gems/verdant_ruin` |
| `gem_verdant_ruin_flawless` | gemme du port, texture Java `apotheosis:item/gems/verdant_ruin` | `apotheosis:item/gems/verdant_ruin` |
| `gem_verdant_ruin_normal` | gemme du port, texture Java `apotheosis:item/gems/verdant_ruin` | `apotheosis:item/gems/verdant_ruin` |
| `gem_verdant_ruin_perfect` | gemme du port, texture Java `apotheosis:item/gems/verdant_ruin` | `apotheosis:item/gems/verdant_ruin` |
| `gem_warlord_chipped` | gemme du port, texture Java `apotheosis:item/gems/warlord` | `apotheosis:item/gems/warlord` |
| `gem_warlord_cracked` | gemme du port, texture Java `apotheosis:item/gems/warlord` | `apotheosis:item/gems/warlord` |
| `gem_warlord_flawed` | gemme du port, texture Java `apotheosis:item/gems/warlord` | `apotheosis:item/gems/warlord` |
| `gem_warlord_flawless` | gemme du port, texture Java `apotheosis:item/gems/warlord` | `apotheosis:item/gems/warlord` |
| `gem_warlord_normal` | gemme du port, texture Java `apotheosis:item/gems/warlord` | `apotheosis:item/gems/warlord` |
| `gem_warlord_perfect` | gemme du port, texture Java `apotheosis:item/gems/warlord` | `apotheosis:item/gems/warlord` |
| `geode_shelf` | `apothic_enchanting:geode_shelf` | `items/geode_shelf.json` → `apothic_enchanting:item/geode_shelf` |
| `glowing_hellshelf` | `apothic_enchanting:glowing_hellshelf` | `items/glowing_hellshelf.json` → `apothic_enchanting:item/glowing_hellshelf` |
| `godforged_pearl` | `apotheosis:godforged_pearl` | `items/godforged_pearl.json` → `apotheosis:item/godforged_pearl` |
| `gold_upgrade_smithing_template` | `apotheosis:gold_upgrade_smithing_template` | `items/gold_upgrade_smithing_template.json` → `apotheosis:item/gold_upgrade_smithing_template` |
| `heart_seashelf` | `apothic_enchanting:heart_seashelf` | `items/heart_seashelf.json` → `apothic_enchanting:item/heart_seashelf` |
| `hellshelf` | `apothic_enchanting:hellshelf` | `items/hellshelf.json` → `apothic_enchanting:item/hellshelf` |
| `improved_scrap_tome` | `apothic_enchanting:improved_scrap_tome` | `items/improved_scrap_tome.json` → `apothic_enchanting:item/improved_scrap_tome` |
| `infused_hellshelf` | `apothic_enchanting:infused_hellshelf` | `items/infused_hellshelf.json` → `apothic_enchanting:item/infused_hellshelf` |
| `infused_seashelf` | `apothic_enchanting:infused_seashelf` | `items/infused_seashelf.json` → `apothic_enchanting:item/infused_seashelf` |
| `iron_upgrade_smithing_template` | `apotheosis:iron_upgrade_smithing_template` | `items/iron_upgrade_smithing_template.json` → `apotheosis:item/iron_upgrade_smithing_template` |
| `luminous_crystal_shard` | `apotheosis:luminous_crystal_shard` | `items/luminous_crystal_shard.json` → `apotheosis:item/luminous_crystal_shard` |
| `melonshelf` | `apothic_enchanting:melonshelf` | `items/melonshelf.json` → `apothic_enchanting:item/melonshelf` |
| `mysterious_scrap_metal` | `apotheosis:mysterious_scrap_metal` | `items/mysterious_scrap_metal.json` → `apotheosis:item/mysterious_scrap_metal` |
| `pearl_endshelf` | `apothic_enchanting:pearl_endshelf` | `items/pearl_endshelf.json` → `apothic_enchanting:item/pearl_endshelf` |
| `prismatic_web` | `apothic_enchanting:prismatic_web` | `items/prismatic_web.json` → `apothic_enchanting:item/prismatic_web` |
| `reforging_table` | `apotheosis:reforging_table` | `items/reforging_table.json` → `apotheosis:item/reforging_table` |
| `salvaging_table` | `apotheosis:salvaging_table` | `items/salvaging_table.json` → `apotheosis:item/salvaging_table` |
| `scrap_tome` | `apothic_enchanting:scrap_tome` | `items/scrap_tome.json` → `apothic_enchanting:item/scrap_tome` |
| `seashelf` | `apothic_enchanting:seashelf` | `items/seashelf.json` → `apothic_enchanting:item/seashelf` |
| `sightshelf` | `apothic_enchanting:sightshelf` | `items/sightshelf.json` → `apothic_enchanting:item/sightshelf` |
| `sightshelf_t2` | `apothic_enchanting:sightshelf_t2` | `items/sightshelf_t2.json` → `apothic_enchanting:item/sightshelf_t2` |
| `sigil_of_enhancement` | `apotheosis:sigil_of_enhancement` | `items/sigil_of_enhancement.json` → `apotheosis:item/sigil_of_enhancement` |
| `sigil_of_rebirth` | `apotheosis:sigil_of_rebirth` | `items/sigil_of_rebirth.json` → `apotheosis:item/sigil_of_rebirth` |
| `sigil_of_socketing` | `apotheosis:sigil_of_socketing` | `items/sigil_of_socketing.json` → `apotheosis:item/sigil_of_socketing` |
| `sigil_of_unnaming` | `apotheosis:sigil_of_unnaming` | `items/sigil_of_unnaming.json` → `apotheosis:item/sigil_of_unnaming` |
| `sigil_of_withdrawal` | `apotheosis:sigil_of_withdrawal` | `items/sigil_of_withdrawal.json` → `apotheosis:item/sigil_of_withdrawal` |
| `simple_reforging_table` | `apotheosis:simple_reforging_table` | `items/simple_reforging_table.json` → `apotheosis:item/simple_reforging_table` |
| `soul_touched_deepshelf` | `apothic_enchanting:soul_touched_deepshelf` | `items/soul_touched_deepshelf.json` → `apothic_enchanting:item/soul_touched_deepshelf` |
| `soul_touched_sculkshelf` | `apothic_enchanting:soul_touched_sculkshelf` | `items/soul_touched_sculkshelf.json` → `apothic_enchanting:item/soul_touched_sculkshelf` |
| `stoneshelf` | `apothic_enchanting:stoneshelf` | `items/stoneshelf.json` → `apothic_enchanting:item/stoneshelf` |
| `timeworn_fabric` | `apotheosis:timeworn_fabric` | `items/timeworn_fabric.json` → `apotheosis:item/timeworn_fabric` |
| `treasure_shelf` | `apothic_enchanting:treasure_shelf` | `items/treasure_shelf.json` → `apothic_enchanting:item/treasure_shelf` |

## Textures des gemmes

`apotheosis:gem` porte son type dans ses données ; son modèle (`minecraft:select`) choisit `apotheosis:item/gem_<nom>`, dont la couche est la texture Java `items/gems/<nom>.png`. Textures importées : `ballast`, `blood_lord`, `brawlers`, `breach`, `combatant`, `earth`, `endersurge`, `guardian`, `inferno`, `lightning`, `lunar`, `mageslayer`, `molten_breach`, `royalty`, `samurai`, `slipstream`, `solar`, `splendor`, `tyrannical`, `verdant_ruin`, `warlord`.

## Correspondances explicites

| Id du port | Id Java |
| --- | --- |
| `apothic_enchanting_table` | `apothic_enchanting:apothic_enchanting_table` |
| `enchantment_library` | `apothic_enchanting:library` |
| `ender_library` | `apothic_enchanting:ender_library` |
| `filtering_shelf` | `apothic_enchanting:filtering_shelf` |
| `geode_shelf` | `apothic_enchanting:geode_shelf` |
| `treasure_shelf` | `apothic_enchanting:treasure_shelf` |

## Contrôles

```bash
python tools/import_java_assets.py C:/Users/yello/horizon-work/java-ref
python tools/pack.py
C:/Users/yello/horizon-work/target/debug/cargo-horizon-mod.exe check target/apotheosis.hmod
git status --porcelain   # ne doit rien montrer sous content-java/
```
