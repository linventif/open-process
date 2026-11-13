# Vérification de la migration

**Statut : contrôles locaux réussis.**

La référence de départ comportait 71 tests réussis. La suite après migration
comprend **90 tests réussis** : 36 dans l'application, 49 dans le cœur et 5 tests
d'intégration des documents historiques. Aucun test n'a été ignoré.

## Commandes exécutées

Depuis la racine du dépôt :

```sh
cargo test --locked --offline --workspace -- --test-threads=1
cargo check --locked --offline -p op-app
cargo check --locked --offline -p op-app --target wasm32-unknown-unknown
git diff --check
```

Le livrable web a également été construit avec Trunk, depuis `crates/op-app/` :

```sh
CARGO_NET_OFFLINE=true trunk build --dist /tmp/open-process-migration-web --public-url /
```

Le build contient la colle JavaScript et le Wasm, les snippets du transport de
tâches et `analysis-worker.js`. La vérification native finale ne produit pas
d'avertissement. Le formatage Rust porte sur les fichiers de cette migration,
sans parcourir les autres modules du dépôt.

## Scénarios supplémentaires

Les tests ajoutés couvrent notamment :

- Les cinq fixtures `.opx` v1 et leur migration/aller-retour v2.
- Les versions futures, enveloppes invalides et codecs d'une notation extérieure au catalogue.
- Les sessions RON v1/v2, leurs vues historiques et leur migration JSON v3 sans effacer les anciennes clés.
- Deux vues UML sur le même modèle, avec caméras indépendantes et historique unique.
- Une configuration graphique commune à BPMN, Petri et aux deux types UML, en clair/sombre et à 1 400, 420 et 280 pixels.
- Les contrôles de zoom, grille, problèmes et inspecteur, leur alignement commun et le menu compact.
- La reprise indépendante des réglages de grille et de problèmes dans plusieurs vues d'un document.
- La caméra du graphe Petri après reprise de session et reconstruction de son cache.
- Le retour à l'état sauvegardé, l'annulation d'un glissement par Échap et les sauvegardes annulées ou échouées.
- La récupération octet pour octet des fichiers inconnus, y compris après sérialisation de session.
- Une extension de table sans canvas avec plusieurs exports, sans ajout de branche dans le shell.
- La détection XML indépendante des préfixes de namespace, les imports ambigus et les identifiants dupliqués.
- Une analyse Petri remplacée après changement sémantique, puis réutilisée après changement de position.

Les tests existants d'interaction, de traduction et de rendu natif ont été
conservés. L'inventaire des textes et sources UI suit maintenant les sous-modules
extraits pour conserver la couverture des traductions et des polices embarquées.

## Test réel du Web Worker

La page [worker-smoke.html](../crates/op-app/tests/worker-smoke.html) exerce le
transport JavaScript du livrable Trunk, puis le même Wasm que l'application dans
un worker. Elle vérifie un petit workflow à deux états, son verdict de soundness,
l'annulation d'une tâche et une erreur de décodage.

Elle a été exécutée dans Firefox sans interface avec un livrable Trunk local,
à la racine et sous `/app/`. Les deux exécutions retournent `ok: true`, avec
calcul, annulation et erreur attendus.

À la racine, une exécution a compté 63 passages du timer du fil principal pendant
658 ms de lancement et de traitement du worker. Cette durée comprend le chargement
et l'initialisation du Wasm ; elle ne constitue pas une mesure de débit sur de gros
modèles. Le test sous `/app/` valide également la résolution des chemins des ressources.

Pour reproduire, construire avec Trunk, copier la page de test dans le dossier
produit, servir ce dossier en HTTP et ouvrir `worker-smoke.html`. Le résultat
s'affiche dans la page et dans `document.documentElement.dataset.result`.
L'option `?report=1` transmet aussi le résultat au point de collecte local `/result`
si le serveur de test le fournit. La page de test n'est pas incluse automatiquement
dans le produit.

## Périmètre

Ces contrôles portent sur Linux et Firefox dans l'environnement local. Les builds
release, les autres navigateurs et la matrice Windows/macOS restent du ressort de
la CI et des vérifications de publication. Aucun déploiement ni publication n'a été effectué.

## Zones et style de l’interface

