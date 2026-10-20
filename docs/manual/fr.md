# Premiers pas
Heron Brush est un éditeur de niveaux à base de brushes. Vous construisez des niveaux à partir de brushes convexes, vous peignez et texturez leurs faces, placez des entités, animez portes et plateformes, puis exportez le résultat vers un moteur de jeu. L’éditeur propose aussi des ressources intégrées, des formes paramétriques, des liaisons en direct avec les éditeurs des moteurs, des sessions d’édition partagées et le rendu d’images fixes. Ce manuel est fourni avec l’application et fonctionne hors ligne.

Sous macOS, `Cmd` fonctionne partout où ce manuel indique `Ctrl`, et Option correspond à Alt.

## L’écran d’accueil
Quand Heron Brush démarre sans carte, l’écran d’accueil s’ouvre. À gauche se trouvent **Continuer** (la dernière carte ouverte), **Nouvelle carte** et **Parcourir** pour ouvrir un `.map` existant, ainsi que **Jeu**, **Manuel** et **Préférences**. À droite s’affiche la liste des cartes récentes avec leurs miniatures, suivie des **Exemples**. L’étoile (**Épingler en haut**) fixe une carte en tête de liste ; la corbeille (**Mettre à la corbeille**) envoie le fichier dans la corbeille du système, d’où il peut être restauré. Les flèches du clavier permettent de parcourir l’écran et Entrée ouvre l’élément qui a le focus. Fichier → Accueil rouvre cet écran à tout moment.

## Votre première carte
1. Choisissez **Nouvelle carte** sur l’écran d’accueil, ou Fichier → Nouvelle carte (`Ctrl+N`).
2. Prenez l’outil Brush (`B`) dans la barre d’outils et faites glisser dans une vue 2D pour dessiner la forme choisie. Vérifiez sa profondeur dans une autre vue 2D.
3. Sélectionnez le brush et déplacez-le en le faisant glisser. L’onglet **Face** permet de lui donner un matériau ou une couleur.
4. Enregistrez la carte source avec `Ctrl+S`. Quand elle est prête pour un jeu, ouvrez Fichier → Exporter et écrivez une scène pour votre moteur.

## Fichiers et enregistrement
Le fichier `.map` est votre source modifiable. Enregistrez souvent : l’export, les liaisons en direct et le rendu n’enregistrent jamais la carte à votre place. Fichier → Enregistrer sous (`Ctrl+Shift+S`) ajoute l’extension `.map` si vous l’omettez. Fichier → Revenir à la carte enregistrée abandonne les modifications non enregistrées et recharge le fichier. Si vous fermez une fenêtre ou ouvrez une autre carte alors que des modifications ne sont pas enregistrées, Heron Brush vous demande s’il faut **Enregistrer**, **Abandonner** ou **Annuler**.

Tant qu’une carte déjà enregistrée contient des modifications non enregistrées, des sauvegardes numérotées sont écrites toutes les dix minutes dans un dossier `autosave` à côté de la carte. Les 50 plus récentes sont conservées.

Si vous ouvrez une carte alors que Heron Brush est déjà lancé, elle est transmise à la fenêtre existante, qui passe au premier plan. Utilisez `--new-instance` en ligne de commande pour obtenir une fenêtre séparée (voir Ligne de commande).

# L’interface
La fenêtre principale comporte une barre de titre avec les menus, une barre d’outils en dessous, les vues de la carte, l’inspecteur à droite et une barre d’état en bas.

## Barre de titre et menus
La barre de titre affiche le logo, les menus, le nom de la carte au centre et les boutons de la fenêtre. Sous macOS, les boutons rouge, jaune et vert habituels se trouvent à gauche. Faites glisser la barre de titre pour déplacer la fenêtre et double-cliquez dessus pour l’agrandir. Lorsqu’une tâche tourne en arrière-plan, un indicateur apparaît vers la fin de la barre : une liaison en direct avec un moteur, une session en direct ou **MCP actif**. Cliquez sur un indicateur pour arrêter la liaison en direct ou ouvrir sa fenêtre.

- **Fichier** : Accueil, Nouvelle carte (`Ctrl+N`), Ouvrir une carte (`Ctrl+O`), Enregistrer (`Ctrl+S`), Enregistrer sous (`Ctrl+Shift+S`), Revenir à la carte enregistrée, Exporter, Rendre une image, Plugins et Préférences (`Ctrl+,`).
- **Modifier** : Annuler l'action (`Ctrl+Z`), Rétablir (`Ctrl+Shift+Z`), Couper, Copier, Coller, Dupliquer (`Ctrl+D`), Supprimer, Tout sélectionner (`Ctrl+A`), Répéter les dernières commandes (`Ctrl+R`) et Oublier les commandes répétables.
- **Vue** : Vue 3D dans une fenêtre séparée, Aller à, Aligner sur la grille (`Alt+0`), Afficher la grille (`0`), Aperçu des animations et Afficher les faces omises.
- **Géométrie** : Insérer du texte 3D, Joindre le coin, Convertir les faces en surfaces courbes, Sortir la sélection de son entité et Synchroniser les groupes liés.
- **Animation** : Animation par images clés, Aperçu des animations et Placer la caméra depuis la vue 3D (`Ctrl+Alt+0`).
- **Carte** : Charger un fichier de points, Recharger le fichier de points et Décharger le fichier de points ; Charger un fichier de portails, Recharger le fichier de portails et Décharger le fichier de portails. Ils affichent les traces de fuite et les portails écrits par les compilateurs de cartes.
- **Collaboration** : Session en direct et Démarrer le serveur pour un agent (MCP).
- **Aide** : Manuel et À propos.

## La barre d’outils
La barre d’outils regroupe tous les outils. Le premier groupe sert en permanence : Sélectionner, Brush, Découpe, Sommet, Arête, Face, Rotation et Échelle. Le second contient Cisaillement, Assembler, Balayage, Point de contrôle, Arrondi et Peindre. Survolez un bouton pour voir son nom et son raccourci. Cliquer de nouveau sur l’outil actif revient à Sélectionner.

Après les outils viennent **Inverser horizontalement** et **Inverser verticalement**, puis deux interrupteurs : **Verrouiller** conserve le placage du matériau quand un brush est déplacé, et **UV** le conserve quand un brush pivote (`U` l’active ou le désactive). Suivent les réglages propres à l’outil actif : l’outil Brush affiche la forme et ses options, et l’outil Arrondi son rayon, ses faces et ses coins avec **Appliquer**. À l’extrémité droite se trouvent le nom de l’outil, la taille de la grille (**Grille 16**, etc. ; cliquez pour en choisir une autre) et le menu **Réglages de la grille** avec Aligner sur la grille, Afficher la grille, Aperçu des animations, Afficher les faces omises et Animation par images clés.

## Vues de la carte
Il existe quatre vues : la vue 3D et trois vues planes, Dessus (XY), Avant (XZ) et Côté (YZ). Préférences → Vue → Disposition propose Une vue, Deux vues, Trois vues ou Quatre vues. Faites glisser les séparateurs pour redimensionner les panneaux. Un clic dans une vue lui donne le clavier. `Space` fait passer un panneau qui affiche une seule vue à la vue suivante, et `Ctrl+Space` agrandit la vue qui a le clavier, puis rétablit la disposition.

Un clic droit sans glisser dans n’importe quelle vue ouvre un menu contextuel pour la sélection et pour ce qui se trouve sous le pointeur. Il permet de grouper, de gérer les calques et les groupes liés, de créer des entités, de placer des brushes dans une entité ou de les en sortir, de copier le nom d’un matériau et de l’afficher dans le navigateur de matériaux.

## L’inspecteur
L’inspecteur, à droite, comporte quatre onglets. Faites glisser son bord gauche pour modifier sa largeur.

- **Carte** (`Ctrl+1`) : Calques, Groupes, Plugins, Problèmes, filtres de la section Vue et Étiquettes.
- **Entité** (`Ctrl+2`) : le navigateur d’entités, les définitions d’entités, les propriétés de l’entité sélectionnée et, pour les boutons et les déclencheurs, Déclencheur d’animation.
- **Face** (`Ctrl+3`) : UV, Matériaux, Couleur, Balayage, Arrondi, Surface courbe et Retrait et biseau. Chaque section peut être repliée.
- **Ressources** : la bibliothèque de modèles de brushes réutilisables.

