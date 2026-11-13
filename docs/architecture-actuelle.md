# Architecture actuelle

**Statut : architecture modulaire implémentée après validation du plan.**

La [cible](architecture-cible.md) décrit les principes ; cette page donne les
chemins réellement utilisés. Les changements locaux UML, accueil et identité
visuelle déjà présents ont été conservés pendant la migration.

## Organisation

Le workspace conserve deux crates : `op-core` pour le métier indépendant d'egui,
`op-app` pour l'application native/WebAssembly. `website/` assemble la vitrine
Astro et l'application web sous `/app/`.

```mermaid
flowchart TD
    Composition["composition.rs"] --> Modules["Modules BPMN, Petri, UML, Mining"]
    Composition --> Shell["app.rs : cycle de vie, menus, docking"]
    Shell --> Contracts["editor/ : registre, sessions, commandes, vues"]
    Modules --> Contracts
    Modules --> Layout["editor/layout.rs : layout graphique commun"]
    Layout --> Canvas["canvas.rs et history.rs"]
    Modules --> Canvas
    Modules --> Core["op-core : modèles typés, codecs, règles"]
    Shell --> Platform["Fichiers et persistance"]
    Modules --> Jobs["platform/jobs.rs : thread ou Web Worker"]
```

## Points d'entrée

| Responsabilité | Implémentation |
| --- | --- |
| Enregistrement statique | [composition.rs](../crates/op-app/src/composition.rs) appelle les fonctions `register` des modules. |
| Descripteurs, fabriques et imports | [editor/registry.rs](../crates/op-app/src/editor/registry.rs). Le registre est injecté dans l'application. |
| Layout graphique commun | [editor/layout.rs](../crates/op-app/src/editor/layout.rs) : configuration, inspecteur, onglets, actions spécifiques et canvas. |
| Session commune et états de vues typés | [editor/session.rs](../crates/op-app/src/editor/session.rs). |
| Nom, destination, état sauvegardé et récupération | [editor/document.rs](../crates/op-app/src/editor/document.rs). |
| Menus, événements et disposition | [app.rs](../crates/op-app/src/app.rs), [home.rs](../crates/op-app/src/home.rs), [dock.rs](../crates/op-app/src/dock.rs). |
| Adaptateurs métier/UI | [BPMN](../crates/op-app/src/bpmn_editor/session.rs), [Petri](../crates/op-app/src/petri_editor/session.rs), [UML](../crates/op-app/src/uml_editor/session.rs), [Mining](../crates/op-app/src/mining_editor/session.rs). |
| Nouvelles notations | [DMN](../crates/op-app/src/dmn_editor.rs) et [graphes EPC/activité](../crates/op-app/src/flow_editor.rs). |
| Schéma relationnel et DDL | [database.rs](../crates/op-core/src/database.rs), [générateurs SQL](../crates/op-core/src/database/ddl.rs), [éditeur](../crates/op-app/src/database_editor.rs). |
| Dessin libre | [modèle typé](../crates/op-core/src/whiteboard.rs), [éditeur](../crates/op-app/src/whiteboard_editor.rs), [session](../crates/op-app/src/whiteboard_editor/session.rs). |
| Enveloppe et codecs typés | [document/envelope.rs](../crates/op-core/src/document/envelope.rs). |
| Reprise des sessions | [platform/persistence.rs](../crates/op-app/src/platform/persistence.rs). |
| Exécution des tâches | [platform/jobs.rs](../crates/op-app/src/platform/jobs.rs), [tâche Petri](../crates/op-app/src/petri_editor/analysis_job.rs), [worker](../crates/op-app/analysis-worker.js). |

## Documents et vues

Le shell possède des `Doc` contenant un `Box<dyn EditorSession>`. L'ancienne enum
`Editor` n'est plus utilisée. Chaque adaptateur fournit le nom, les commandes,
les exports et les vues de sa notation, et conserve son modèle Rust concret.

Le sélecteur de création commun se trouve dans
[`app/new_document.rs`](../crates/op-app/src/app/new_document.rs). Il consomme
les descripteurs du registre et réutilise la grille de cartes de `home.rs`.
File, le bouton d’onglets et le raccourci clavier partagent ce sélecteur ; le
shell ne possède pas une deuxième liste de notations. Les exports déclarés par
chaque session sont présentés dans un sous-menu File → Export.

