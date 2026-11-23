# Plan de test en jeu — Apotheosis pour Horizon

Ce fichier décrit comment vérifier **tout** le mod, chantier par chantier, sur une
partie réelle. Il est écrit pour être suivi dans l'ordre : chaque section dit quoi
taper, ce qu'on doit voir, et comment le prouver avec `horizon-ctl` plutôt qu'à
l'œil.

Le mod est un seul identifiant `apotheosis` (une moitié contenu `apotheosis.hmod`
et une moitié native `apotheosis.hnmod`) qui réunit quatre chantiers :

| Chantier | Ce qu'il apporte |
| --- | --- |
| Gemmes, attributs et affixes | douze gemmes, poussière, douze blocs de gemme, affixes sur le butin des mobs, boss Apothic Invader |
| Étagères et table d'enchantement | vingt-trois étagères, Eterna à la table, trois tomes et la Toile prismatique |
| Sockets, pureté, reforge, recyclage | sockets sur les pièces, pureté des gemmes, `/apoth reforge`, `/apoth salvage`, cinq matériaux de rareté |
| Spawners | spawners récupérables au Toucher de soie, seize modificateurs, inverseur quartz, stats en barre d'action et en infobulle |

---

## 0. Préparation

### 0.1 Installer et lancer

1. Construire les deux moitiés : `python tools/pack.py` écrit
   `target/apotheosis.hmod` et `target/apotheosis.hnmod`.
2. Les poser dans `horizon-mods/` et `horizon-native-mods/`, et activer le mod
   dans `horizon-mods.txt` (`mod.apotheosis:true`).
3. Construire la moitié native dans le jeu (page **Mods** du launcher, ou
   `horizon-launcher native-build <dossier>`). Un serveur dédié a besoin de son
   propre moteur : `horizon-launcher native-build <dossier> --server-output <dir>`.
4. Lancer une partie neuve en créatif avec les tricheurs, sous contrôle de
   l'agent :

```
horizon-ctl launch --world test-apoth --seed 42 --creative --wait
horizon-ctl wait-ready 120
```

Si le jeu tourne déjà, il faut **Agent Control** allumé (Horizon Settings >
AI Agent) pour que `horizon-ctl` réponde.

### 0.2 Les commandes de preuve

Toutes les vérifications « ce qu'on doit voir » de ce document se font avec ces
commandes ; elles renvoient du JSON, pas une impression visuelle.

| Commande | Ce qu'elle prouve |
| --- | --- |
| `horizon-ctl state` | position, dimension, vie, cible du réticule, objet en main et en secondaire |
| `horizon-ctl inventory` | chaque pile avec ses **composants** : `attribute_modifiers`, `lore`, `custom_data`, `block_entity_data`… |
| `horizon-ctl messages 30` | le chat **et la barre d'action** (donc les lignes du mod) |
| `horizon-ctl entities 32 <type>` | les mobs et les objets au sol autour du joueur |
| `horizon-ctl container X Y Z` | le contenu d'un coffre |
| `horizon-ctl logs --lines 200` | les erreurs de commande (`That position is not loaded`, `Unknown item`…) |
| `horizon-ctl events --since SEQ` | ce qui vient de se passer : `hurt`, `death`, `item`, `chat` |
| `horizon-ctl screenshot <chemin>.png` | la trame suivante, pour une infobulle ou un écran |
| `horizon-ctl wait 20` | laisser passer une seconde de jeu (20 ticks) |

Deux règles :

- **`horizon-ctl command "…"`** envoie une commande vanilla (le `/` est
  facultatif). C'est ainsi qu'on donne des objets avec composants, qu'on écrit du
  NBT, etc.
- **`horizon-ctl screenshot`** ne voit que ce qui est **dessiné** : une infobulle
  demande qu'une personne passe la souris sur la case, puis un `screenshot`.

### 0.3 Vérifier que le mod est bien là

```
horizon-ctl command "give @s apotheosis:gem"
horizon-ctl inventory
```

