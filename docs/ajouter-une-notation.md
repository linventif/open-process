# Ajouter une notation

Ce guide décrit les contrats implémentés. Un module peut proposer un diagramme,
plusieurs types proches ou un document sans canvas, comme une table.

## 1. Modèle et codec dans op-core

Définir un modèle typé, ses invariants et ses opérations sans dépendance à egui.
Choisir un identifiant stable, par exemple `state-machine`, et une version métier.
`DocumentCodec<M>` définit l'encodage et le décodage ; `JsonCodec<M>` fournit la
version initiale JSON et refuse les versions métier inconnues.

Pour une évolution de schéma, implémenter les migrations dans le codec de la
notation avant d'augmenter sa version. Conserver des fixtures historiques. Les
algorithmes consomment le modèle typé ; l'enveloppe JSON sert à l'intégration et au stockage.

## 2. Adaptateur dans op-app

Implémenter `EditorBackend` près de l'éditeur. Les adaptateurs existants se trouvent
dans les fichiers `*_editor/session.rs`. Les responsabilités sont :

- Fournir le modèle persistant, son nom et les opérations de renommage.
- Définir un état de vue typé contenant caméra, inspecteur et interactions locales.
- Capturer/restaurer cet état et sérialiser seulement les éléments utiles à la reprise de session.
- Déclarer commandes, disponibilité, vues secondaires et formats d'export.
- Produire les clés sémantiques et graphiques ; une relation dérivée de la géométrie,
  comme l'appartenance à un système UML, doit rester dans la clé sémantique.
- Regrouper les mutations dans l'historique et permettre l'annulation d'une transaction.

`Session::boxed(editor)` apporte l'interface applicative, la gestion des vues et
le suivi des révisions. Le modèle et l'historique n'entrent pas dans l'état d'une vue.
Un document textuel peut utiliser `()` comme état de vue et ignorer le canvas.

Pour un éditeur graphique, utiliser `editor::layout::DiagramLayout` pour
l'inspecteur, ses onglets (`inspector_tabs`), son contenu défilant (`inspector_body`),
la configuration et le canvas. Le module fournit `CanvasControls`, `FooterStatus`
et les options propres à la notation. La hauteur de barre, les marges, le zoom
et le passage au menu compact restent définis dans ce composant commun.
Les actions métier supplémentaires peuvent occuper l'emplacement `tools`.

Conserver `snap`, l'ouverture d'inspecteur et les éventuels indicateurs de problèmes
dans l'état de vue capturé et restauré ; leur sérialisation est indépendante du
modèle. Les anciennes valeurs booléennes d'inspecteur restent lisibles. Les styles
de dessin déjà sauvegardés dans les modèles restent persistants.

L'export standard utilise `ExportData`. Pour plusieurs formats, redéfinir
`exports()` et `export_as(id)` ; l'application construit les boutons à partir
des descripteurs. Le PNG est une commande de capture du canvas quand elle est disponible.

## 3. Enregistrement

Le module expose une fonction `register(&mut DiagramRegistry)` qui ajoute :

- Un `DiagramDescriptor` par type de document, avec fabrique vide et fabrique d'ouverture.
- Ses importeurs externes, s'il en possède ; un export n'implique pas un import.
- Ses exemples et éventuels lanceurs de workflow pour l'accueil.

Déclarer le module Rust puis appeler son `register` dans
[composition.rs](../crates/op-app/src/composition.rs). Les identifiants de types,
les alias et les exemples sont vérifiés pour éviter les doublons.

Les commandes et vues utilisent des identifiants explicites tels que
`state-machine.validate`. Les libellés passent par `key`, `tr` ou `trf` et les
traductions. Les filtres de fichiers sont issus des importeurs enregistrés.

## 4. Workflows et calculs

Une conversion produit un nouveau `Document` et des diagnostics. L'adaptateur
émet `HostEvent::Open` et, si nécessaire, `HostEvent::Message`. Il ne manipule pas
les onglets de l'application ni l'éditeur cible.

Pour un calcul long, partir d'un instantané immutable et identifier toutes ses
entrées : contenu métier, paramètres et langue si les diagnostics en dépendent.
Le service `platform/jobs.rs` fournit le transport et l'annulation. Ajouter le
raccordement du traitement web dans la composition ; le traitement typé reste dans
son module. Ne jamais remplacer un résultat récent par une réponse pour des entrées périmées.

## 5. Vérifier l'extension

Tester les invariants, le codec et ses versions, les transactions et le retour à
l'état sauvegardé. Vérifier plusieurs vues d'un même document et les conversions
éventuelles. Compiler le natif et le web ; si un worker est ajouté, vérifier son
fonctionnement avec le livrable Trunk réel, à la racine et sous le chemin de publication.

Le test `extension_without_canvas_uses_the_same_document_shell` dans
[editor/registry.rs](../crates/op-app/src/editor/registry.rs) donne une preuve minimale
d'intégration d'un type absent du catalogue du produit.