Après l’ajout du [layout SVG annoté](diagrams/interface-zones.svg) et du style
commun des barres, les mêmes commandes Rust ont été exécutées à nouveau :

```sh
cargo test --locked --offline --workspace -- --test-threads=1
cargo check --locked --offline -p op-app
cargo check --locked --offline -p op-app --target wasm32-unknown-unknown
git diff --check
```

Résultat : **90 tests réussis**, compilation native et vérification WebAssembly
réussies. Le test des configurations partagées couvre toujours BPMN, Petri et
les deux éditeurs UML, à 1 400, 420 et 280 px. Le test de captures existant produit
désormais les survols de Home, File, Style et Inspector en clair et en sombre
dans `target/screenshots/toolbar_hover_*.png`. Les captures File en clair et Style
en sombre ont été inspectées visuellement.

Le SVG a été parsé comme XML, ses douze groupes de zones et l’unicité des
identifiants contrôlés, puis son rendu rasterisé avec Rsvg et inspecté. Les liens
locaux des pages de `docs/` ont été vérifiés. Le site et le transport Web Worker
n’ont pas été modifiés ni revalidés pour cette évolution visuelle.

<a id="notations-dmn-epc-et-activite"></a>

## Notations DMN, EPC et activité

Les premières versions décrites dans [les notations ajoutées](notations-ajoutees.md)
ont été vérifiées avec :

```sh
cargo test --locked --offline --workspace -- --test-threads=1
cargo check --locked --offline -p op-app
cargo check --locked --offline -p op-app --target wasm32-unknown-unknown
git diff --check
```

Résultat : **100 tests réussis** — 39 application, 56 cœur et 5 intégration —,
aucun ignoré. Compilation native et vérification WebAssembly réussies, sans
avertissement. Après le dernier ajustement des icônes de palette, le test de
sessions multi-vues et les deux vérifications de compilation ont été rejoués.

Les contrôles ajoutés ou étendus portent sur :

- Les règles de connexion et de degrés EPC/activité, les suppressions et la
  duplication avec remappage des identifiants.
- Les politiques DMN UNIQUE/FIRST/ANY/COLLECT, les frontières d’intervalles,
  les alternatives avec chaînes contenant une virgule et le refus d’expressions inconnues.
- Les allers-retours JSON et XML DMN, le refus de modèles multiples ou d’agrégations,
  et les exports SVG/DOT avec échappement des libellés.
- La création depuis l’accueil des trois types et l’accès aux cartes après défilement.
- La création et la connexion d’éléments à la souris, leur déplacement,
  l’annulation, la réouverture et les exports.
- L’édition d’une table, l’évaluation, l’invalidation du résultat, l’annulation
  d’une colonne et la réouverture de l’export DMN par le registre.
- Plusieurs vues d’un même document avec réglages d’inspecteur indépendants.
- La barre graphique commune, étendue à EPC et UML activité, en clair/sombre
  et aux largeurs 1 400, 420 et 280 px.
- Les traductions et la couverture des glyphes par les polices embarquées.

Les captures locales `target/screenshots/example_epc.png`,
`example_uml.activity.png` et `example_dmn.table.png` ont été inspectées visuellement.
Le SVG des zones de l’interface et les liens locaux de la documentation ont
également été contrôlés.

Il ne s’agit pas d’une certification OMG ou d’un essai exhaustif d’interopérabilité
avec les outils externes. Aucun test de parité Looping/Bizagi, build release,
publication ou déploiement n’a été exécuté. La comparaison des concurrents repose
sur leurs sources publiques, référencées dans [l’analyse](parite-looping-bizagi.md).

<a id="base-de-donnees-et-ddl"></a>

## Base de données et DDL

Le [module relationnel](base-de-donnees-ddl.md) a été vérifié avec les commandes
suivantes :

```sh
cargo test --locked --offline --workspace -- --test-threads=1
cargo check --locked --offline -p op-app
cargo check --locked --offline -p op-app --target wasm32-unknown-unknown
cargo run --locked --offline -p op-core --example database-ddl -- /tmp/open-process-ddl
python3 scripts/check-database-ddl.py /tmp/open-process-ddl
git diff --check
```