`Ctrl+5` masque ou affiche l’inspecteur et `Ctrl+Alt+T` masque ou affiche la barre d’outils, ce qui laisse plus de place aux vues.

## La barre d’état et le panneau de sortie
La barre d’état, en bas, affiche le dernier message de l’éditeur. Les erreurs sont en rouge. Cliquez sur la barre, ou appuyez sur `Ctrl+4`, pour ouvrir le panneau de sortie avec l’historique complet des messages. La plupart des commandes y indiquent ce qu’elles ont fait, par exemple combien de brushes ont été découpés.

## Boîtes de dialogue et apparence
Les boîtes de dialogue comme Préférences, Exporter et Rendre une image s’ouvrent dans leur propre fenêtre. Préférences → Vue → Palette de couleurs propose Suivre le système, Clair ou Sombre. **Opacité des panneaux (%)** sur la page Vue d’ensemble rend les panneaux transparents au-dessus des vues. La fenêtre dessine ses propres boutons dans le style de votre système : natifs sous macOS, style Windows 11 sous Windows et style GNOME sous Linux.

# Navigation et clavier
## Se déplacer dans la vue 3D
Maintenez le bouton droit de la souris et faites glisser pour regarder autour de vous. Faites glisser avec le bouton central pour vous déplacer latéralement. La molette avance et recule. `W`, `A`, `S` et `D` volent vers l’avant, la gauche, l’arrière et la droite, `Q` monte et `X` descend ; maintenez `Shift` pour des pas plus grands. Lorsque **Alt + glisser avec le bouton central déplace la caméra** est activé dans Préférences → Souris, Alt avec le bouton central se déplace dans l’axe de la vue. Les vitesses, les inversions et le champ de vision se règlent dans Préférences → Souris.

## Se déplacer dans les vues 2D
Faites glisser avec le bouton central pour vous déplacer et utilisez la molette pour zoomer autour du pointeur. Déplacer ou zoomer une vue plane garde les autres alignées ; **Lier les vues 2D** dans Préférences → Souris contrôle ce comportement. `Ctrl+Alt+Z` réinitialise le zoom et `Ctrl+Shift+U` ramène les vues planes à l’origine.

`Ctrl+U` cadre la sélection dans toutes les vues. Vue → Aller à (`Ctrl+Alt+M`) déplace la caméra vers des coordonnées saisies, sélectionne ce qui se trouve à une ligne donnée du fichier de carte ou déplace la sélection d’une valeur saisie.

## Grille et magnétisme
`+` et `-` changent la taille de la grille, et les touches numériques `1` à `9` choisissent directement une taille, de 1 à 256 unités. `0` ou `#` affiche ou masque la grille, et `Alt+0` active ou désactive le magnétisme. `S` aligne les sommets des brushes sélectionnés sur la grille actuelle, `Ctrl+Shift+V` les arrondit à l’unité entière et `Ctrl+Alt+Shift+V` les aligne sur la grille.

## Raccourcis courants
- `Ctrl+S` : enregistrer la carte. `Ctrl+Shift+S` : enregistrer sous.
- `Ctrl+Z` : annuler. `Ctrl+Shift+Z` ou `Ctrl+Y` : rétablir.
- `Ctrl+C`, `Ctrl+X`, `Ctrl+V` : copier, couper et coller. Le collage place les objets sous le pointeur ; `Ctrl+Alt+V` conserve leurs coordonnées d’origine. Avec des faces sélectionnées, la copie prend leur matériau et leurs réglages UV.
- `Ctrl+D` : dupliquer. `Ctrl+Shift+D` : duplicata lié d’un groupe.
- `Delete` ou `Backspace` : supprimer la sélection.
- `Esc` : annuler le glissement en cours, revenir d’une étape dans l’outil actif, revenir à Sélectionner, et enfin vider la sélection. `Shift+Esc` revient directement à Sélectionner.
- `Ctrl+R` : répéter les dernières commandes répétables. `Ctrl+Shift+R` : les oublier.
- `Ctrl+Alt+0` ou `Ctrl+Alt+Num0` : placer la caméra de rendu à la position de la vue 3D.
- `F5` : recharger les matériaux. `F6` : recharger les définitions d’entités.

## Raccourcis de sélection
- `Ctrl+A` : tout sélectionner, ou toutes les poignées lorsqu’un outil de sommets, d’arêtes ou de faces est actif. `Ctrl+Shift+A` : ne rien sélectionner. `Ctrl+Alt+A` : inverser la sélection.
- `Ctrl+B` : sélectionner les autres objets de la même entité ou du même groupe.
- `Ctrl+T` : sélectionner tout ce que touchent les brushes sélectionnés. `Ctrl+E` : sélectionner tout ce qu’ils contiennent. `Ctrl+Shift+E` : sélectionner tout ce qui se trouve dans leur colonne selon l’axe de la vue. Les brushes servant à la sélection sont consommés.
- `Ctrl+I` : isoler la sélection. `Ctrl+Alt+I` : la masquer. `Ctrl+Shift+I` : tout réafficher.
- `Ctrl+G` : grouper. `Ctrl+Shift+G` : dégrouper.

## Raccourcis de transformation
- Les flèches et `Page Up` / `Page Down` déplacent la sélection d’un pas de grille dans la direction indiquée, dans la vue active. Avec `Ctrl`, elles dupliquent en déplaçant, et avec `Alt`, elles font pivoter.
- `Alt+X`, `Alt+Y`, `Alt+Z` : rotation d’un quart de tour autour de cet axe ; ajoutez `Shift` pour l’autre sens.
- `Ctrl+F` : retourner selon la vue ; `Ctrl+Alt+F` : retourner de haut en bas. `Ctrl+Shift+X` et `Ctrl+Shift+Y` font une symétrie selon X ou Y.
- `Insert` : placer une entité ponctuelle au centre de la sélection.
- `Tab` : avec un brush sélectionné, passer de Sélectionner → Sommet → Arête → Face → Sélectionner.

## Le jeu de raccourcis Blender
Préférences → Clavier → Raccourcis clavier propose Heron Brush (par défaut, utilisé dans tout ce manuel) et Blender. Avec Blender : `G` sélectionne et déplace, `R` fait pivoter, `S` met à l’échelle, `E` balaie, `1`, `2` et `3` choisissent les outils de sommets, d’arêtes et de faces, `K` découpe, `B` sélectionne, `A` sélectionne tout et `Alt+A` vide la sélection, `X` supprime, `Shift+D` duplique, `H` masque et `Alt+H` réaffiche tout, et `Ctrl+Shift+Z` rétablit. Tout le reste est identique.

# Brushes et outils de modélisation
Les brushes sont des solides convexes définis par des plans. Construisez les objets complexes à partir de plusieurs brushes simples plutôt que d’essayer de rendre un brush concave. Limitez le nombre de brushes là où une forme simple suffit.

## Dessiner des brushes
Choisissez l’outil Brush (`B`) et faites glisser dans une vue. La barre d’outils affiche **Forme à dessiner** : Pavé, Escalier, Arche, Cylindre, Cône, Sphéroïde (UV), Sphéroïde (icosaèdre) et Insérer du texte 3D. `Alt+1` à `Alt+7` choisissent directement une forme. Selon la forme, la barre affiche **Évider** et **Épaisseur :**, **Hauteur des marches :** et **Direction :** pour les escaliers, **Axe :**, **Côtés :** et le mode de cercle pour les formes rondes, **Anneaux :** pour les sphères et **Précision :** pour l’icosaèdre. Réglez les options avant de faire glisser. Vérifiez le résultat dans plusieurs vues.

## Sélectionner
Cliquez sur un brush pour le sélectionner, et `Ctrl`-clic pour l’ajouter à la sélection. Un glissement depuis un espace vide trace un rectangle de sélection. Dans la vue 3D, `Shift`-clic sélectionne une seule face et `Ctrl+Shift`-clic ajoute d’autres faces. Faire glisser un brush sélectionné le déplace ; faire glisser un brush non sélectionné sélectionne et déplace uniquement celui-ci.