On doit voir une pile `apotheosis:gem` (sinon : `Unknown item`, le `.hmod`
n'est pas chargé). Puis, pour les hooks natifs :

```
horizon-ctl command "give @s apotheosis:scrap_tome"
horizon-ctl messages 10
horizon-ctl logs --lines 100
```

Aucune ligne d'erreur ne doit nommer `apotheosis` dans les logs. Si
`/apoth` n'existe pas (`Unknown command`), la moitié `.hnmod` n'est pas
construite dans ce moteur.

**Sanity check global, à refaire à la fin :**

```
horizon-ctl logs --lines 400
```

Rien de `Exception`, `Unknown`, `not loaded`, `Failed` ne doit venir de
`apotheosis`.

---

## 1. Chantier « gemmes, attributs et affixes »

### 1.1 Une gemme Java a une infobulle et un bonus une fois sertie

Les gemmes sont les 21 gemmes Java, réunies dans un seul objet `apotheosis:gem` :
son type et sa pureté sont dans ses données (`apotheosis:gem`,
`apotheosis:gem_purity`), et une gemme ne fait rien avant d'être sertie.

**Une gemme de type et de pureté choisis** s'obtient par le butin (mob ou
coffre), pas par `/give` : tuer des mobs ouvre des coffres jusqu'à en tenir une,
puis lire sa pile.

```
horizon-ctl command "give @s apotheosis:gem"
horizon-ctl inventory
```

**Attendu :** la pile est `apotheosis:gem`, porte `minecraft:custom_data` avec
`apotheosis:gem` (id du type, par exemple `apotheosis:core/ballast`) et
`apotheosis:gem_purity` (0 à 5), et `minecraft:custom_model_data` avec ce même
id de type. Son modèle affiche la texture Java du type.

Infobulle :

```
horizon-ctl open-inventory
# une personne passe la souris sur la case de la gemme, puis :
horizon-ctl screenshot /tmp/gemme.png
```

**Attendu :** le nom Java du type (« Gem of the Molten Breach »…), puis une
ligne par catégorie d'objet avec la valeur de la pureté (« Melee Weapon: +5 Attack
Damage »…). Les bonus que ce port n'applique pas n'apparaissent pas.

**Le bonus actif** : sertir la gemme dans une pièce à socket libre (clic droit,
gemme en main, pièce en main secondaire), puis lire la pièce :

```
horizon-ctl command "attribute @s minecraft:attack_damage get"
```

→ la pièce porte `minecraft:attribute_modifiers` avec un modificateur
`apotheosis:socket_0_0` (un par bonus résolu), et la valeur suit celle du JSON
Java à cette pureté. Une gemme non sertie ne change aucun attribut.

### 1.2 Poussière de gemme

```
horizon-ctl command "give @s apotheosis:gem_dust 64"
horizon-ctl inventory
```

**Attendu :** la pile `apotheosis:gem_dust` existe et se montre avec sa texture.

Recettes de survie (à vérifier à l'établi) :

- 1 éclat d'améthyste → 2 poussières ; 1 gemme (tag `apotheosis:gems`, qui ne
  contient que `apotheosis:gem`) → 3 poussières ;
- 4 poussières en main secondaire, gemme en main, clic droit → pureté +1.

Les blocs de gemme et la taille de gemme à l'établi n'existent plus : ce port
n'a que l'objet gemme Java et la poussière.

### 1.3 Affixes sur le butin des mobs

```
horizon-ctl command "give @s minecraft:diamond_sword"
horizon-ctl command "xp 60"
horizon-ctl select minecraft:diamond_sword
horizon-ctl command "apoth reforge"
horizon-ctl inventory
```

**Attendu :** l'épée garde son nom, gagne une ligne de lore par affixe, un
`minecraft:rarity` (donc la couleur du nom) et des entrées
`minecraft:attribute_modifiers`. Les 3 niveaux d'expérience sont pris (le
`xp 60` d'avant doit suffire ; sinon `Unknown`/message traduit dans `messages`).
`inventory` est la preuve : la pile doit lister `minecraft:lore` et
`minecraft:attribute_modifiers`.