Résultat : **107 tests réussis** — 41 application, 61 cœur et 5 intégration —,
aucun ignoré. Les vérifications de compilation native et WebAssembly ont réussi,
sans avertissement.

Les tests vérifient les variantes de dialectes, identités et noms SQL, les clés
simples/composées, les index uniques, les cycles, les actions référentielles, les
valeurs par défaut, la précision décimale et le refus des schémas incohérents.
Les scénarios UI créent des tables et une relation à la souris, déplacent une
table sans changer son SQL, annulent, changent de dialecte, rouvrent le modèle et
affichent/copient le SQL dans un second panneau. La barre commune est également
testée pour ce module aux largeurs 1 400, 420 et 280 px.

Le contrôle Python a exécuté **trois scripts SQLite générés** dans des bases
en mémoire : boutique, clés composées/cycles et valeurs littérales. Il vérifie
les métadonnées de tables et clés, le rejet d’orphelins et de doublons, NOT NULL,
les valeurs par défaut et une suppression en cascade. Les littéraux contenant
apostrophes, antislash et caractères SQL ont été conservés comme données.

Les captures locales `target/screenshots/database_example_light.png`,
`database_example_dark.png` et `database_ddl_split.png` montrent les rendus
clair/sombre et la vue SQL partagée. Les rendus clair et SQL partagé ont été
inspectés visuellement. Les liens de documentation et le SVG des zones ont été
contrôlés ; les [exemples SQL](base-de-donnees-ddl.md) ont été régénérés.

Les scripts PostgreSQL/MySQL n’ont pas été exécutés sur des serveurs dans cette
session. Aucun test navigateur, build release, déploiement ou essai d’import SQL
n’a été réalisé pour ce module. Les tests de compatibilité `.opx` existants
restent réussis ; aucun schéma historique n’a changé.

## Création en grille, exports et disposition BPMN

Commandes exécutées après cette évolution de l’interface :

```sh
cargo fmt --all -- --check
cargo test --locked --offline --workspace -- --test-threads=1
cargo check --locked --offline -p op-app
cargo check --locked --offline -p op-app --target wasm32-unknown-unknown
git diff --check
```

Résultat : **115 tests réussis** — 46 application, 64 cœur et 5 intégration —,
aucun ignoré. Les compilations native et WebAssembly ont réussi sans avertissement.

Les nouveaux tests couvrent :

- Le sélecteur commun depuis File → New…, le bouton « + » et Ctrl/Cmd+N ;
  la recherche, l’absence de résultat, l’annulation et la création dans le
  groupe d’onglets d’origine. La grille est vérifiée à 1 400, 420 et 280 px.
- File → Export, avec des formats différents pour BPMN, DMN, EPC et base de données.
- Auto arrange BPMN en configuration large et dans Options en vue compacte ;
  conservation du contenu métier, de la révision sémantique et restauration
  exacte des instantanés par annulation/rétablissement.
- L’algorithme de disposition : absence de chevauchement des nœuds, conservation
  des appartenances pools/couloirs, cycles, composants déconnectés, annotations,
  déterminisme et refus atomique d’une appartenance dans un en-tête de pool.

Les captures `target/screenshots/new_document_grid_1400.png`,
`new_document_grid_280.png`, `file_export_submenu.png` et
`bpmn_auto_arranged_1400.png` ont été inspectées visuellement. Le contrôle natif
des interactions n’est pas un test dans un navigateur : aucun nouveau test
navigateur, build release ou déploiement n’a été exécuté pour cette évolution.

## Taille des places Petri

Le rayon des places sur le canvas passe de 11 à **5,5 unités**, soit une
nouvelle réduction de 50 % du diamètre. Les jetons se dimensionnent à partir
du rayon effectif ; les attaches d’arcs, la détection de sélection et l’aperçu
de création réutilisent `PLACE_R`. Les libellés gardent leur taille.

Vérifications exécutées : formatage, `git diff --check`, liens locaux,
tests du workspace avec `--test-threads=1`, compilation native et cible
`wasm32-unknown-unknown`, avec `--locked --offline` pour Cargo.
**123 tests réussis** — 50 application, 68 cœur et 5 intégration —, aucun ignoré ;
compilations sans avertissement. La capture `target/screenshots/mining_petri.png`
a été inspectée visuellement. Aucun nouveau test navigateur, build release
ou déploiement n’a été exécuté ; le schéma des documents reste inchangé.