Dans la vue 3D, faites `Shift`-glisser une face d’un brush sélectionné pour l’extruder. `Alt`-cliquez sur une autre face pour y copier le matériau et l’alignement de la face sélectionnée ; `Alt+Ctrl` ne copie que le matériau et `Alt+Shift` copie la rotation.

## Outils de modelage
- **Sélectionner** (`Esc`) : sélectionner et déplacer.
- **Sommet** (`V`), **Arête** (`E`), **Face** (`F`) : faire glisser des sommets, des arêtes ou des faces. `Shift`-glisser depuis le milieu d’une arête ou d’une face y ajoute un sommet. `Delete` supprime les sommets choisis.
- **Rotation** (`R`), **Échelle** (`T`), **Cisaillement** (`G`) : transformer la sélection à l’aide de poignées.
- **Découpe** (`C`) : cliquez deux ou trois points pour définir un plan de coupe. `Enter` coupe, `Ctrl+Enter` choisit le côté à conserver, et `Backspace` ou `Esc` supprime le dernier point.
- **Assembler** (`N`) : placez des points autour d’un volume et appuyez sur `Enter` pour en faire un brush convexe. `Backspace` supprime le dernier point.
- **Balayage** (`Y`) : voir la section Balayage ci-dessous.
- **Point de contrôle** (`P`) : remodeler la grille de contrôle d’une surface courbe.
- **Arrondi** (`O`) : cliquez sur une arête pour l’arrondir, ou choisissez des arêtes ou des sommets avec leurs outils et appuyez sur `Enter`. Plus de faces donnent un résultat plus lisse, mais plus de géométrie.
- **Peindre** (`K`) : voir Matériaux et UV.

## Combiner des brushes
- `Ctrl+J` : fusionner les brushes sélectionnés en leur enveloppe convexe.
- `Ctrl+K` : soustraire les brushes sélectionnés des brushes qu’ils touchent.
- `Ctrl+Shift+K` : évider la sélection, avec des parois d’un pas de grille d’épaisseur.
- `Ctrl+L` : intersection de la sélection.

## Balayage, arrondi, surface courbe et biseau
**Balayage** crée de la géométrie le long d’une trajectoire à partir des faces sélectionnées. Sélectionnez les faces (dans l’outil Balayage, par `Alt`-clic), puis réglez **Déplacement**, **Rotation** (axe et angle), **Échelle**, **Étapes** et **Trajectoire** (alignement Entier ou Libre ; Arc, Droite ou Courbe en S) dans la section Balayage de l’onglet Face, et appuyez sur **Balayage**. La ligne sous les contrôles indique combien de brushes seront créés.

La section **Arrondi** arrondit les arêtes choisies avec **Rayon**, **Faces** et **Coins** (En onglet ou Arrondi effectué) ; une seule face donne un chanfrein.

**Surface courbe** affiche les valeurs **Colonnes** et **Lignes** de la surface courbe sélectionnée, avec des boutons pour en ajouter ou en retirer une paire. Géométrie → Convertir les faces en surfaces courbes (`Ctrl+P`) crée des surfaces courbes à partir des faces sélectionnées.

**Retrait et biseau** agit sur les faces sélectionnées : **Retrait** rétrécit la face depuis ses bords et **Profondeur** la pousse vers l’extérieur (ou vers l’intérieur si la valeur est négative). **Biseauter les faces** applique les deux ; le troisième bouton ajoute un panneau ou creuse un renfoncement, selon le signe de la profondeur. **Joindre le coin** ouvre une boîte de dialogue qui relie deux brushes par un coin en onglet ou arrondi, avec un aperçu.

## Texte 3D
Géométrie → Insérer du texte 3D (aussi la dernière entrée de la liste des formes de l’outil Brush) crée des brushes à partir d’un texte. Il utilise Heron Block, une police polygonale intégrée aux coins coupés, à chasse proportionnelle et avec les accents tchèques ; les minuscules prennent la forme des capitales. Saisissez le **Texte**, réglez **Hauteur** (la hauteur des capitales ; les accents dépassent au-dessus) et **Profondeur**, observez l’aperçu en direct et appuyez sur **Insérer**.

Si une opération ne peut pas produire de géométrie valide, ajustez ses dimensions, ou annulez et simplifiez la forme.

## Réseaux
Géométrie → Réseau copie la sélection et montre les copies dans la carte pendant que vous modifiez les valeurs. Les nombres incluent l'original, et un réseau crée au plus 1024 copies.

- **Rangée** décale chaque copie de **Pas X**, **Pas Y** et **Pas Z** par rapport à la précédente et la tourne de **Rotation par copie (degrés)** autour de l'axe choisi.
- **Grille** remplit **Nombre X**, **Nombre Y** et **Nombre Z** éléments, espacés de **Pas X**, **Pas Y** et **Pas Z**.
- **Anneau** répartit les éléments sur **Angle (degrés)** autour d'un centre situé à **Rayon (unités)** de la sélection. Avec **Orienter vers le centre**, chaque copie tourne avec l'anneau.
- **Dispersion** place les copies à des endroits aléatoires dans le rayon, en travers de l'axe choisi. La même **Graine** donne la même disposition, et **Rotation aléatoire** tourne chaque copie.

Quand la sélection est un groupe entier, **Copies liées** fait de chaque copie un duplicata lié : une modification de l'une atteint toutes les autres. **Appliquer** garde les copies et les sélectionne avec l'original ; **Annuler** les supprime. Le réseau entier s'annule en une seule étape.

## Sculpter le terrain
Le terrain est fait de nombreux petits brushes dont le pinceau de sculpture monte et descend le dessus. Sélectionnez une boîte plate et choisissez Géométrie → Découper en colonnes : la boîte est découpée en colonnes triangulaires larges d'un pas de grille, placées dans un groupe nommé Terrain. Le dessus d'un triangle est toujours plan, les colonnes peuvent donc former des crêtes et des vallées.

Choisissez l'outil **Sculpter** (`J`) et faites glisser sur le terrain dans la vue 3D. L'anneau orange montre le pinceau. Sans sélection, le pinceau modifie tous les brushes situés dessous ; avec une sélection, seulement ceux sélectionnés. La barre en haut de la fenêtre choisit l'effet d'un trait :

- **Élever** et **Abaisser** montent ou descendent la surface, surtout au centre de l'anneau et pas du tout au bord. Maintenez `Shift` pour les inverser.
- **Lisser** atténue les bosses.
- **Aplanir** ramène la surface à la hauteur où le trait a commencé.

Les deux nombres à côté des modes sont **Rayon (unités)** et **Force (%)**. Seuls les coins du dessus bougent, à la verticale ; le dessous reste en place et une colonne garde au moins une unité d'épaisseur. Un trait s'annule en une seule étape. Dans les vues 2D, l'outil sélectionne comme l'outil Sélection.

# Matériaux et UV
## Charger et appliquer des matériaux
Ouvrez la section **Matériaux** de l’onglet Face. **Charger un dossier** ajoute un dossier d’images et **Charger un .wad** ajoute un paquet de textures ; tous deux apparaissent sous **Collections**, où la croix en retire un. Tapez dans **Filtrer les matériaux** pour restreindre la liste. Cliquez sur un matériau pour le rendre courant, puis **Appliquer à la sélection** le pose sur toutes les faces sélectionnées. **Sélectionner** propose Sélectionner les brushes l'utilisant et Sélectionner les faces l'utilisant. **Remplacer** substitue un matériau à un autre dans toute la carte ou seulement dans la sélection.

L’outil **Peindre** (`K`) applique le matériau courant à la face sous le pointeur : un clic peint une face, un double-clic peint tout le brush, et `Shift`-clic prélève le matériau de la face comme matériau courant, à la manière d’une pipette.

## L’éditeur UV
La section **UV** de l’onglet Face affiche le matériau et le contour de la face sélectionnée. Commandes de la souris sur le canevas :
- La molette zoome autour du pointeur. Faites glisser avec le bouton droit ou central pour vous déplacer. Double-cliquez avec le bouton droit pour recadrer la face ; sélectionner une autre face la cadre automatiquement.
- Glisser avec le bouton gauche déplace le matériau tandis que le contour de la face reste en place. L’anneau fait pivoter le placage, les poignées d’angle le mettent à l’échelle et la poignée centrale fixe l’origine de ces transformations.
- `Alt`-glisser avec le bouton gauche sur un bord de tuile applique un cisaillement, tout comme `Shift`-glisser avec le bouton central.
- Chaque glissement forme une seule étape d’annulation, et `Esc` annule un glissement en cours.