Un affixe n'apparaît que sur une pièce qu'il peut toucher : un arc ne roule pas
`attack_damage`, un plastron ne roule pas un affixe d'arc. Vérifier en reforgeant
plusieurs pièces (`/apoth reforge` sur `minecraft:bow`, `minecraft:diamond_chestplate`)
et en lisant le lore.

Butin des mobs (butin natif, pas de commande) :

```
horizon-ctl summon minecraft:zombie
horizon-ctl entities 16 zombie
horizon-ctl hunt <id>
horizon-ctl collect
horizon-ctl inventory
```

**Attendu :** une épée en fer ou un objet d'équipement affixé peut tomber (4 % de
chance), et une gemme basse (rubis, ambre, topaz, améthyste) dans 12 % des cas.
Comme c'est aléatoire, répéter 10 à 20 mobs. Les hauts paliers ne tombent
**jamais** d'un mob ordinaire : le lore d'une pièce de zombie ne doit nommer que
des paliers Uncommon à Epic.

Coffres vanilla (8 % par table `minecraft:chests/*`, gemmes Common et Uncommon
seulement) :

```
horizon-ctl command "setblock ~3 ~ ~ minecraft:chest"
horizon-ctl command "loot insert <X> <Y> <Z> loot minecraft:chests/simple_dungeon"
horizon-ctl container <X> <Y> <Z>
```

Répéter une dizaine de coffres (changer la position à chaque fois) : une gemme
Common ou Uncommon doit finir par apparaître, jamais une Mythic ou Ancient.

### 1.4 Effets d'affixes (Venomous, Withering, Executing…)

Ces affixes n'existent qu'à partir d'Epic et sur la bonne classe d'objet. Le plus
simple est de reforger jusqu'à obtenir une pièce Epic ou plus :

```
horizon-ctl command "give @s minecraft:diamond_sword"
horizon-ctl command "xp 100"
horizon-ctl select minecraft:diamond_sword
horizon-ctl command "apoth reforge"
horizon-ctl inventory      # lire minecraft:custom_data."apotheosis:effects"
```

Puis frapper un mob (`horizon-ctl summon minecraft:zombie`, `attack <id>`) :

- **Venomous / Withering / Weakening / Ensnaring** : sur le mob,
  `horizon-ctl effects` ne s'applique qu'au joueur ; pour la cible, viser une
  exécution qui la tue en quelques coups, et vérifier dans `messages`/particules ;
- **Executing** : un mob à faible vie meurt en un coup ;
- **Cleaving** : les mobs voisins (`horizon-ctl entities 4`) perdent de la vie ;
- **Thunderstruck** : un éclair tombe (bloc brûlé, son) ;
- **Blinding** : le mob touché ne suit plus (marche au hasard) ;
- **Retreating** : avec un bouclier en secondaire, le mob est repoussé ;
- **Durable** : l'usure du stack baisse (`inventory`, champ `damage`) ;
- **Telepathic** : le butin du mob est directement dans l'inventaire, pas au sol ;
- **Festive** : le butin est doublé ;
- **Spectral / Magical** (arc) : une flèche spectrale apparaît, les dégâts montent.

**Preuve minimale acceptable :** `inventory` montre
`minecraft:custom_data."apotheosis:effects"` avec l'id de l'effet, et la cible
meurt ou subit un effet visible dans les 3 secondes.

### 1.5 Boss Apothic Invader

L'invader n'apparaît que sur une **apparition naturelle** (2 % de chance), la
nuit en surface, ou partout au Nether et dans l'End.

```
horizon-ctl command "time set night"
horizon-ctl command "gamerule doMobSpawning true"
# rester en surface, laisser tourner plusieurs minutes
horizon-ctl events
horizon-ctl messages 20
```

