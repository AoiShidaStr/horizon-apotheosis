# Résultats du test en jeu — Apotheosis 0.6.0 pour Horizon

Test du 2026-11-24 : serveur dédié Horizon dans Docker (`tools/docker/`, moteur
compilé avec le mod, Horizon a85af177) et client Horizon de test compilé avec le
mod, connecté à ce serveur, piloté par le contrôle d'agent. Assets Java importés
localement depuis les jars de la saison 1. Plan suivi : `TEST-EN-JEU.md`.

## Ce qui marche en jeu

| Fonction | Vérifié |
| --- | --- |
| Chargement | serveur et client chargent le mod (même `.hmod` des deux côtés, aucune erreur ni plantage dans les journaux) |
| `/apoth` | aide traduite en français, `reforge` et `salvage` |
| Affixes | noms Java « préfixe + objet + suffixe », raretés et couleurs, attributs réellement appliqués (`/attribute` : +1 chance, +22,61 % vitesse d'attaque, base de l'épée conservée), effets traduits |
| Reforge | commande et table de reforge (coût Java : niveaux, matériau de rareté, sceaux de renaissance), rareté conservée à la table, tirage différent à chaque reforge |
| Sockets | sceau d'emplacement (« Socket ajouté : 1/3 »), sertissage d'une gemme Java (« Gemme sertie : Gemme de la Stabilité (Normale) »), bonus appliqué en main principale seulement (15,25 dégâts), lore limité à la catégorie de l'objet |
| Gemmes | objet unique, texture Java par type (Ballast verte), nom traduit |
| Recyclage | table de recyclage : une pièce peu commune donne 4 tissus usés |
| Enchantement | anneau d'étagères infernales : offres 13 / 31 / 45 niveaux sur serveur dédié (Eterna Java) ; étagères avec leur texture Java |
| Tomes | livre de démantèlement : objet détruit, livre enchanté obtenu |
| Bibliothèque | absorbe un livre enchanté, propose un livre à reprendre (barre d'action) |
| Spawners | modificateur au clic droit (sucre : délai 200 → 190, confirmé par `/data`), stats en barre d'action, ramassage au Toucher de soie avec le mob et les stats, reposé à l'identique |
| Boss | un Apothic Invader est apparu (« Vil Squelette le Ravageur s'est éveillé tout près ! ») |
| Messages | refus et résultats des tables, sceaux et tomes visibles par le joueur |

## Défauts trouvés en jeu, tous corrigés et revérifiés

1. Les gemmes affichaient la texture par défaut : l'import Java écrasait la sélection de modèle par type.
2. La gemme s'appelait `block.apotheosis.gem` (clé de base absente) et les gemmes du butin avaient un nom personnalisé en italique.
3. Le bonus d'une gemme sertie sur une arme s'appliquait aussi depuis la main secondaire.
4. Le lore d'une pièce sertie listait les bonus de toutes les catégories.
5. Les messages des tables, sceaux et tomes n'étaient pas affichés.
6. Des noms restaient en anglais en jeu français (priorité des traductions de l'import).
7. Le nom de l'effet d'un affixe (« Weakness ») était écrit en dur.
8. La table de reforge faisait baisser la rareté.
9. La reforge donnait toujours le même résultat.
10. Les livres de démantèlement, la toile prismatique, la bibliothèque et la table d'infusion ne réagissaient pas sur serveur dédié.
11. Un spawner cassé au Toucher de soie ne tombait pas (Toucher de soie lu au mauvais endroit), puis ses données n'étaient pas visibles côté client.

## Pas encore testé en jeu

- La table de taille des gemmes, le sceau de retrait, de renaissance et d'amélioration, la toile prismatique et la table d'infusion (corrigées, couvertes par les tests unitaires, pas rejouées).
- Le butin des monstres (gemmes et équipement affixé) et des coffres.
- Les affixes à effet en combat (poison, exécution, foudre…), les attributs Apothic en combat (critiques, vol de vie, esquive).
- Le jeu à plusieurs joueurs.