Sous le canevas se trouvent **Décalage**, **Échelle**, **Angle** et, si le jeu les utilise, **Indicateurs de contenu**, **Indicateurs de surface** et **Valeur de surface**. Les boutons sont **Réinitialiser**, **Aligner sur le monde**, **Ajuster**, **Inverser H**, **Inverser V** et les deux rotations de 90°. **Aligner** oriente le matériau le long de l’une des arêtes de la face (appuyez de nouveau pour l’arête suivante, avec `Shift` pour la précédente). Les quatre boutons de justification plaquent le matériau contre un bord. **Ajuster U** et **Ajuster V** mettent à l’échelle pour qu’un nombre entier de répétitions couvre la face ; appuyez de nouveau pour passer au nombre suivant, et maintenez `Ctrl` pour conserver la subdivision d’une trim sheet.

## Touches UV
Avec des faces sélectionnées, les flèches déplacent le matériau, `[` et `]` le font pivoter de 15°, `,` et `.` divisent par deux ou doublent son échelle, `Ctrl+F` et `Ctrl+Alt+F` le retournent, `Shift+R` le réinitialise, `Alt+Shift+R` le réaligne sur les axes du monde et `Ctrl+Shift+F` l’ajuste à chaque face.

**Verrouiller**, dans la barre d’outils, conserve le placage quand les brushes sont déplacés, et **UV** (`U`) le conserve quand ils pivotent.

## Matériaux d’outils
- skip : retire la face marquée des exports vers les moteurs.
- clip : crée une collision sans géométrie visible.
- origin : marque le pivot d’une entité brush.

Ces noms peuvent être modifiés dans la carte **Matériaux d'outils** de la fenêtre Exporter. Utilisez skip pour les faces que vous savez cachées.

## Omettre des faces à l’export
Sélectionnez une face et activez **Omettre la face à l’export** dans la section UV. Des hachures orange la signalent dans l’aperçu UV. Son matériau et sa peinture restent enregistrés, et le brush reste fermé et modifiable. Les maillages de rendu exportés laissent cette face de côté, tandis que la collision est conservée. Omettre un quad d’un cube produit deux triangles de rendu en moins. Cela ne rend pas les faces double face et ne fusionne pas les maillages.

**Omettre les faces couvertes** marque les faces entièrement couvertes par des brushes statiques, y compris les faces identiques en contact et les brushes enfermés. Il agit sur les brushes sélectionnés, ou sur toute la géométrie du monde visible et non verrouillée si rien n’est sélectionné. Les entités brush, les matériaux transparents et les brushes ayant déjà des faces omises sont ignorés. Les faces partiellement couvertes ne sont pas découpées. Une seule annulation rétablit tout. Vue → Afficher les faces omises masque ou affiche ces faces dans les vues de l’éditeur ; la sélection utilise toujours le brush entier.

# Couleurs et dégradés
La section **Couleur** de l’onglet Face peint les faces avec des couleurs unies ou des dégradés par-dessus leurs matériaux. Elle est utile pour maquetter un niveau et pour les styles graphiques stylisés. Les faces peintes gardent leurs vraies couleurs dans les vues ; la sélection est signalée par son contour.

## Palette et couleur de mélange
Cliquez sur l’une des douze pastilles de la palette pour en peindre les faces sélectionnées, ou les brushes sélectionnés entiers. **Opacité** règle l’opacité de la prochaine peinture, en pourcentage. **Mélanger** comporte un bouton de couleur qui ouvre le sélecteur de couleur ; faites glisser dans le sélecteur et la sélection suit en direct, ou appuyez sur **Peindre** pour appliquer la couleur de mélange en une seule étape. **Retour au matériau** retire la peinture pour que la face montre de nouveau son matériau. La ligne en haut de la section indique ce qui est sélectionné et ce qui y est appliqué.

## Le sélecteur de couleur
Chaque bouton de couleur de l’éditeur ouvre le même sélecteur. Faites glisser dans le carré pour choisir la saturation et la luminosité, et dans la bande à côté pour choisir la teinte. En dessous se trouvent les valeurs **R**, **G** et **B** et, quand la couleur en a une, un curseur **Opacité**. Le champ hexadécimal accepte `#RRGGBB` ou `#RRGGBBAA` ; il devient rouge tant que le texte n’est pas une couleur valide. **Copier** place le code hexadécimal dans le presse-papiers. Cliquez en dehors du sélecteur pour le fermer.

## Dégradés
**Dégradé** choisit Dégradé linéaire ou Dégradé radial, et le bouton à côté inverse l’ordre des couleurs. Chaque point a un bouton de couleur et **Position (%)**, et les points intermédiaires ont un bouton de suppression. **Ajouter un point de couleur** ajoute un point dans le plus grand intervalle ; un dégradé peut avoir jusqu’à huit points. **Angle du dégradé** fixe la direction d’un dégradé linéaire, avec les boutons **+45°** et **−45°**.

Sous l’aperçu se trouve un rail avec un repère pour chaque point. Cliquez sur le rail pour ajouter un point, cliquez sur un repère pour ouvrir son sélecteur, faites glisser un repère pour le déplacer, et faites un clic droit sur un repère pour le supprimer. Les trois pastilles sous le rail sont des préréglages : Dégradé chaud, Dégradé froid et Dégradé de gris. Les modifications d’un dégradé s’appliquent en direct à la sélection.

La direction d’un dégradé compte : orientez-le selon la pente d’un toit ou le long d’un objet courbe, et non à travers des faces sans rapport entre elles. Vérifiez le modèle des deux côtés après la peinture.

## Matériau de surface
**Matériau de surface** règle **Rugosité** (0 est brillant, 100 est mat), **Métallicité** (0 pour le bois, la pierre ou le plastique, 100 pour le métal) et **Émission** (de 0 à 8 ; la surface brille de sa propre couleur, dégradés compris, mais n’éclaire pas les objets voisins). Les modifications s’appliquent en direct aux faces ou aux brushes sélectionnés. **Utiliser la surface par défaut** retire ces réglages. Toutes les modifications peuvent être annulées.

Les effets de surface apparaissent dans la vue 3D éclairée et dans les rendus de caméra. Les affichages plat et fil de fer omettent volontairement les reflets spéculaires. Le halo lumineux (bloom) peut ajouter une lueur autour des surfaces émissives. Godot et glTF/GLB reçoivent les valeurs du matériau, et l’importateur Unity convertit la rugosité en lissé (smoothness). Voir `docs/surface-materials.md` pour les limites de chaque cible.

# Ressources, calques et groupes
## La bibliothèque de ressources
Ouvrez l’onglet **Ressources**. Ajoutez un dossier de modèles `.map` avec **Ajouter un dossier** ; la bibliothèque intégrée est toujours présente. Tapez dans **Filtrer les ressources** pour chercher. Cliquez sur un modèle pour le sélectionner, puis double-cliquez dessus, appuyez sur `Enter` ou cliquez sur **Insérer** pour l’ajouter là où se trouvait le pointeur en dernier. Vous pouvez aussi faire glisser un modèle dans une vue. Les brushes d’une ressource sont insérés sous forme d’un groupe, afin que le modèle se déplace d’un bloc.

**Enregistrer la sélection comme ressource** écrit les brushes sélectionnés dans la bibliothèque sous forme de `.map`. Les ressources de la bibliothèque sont des brushes ordinaires et modifiables, pas des maillages importés.

## Calques
La liste **Calques** de l’onglet Carte organise les grandes zones. Saisissez un nom et cliquez sur **Ajouter** pour créer un calque, ou appuyez sur `Enter` pour renommer le calque courant. Chaque calque peut être masqué, verrouillé ou exclu des exports. **Déplacer ici** déplace la sélection dans le calque courant. Le menu contextuel des vues propose aussi Déplacer vers un calque, Définir comme calque actif, Masquer les calques et Isoler les calques.