**Attendu :** une annonce traduite dans le chat (le nom généré du boss), un mob
avec un nom en `CustomName`, plus de vie que son espèce, de l'équipement en diamant
affixé, et le tag persistant `apotheosis:boss`. Le tuer (`hunt`) doit donner une
gemme haute (onyx, moonstone, bloodstone ou opal, en cycle) : c'est la seule
source de ces gemmes avec `/apoth reforge` et les recettes.

Vérification du tag :

```
horizon-ctl command "data get entity @e[type=minecraft:zombie,limit=1,sort=nearest] Tags"
```

---

## 2. Chantier « étagères et table d'enchantement »

### 2.1 Les vingt-trois étagères se posent et comptent

Les identifiants : `basic_bookshelf`, `hellshelf`, `blazing_hellshelf`,
`glowing_hellshelf`, `infused_hellshelf`, `seashelf`, `crystal_seashelf`,
`heart_seashelf`, `infused_seashelf`, `deepshelf`, `dormant_deepshelf`,
`echoing_deepshelf`, `soul_touched_deepshelf`, `echoing_sculkshelf`,
`soul_touched_sculkshelf`, `endshelf`, `pearl_endshelf`, `draconic_endshelf`,
`sightshelf`, `sightshelf_t2`, `beeshelf`, `melonshelf`, `stoneshelf` — tous sous
`apotheosis:`.

```
horizon-ctl command "give @s apotheosis:draconic_endshelf 64"
horizon-ctl command "setblock ~2 ~ ~ apotheosis:draconic_endshelf"
horizon-ctl block <X> <Y> <Z>
horizon-ctl inventory
```

**Attendu :** le bloc prend, l'objet a un nom traduit
(`bloc.apotheosis.draconic_endshelf` → « Étagère draconique » en FR), une texture
propre, et il se casse à la hache en rendant l'étagère.

### 2.2 Eterna à la table (multijoueur ou serveur dédié)

L'API 291 n'est distribuée qu'aux invités d'un monde ouvert en LAN ou d'un
serveur dédié ; en solo intégré, les étagères comptent comme des bibliothèques
(plafond vanilla de 15, donc niveau 30). Tester les deux cas.

Montage : une table d'enchantement avec les étagères sur son périmètre à **deux
blocs**, et rien devant elles.

```
horizon-ctl command "setblock ~5 ~ ~ minecraft:enchanting_table"
horizon-ctl command "fill ~3 ~ ~-3 ~7 ~4 ~3 minecraft:air"
# l'anneau d'étagères : deux blocs autour de la table (en ~5 ~ ~), sur deux
# hauteurs, aucune étagère cachée par un bloc plein devant elle
horizon-ctl command "fill ~3 ~ ~-2 ~7 ~1 ~-2 apotheosis:draconic_endshelf"
horizon-ctl command "fill ~3 ~ ~2 ~7 ~1 ~2 apotheosis:draconic_endshelf"
horizon-ctl command "fill ~3 ~ ~-1 ~3 ~1 ~1 apotheosis:draconic_endshelf"
horizon-ctl command "fill ~7 ~ ~-1 ~7 ~1 ~1 apotheosis:draconic_endshelf"
horizon-ctl command "give @s minecraft:diamond_sword"
horizon-ctl command "xp 120"
horizon-ctl select minecraft:diamond_sword
horizon-ctl interact <X> <Y> <Z>          # la table
horizon-ctl ui
horizon-ctl screenshot /tmp/table.png
```

**Attendu (LAN ou serveur) :** les trois offres dépassent largement les 30
niveaux vanilla et montent vers **45, 70 et 100** ; la plus chère vaut l'Eterna
totale du montage (plafonnée à 100), les deux autres en sont des fractions
(20-40 % et 60-80 %). Le montant exact dépend du nombre d'étagères vues :
l'important est que la troisième offre **dépasse 30** et que retirer une étagère
la fasse baisser. En solo intégré : les trois offres restent plafonnées à 30
niveaux (comportement vanilla), mais les particules d'étagère autour de la table
doivent apparaître.

Variante négative : remplacer une étagère par un bloc plein devant elle
(`setblock` juste devant) → les offres baissent, l'étagère cachée ne compte plus.