Les descripteurs de diagrammes et de workflows portent une fonction `icon`.
Les modules choisissent un pictogramme dans `icons.rs`, puis l’accueil et le
sélecteur l’affichent sans déduire la notation de sa position. Ce module UI
contient également le curseur de sélection commun aux éditeurs graphiques.

`Session<E>` conserve un éditeur et son historique uniques, avec un état typé par
instance de panneau. Les caméras (y compris celle du graphe Petri) et inspecteurs se sauvegardent séparément du
modèle. Les identifiants de vues sont des chaînes extensibles ; les anciennes
valeurs Petri restent lisibles. Fermer une vue parmi plusieurs conserve le document.
La fermeture de sa dernière vue prend en compte les modifications non sauvegardées.

Les anciennes API publiques `document::Model`, `load_any` et `to_json(&Model)`
restent disponibles comme façade de compatibilité. Les chemins communs de
l'application utilisent l'enveloppe ouverte `Document`. Les accès aux éditeurs
concrets par `Any` sont limités aux sondes des tests d'interface, sous `cfg(test)`.

## Découpage des éditeurs

Les dossiers `bpmn_editor/` et `petri_editor/` séparent interactions, rendu,
inspecteur/panneaux et adaptation de session. `uml_editor/` sépare interactions,
rendu, inspecteur et session. `mining_editor/results.rs` contient ses résultats,
et `mining_editor/session.rs` son intégration applicative.

Les noms de modules historiques restent en place : l'arborescence logique de la
cible n'impose pas de déplacer tous les fichiers ni de changer les chemins publics.

## Layout graphique partagé

Le [layout SVG et son vocabulaire](interface.md) identifient les zones de
l’interface : navigation, onglets, canvas, inspecteur et configuration de vue.

Les éditeurs BPMN, Petri et UML utilisent `DiagramLayout`. La barre de configuration
partage sa hauteur, son alignement et ses widgets : inspecteur, zoom, aimantation,
affichage des problèmes quand il est disponible et options spécifiques au module.
Les réglages UML ne possèdent plus une deuxième barre de configuration en haut.
Ses actions de relation et d'alignement restent dans l'emplacement d'outils métier.

Le même inspecteur s'affiche à droite dans une vue large. Il est masqué par défaut
sous 760 pixels ; une ouverture explicite dans une vue étroite affiche un panneau
borné au-dessus du canvas. Les onglets BPMN et Petri utilisent le même composant.
La barre regroupe les options dans un menu sous 720 pixels disponibles et y déplace
également le bouton d'inspecteur sous 360 pixels.

L'aimantation, la visibilité de l'inspecteur et l'affichage des problèmes sont des
réglages de vue, enregistrés dans la session. Ils ne modifient pas le modèle `.opx`.
Les styles de dessin persistants restent la propriété des modèles et sont fournis
à la barre par un emplacement d'options spécifique à la notation.

Le style des boutons des barres et des onglets de l’inspecteur est également
partagé via `theme::toolbar_style`. Le thème et les états de survol/sélection
restent indépendants du contenu métier et de la disposition des panneaux.

Les nouveaux éditeurs EPC et UML activité utilisent le même layout graphique.
Leurs modèles typés `Diagram<EpcKind>` et `Diagram<ActivityKind>` partagent les
mécanismes d’identité et de géométrie de `op-core/src/flow.rs` ; chaque type de
nœud conserve ses règles. `flow_editor.rs` réutilise les interactions pour ces
deux usages concrets sans ajouter de cas au shell.

Les tables DMN disposent d’un modèle, d’un évaluateur et d’un éditeur tabulaire
distincts. Elles utilisent le shell, l’historique, l’inspecteur et les actions
communs, avec un défilement de table adapté. Leurs fonctions et limites sont
précisées dans [les notations ajoutées](notations-ajoutees.md). Les chantiers
Looping et Bizagi restent décrits comme cibles dans [la matrice de couverture](parite-looping-bizagi.md).