## Groupes et groupes liés
`Ctrl+G` groupe la sélection et `Ctrl+Shift+G` la dégroupe. La section **Groupes** de l’onglet Carte liste les groupes et propose **Ouvrir**, **Renommer**, **Fusionner**, **Dégrouper**, **Retirer du groupe** et **Extraire**. Un groupe lié est un ensemble de copies qui partagent leurs modifications : `Ctrl+Shift+D` crée un duplicata lié, Géométrie → Synchroniser les groupes liés met les copies à jour, et **Délier** rend une copie indépendante. Dans l’onglet Entité, une propriété peut rester propre à une copie lors de la synchronisation des groupes liés.

## Une échelle commune
L’échelle d’export par défaut est de 32 unités de carte par mètre. Le repère de départ du joueur intégré mesure 64 unités de haut, soit une référence de 2 mètres. Comparez-y les portes, les véhicules, le mobilier et les plantes. Ne changez l’échelle d’export que délibérément et gardez les ressources apparentées cohérentes.

# Entités et gameplay
Les définitions d’entités courantes sont intégrées et toujours disponibles : lumières, points de départ du joueur, portes, boutons, plateformes, déclencheurs, objets statiques, sons et points de passage. Charger le FGD d’un projet avec **Définitions d'entités** dans l’onglet Entité remplace les classes portant le même nom et conserve les autres classes intégrées.

## Placer des entités
Dans le navigateur **Entités** de l’onglet Entité, tapez dans **Filtrer les classes** et choisissez une classe. Pour une classe ponctuelle comme une lumière, cliquez sur **Placer** ou double-cliquez sur la classe ; `Insert` en place une au centre de la sélection. Pour une classe brush comme une porte ou un déclencheur, sélectionnez les brushes et cliquez sur **Créer**. Les classes peuvent aussi être glissées dans une vue.

## Propriétés
La liste des propriétés modifie l’entité sélectionnée. Les entités de même classe sont modifiées ensemble. **Ajouter une propriété** ajoute une clé que la définition ne connaît peut-être pas ; **Valeurs par défaut** peut rétablir les valeurs par défaut des propriétés existantes ou ajouter celles qui manquent. Les propriétés définies par la classe mais absentes de l’entité sont affichées en grisé ; saisir une valeur les ajoute. Utilisez targetname et target pour relier des entités.

## Ce qu’un export peut faire
Les directives Godot créent des nœuds natifs, des corps de collision et des zones de déclenchement. Unity et Unreal prennent en charge les lumières courantes natives, et Unreal crée aussi des acteurs PlayerStart. Les autres données sont conservées autant que l’importateur de la cible le permet.

Une entité porte ne suffit pas à ajouter un comportement de jeu. Le mouvement, les réactions aux déclencheurs, les contrôleurs de joueur et les scripts du projet restent à faire dans le moteur. Defold exporte les propriétés des entités et les objets de collision ; Flax place les modèles et des entités génériques. Vérifiez le résultat dans votre moteur avant d’y bâtir le gameplay.

# Animation
Heron Brush peut prévisualiser des mouvements simples et éditer des animations par images clés d’entités brush, puis les exporter vers plusieurs moteurs.

## Aperçu des animations
Activez **Aperçu des animations** dans le menu Vue, le menu Animation, le menu Réglages de la grille ou la section Vue de l’onglet Carte. L’aperçu joue la rotation de func_rotate et le va-et-vient de func_shuttle lorsque start_on est activé, d’après les propriétés d’axe, de vitesse, de direction, de course et d’attente de l’entité, et il joue les clips par images clés réglés sur Lire au démarrage. Les ressources ventilateur et moulin à vent contiennent déjà des pièces en rotation.

Désactiver l’aperçu rétablit la pose d’origine. Un clic dans une vue arrête la lecture avant de sélectionner ou de modifier ; la navigation de la caméra continue de fonctionner. L’aperçu ne modifie ni la carte, ni l’historique d’annulation, ni les exports, et il n’exécute ni physique ni logique de jeu.

## L’éditeur d’images clés
Ouvrez-le avec Animation → Animation par images clés ou depuis le menu Réglages de la grille. Il s’ancre sous les vues, ou dans la fenêtre 3D quand celle-ci est séparée. L’icône de fenêtre de son en-tête le place dans sa propre fenêtre ; il conserve l’image courante et l’historique d’annulation. **Modifier l’animation** fait passer les vues à la pose animée et **Modifier la carte** revient à la géométrie au repos. En mode animation, chaque vue affiche **Modifier l’animation** et le numéro de l’image en bas.

Sélectionnez une entité brush, ou un brush ou un groupe de brushes, pour l’animer. Un groupe de brushes simple partage une seule animation et un seul pivot ; les groupes qui contiennent plusieurs entités demandent des animations séparées.

## Clips
Le sélecteur **Animation** passe d’un clip nommé à l’autre. Saisissez un nom dans **Nom de l’animation** et cliquez sur **Enregistrer sous** pour copier le clip courant sous ce nom ; **Supprimer** retire un clip. **Déplacer** crée un clip `translate` sans rotation. **Animations de porte** crée des clips fluides `open` et `close` avec une rotation de 90 degrés autour du pivot courant ; placez d’abord le pivot sur la charnière. **Lire au démarrage** fait jouer le clip au lancement du niveau ; le préréglage de porte le désactive.

## Transport et lecture
Les boutons de transport sont **Début**, **Clé précédente**, **Lecture**, **Clé suivante** et **Fin**. **Image** affiche et définit l’image courante, et **Durée (s)** fixe la longueur du clip. Le mode est **Une fois**, **Boucle** ou **Aller-retour**.

## La timeline
La timeline comporte une règle avec la tête de lecture, une piste pour **Position** (X, Y et Z animés ensemble) et une pour chaque angle de rotation, RX, RY et RZ. Cliquez ou faites glisser dans la règle pour déplacer la tête de lecture et prévisualiser cette pose dans toutes les vues. Faites glisser avec le bouton central pour faire défiler la timeline. Faites glisser les clés pour les déplacer dans le temps ; `Shift` ajoute à la sélection ; double-cliquez sur une piste pour insérer une clé ; `Ctrl`+molette zoome. **Courbes** affiche les courbes de valeurs à la place des pistes.

Les boutons au-dessus sont **Ajouter une clé**, **Supprimer**, **Copier**, **Coller** et **Dupliquer**. **Aligner sur les images** garde les clés sur des images entières et **Ajuster** affiche le clip entier. **Modifier le temps des clés** ouvre **Échelle temporelle**, **Décalage (images)**, **Échelle depuis la tête de lecture**, **Prolonger la durée** et **Modifier le temps de la sélection**.

L’inspecteur de clés à côté de la timeline modifie la clé sélectionnée : **Image**, **Valeur** (ou les trois coordonnées d’une clé de Position), l’interpolation **Linéaire**, **Constante** ou **Bézier**, **Pente entrante** et **Pente sortante**, et **Lisser les tangentes**.

## Touches dans l’éditeur
Quand la timeline a le clavier : `Space` lance ou met en pause la lecture, les flèches gauche et droite avancent d’une image, `Ctrl+A` sélectionne toutes les clés, `Ctrl+C`, `Ctrl+X` et `Ctrl+V` copient, coupent et collent des clés, `Ctrl+D` les duplique, `Delete` les supprime, `Ctrl+Z` annule et `Esc` annule un glissement. En dehors de la timeline, `Esc` quitte le mode animation. L’annulation laisse le zoom, la sélection et la tête de lecture là où ils étaient.

## Poser dans les vues
La rangée sous la timeline contient **Pelure d’oignon** avec un nombre et un **Espacement**, qui affiche les poses voisines en transparence ; les coordonnées du **Pivot** ; **Clés automatiques** ; et les outils de pose **Déplacer**, **Rotation** et **Pivot**.

