# Dessin libre natif et web

**Implémenté : un éditeur Rust/egui de dessin libre, inspiré de l’usage
d’Excalidraw, dans le shell et le layout communs.** Le choix retenu est un module
natif + WebAssembly. Le composant React officiel Excalidraw n’est pas embarqué.

## Utilisation

Dans **File → New…**, le bouton **+** ou **Ctrl/Cmd+N**, rechercher « dessin »
en français ou « whiteboard » et choisir **Nouveau dessin libre**. La carte
**Dessiner librement** de l’accueil crée le même document. Un exemple est
disponible depuis le bouton **Exemples** de la carte **Nouveau dessin libre**,
dans la grille **File → New…** ou sur la carte **Dessiner librement** de l’accueil,
puis **Dessin libre – Idée partagée**.

La palette verticale à gauche du canvas propose sélection, main, rectangle, ellipse, losange, ligne,
flèche, main levée, texte et note adhésive. Glisser sur le canvas trace l’élément.
Elle utilise le même panneau flottant que les autres éditeurs graphiques et
défile verticalement lorsque la hauteur disponible ne suffit pas.
Les outils sont des boutons à pictogramme avec nom accessible et infobulle ;
la sélection réutilise le curseur BPMN partagé par les autres éditeurs.
Un clic avec un outil de forme crée un élément de taille initiale. Le crayon
reste actif après un tracé ; les autres outils reviennent à la sélection.

- Cliquer pour sélectionner ; Maj+clic ajoute ou retire un élément.
- Glisser dans le vide dessine un rectangle de sélection ; Maj conserve la
  sélection précédente.
- Glisser une sélection la déplace. La poignée en bas à droite redimensionne
  un élément seul, y compris les tracés et les flèches.
- L’inspecteur édite le texte multiligne, les dimensions, la couleur du trait,
  le remplissage, l’épaisseur, la taille du texte et le mode esquissé.
  Un double-clic sur un élément ouvre l’inspecteur et donne le focus au texte.
- Les réglages de style s’appliquent aux éléments sélectionnés et deviennent
  les réglages des prochains éléments. Sans sélection, ils règlent le prochain outil.
- **Ctrl/Cmd+D** duplique, Suppr efface, les flèches déplacent de 1 unité
  (10 avec Maj), **Ctrl/Cmd+A** sélectionne tout. Ces raccourcis de dessin
  s’appliquent lorsque le pointeur est sur le canvas et qu’aucun champ n’est édité.
- **Échap** annule le geste en cours ; les modifications terminées sont annulables
  et rétablissables avec l’historique commun.
- La main, Espace+glisser, le bouton central/droit et la molette déplacent la vue.
  Les contrôles de zoom et de cadrage sont ceux des autres éditeurs.

Raccourcis d’outils : **V** sélection, **H** main, **R** rectangle, **E** ellipse,
**D** losange, **L** ligne, **A** flèche, **P** main levée, **T** texte,
**N** note. **F** cadre le dessin.

La barre de configuration commune contient **Afficher la grille**,
**Snap to grid**, le zoom et l’inspecteur. L’aimantation est désactivée par défaut ;
les points du crayon restent libres même lorsqu’elle est activée. Les options
rejoignent le menu compact lorsque la vue se rétrécit.

## Documents et exports

Les dessins se sauvegardent en `.opx` v2, sous le type `whiteboard`, version
métier 1. Le modèle contient les éléments, leurs identifiants, géométries,
textes, couleurs et styles. Le schéma des autres notations reste inchangé.

La session conserve séparément caméra, aimantation, grille, inspecteur et style
du prochain élément. Plusieurs panneaux partagent un document et un historique,
avec des caméras et sélections indépendantes. Dans un dessin libre, la géométrie
fait partie du contenu : déplacer une forme change aussi la révision sémantique.

**File → Export** propose :

- **SVG** : tout le dessin, géométrie vectorielle et texte éditable, sans sélection
  ni grille. L’export utilise des contours réguliers et une police sans-serif ;
  il ne reproduit pas exactement les irrégularités et la police manuscrite du canvas.
- **PNG** : capture de la partie visible du canvas, avec son thème et son réglage
  de grille, sans les poignées de sélection.

## Frontières et limites

| Responsabilité | Implémentation |
| --- | --- |
| Modèle, géométrie, sélection et validation | [whiteboard.rs](../crates/op-core/src/whiteboard.rs) |
| Export SVG | [whiteboard/svg.rs](../crates/op-core/src/whiteboard/svg.rs) |
| Outils et inspecteur | [whiteboard_editor.rs](../crates/op-app/src/whiteboard_editor.rs) |
| Gestes et clavier | [interaction.rs](../crates/op-app/src/whiteboard_editor/interaction.rs) |
| Rendu | [render.rs](../crates/op-app/src/whiteboard_editor/render.rs) |
| Sessions, commandes et enregistrement | [session.rs](../crates/op-app/src/whiteboard_editor/session.rs) |
| Layout | [editor/layout.rs](../crates/op-app/src/editor/layout.rs) |

Le module s’enregistre dans `composition.rs`. Il ne nécessite aucune nouvelle
dépendance, couche web ou modification du catalogue du shell.

Cette première version ne propose pas la collaboration réseau, les images,
les bibliothèques de symboles, les groupes persistants, la rotation, le
presse-papiers d’éléments entre documents, les flèches attachées aux formes
ou l’import/export `.excalidraw`. Les flèches restent des tracés indépendants.
Le texte conserve son contenu mais peut être masqué dans une boîte trop petite.

Un document est borné à 20 000 éléments et 200 000 points ; un geste de crayon
échantillonne au plus 20 000 points. La validation refuse les géométries et styles
invalides. Les créations et duplications qui dépasseraient ces limites sont
refusées avant de remplacer le document. Voir les
[vérifications exécutées](verification.md#dessin-libre).