## Espagnol dans l’application et la vitrine

Commandes exécutées pour l’ajout de la langue `es` :

```sh
cargo test --locked --offline --workspace -- --test-threads=1
cargo check --locked --offline -p op-app
cargo check --locked --offline -p op-app --target wasm32-unknown-unknown
cd website
bun run build:site
CARGO_NET_OFFLINE=true bun run build:app
bun test
```

Résultat : **123 tests Rust réussis** — 50 application, 68 cœur et 5 intégration —,
aucun ignoré. Les vérifications native et WebAssembly et le build web release avec
Trunk ont réussi. Le contrôle Astro/TypeScript ne signale aucune erreur ni aucun
avertissement ; les **9 tests Bun** réussissent sur le livrable complet.

Les 860 traductions espagnoles sont contrôlées par les tests existants de
couverture, d’unicité, de paramètres, de sauts de ligne et de glyphes. Les tests
ajoutés ou étendus vérifient le cycle EN → FR → ES → EN depuis l’accueil, la
mémorisation/restauration des langues, les anciennes valeurs sérialisées et le
repli sur l’anglais. La capture `target/screenshots/home_spanish.png` a été
inspectée visuellement.

Les tests du site vérifient le HTML espagnol à `/es/`, les liens entre les trois
langues, les URL canoniques, `hreflang`, Open Graph, les données structurées et
le sitemap. Les liens locaux de la documentation, le formatage des fichiers Rust
de traduction modifiés et `git diff --check` ont été vérifiés.

Aucun nouveau test dans un navigateur, build natif release, contrôle Windows/macOS,
publication ou déploiement n’a été effectué pour cet ajout.

<a id="dessin-libre"></a>

## Dessin libre natif et web

Le [module de dessin libre](dessin-libre.md) a été vérifié avec :

```sh
cargo fmt --all -- --check
cargo test --locked --offline --workspace -- --test-threads=1
cargo check --locked --offline -p op-app
cargo check --locked --offline -p op-app --target wasm32-unknown-unknown
git diff --check
```

Résultat : **123 tests réussis** — 50 application, 68 cœur et 5 intégration —,
aucun ignoré. Les vérifications native et WebAssembly réussissent sans avertissement.

Les nouveaux scénarios vérifient les huit outils à la souris, les points du
crayon, le texte multiligne, l’export SVG échappé et la réouverture `.opx`.
Ils couvrent le déplacement, le redimensionnement, la duplication, la suppression,
l’annulation exacte d’un geste et l’historique. Une sélection par rectangle
et l’édition d’un style commun conservent les remplissages différents des éléments ;
un clic au crayon produit un point. Les tests du cœur vérifient la direction
des chemins, les redimensionnements, les identifiants, la détection de formes,
le refus de géométries invalides et la validité XML du SVG.

Les tests communs incluent désormais ce module dans l’accueil, le sous-menu
d’exports, la restauration de vues séparées et les contrôles de configuration
aux largeurs 1 400, 420 et 280 px. Les traductions françaises/espagnoles et glyphes sont
également vérifiés.

Les captures `target/screenshots/whiteboard_all_tools_light.png` et
`whiteboard_example_dark.png` ont été inspectées visuellement. Les liens locaux
et le SVG des zones ont été contrôlés. Le composant officiel Excalidraw et
son format d’échange ne sont pas intégrés. La cible web a été compilée ;
aucun nouveau test dans un navigateur, build release ou déploiement n’a été exécuté.

## Pictogrammes de notations et sélection commune

Les icônes des cartes sont désormais déclarées par les modules, dans
`DiagramDescriptor::icon` et `WorkflowDescriptor::icon`. Leur choix est indépendant
de l’ordre et du filtrage. La barre du dessin libre utilise des boutons à
pictogramme ; les palettes BPMN, Petri, UML, EPC/activité, base de données et
dessin libre partagent le curseur de sélection BPMN défini dans `icons.rs`.

Vérifications exécutées :