- **Ajouter une clé** active Clés automatiques et met la lecture en pause. Avec Clés automatiques actif et **Déplacer** choisi, faites glisser un axe coloré de la poignée de l’entité sélectionnée dans la vue 3D pour enregistrer une clé de position à l’image courante.
- Dans une vue 2D, faites glisser le brush sélectionné ou un membre de son groupe pour enregistrer une clé de translation selon les deux axes visibles, avec magnétisme sur la grille. Clés automatiques n’a pas besoin d’être actif pour cela.
- Avec **Rotation**, faites tourner le brush dans une vue 2D autour de son repère de pivot orange. Dessus enregistre RZ, Avant RY et Côté RX, par pas d’angle de la grille.
- Avec **Pivot**, faites glisser le centre de rotation jaune dans n’importe quelle vue 2D. Il s’aligne sur la grille et conserve sa profondeur. Les clés et la géométrie au repos ne changent pas.

Le tracé cyan en 2D et en 3D montre la course du pivot. Cliquez sur un point du tracé pour sélectionner son instant et ses clés, ou faites-le glisser pour changer la position ; avec Clés automatiques actif, vous pouvez aussi faire glisser les points du tracé dans la vue 3D. Des étiquettes indiquent les numéros d’image et des segments en pointillés signalent les sauts. `Esc` annule n’importe lequel de ces glissements, et une seule annulation défait un glissement entier.

Les nouvelles animations commencent au repos avec des clés à l’instant zéro. Créez le mouvement en changeant la pose à une autre image. Pour un tracé en L bien net, placez une clé au départ, au coin et à l’arrivée, et utilisez Linéaire avec Aller-retour.

## Relier des boutons à des animations
Sélectionnez un func_button, un trigger_multiple ou un trigger_once. L’onglet Entité affiche **Déclencheur d’animation** au-dessus des propriétés. Choisissez une cible dans la liste, ou cliquez sur **Choisir la cible dans la vue** puis sur un objet animé dans n’importe quelle vue ; `Esc` annule le choix. Les cibles ou clips manquants sont signalés sous les contrôles. Les liens entre entités affichent des flèches vers leurs cibles lorsque l’affichage des liens est activé.

**Action** choisit l’effet de l’activation : **Lire** lance un clip nommé ou l’animation sélectionnée ; **Basculer** alterne `open` et `close` s’ils existent, sinon relance l’animation sélectionnée ; **Ouvrir** et **Fermer** lancent ces clips ; **Pause** fige la pose, **Reprendre** continue et **Arrêter** rétablit la pose d’origine. **Une seule fois** remplace la valeur par défaut de la classe. **Intervalle de répétition (s)** fixe le délai minimal entre deux activations ; il ne retarde pas la première.

**Tester le déclencheur** exécute l’action dans l’aperçu sans modifier la carte ni ajouter d’étapes d’annulation. **Réinitialiser l’aperçu** rétablit les poses d’origine et efface l’état « une seule fois » et les délais en cours. Cela ne simule ni les collisions du joueur ni les scripts du jeu.

## L’animation dans les exports
Godot exporte des ressources natives AnimationPlayer et AnimationLibrary, avec les courbes et les collisions en mouvement. Un CharacterBody3D qui entre dans la zone d’un bouton active sa cible. Les modifications apportées à ces clips dans Godot ne reviennent pas dans la carte et sont remplacées au prochain export.

L’export raylib ajoute `<map>_animation.h` avec un lecteur en C et `<map>_preview.c` avec un aperçu en caméra libre ; suivez `<map>_raylib.md` pour le compiler. Dans l’aperçu, `SPACE` met en pause, `R` relance les clips par défaut et `N` parcourt les clips nommés. Defold intègre la lecture dans le script de l’entité avec des messages `heron_*_animation` documentés. Le Build Level d’Unity ajoute `HeronBrushAnimation` et un corps cinématique. L’export Bevy inclut `heron_animation.rs` et des instructions. Godot, Unity, Defold, Bevy et raylib conservent les liens de déclenchement, l’option « une seule fois » et l’intervalle de répétition. Votre jeu doit encore enregistrer le joueur et relier ses propres boutons à l’API de chaque environnement d’exécution.

## L’exemple Motion lab
Ouvrez `motion_lab` depuis les Exemples de l’écran d’accueil. Il contient une plateforme sur un tracé en L, un ascenseur, une plateforme tournante et une porte à charnière avec des boutons. Ouvrez le `godot/project.godot` voisin dans Godot et appuyez sur `F5`. ZQSD (WASD) et la souris déplacent le joueur, Espace fait sauter, 1 à 4 mènent aux différentes stations et R relance la station courante. Montez sur un plot coloré pour activer sa cible. Le README fourni explique comment réexporter.

# Exporter vers un moteur
Fichier → Exporter ouvre la fenêtre Exporter. Les moteurs sont listés à gauche avec leurs logos : Godot, Defold, three.js, Bevy, raylib, Unity, Unreal Engine, Flax Engine, Cocos Creator, Wavefront .obj, glTF .gltf, GLB .glb, et enfin `.map`. La page de droite indique ce que la cible recevra. **Exporter** écrit l’export, **Vérifier** construit le niveau sans l’écrire et liste les problèmes, et **Fermer** ferme la fenêtre.

L’export est distinct de l’enregistrement : le `.map` reste votre source modifiable. Chaque contrôle enregistre les réglages de son moteur dès qu’il change, si bien que chaque moteur garde ses propres choix et que la fenêtre se rouvre comme vous l’avez laissée.

## Options communes
- **Échelle** : **Unités par mètre**, 32 par défaut.
- **Éléments à exporter** : **Géométrie de collision**.
- **Optimisations** : **Grouper les maillages statiques par cellules de 1024 unités**, **Supprimer les faces enfouies dans la géométrie**, **Fusionner les faces coplanaires** et **Fusionner les formes de collision**.
- **Chemins du projet** : l’emplacement du niveau et de ses ressources dans le projet.
- **Matériaux d'outils** : les noms des matériaux clip, skip et origin.
- **Plus** : des options propres à chaque moteur, comme **Nommer les nœuds d'après**, **Recréer les calques et groupes comme nœuds parents**, **Inverser l'ordre des sommets des triangles**, le retournement des coordonnées de texture et un second jeu d’UV pour l’éclairage précalculé avec **Texel de lightmap**.
- **Suivre les modifications de la carte** : **Réexporter à chaque enregistrement de la carte**. L’option devient disponible après un premier export vers le moteur, car c’est cet export qui indique où écrire.

Les cellules statiques répartissent les brushes du monde selon leur centre et les combinent par matériau dans chaque cellule de 1024 unités, ce qui permet aux moteurs d’éliminer (culling) de plus petites parties de la carte. La collision est conservée et les entités de gameplay restent séparées. Cela peut entraîner davantage d’appels de dessin qu’un maillage unique pour le monde ; l’option est donc désactivée par défaut. Les exports OBJ, glTF et GLB autonomes n’utilisent pas de cellules.

## Notes par cible
- Godot : les chemins `res://` des textures et des matériaux, la couche physique des déclencheurs et l’UV2 de lightmap. Choisissez un dossier d’exportation dans le projet Godot.
- Defold : le répertoire des ressources, le matériau du modèle et l’échantillonneur de texture, qui doit exister dans le matériau choisi.
- Unity : des maillages OBJ et un importateur pour l’éditeur ; exportez dans le dossier Assets du projet.
- Unreal Engine : un script d’import généré et un chemin de contenu, normalement /Game/<level>.
- Flax Engine : un chemin de contenu pour les modèles, relatif au dossier Content du projet.
- three.js, Bevy, raylib et Cocos Creator : des scènes ou du code pour ces environnements, avec des instructions à côté de l’export.
- glTF / GLB : une géométrie portable pour les outils compatibles ; GLB en est la forme binaire. OBJ : géométrie et matériaux pour les logiciels de modélisation.
- `.map` : **Exporter sous** choisit Valve 220, Standard, Quake 2 ou Quake 2 (Valve). La fenêtre indique ce que le dialecte choisi perdrait.

Lisez les avertissements après l’écriture, par exemple les textures manquantes. **Résultat de la vérification** les liste dans la fenêtre.

# Liaisons en direct : Godot, Unity, Defold et Flax
Une liaison en direct envoie les modifications non enregistrées de Heron Brush vers l’éditeur d’un moteur au fil de votre travail. Elle est à sens unique : les modifications faites dans le moteur ne reviennent pas. Une seule liaison en direct peut tourner par fenêtre de carte. Arrêtez-la avant de changer de moteur, et relancez-la pour appliquer des options d’export modifiées. Les commandes se trouvent dans la fenêtre Exporter, sur la page du moteur, dans la carte **Liaison en direct avec**.

