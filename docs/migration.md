# Migration vers l'architecture modulaire

**Statut : plan autorisé par l'utilisateur et exécuté.**

La [cible](architecture-cible.md) reste la référence de conception. Les chemins et
interfaces réellement utilisés sont dans [l'architecture actuelle](architecture-actuelle.md).

## Suivi des étapes

| Étape | Résultat livré |
| --- | --- |
| 0 — Valider la direction | Direction retenue après la relecture documentaire et la demande de lancer le plan. |
| 1 — Référence et séparation | Tests initiaux relevés ; cinq fixtures `.opx` v1 conservées ; rendu, interactions et panneaux extraits des éditeurs. |
| 2 — Pilote UML | Session typée commune, modèle/historique uniques et caméras indépendantes de plusieurs vues. |
| 3 — Catalogue | Registre statique injecté, déclarations dans chaque module, accueil/menus/imports/exports/vues pilotés par les déclarations. BPMN, Petri, UML et Mining utilisent le contrat. |
| 4 — Stockage | Enveloppe ouverte v2, lecture v1, reprise des sessions RON v1/v2 vers JSON v3, conservation des fichiers non pris en charge et tests de compatibilité. |
| 5 — Transactions | Instantanés conservés, annulation de geste, historique Mining, état sauvegardé par contenu et révisions sémantiques/graphiques séparées. Layout graphique partagé pour configuration et inspecteur. |
| 6 — Tâches | Analyse Petri en thread natif ou Web Worker ; abandon des entrées périmées, annulation et réutilisation du cache après un déplacement. |

La migration conserve les deux crates et les chemins publics d'`op-core`. Les
noms de dossiers de la cible étaient des repères ; les extractions ont gardé les
modules historiques afin de limiter les déplacements et de préserver les travaux locaux.

## Vérification de l'extensibilité

Un module de table de décision, limité aux tests, s'enregistre avec son codec et
sa session sans canvas et sans modifier le shell. Il exerce la création, les
fabriques, le document commun et la sauvegarde. Les identifiants dupliqués et les
imports ambigus sont aussi vérifiés.

Cette preuve n'ajoute pas une fonctionnalité DMN ou une nouvelle notation visible.
Le choix du prochain éditeur métier reste une décision produit séparée.

## Scénarios vérifiés

- Interactions BPMN, Petri et UML, accueil, modes manuels et traductions existants.
- Plusieurs vues d'un document avec caméras distinctes, historique commun et fermeture d'une seule vue.
- Annulation par Échap d'un déplacement sans étape d'historique supplémentaire.
- Retour par annulation à l'état sauvegardé ; fichiers inconnus récupérables octet pour octet.
- Cinq familles de fichiers v1 migrées et réencodées sans perte du contenu métier.
- Sessions RON v1/v2, vues Petri historiques, nouvelle session JSON et refus d'une session malformée.
- Résultat d'analyse remplacé après changement du modèle ; déplacement graphique qui conserve le cache.
- Test Firefox du worker compilé : calcul, annulation, retour d'erreur et activité du fil principal pendant le traitement.

## Contrôles et limites

Les commandes et résultats finaux de cette migration sont consignés dans
[verification.md](verification.md). Les contrôles locaux ne remplacent pas la
matrice Linux/Windows/macOS de la CI.

Les calculs Mining et l'évaluation n'ont pas été déplacés dans le service de
tâches : leur migration dépendra de mesures et d'un besoin identifié. Le format
v2 n'est pas annoncé comme lisible par les anciennes versions de l'application.
L'API de compatibilité `to_json(&Model)` conserve intentionnellement l'écriture v1.

Les extensions futures doivent suivre [le guide d'ajout](ajouter-une-notation.md).
Les évolutions du format et les suppressions de façades de compatibilité demandent
toujours des fixtures historiques et une vérification de leurs consommateurs.