```sh
cargo fmt --all -- --check
cargo test --locked --offline --workspace -- --test-threads=1
cargo check --locked --offline -p op-app
cargo check --locked --offline -p op-app --target wasm32-unknown-unknown
git diff --check
```

Résultat : **123 tests réussis** — 50 application, 68 cœur et 5 intégration —,
aucun ignoré. Les compilations native et WebAssembly réussissent sans avertissement.
Les tests d’interaction existants couvrent les outils, noms accessibles,
sélections et contrôles communs après ce changement visuel.

Les captures `target/screenshots/home_narrow.png`, `new_document_grid_1400.png`
et `whiteboard_all_tools_light.png` ont été inspectées visuellement : symboles
distincts par notation, curseur commun et outils du dessin libre.
Le SVG documentaire des zones utilise le même tracé
de sélection ; sa syntaxe XML et les liens locaux ont été contrôlés. Aucun
nouveau test navigateur, build release ou déploiement n’a été exécuté.

## Icônes dans les menus de types BPMN

Les choix des menus contextuels Task type et Gateway type, les listes
correspondantes de l’inspecteur et la création par clic droit dans le vide
utilisent des boutons standards avec pictogramme décoratif. Les tâches
partagent leurs marqueurs avec le canvas ; les passerelles réutilisent son
rendu vectoriel. Le type courant est surligné.

Le scénario existant `bpmn_editing_by_mouse` a été étendu : ouverture des deux
sous-menus, présence de tous les libellés, changement du type et restauration
exacte par annulation/rétablissement, en thèmes clair et sombre. Les captures
`target/screenshots/bpmn_task_menu_dark.png` et
`bpmn_gateway_menu_dark.png` ont été inspectées visuellement.

Les commandes de la section précédente ont été exécutées pour cette évolution :
**123 tests réussis** — 50 application, 68 cœur et 5 intégration —, aucun ignoré ;
compilations native et WebAssembly sans avertissement, formatage, liens locaux
et `git diff --check` valides. Le schéma `.opx` n’a pas changé. Aucun test
navigateur, build release ou déploiement n’a été exécuté pour cette évolution.

## Menus déroulants de langue

Le bouton de langue de l’application et les liens de la vitrine sont remplacés
par des menus proposant directement English, Français et Español. La préférence
existante de l’application reste mémorisée. Le menu HTML conserve des liens
utilisables sans JavaScript et indique la langue active.

Vérifications exécutées :

```sh
cargo test --locked --offline --workspace -- --test-threads=1
cargo check --locked --offline -p op-app
cargo check --locked --offline -p op-app --target wasm32-unknown-unknown
cd website
bun run build:site
CARGO_NET_OFFLINE=true bun run build:app
bun test
```

Résultat : **123 tests Rust réussis** (50 application, 68 cœur et 5 intégration),
**9 tests Bun réussis**, vérifications native/WebAssembly et build web release
réussis. Astro ne signale aucune erreur ni aucun avertissement. Le test d’accueil
vérifie que l’ouverture du menu ne change pas la langue, que les trois choix sont
visibles, que la sélection directe fonctionne et que le menu se ferme après le
choix. La capture `target/screenshots/language_dropdown.png` a été inspectée.
Les tests HTML vérifient les liens et l’indication unique de langue active sur
les trois pages de la vitrine. Les liens locaux de documentation, le formatage
du module UI de langue et `git diff --check` ont été vérifiés.

La fermeture par Échap ou clic extérieur sur la vitrine est implémentée mais
n’a pas été exercée dans un navigateur. Aucun déploiement ni contrôle
Windows/macOS n’a été exécuté pour cette évolution.

## Clés de traduction stables

Les **859 messages** de l’application utilisent désormais des identifiants
stables. Le nouveau catalogue anglais `op-core/src/i18n/en.rs` et les catalogues
français/espagnol partagent les mêmes clés. Une comparaison ponctuelle avec les
catalogues avant migration a vérifié que les **2 577 valeurs** (859 par langue)
sont conservées exactement, paramètres et sauts de ligne compris. Un inventaire
lexical a contrôlé les **1 057 appels littéraux** de traduction dans les sources.

Commandes exécutées :