L’indicateur de la barre de titre montre l’état de la liaison ; cliquez dessus pour l’arrêter. Fermer ou remplacer la carte arrête la publication. Après un échec d’import, corrigez le problème et relancez la liaison.

## Godot
1. Exportez une première fois dans un projet Godot.
2. Cliquez sur **Installer le plugin Godot**.
3. Activez Heron Brush Live dans Godot, sous Paramètres du projet → Plugins.
4. Cliquez sur **Démarrer la liaison en direct** et attachez la carte à une scène 3D ouverte depuis le dock Godot.

Le plugin ne remplace que sa branche générée et conserve le placement de l’attache et les autres nœuds. Enregistrez vous-même la scène Godot, et gardez vos propres scripts et nœuds en dehors de la branche générée.

## Unity
1. Exportez une première fois dans le dossier Assets du projet Unity.
2. Cliquez sur **Installer le plugin Unity** et laissez Unity le compiler.
3. Dans Unity, ouvrez Window → Heron Brush → Live Link.
4. Démarrez la liaison et attachez la carte à la scène Unity active.

Gardez la fenêtre live d’Unity ouverte. Les mises à jour sont suspendues en Play Mode et en Prefab Mode. Seul l’enfant Generated de l’objet carte est remplacé. Ne modifiez pas les fichiers générés.

## Defold
1. Exportez une première fois dans votre projet Defold et laissez-le ouvert dans Defold.
2. Cliquez sur **Démarrer la liaison en direct**. Aucun plugin n’est nécessaire.
3. Ouvrez heron_brush_live/preview.collection dans Defold, ou référencez-la depuis votre propre scène.

La liaison écrit dans un dossier généré séparé et ne touche ni à l’export final ni à game.project. **Recharger le jeu en cours** met aussi à jour un jeu lancé depuis Defold ; ajouter des objets ou modifier la collision peut demander de relancer le niveau. Les échecs de connexion sont automatiquement retentés.

## Flax
Exportez dans le dossier Content du projet et cliquez sur **Installer le module Flax**. Laissez Flax le compiler, démarrez la liaison, puis choisissez Tools → Heron Brush: attach live map dans la scène Flax ouverte. La racine générée conserve son placement et les autres objets ne sont pas modifiés. Enregistrez vous-même la scène. Les mises à jour sont suspendues en Play Mode. Tools → Heron Brush: stop live map suspend la réception.

# Sessions collaboratives en direct
Collaboration → Session en direct ouvre une fenêtre pour héberger un autre utilisateur de Heron Brush ou le rejoindre. Cela partage l’édition de la carte entre plusieurs personnes ; c’est distinct d’une liaison en direct avec un moteur.

## Héberger et rejoindre
Saisissez **Votre nom**. Pour héberger, choisissez **Héberger une session**, cliquez sur **Démarrer la session** et envoyez à vos invités les informations obtenues avec **Copier les informations de connexion**. Pour rejoindre, choisissez **Rejoindre une session**, collez les informations et cliquez sur **Rejoindre**. Rejoindre remplace la carte ouverte par celle de l’hôte : enregistrez donc votre travail avant.

La version standard fonctionne sur le réseau local, et **Port réseau local** définit le port. Les sessions par Internet (**Se connecter par Internet**) nécessitent une version compilée avec la fonctionnalité réseau optionnelle. Les participants voient les repères de caméra et les noms des autres dans la vue 3D.

Fermer la fenêtre laisse la session active ; l’indicateur de la barre de titre montre son état. **Arrêter la session** ou **Quitter la session** y met fin. Quand deux personnes modifient le même objet, l’ordre de l’hôte détermine la version finale, et l’annulation est elle aussi une modification partagée. Travaillez autant que possible sur des zones séparées et enregistrez des points de contrôle.

# Affichage 3D et second écran
## Modes d’affichage
La section **Vue** de l’onglet Carte choisit ce que dessinent les vues : Brushes, Entités ponctuelles, Modèles d'entités, Noms de classe des entités, Contours des groupes, Surfaces courbes, Matériaux, Afficher les faces omises, Aperçu des animations, Fil de fer, Arêtes et Contours des entités brush. Fil de fer dessine les arêtes des triangles, y compris les diagonales et les arêtes situées derrière les surfaces, sans matériaux. C’est un affichage de débogage temporaire, sans effet sur les exports ni sur les rendus.

L’éclairage, les ombres, l’occlusion ambiante, les réflexions, le halo lumineux et le lissage de la vue 3D se règlent dans Préférences → Moteur de rendu. Ce sont des réglages d’aperçu ; les images finales utilisent les réglages propres à la caméra de rendu.

## Une fenêtre 3D séparée
Vue → Vue 3D dans une fenêtre séparée place la vue 3D dans sa propre fenêtre, que vous pouvez mettre sur un autre écran. Elle partage la caméra, le document et les outils, et les vues planes occupent l’espace de la fenêtre principale. Son menu propose Matériaux, Vue solide et Fil de fer ; un sous-menu Vue avec Afficher les faces omises, Aperçu des animations, Arêtes, Contours des groupes, Entités ponctuelles, Modèles d'entités et Contours des entités brush ; Faire la mise au point sur la sélection ; Avant, Côté et Dessus, qui orientent la caméra autour de la sélection (ou d’un point devant la caméra) ; Placer la caméra depuis la vue 3D ; Plein écran (`F11`) ; Rendre une image ; et Revenir à la fenêtre principale. Fermer la fenêtre réintègre la vue.

## Ciel procédural
Le ciel procédural est activé par défaut dans la vue 3D et peut être désactivé dans Préférences → Moteur de rendu → Lumière. Les caméras de rendu ont leur propre option **Ciel procédural**, enregistrée avec la carte. Les couleurs du ciel, de l’horizon et du sol viennent des réglages d’environnement, et le soleil suit l’éclairage de la scène. Le ciel n’a pas de géométrie et n’est pas exporté.

# Caméras de rendu et images PNG
Fichier → Rendre une image ouvre la fenêtre de rendu : la caméra et ses réglages à gauche, et à droite un aperçu en direct qui se redessine à chaque modification. Les caméras sont enregistrées dans le `.map`, dans un calque Render cameras réservé à l’éditeur et exclu des exports vers les moteurs.

## Placer la caméra
Cadrez le plan dans la vue 3D et appuyez sur `Ctrl+Alt+0` (ou `Ctrl+Alt+Num0`), ou cliquez sur **Placer la caméra depuis la vue 3D**. Si aucune caméra n’est sélectionnée, la première caméra enregistrée est mise à jour ; s’il n’en existe aucune, une caméra est créée. L’opération peut être annulée et conserve les réglages de sortie et d’éclairage. **Sélectionner la caméra dans la carte**, **Charger la caméra depuis la carte** et **Déplacer la vue 3D vers la caméra** fonctionnent dans l’autre sens. **Cadre de la caméra dans la vue 3D** montre les limites de l’image, et **Contrôler la caméra depuis la vue 3D** permet à la navigation normale de déplacer la caméra enregistrée.

## Réglages
- **Position de la caméra** : X, Y, Z, lacet et tangage.
- **Sortie et objectif** : **Préréglages** (Full HD, Carré, Portrait), largeur, hauteur, échantillons, **Champ de vision vertical** et **Plan de coupe lointain**.
- **Éclairage** : Lumière du jour, Coucher de soleil ou Ciel couvert, ou bien l’exposition, le soleil, les couleurs du ciel, de l’horizon et du sol, **Ciel procédural**, **Ombres**, **Occlusion ambiante** et **Halo lumineux**.
- **Profondeur de champ** : **Activer la profondeur de champ**, distance de mise au point, zone de netteté et rayon de flou. **Faire la mise au point sur la sélection** fait la mise au point sur le centre d’un objet sélectionné devant la caméra.

**Enregistrer la caméra et les réglages** stocke le formulaire dans la carte ; enregistrez la carte pour le conserver sur le disque.