Le module de base de données ajoute un modèle relationnel typé, un éditeur
ERD/MPD et une vue `database.ddl`. Les deux vues partagent tables, clés et
historique ; les caméras et sélections restent locales. Le SQL est généré dans
`op-core` pour PostgreSQL, MySQL/MariaDB et SQLite. La géométrie ne déclenche pas
une nouvelle génération. Voir [les fonctions et limites](base-de-donnees-ddl.md).

Le [dessin libre](dessin-libre.md) utilise le type `whiteboard`, un modèle et un
historique typés indépendants des notations métier. Les points des tracés sont
normalisés dans les bornes de l’élément pour conserver leur direction lors
d’un déplacement ou redimensionnement. Le module réutilise `DiagramLayout`
pour son inspecteur, sa configuration et son canvas. Sa palette verticale
réutilise `canvas::floating`, comme BPMN et Petri. Il s’enregistre
dans la composition, avec son exemple et son export SVG. Caméra et préférences
restent dans chaque vue ; la géométrie du dessin est son contenu sémantique.

## Langues de l’interface

L’application propose l’anglais (`en`), le français (`fr`) et l’espagnol (`es`).
Les tables de traduction et le type `Lang` vivent dans
[`op-core/src/i18n/`](../crates/op-core/src/i18n/mod.rs). Les appels utilisent des
clés stables comme `file.save` ou `drawing.choose_tool`. Les textes anglais vivent
dans `en.rs`, les traductions dans `fr.rs` et `es.rs`. Modifier un texte anglais
ne demande plus de changer sa clé ni les autres catalogues. Une traduction
absente utilise le catalogue anglais ; une clé inconnue est affichée telle quelle.
Les tests vérifient les clés utilisées et la parité des catalogues, les paramètres
et les sauts de ligne. Voir [le guide des traductions](traductions.md).

Le menu déroulant de [`op-app/src/i18n.rs`](../crates/op-app/src/i18n.rs) affiche
le drapeau et le code de la langue active. Il permet de choisir directement
English, Français ou Español, chaque nom précédé de son drapeau, et indique la
langue sélectionnée. Les drapeaux des options et du bouton réutilisent le même
dessin vectoriel via les atomes d’egui. Le choix est
restauré avec la clé existante `open-process-lang`, sans modifier les documents
`.opx` ni les anciennes valeurs de préférence. La vitrine possède les routes
`/`, `/fr/` et `/es/`, accessibles depuis un menu déroulant HTML utilisable sans
JavaScript ; voir [son README](../website/README.md).

## Historique, sauvegarde et analyses

La disposition automatique BPMN est calculée dans
[`bpmn/layout.rs`](../crates/op-core/src/bpmn/layout.rs), sans dépendance UI.
Le module BPMN expose la commande `bpmn.auto-arrange` et son bouton via
l’emplacement d’options de `DiagramLayout`. L’opération forme une étape
d’historique ; elle change la géométrie et conserve les appartenances aux
pools/couloirs ainsi que les propriétés et connexions métier.

L'historique par instantanés regroupe les gestes et sait les annuler sans ajouter
une étape. Il couvre aussi le texte et les réponses Mining. Les révisions
sémantiques et graphiques sont distinctes ; revenir à l'état sauvegardé retire
l'indicateur de modification.

L'application écrit les `.opx` v2, lit les v1 et restaure les anciennes sessions
RON dans une nouvelle session JSON v3. Les données d'un fichier inconnu sont
conservées pour récupération. Voir [la compatibilité](documents-et-compatibilite.md).

L'analyse Petri s'exécute sur un instantané privé, en thread natif ou dans un
Web Worker utilisant le même binaire Wasm. Les résultats sont acceptés seulement
pour le même contenu sémantique, la même limite et la même langue. Un déplacement
seul conserve le cache. La fermeture du document abandonne la tâche.

L'annulation native est vérifiée pendant l'exploration et entre les phases ; une
phase d'invariants déjà commencée peut finir en arrière-plan. Le worker web est
terminé directement. Mining et l'évaluation conservent leur exécution actuelle :
le pilote de tâches porte sur Petri, conformément au périmètre du plan.
