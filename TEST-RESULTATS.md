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

## Second test en jeu (2026-11-26) : les tables d'Apotheosis avec leurs vrais écrans

Même installation (serveur dédié Docker, client de test en français).

| Fonction | Vérifié |
| --- | --- |
| Table de reforge | un clic ouvre l'écran Java (texture `reforge.png`) : objet, matériau de rareté, sceaux de renaissance, trois choix avec leur coût. En survie le premier choix « Rare » a pris 2 éclats de cristal lumineux, 2 sceaux et 1 niveau et rendu une épée rare avec ses affixes ; gratuit en créatif comme en Java |
| Table de recyclage | écran Java : une épée rare a donné des éclats de cristal lumineux dans les cases de sortie |
| Table de taille | écran Java : gemme de ballast fêlée + deuxième gemme identique + 1 poussière + 3 ferrailles → gemme ébréchée (coûts Java) |
| Gemmes | 107 objets (une gemme × chaque pureté autorisée) dans l'onglet créatif, nommés dans la langue du joueur |
| Infobulles des tables | dans la langue du joueur seulement, avec le nom réel des matériaux |

Défauts trouvés et corrigés pendant ce test : le serveur refusait tout le contenu du mod (`minecraft:custom_data` n'est pas accepté dans les composants par défaut d'une déclaration d'objet) ; infobulles bilingues ; nom `block.apotheosis.gem_…` dans le chat.

Différences avec le Java qui restent (limites de Horizon a85af177, API 415) :
- les trois choix de reforge montrent la rareté et le coût, pas l'aperçu de l'objet ; le recyclage n'a pas d'aperçu grisé (l'infobulle donne les fourchettes) ;
- les objets posés restent dans la table quand on ferme l'écran, comme dans un four ;
- le sertissage, les sceaux d'emplacement, de retrait et de dé-nommage se font encore au clic droit et non dans la table de forgeron : le crochet de forge de Horizon ne transmet pas le contenu des cases et n'a pas d'événement à la prise du résultat.

### Infobulle des gemmes (2026-11-26)

Au survol, une gemme affiche l'infobulle de Java : nom dans la couleur de sa pureté, « S'adapte à : » et les types d'équipement, « Lorsqu'emboîté dans : » et le bonus par type (vérifié sur la Gemme Tyrannique : +2 recul d'attaque, +3 robustesse, +30 % robustesse sur bouclier, +17,5 % vie maximale, −10 % vol de vie), plus une ligne propre au port qui rappelle que le sertissage se fait au clic droit. Défauts trouvés et corrigés : le client ignore le lore et la couleur déclarés sur un objet (l'infobulle est donc écrite par le crochet natif) ; toutes les lignes d'infobulle du mod étaient invisibles (couleur sans alpha, Horizon attend de l'ABGR opaque) ; un bonus multiplicateur s'affichait « +0.30 » au lieu de « +30 % ».

### Étagères et invaders (2026-11-26)

- Au survol, une étagère affiche ses statistiques d'enchantement comme Apothic Enchanting (vérifié : Bibliothèque Profonde, Éterna +5.00 (Max 70.00), Quanta +5.00 %, Arcana +5.00 %). La table compte maintenant aussi les bibliothèques et crânes vanilla, comme l'annonce leur infobulle.
- Taux de butin : un monstre ordinaire tué par un joueur a 4 % de chance de gemme et 4 % d'équipement affixé, comme en Java. Mais 2 % des apparitions nocturnes devenaient des boss sans aucun délai : corrigé aux taux Java (1,8 % surface et End, 2,5 % Nether) avec le délai de 3 minutes entre deux boss (par dimension).
