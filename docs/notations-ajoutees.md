# DMN, EPC et UML activité : première intégration

**Implémenté : trois nouveaux types de documents, présents dans l’accueil,
le sélecteur de création, avec exemples, sauvegarde `.opx`, historique
et reprise de session.** Il s’agit de premières versions, avec les limites
indiquées ci-dessous ; elles ne constituent pas une parité Looping/Bizagi.

File → New… et le bouton « + » ouvrent maintenant une grille commune de choix
des types. Les formats d’export ci-dessous se trouvent dans File → Export.

## Tables de décision DMN

- Identifiant de document : `dmn.table`, alias `dmn`.
- Colonnes d’entrée et de sortie nommées et typées : `string`, `number`, `boolean`.
- Ajout/suppression de colonnes et règles ; annotation par règle.
- Politiques `UNIQUE`, `FIRST`, `ANY` et `COLLECT` sans agrégation.
- Évaluation sur des valeurs d’entrée JSON ; règles correspondantes et résultats.
- Tests unaires pris en charge : `-`, littéraux, comparaisons numériques,
  intervalles, alternatives séparées par virgule et `not(...)`.
- Sorties limitées aux littéraux texte, nombre, booléen ou `null`.
- Export XML DMN et CSV. Import XML d’une table unique dans le sous-ensemble
  représentable ; les décisions multiples, graphes de dépendances, agrégations
  et constructions structurelles non prises en charge sont refusés.
- Édition et sauvegarde d’un brouillon dont les noms ou expressions sont invalides ;
  les diagnostics empêchent son évaluation. Les cellules doivent garder la
  même dimension que les colonnes pour que le document soit ouvrable.

Ce moteur n’est **pas un interpréteur FEEL complet**. Les fonctions, contextes,
dates, expressions arithmétiques, DRD, BKM et services de décision restent à
développer. La taille est bornée à 128 entrées, 128 sorties et 10 000 règles ;
les expressions de tests sont limitées à 4 096 octets et à 32 imbrications.
Les comparaisons numériques utilisent les nombres flottants `f64` ; la précision
décimale complète de FEEL n’est pas implémentée.

Le contenu est une table défilante ; il n’y a pas de faux zoom de canvas.
L’inspecteur et la barre d’actions réutilisent le layout commun. La documentation
de référence est [DMN 1.5 de l’OMG](https://www.omg.org/spec/DMN/1.5/About-DMN) ;
l’intégration actuelle ne revendique pas un niveau complet de conformité.

## EPC

- Identifiant : `epc`.
- Événements, fonctions et connecteurs AND / OR / XOR.
- Création, sélection simple/multiple, sélection par rectangle, déplacement,
  redimensionnement, duplication de sélection avec `Ctrl/Cmd+D`, suppression et
  édition des noms dans l’inspecteur.
- Connexion par deux clics ou par glissement, libellé de flux et diagnostics
  de degrés entrants/sortants. Les liens directs événement → événement et
  fonction → fonction sont refusés.
- Export SVG, Graphviz DOT et capture PNG du canvas.

La validation est structurelle. L’alternance transitive à travers les connecteurs,
les règles de choix après événement, les extensions organisationnelles/eEPC et
la simulation EPC ne sont pas encore implémentées. Il n’y a pas encore d’import
ou export EPML.

## UML activité

- Identifiant : `uml.activity`.
- Nœud initial, action, décision, fusion, fork, join, fin d’activité et fin de flux.
- Même édition graphique partagée que l’EPC, avec règles métier distinctes.
- Flux de contrôle et libellés de gardes ; interdiction d’entrer dans un nœud
  initial ou de sortir d’une fin. Diagnostics de degrés propres aux nœuds.
- Export SVG, Graphviz DOT et capture PNG du canvas.

Les gardes sont du texte documentaire, elles ne sont pas évaluées. Les flux
d’objets, partitions, pins, régions interruptibles, activités structurées,
simulation et échanges XMI ne sont pas encore implémentés. Référence :
[UML 2.5.1 de l’OMG](https://www.omg.org/spec/UML/2.5.1/About-UML).

## Frontières dans le code

| Responsabilité | Implémentation |
| --- | --- |
| Modèle, évaluation et XML DMN | [dmn.rs](../crates/op-core/src/dmn.rs) |
| Mécanismes de graphe typés | [flow.rs](../crates/op-core/src/flow.rs) |
| Règles EPC | [epc.rs](../crates/op-core/src/epc.rs) |
| Règles activité | [activity.rs](../crates/op-core/src/activity.rs) |
| Éditeur et session DMN | [dmn_editor.rs](../crates/op-app/src/dmn_editor.rs) |
| Éditeur et sessions graphiques | [flow_editor.rs](../crates/op-app/src/flow_editor.rs) |
| Composition statique | [composition.rs](../crates/op-app/src/composition.rs) |

`Diagram<EpcKind>` et `Diagram<ActivityKind>` partagent les mécanismes de graphe,
mais restent des modèles Rust de types différents. Le shell utilise le registre
et ne contient pas de branche pour ces nouvelles notations. Les caméras,
sélections et inspecteurs restent propres à chaque vue ; les modèles et
historiques sont partagés par document. L’évaluation DMN mémorisée est invalidée
si la table ou les valeurs d’entrée changent, y compris entre deux panneaux.

La détection XML BPMN distingue désormais ses éléments `process` des éléments
`decision` DMN, qui utilisent également une racine `definitions`. L’enveloppe
`.opx` reste en version 2, chaque nouveau modèle commence en version 1 ; aucun
schéma historique n’a été modifié.

Voir [les vérifications](verification.md#notations-dmn-epc-et-activite) pour les
contrôles réellement exécutés. La priorité de développement suivante est la
[chaîne de conception de données Looping](parite-looping-bizagi.md).
