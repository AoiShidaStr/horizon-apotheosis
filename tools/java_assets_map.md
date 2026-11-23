# Carte des assets Java — Apotheosis pour Horizon

Rapport généré par `tools/import_java_assets.py`. **Aucun asset Java n'est dans ce dépôt** : les textures, modèles et textes importés vivent dans `content-java/`, ignoré par Git, et sont ajoutés au `.hmod` par `tools/pack.py` (sauf avec `--no-java`).

Source inspectée : `java-ref`

## Résumé

- blocs avec blockstate Java : 27
- objets avec modèle Java : 41
- modèles copiés : 99
- textures copiées : 85
- clés lang en_us : 1761
- clés lang fr_fr : 1635

Les ids sans équivalent Java gardent nos définitions et nos textures originales de `content/`. Les gemmes et blocs de gemmes sont volontairement laissés au chantier « gemmes » ; `JAVA_ALIASES` dans le script est la table à compléter pour eux.

## Blocs

| Id du port | Source Java | Fichiers importés |
| --- | --- | --- |
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
| `endshelf` | `apothic_enchanting:endshelf` | `blockstates/endshelf.json` → `apothic_enchanting:block/endshelf` |
| `gem_block_amber` | gemme — non importée (chantier gemmes) | — |
| `gem_block_amethyst` | gemme — non importée (chantier gemmes) | — |
| `gem_block_bloodstone` | gemme — non importée (chantier gemmes) | — |
| `gem_block_citrine` | gemme — non importée (chantier gemmes) | — |
| `gem_block_jade` | gemme — non importée (chantier gemmes) | — |
| `gem_block_moonstone` | gemme — non importée (chantier gemmes) | — |
| `gem_block_onyx` | gemme — non importée (chantier gemmes) | — |
| `gem_block_opal` | gemme — non importée (chantier gemmes) | — |
| `gem_block_peridot` | gemme — non importée (chantier gemmes) | — |
| `gem_block_ruby` | gemme — non importée (chantier gemmes) | — |
| `gem_block_sapphire` | gemme — non importée (chantier gemmes) | — |
| `gem_block_topaz` | gemme — non importée (chantier gemmes) | — |
| `gem_cutting_table` | `apotheosis:gem_cutting_table` | `blockstates/gem_cutting_table.json` → `apotheosis:block/gem_cutting_table`, variant `""` ajouté (notre bloc n'a pas la propriété Java) |
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
| `soul_touched_deepshelf` | `apothic_enchanting:soul_touched_deepshelf` | `blockstates/soul_touched_deepshelf.json` → `apothic_enchanting:block/soul_touched_deepshelf` |
| `soul_touched_sculkshelf` | `apothic_enchanting:soul_touched_sculkshelf` | `blockstates/soul_touched_sculkshelf.json` → `apothic_enchanting:block/soul_touched_sculkshelf` |
| `stoneshelf` | `apothic_enchanting:stoneshelf` | `blockstates/stoneshelf.json` → `apothic_enchanting:block/stoneshelf` |

## Objets

| Id du port | Source Java | Fichiers importés |
| --- | --- | --- |
| `arcane_sands` | `apotheosis:arcane_sands` | `items/arcane_sands.json` → `apotheosis:item/arcane_sands` |
| `basic_bookshelf` | `apothic_enchanting:basic_bookshelf` | `items/basic_bookshelf.json` → `apothic_enchanting:item/basic_bookshelf` |
| `beeshelf` | `apothic_enchanting:beeshelf` | `items/beeshelf.json` → `apothic_enchanting:item/beeshelf` |
| `blazing_hellshelf` | `apothic_enchanting:blazing_hellshelf` | `items/blazing_hellshelf.json` → `apothic_enchanting:item/blazing_hellshelf` |
| `crystal_seashelf` | `apothic_enchanting:crystal_seashelf` | `items/crystal_seashelf.json` → `apothic_enchanting:item/crystal_seashelf` |
| `deepshelf` | `apothic_enchanting:deepshelf` | `items/deepshelf.json` → `apothic_enchanting:item/deepshelf` |
| `dormant_deepshelf` | `apothic_enchanting:dormant_deepshelf` | `items/dormant_deepshelf.json` → `apothic_enchanting:item/dormant_deepshelf` |
| `draconic_endshelf` | `apothic_enchanting:draconic_endshelf` | `items/draconic_endshelf.json` → `apothic_enchanting:item/draconic_endshelf` |
| `echoing_deepshelf` | `apothic_enchanting:echoing_deepshelf` | `items/echoing_deepshelf.json` → `apothic_enchanting:item/echoing_deepshelf` |
| `echoing_sculkshelf` | `apothic_enchanting:echoing_sculkshelf` | `items/echoing_sculkshelf.json` → `apothic_enchanting:item/echoing_sculkshelf` |
| `enchantment_library` | `apothic_enchanting:library` | `items/enchantment_library.json` → `apothic_enchanting:item/library` |
| `endshelf` | `apothic_enchanting:endshelf` | `items/endshelf.json` → `apothic_enchanting:item/endshelf` |
| `extraction_tome` | `apothic_enchanting:extraction_tome` | `items/extraction_tome.json` → `apothic_enchanting:item/extraction_tome` |
| `gem_amber` | gemme — non importée (chantier gemmes) | — |
| `gem_amethyst` | gemme — non importée (chantier gemmes) | — |
| `gem_block_amber` | gemme — non importée (chantier gemmes) | — |
| `gem_block_amethyst` | gemme — non importée (chantier gemmes) | — |
| `gem_block_bloodstone` | gemme — non importée (chantier gemmes) | — |
| `gem_block_citrine` | gemme — non importée (chantier gemmes) | — |
| `gem_block_jade` | gemme — non importée (chantier gemmes) | — |
| `gem_block_moonstone` | gemme — non importée (chantier gemmes) | — |
| `gem_block_onyx` | gemme — non importée (chantier gemmes) | — |
| `gem_block_opal` | gemme — non importée (chantier gemmes) | — |
| `gem_block_peridot` | gemme — non importée (chantier gemmes) | — |
| `gem_block_ruby` | gemme — non importée (chantier gemmes) | — |
| `gem_block_sapphire` | gemme — non importée (chantier gemmes) | — |
| `gem_block_topaz` | gemme — non importée (chantier gemmes) | — |
| `gem_bloodstone` | gemme — non importée (chantier gemmes) | — |
| `gem_citrine` | gemme — non importée (chantier gemmes) | — |
| `gem_cutting_table` | `apotheosis:gem_cutting_table` | `items/gem_cutting_table.json` → `apotheosis:item/gem_cutting_table` |
| `gem_dust` | `apotheosis:gem_dust` | `items/gem_dust.json` → `apotheosis:item/gem_dust` |
| `gem_jade` | gemme — non importée (chantier gemmes) | — |
| `gem_moonstone` | gemme — non importée (chantier gemmes) | — |
| `gem_onyx` | gemme — non importée (chantier gemmes) | — |
| `gem_opal` | gemme — non importée (chantier gemmes) | — |
| `gem_peridot` | gemme — non importée (chantier gemmes) | — |
| `gem_ruby` | gemme — non importée (chantier gemmes) | — |
| `gem_sapphire` | gemme — non importée (chantier gemmes) | — |
| `gem_topaz` | gemme — non importée (chantier gemmes) | — |
| `glowing_hellshelf` | `apothic_enchanting:glowing_hellshelf` | `items/glowing_hellshelf.json` → `apothic_enchanting:item/glowing_hellshelf` |
| `godforged_pearl` | `apotheosis:godforged_pearl` | `items/godforged_pearl.json` → `apotheosis:item/godforged_pearl` |
| `heart_seashelf` | `apothic_enchanting:heart_seashelf` | `items/heart_seashelf.json` → `apothic_enchanting:item/heart_seashelf` |
| `hellshelf` | `apothic_enchanting:hellshelf` | `items/hellshelf.json` → `apothic_enchanting:item/hellshelf` |
| `improved_scrap_tome` | `apothic_enchanting:improved_scrap_tome` | `items/improved_scrap_tome.json` → `apothic_enchanting:item/improved_scrap_tome` |
| `infused_hellshelf` | `apothic_enchanting:infused_hellshelf` | `items/infused_hellshelf.json` → `apothic_enchanting:item/infused_hellshelf` |
| `infused_seashelf` | `apothic_enchanting:infused_seashelf` | `items/infused_seashelf.json` → `apothic_enchanting:item/infused_seashelf` |
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
| `sigil_of_withdrawal` | `apotheosis:sigil_of_withdrawal` | `items/sigil_of_withdrawal.json` → `apotheosis:item/sigil_of_withdrawal` |
| `soul_touched_deepshelf` | `apothic_enchanting:soul_touched_deepshelf` | `items/soul_touched_deepshelf.json` → `apothic_enchanting:item/soul_touched_deepshelf` |
| `soul_touched_sculkshelf` | `apothic_enchanting:soul_touched_sculkshelf` | `items/soul_touched_sculkshelf.json` → `apothic_enchanting:item/soul_touched_sculkshelf` |
| `stoneshelf` | `apothic_enchanting:stoneshelf` | `items/stoneshelf.json` → `apothic_enchanting:item/stoneshelf` |
| `timeworn_fabric` | `apotheosis:timeworn_fabric` | `items/timeworn_fabric.json` → `apotheosis:item/timeworn_fabric` |

## Gemmes laissées au chantier « gemmes »

Objets : `gem_amber`, `gem_amethyst`, `gem_bloodstone`, `gem_citrine`, `gem_jade`, `gem_moonstone`, `gem_onyx`, `gem_opal`, `gem_peridot`, `gem_ruby`, `gem_sapphire`, `gem_topaz`.

Blocs : `gem_block_amber`, `gem_block_amethyst`, `gem_block_bloodstone`, `gem_block_citrine`, `gem_block_jade`, `gem_block_moonstone`, `gem_block_onyx`, `gem_block_opal`, `gem_block_peridot`, `gem_block_ruby`, `gem_block_sapphire`, `gem_block_topaz`.

## Correspondances explicites

| Id du port | Id Java |
| --- | --- |
| `enchantment_library` | `apothic_enchanting:library` |

## Contrôles

```bash
python tools/import_java_assets.py C:/Users/yello/horizon-work/java-ref
python tools/pack.py
C:/Users/yello/horizon-work/target/debug/cargo-horizon-mod.exe check target/apotheosis.hmod
git status --porcelain   # ne doit rien montrer sous content-java/
```