```
horizon-ctl command "setblock <devant l'étagère> minecraft:stone"
horizon-ctl ui
```

### 2.3 Tomes et Toile prismatique

Les quatre objets : `apotheosis:scrap_tome` (6 niveaux par enchantement gardé,
garde `ceil(n/2)`), `apotheosis:improved_scrap_tome` (10 niveaux, garde tout),
`apotheosis:extraction_tome` (16 niveaux, vide l'objet), `apotheosis:prismatic_web`
(30 niveaux, retire les malédictions).

```
horizon-ctl command "give @s minecraft:diamond_sword[minecraft:enchantments={\"minecraft:sharpness\":5,\"minecraft:unbreaking\":3}]"
horizon-ctl command "give @s apotheosis:scrap_tome"
horizon-ctl command "xp 60"
horizon-ctl command "item replace entity @s weapon.offhand with minecraft:diamond_sword[minecraft:enchantments={\"minecraft:sharpness\":5,\"minecraft:unbreaking\":3}]"
horizon-ctl select apotheosis:scrap_tome
horizon-ctl use-item
horizon-ctl inventory
```

**Attendu :** le tome est consommé, 6 × 2 = 12 niveaux sont pris, et
l'inventaire contient un `minecraft:enchanted_book` avec une partie des
enchantements (au moins un, généré par une graine stable : refaire la même
manipulation avec le même joueur et le même objet redonne la même sélection).
L'épée en secondaire garde ses enchantements d'origine pour le tome de rebut
(seul l'`extraction_tome` les vide).

Malédictions :

```
horizon-ctl command "item replace entity @s weapon.mainhand with minecraft:diamond_helmet[minecraft:enchantments={\"minecraft:binding_curse\":1,\"minecraft:protection\":2}]"
horizon-ctl command "item replace entity @s weapon.offhand with apotheosis:prismatic_web"
horizon-ctl command "xp 40"
horizon-ctl use-item
horizon-ctl inventory
```

**Attendu :** `binding_curse` a disparu, `protection` reste, la Toile est
consommée, 30 niveaux sont partis. En créatif, aucun niveau ne doit être pris
(mais le tome est consommé).

Cas d'échec à vérifier : sac de tomes avec un inventaire plein → l'objet ne doit
**pas** être pris (message traduit), et l'extraction doit rendre l'objet si le
livre ne rentre pas.

---

## 3. Chantier « sockets, pureté, reforge, recyclage »

### 3.1 Reforge d'une pièce affixée

```
horizon-ctl command "give @s minecraft:diamond_chestplate"
horizon-ctl command "xp 200"
horizon-ctl select minecraft:diamond_chestplate
horizon-ctl command "apoth reforge"
horizon-ctl wait 5
horizon-ctl inventory
```

**Attendu :** la pièce porte `minecraft:custom_data` avec `apotheosis:tier`,
`apotheosis:sockets` (0 à 3 selon la rareté : Common 0, Uncommon 0-1, Rare 0-2,
Epic 1-3, Mythic 2-3, Ancient 3), une entrée `apotheosis:socketed` (vide d'abord),
un lore par affixe et `minecraft:rarity`. 3 niveaux sont pris. Relancer
`/apoth reforge` sur la même pièce la **relance** (nouveau tirage) au lieu d'être
refusé.

### 3.2 Socketage par clic droit

```
# se procurer une pièce avec au moins un socket : reforger jusqu'à un palier
# Epic ou plus (Epic 1-3 sockets, Mythic 2-3, Ancient 3)
horizon-ctl command "give @s minecraft:diamond_chestplate"
horizon-ctl command "xp 200"
horizon-ctl select minecraft:diamond_chestplate
horizon-ctl command "apoth reforge"
horizon-ctl wait 5
horizon-ctl inventory        # lire apotheosis:sockets ; recommencer si 0

# la pièce reforgée passe en main secondaire (touche F), puis la gemme en main
horizon-ctl key f tap
horizon-ctl command "give @s apotheosis:gem_sapphire"
horizon-ctl select apotheosis:gem_sapphire
horizon-ctl use-item
horizon-ctl inventory
```

**Attendu :** la gemme est consommée, le plastron gagne une entrée
`minecraft:attribute_modifiers` d'id `apotheosis:socket_0_gem_sapphire`
(`add_value`, slot `armor`), la clé `apotheosis:socketed` liste la gemme avec sa
pureté, et le lore ajoute « Sockets : 1/2 » puis une ligne de gemme colorée par la
pureté. Le bonus s'applique quand la pièce est portée :

```
horizon-ctl command "item replace entity @s armor.chest with <le plastron socketé>"
horizon-ctl command "attribute @s minecraft:max_health get"
```

Infobulle du plastron : `horizon-ctl open-inventory`, survol à la souris,
`horizon-ctl screenshot /tmp/sockets.png`.

### 3.3 Pureté et polissage

```
horizon-ctl summon minecraft:zombie
horizon-ctl hunt <id>
horizon-ctl collect
horizon-ctl inventory
```

**Attendu :** une gemme ramassée sur un mob porte une pureté dans son
`custom_data` (Cracked 30 %, Chipped 40 %, Flawed 20 %, Normal 8 %, Flawless 2 %,
Perfect 1 % ; sur un boss : 5/20/35/28/10/2), affichée dans son infobulle.

Polissage : gemme en main, **4 poussières de gemme** en secondaire, clic droit.

```
horizon-ctl command "give @s apotheosis:gem"
horizon-ctl command "item replace entity @s weapon.offhand with apotheosis:gem_dust 4"
horizon-ctl select apotheosis:gem
horizon-ctl use-item
horizon-ctl inventory
```

**Attendu :** la pureté monte d'un cran, 4 poussières sont consommées, la gemme
est gardée. À Perfect, rien n'est consommé.

### 3.4 Recyclage

```
horizon-ctl command "give @s minecraft:diamond_sword"
horizon-ctl command "xp 100"
horizon-ctl select minecraft:diamond_sword
horizon-ctl command "apoth reforge"
horizon-ctl wait 5
horizon-ctl command "apoth salvage"
horizon-ctl inventory
horizon-ctl messages 10
```

**Attendu :** la pièce disparaît ; l'inventaire reçoit 1 à 4 du matériau de sa
rareté (`mysterious_scrap_metal` Common, `timeworn_fabric` Uncommon,
`luminous_crystal_shard` Rare, `arcane_sands` Epic, `godforged_pearl` Mythic et
Ancient), 1 à 4 poussières de gemme, et **toutes** les gemmes socketées à leur
pureté d'origine.

Les cinq matériaux : `apotheosis:mysterious_scrap_metal`,
`timeworn_fabric`, `luminous_crystal_shard`, `arcane_sands`, `godforged_pearl`.
Ils se craftent (fer+silex, ficelle+cuir, améthyste+glowstone, sable des âmes+lapis,
or autour d'une perle de l'Ender) et se recyclent en 3 poussières de gemme.

### 3.5 `/apoth` seul

```
horizon-ctl command "apoth"
horizon-ctl messages 5
```

**Attendu :** un message traduit qui explique `reforge` et `salvage`. Une
sous-commande inconnue (`/apoth nimportequoi`) doit répondre par le même aide,
pas par une erreur de commande.

---

## 4. Chantier « spawners »

### 4.1 Récupérer un spawner au Toucher de soie

```
horizon-ctl command "give @s minecraft:diamond_pickaxe[minecraft:enchantments={\"minecraft:silk_touch\":1}]"
horizon-ctl command "setblock ~2 ~ ~ minecraft:spawner"
horizon-ctl command "data merge block <X> <Y> <Z> {SpawnData:{entity:{id:\"minecraft:zombie\"}},MinSpawnDelay:60,MaxSpawnDelay:120,SpawnCount:2,MaxNearbyEntities:8,RequiredPlayerRange:24,SpawnRange:6}"
horizon-ctl select minecraft:diamond_pickaxe
horizon-ctl look-at-block <X> <Y> <Z>
horizon-ctl break <X> <Y> <Z>
horizon-ctl collect
horizon-ctl inventory
```

**Attendu :** l'inventaire contient un `minecraft:spawner` dont la pile porte
`minecraft:block_entity_data` avec `SpawnData` (le zombie) et les réglages
ci-dessus. Sans Toucher de soie, le spawner ne donne **rien** (vanilla).

Infobulle de l'objet ramassé : `horizon-ctl open-inventory`, survol,
`horizon-ctl screenshot /tmp/spawner-item.png`.

**Attendu :** « Mob : zombie », puis les délais (60–120 ticks), les mobs par
vague, les trois portées, puis les options actives. Rien ne doit apparaître pour
un `minecraft:spawner` vide (sans `block_entity_data`).

### 4.2 Le reposer à l'identique

```
horizon-ctl command "setblock ~4 ~ ~ minecraft:stone"
horizon-ctl select minecraft:spawner
horizon-ctl place <X> <Y> <Z>
horizon-ctl command "data get block <X> <Y> <Z>"
```

**Attendu :** le bloc repose, et `data get block` renvoie `SpawnData`,
`MinSpawnDelay:60`, `MaxSpawnDelay:120`, `SpawnCount:2`, etc. Le mob doit
apparaître (se mettre à moins de 24 blocs) : `horizon-ctl entities 24 zombie`.

Même test avec un spawner contenant un autre mob (`minecraft:allay`,
`minecraft:blaze`) pour vérifier que ce n'est pas le zombie qui est codé en dur.

### 4.3 Lire les stats et appliquer les seize modificateurs

Clic simple sur un spawner (objet quelconque ou main vide) :

```
horizon-ctl interact <X> <Y> <Z>
horizon-ctl messages 3
```

**Attendu :** une ligne en **barre d'action** : « Spawner : zombie · délai 200–800
ticks · 4 par vague · portée joueur 16 · proches max 6 · portée d'apparition 4 ».

Les seize modificateurs, main en main, un clic par objet. Preuve :
`horizon-ctl command "data get block <X> <Y> <Z> <clé>"` avant et après.

| Objet | Clé NBT | Effet d'un clic | Bornes |
| --- | --- | --- | --- |
| `minecraft:sugar` | `MinSpawnDelay` | −10 | 20 … 1600 |
| `minecraft:clock` | `MaxSpawnDelay` | −20 | 20 … 1600 |
| `minecraft:fermented_spider_eye` | `SpawnCount` | +2 | 1 … 16 |
| `minecraft:ghast_tear` | `MaxNearbyEntities` | +2 | 1 … 32 |
| `minecraft:prismarine_crystals` | `RequiredPlayerRange` | +4 | 1 … 48 |
| `minecraft:piston` | `SpawnRange` | +2 | 1 … 32 |
| `minecraft:pointed_dripstone` | `InitialHealth` | −0,05 | 0,20 … 1,0 |
| `minecraft:echo_shard` | `Echoing` | +1 | 0 … 3 |
| `minecraft:nether_star` | `IgnorePlayers` | 1 | booléen |
| `minecraft:soul_lantern` | `IgnoreLight` | 1 | booléen |
| `minecraft:conduit` | `IgnoreConditions` | 1 | booléen |
| `minecraft:chorus_fruit` | `NoAI` | 1 | booléen |
| n'importe quelle `*_wool` | `Silent` | 1 | booléen |
| `minecraft:comparator` | `RedstoneControl` | 1 | booléen |
| `minecraft:turtle_egg` | `Youthful` | 1 | booléen |
| `minecraft:campfire` | `Burning` | 1 | booléen |

Exemple complet (sucre) :

```
horizon-ctl command "give @s minecraft:sugar"
horizon-ctl select minecraft:sugar
horizon-ctl interact <X> <Y> <Z>         # face du haut du spawner
horizon-ctl wait 2
horizon-ctl command "data get block <X> <Y> <Z> MinSpawnDelay"
horizon-ctl inventory                    # le sucre n'est PAS consommé
```

**Attendu :** 200 → 190 (le spawner neuf vaut 200/800), l'objet reste en main, la
barre d'action réaffiche les stats à jour. Aux bornes : encore du sucre à 20 ne
change rien (`190 → 20 → 20`), et la barre d'action doit le montrer sans écrire
une valeur hors bornes.

Deux clics très rapides sur le même spawner doivent donner deux pas (pas un) :
c'est la preuve que deux jobs d'un même tick ne se volent pas leurs données.

### 4.4 L'inverseur quartz

```
horizon-ctl command "give @s minecraft:sugar"
horizon-ctl command "item replace entity @s weapon.offhand with minecraft:quartz"
horizon-ctl select minecraft:sugar
horizon-ctl interact <X> <Y> <Z>
horizon-ctl command "data get block <X> <Y> <Z> MinSpawnDelay"
```

**Attendu :** le délai **monte** de 10 au lieu de descendre (le quartz défait le
pas). Avec un objet à flag (`nether_star`) et du quartz en secondaire, le flag
reste à 0 : un flag ne s'éteint jamais.

**Limite connue à noter dans le compte rendu :** l'API 415 ne donne pas la main
secondaire, donc l'inversion dépend de l'événement de clic que le jeu envoie pour
la seconde main. Si le quartz en secondaire n'est pas vu, noter « non
reproductible : voir la limite du README », ne pas le compter comme un échec du
reste.

### 4.5 Les cas limites de la sonde (fiabilité)

1. **Deux joueurs cliquent le même spawner au même tick.** Ouvrir le monde en
   LAN (`horizon-ctl lan open 25565 commands`), faire entrer un second client,
   puis cliquer les deux en même temps sur le même bloc avec du sucre. Attendu :
   deux pas appliqués (−20), aucune erreur, aucune donnée perdue, et chaque
   joueur voit sa propre ligne.
2. **Spawner cassé entre le clic et le tick.**
   `horizon-ctl interact <X> <Y> <Z>` puis, tout de suite,
   `horizon-ctl command "setblock <X> <Y> <Z> minecraft:stone"`. Attendu :
   aucune ligne d'erreur dans `horizon-ctl logs`, rien n'est écrit dans la pierre
   (`data get block` répond que le bloc n'a pas de block entity), et un spawner
   posé ensuite au même endroit ne reçoit **pas** les anciennes données.
3. **Chunk déchargé.** Se placer à la limite de la distance de simulation et
   cliquer un spawner : la lecture échoue sans bruit (pas de
   `That position is not loaded` dans les logs), l'action bar reste vide.
4. **Sonde orpheline.** Après les tests 1 à 3 :
   `horizon-ctl entities 8` ne doit montrer **aucun** objet `minecraft:stone`
   au sol dans le spawner (les sondes sont tuées dans le tick, et balayées).

### 4.6 Modificateurs retirés puis reposés

Refaire 4.1 avec un spawner modifié (sucre ×4, laine, écho) : le ramasser et le
reposer doit rendre **les réglages modifiés**, pas ceux d'origine. C'est la
preuve que le bloc entier voyage avec l'objet.

---

## 5. Clôture

```
horizon-ctl logs --lines 400
horizon-ctl events --limit 50
horizon-ctl save
```

À noter dans le compte rendu, pour chaque point de ce document :

- **OK** avec la preuve (la sortie JSON ou la capture) ;
- **KO** avec la ligne exacte de log ou de sortie ;
- **non testable** avec la raison (aléatoire trop rare, deux clients
  indisponibles, LAN impossible sur cette machine…).

Les limites connues du portage (et donc pas des bugs) sont listées dans les
sections *Missing* du `README.md` : pas d'écran de spawner, pas de table de
sockets, Quanta et Arcana non appliqués, pas d'enchantements au-dessus du plafond
vanilla, affixes qui remplacent les modificateurs de base, pas de génération de
minerai.