```sh
cargo test --locked --offline -p op-core
cargo test --locked --offline --workspace -- --test-threads=1
cargo check --locked --offline -p op-app
cargo check --locked --offline -p op-app --target wasm32-unknown-unknown
cargo fmt --all -- --check
git diff --check
cd website
CARGO_NET_OFFLINE=true bun run build:app
bun test
```

Résultat final : **124 tests Rust réussis** — 50 application, 69 cœur et
5 intégration —, aucun ignoré. Les vérifications native/WebAssembly, le build web
release et les **9 tests Bun** réussissent. Les fichiers Rust modifiés ont été
formatés et le contrôle de formatage du workspace réussit.

Le nouveau test du cœur vérifie qu’une correction du texte anglais laisse la
clé et la traduction française inchangées, et que le repli utilise le nouveau
texte anglais. Les contrôles des catalogues comparent les paramètres et sauts de
ligne avec les valeurs anglaises, vérifient les clés identiques et leur syntaxe.
Le contrôle des usages inclut maintenant le catalogue anglais ; celui des glyphes
inclut explicitement les trois langues. Les tests UI qui consultent les clés des
palettes et du registre traduisent ces clés avant de rechercher le libellé affiché.
Les tests existants de documents historiques et de restauration des sessions
réussissent. Les liens locaux des pages de documentation concernées ont été vérifiés.

La vitrine Astro utilisait déjà des clés stables : son catalogue et ses pages
n’ont pas été modifiés pour cette migration. Son livrable a reçu l’application
WebAssembly reconstruite. Aucun nouveau test navigateur, build natif release,
contrôle Windows/macOS, publication ou déploiement n’a été effectué.

## Palette verticale du dessin libre

Les outils du dessin libre sont placés dans une palette verticale flottant à
gauche du canvas, comme BPMN et Petri. Elle réutilise `canvas::floating` et
propose un défilement vertical lorsque la hauteur disponible est insuffisante.
Les options, le zoom et l’inspecteur restent dans le layout partagé.

Les trois scénarios UI du dessin libre passent après ce changement. La sélection
par rectangle commence à côté de la palette ; création, édition, déplacements,
redimensionnements, styles et annulation restent fonctionnels. L’instantané des
fonctionnalités terminées passe les **123 tests Rust** pendant la préparation
des commits, indépendamment de la migration des clés de traduction encore en cours.

## Drapeaux dans les menus de langue

Les choix English, Français et Español affichent leur drapeau dans l’application
et sur la vitrine, avec le même visuel dans le bouton de langue active. La capture
`target/screenshots/language_dropdown.png` a été inspectée. Le test d’accueil
existant vérifie toujours les noms accessibles et le changement de langue.

La suite séquentielle passe les **124 tests Rust** (50 application, 69 cœur et
5 intégration). Les vérifications native/WebAssembly, le formatage, le build Astro
et le build web release réussissent. Les **9 tests Bun** passent ; le contrôle du
HTML vérifie désormais les drapeaux décoratifs et leurs fichiers SVG pour chaque
option des trois pages. Les trois SVG ont été parsés et les 156 liens locaux de
la documentation contrôlés, sans erreur. `git diff --check` réussit.

Aucun nouveau test interactif dans un navigateur ni contrôle Windows/macOS
n’a été exécuté pour cet ajout.

## Exemples dans la grille de sélection

Le bouton **Exemples** du sélecteur de documents remplace File → Examples et
utilise les déclarations du registre. Les tests existants du sélecteur ont été
étendus : accès à tous les exemples à 1 400, 420 et 280 px, fermeture du menu
par Échap sans fermer le sélecteur, annulation sans modifier le document et
ouverture d’un exemple dans le groupe d’onglets d’origine. Le contenu de
l’exemple ouvert est comparé à celui du registre. Les captures
`target/screenshots/new_document_examples_1400.png` et
`new_document_examples_280.png` ont été inspectées visuellement.

La suite séquentielle `cargo test --locked --offline --workspace -- --test-threads=1`
passe les **124 tests Rust** (50 application, 69 cœur et 5 intégration).
Les vérifications native et WebAssembly de `op-app`, `cargo fmt --all -- --check`,
les 156 liens locaux de documentation et `git diff --check` réussissent.
Le site n’a pas changé ; aucun nouveau test navigateur ou build release
n’a été exécuté pour ce déplacement.