## Rendu
**Rendu PNG** demande où enregistrer et lance un rendu progressif, qui s’affine échantillon après échantillon dans l’aperçu. Plus d’échantillons donnent un meilleur résultat mais prennent plus de temps. **Annuler le rendu** s’arrête sans écrire de fichier. Le moteur de rendu utilise le pipeline de rastérisation de l’éditeur, et non le path tracing. Le PNG a un fond opaque, et les animations ne sont pas rendues. Les très grandes images peuvent figer brièvement l’éditeur.

# Préférences
Fichier → Préférences (`Ctrl+,`) ouvre les réglages. Les modifications s’appliquent immédiatement et sont enregistrées. La barre latérale liste Vue d’ensemble, Jeux, Vue, Couleurs, Souris, Clavier et Moteur de rendu. Le champ de recherche en haut trouve un réglage par son nom ou sa description, sur n’importe quelle page. Chaque réglage modifié a son propre bouton pour revenir à sa valeur par défaut, et **Rétablir les valeurs par défaut** réinitialise la page courante.

## Les pages
- **Vue d’ensemble** : les réglages courants réunis : Apparence (Opacité des panneaux (%), Palette de couleurs, Disposition, Langue), Édition, Vue et Manuel.
- **Jeux** : le jeu, l’emplacement de ses fichiers et votre nom pour la collaboration. Un jeu indique quel format de carte écrire, quelles définitions d’entités charger et où se trouvent ses matériaux. Les configurations de jeu sont lues dans le dossier `games` du dossier de configuration (`~/.config/heron-brush/games` sous Linux et macOS, `%LOCALAPPDATA%\heron-brush\games` sous Windows) et dans le dossier `games` situé à côté de l’application, où les paquets placent la configuration fournie.
- **Vue** : Disposition, Palette de couleurs, Langue, axes du monde, opacité de la grille, taille des cellules de détection des fuites et verrous de placage.
- **Couleurs** : les couleurs des vues, de la géométrie et des poignées.
- **Souris** : vitesses et inversions du regard, du déplacement latéral et du mouvement, vitesse de vol, champ de vision, Lier les vues 2D et taille des poignées.
- **Clavier** : le jeu de raccourcis, Heron Brush ou Blender.
- **Moteur de rendu** : ombrage réaliste, contour de sélection, lumière, ombres, occlusion ambiante, réflexions, halo lumineux et surfaces.

## Langue
**Langue** propose Suivre la langue du système et toutes les langues traduites. L’interface change aussitôt. Le manuel s’ouvre dans la langue de l’interface lorsqu’il en existe une traduction, et en anglais sinon.

# Plugins et MCP
## Plugins
Les plugins étendent l’éditeur avec des scripts Rhai. Fichier → Plugins ouvre la fenêtre Plugins. **Installer un plugin** installe un fichier `.rhai` et **Installer un dossier de plugin** installe un dossier contenant `main.rhai` ; des métadonnées `plugin.toml` facultatives décrivent le plugin et ses réglages. Les plugins sont classés sous **Plugins actifs** et **Plugins inactifs**, et **Activer le plugin** en active ou désactive un ; ce choix persiste après un redémarrage. **Recharger les plugins** relit les scripts depuis le disque et réinitialise les valeurs de leurs panneaux.

Les plugins actifs apparaissent aussi dans la section **Plugins** de l’onglet Carte, avec leurs réglages, leurs boutons d’action (**Exécuter**) et leur sortie. Une action agit sur la sélection, ou sur tout si rien n’est sélectionné, et forme une seule étape d’annulation. Le plugin Selection inspector fourni affiche des compteurs et des actions pour monter et descendre.

## MCP
Collaboration → Démarrer le serveur pour un agent (MCP) permet à un agent d’IA ou à un autre outil de piloter l’éditeur ouvert via le Model Context Protocol. La fenêtre affiche l’adresse locale avec **Copier**, et **Arrêter le serveur** l’arrête ; tant qu’il tourne, la barre de titre affiche **MCP actif**. L’option de ligne de commande `--mcp` lance la même interface sans fenêtre. Les clients peuvent découvrir des outils pour la géométrie, les ressources, les propriétés, l’animation, l’export et les sessions. MCP et les plugins de l’éditeur sont deux interfaces différentes.

# Ligne de commande
Le programme s’appelle `heron-brush`. Sans option, il ouvre l’écran d’accueil.

## Ouvrir des fichiers
`heron-brush map.map [entities.fgd] [materials/]` ouvre une carte, éventuellement avec des définitions d’entités et un dossier de matériaux. Un argument vide en saute un, par exemple `heron-brush map.map "" materials/`. Si un éditeur est déjà lancé, la carte s’y ouvre ; `--new-instance` ouvre plutôt une fenêtre séparée.

## Commandes
- `--help` ou `-h` : afficher l’aide. `--version` ou `-V` : afficher la version.
- `--mcp` : servir MCP sur l’entrée et la sortie standard, sans fenêtre.
- `--merge-map <base> <ours> <theirs> [<output>]` : fusion à trois voies de fichiers `.map` pour git. Par défaut, la sortie est écrite dans ours. Les conflits sont affichés et donnent un code de sortie non nul.
- `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]` : convertir un maillage en brushes. size est la longueur du plus grand côté en unités de carte (256 par défaut), snap est une grille pour les sommets (0 pour aucune), et les mots `solid` ou `layered` construisent des blocs de grille ou des tranches convexes au lieu de fins coins.
- `--render-preview <in.map> <out.png> [size]` : écrire une miniature PNG d’une carte, de 512 pixels par défaut.
- `--run-script <script.rhai> <in.map> [out.map] [key=value ...]` : exécuter un script de plugin sur une carte avec tout sélectionné et écrire le résultat (par défaut dans le fichier d’entrée). `key=value` répond aux réglages du plugin.

# Dépannage
## Quand quelque chose semble anormal
- Matériau manquant : vérifiez le dossier ou le paquet de matériaux et les chemins du projet de l’export.
- Mauvaise taille de modèle : comparez avec le repère de joueur de 64 unités et l’échelle d’export.
- Mauvaise direction de dégradé : vérifiez l’angle du dégradé et l’orientation de la face.
- Scène vide dans le moteur : lisez les avertissements d’export et suivez les instructions d’import de la cible.
- Liaison en direct en attente : vérifiez que le plugin est installé, que la scène est attachée et que les mises à jour ne sont pas suspendues.
- Uniquement des lignes, pas de faces : désactivez Fil de fer dans la section Vue de l’onglet Carte, ou choisissez Matériaux ou Vue solide dans le menu de la fenêtre 3D.
- Objets absents des vues : `Ctrl+Shift+I` affiche les objets masqués ; vérifiez les calques et les filtres de la section Vue.
- Une touche ne fait rien : cliquez d’abord dans la vue pour qu’elle ait le clavier, et vérifiez le jeu de raccourcis dans Préférences → Clavier.
- Une modification a échoué : annulez, simplifiez la géométrie, et utilisez **Vérifier la carte** et **Rechercher les fuites** dans la section **Problèmes** de l’onglet Carte.
- Une carte ne s’ouvre pas depuis le gestionnaire de fichiers : elle a peut-être été transmise à un éditeur déjà lancé ; cherchez cette fenêtre.

## Réglages et sauvegardes
Les préférences, les cartes récentes, les étoiles, les réglages d’export et les plugins se trouvent dans le dossier de configuration : `~/.config/heron-brush` sous Linux et macOS, et `%LOCALAPPDATA%\heron-brush` sous Windows. Les sauvegardes automatiques se trouvent dans le dossier `autosave` à côté de chaque carte.

## Signaler un bug
Indiquez la version de l’éditeur (Aide → À propos), votre système d’exploitation, une petite carte qui montre le problème et les étapes exactes. Le panneau de sortie (`Ctrl+4`) indique souvent ce qui s’est mal passé. Enregistrez une copie avant de faire des essais.

## Utiliser ce manuel
Choisissez un chapitre dans le sommaire à gauche. `Ctrl+F` affiche le sommaire et place le curseur dans le champ de recherche ; `Esc` efface la recherche, ou ferme le manuel si la recherche est vide. **Sommaire** masque ou affiche la liste des chapitres et **Taille du texte** change la taille de la police ; les deux réglages sont mémorisés. La liste des langues en haut permet de passer d’une traduction disponible à l’autre.
