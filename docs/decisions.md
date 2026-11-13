# Décisions d'architecture

**Statut : décisions retenues après validation du plan par l'utilisateur.**

La recommandation est détaillée dans [l'architecture cible](architecture-cible.md).
Leur mise en œuvre est décrite dans [l'architecture actuelle](architecture-actuelle.md).

## D1 — Conserver une application unique et deux crates au départ

La séparation métier/UI existe et permet une migration incrémentale. Des modules
internes apportent les premières frontières sans multiplier les dépendances et
les déplacements. Une crate par notation sera envisageable pour une réutilisation
autonome ou un besoin d'isolation démontré. Le coût du choix initial est que les
frontières internes reposent sur la visibilité Rust, les revues et les tests.

## D2 — Enregistrer les modules statiquement

Un point de composition connaît tous les modules livrés. Le registre expose leur
catalogue au reste du produit. L'ajout d'une notation nécessite une recompilation,
acceptable pour les extensions développées dans ce dépôt. Des bibliothèques
dynamiques demanderaient une ABI, une gestion des versions et une stratégie web
supplémentaires ; ce besoin n'est pas établi aujourd'hui.

## D3 — Conserver des modèles typés par notation

Un modèle universel nœuds/liens déplacerait les invariants dans des propriétés
informelles et représenterait mal les logs ou les tables. Les modèles concrets
restent typés ; la géométrie et les mécanismes d'édition peuvent être partagés.
Le coût assumé est de conserver des validations, codecs et adaptations spécifiques.

## D4 — Utiliser une interface commune à la frontière de l'éditeur

L'enum central des éditeurs a été remplacé par un registre et des sessions
derrière des objets traits. Le shell ne connaît plus chaque variante. Les modèles
restent concrets dans les modules ; les objets traits ne sont pas sérialisés. Les enums locaux, comme les outils
BPMN ou les relations UML, restent adaptés à leur domaine.

## D5 — Déclarer les commandes et vues, évaluer leur disponibilité

Une liste de capacités statiques ne suffit pas : « annuler » dépend de l'historique
et une simulation peut interdire certaines mutations. Les descripteurs annoncent
les actions et vues ; la session en calcule l'état courant. Le shell reste
responsable de la présentation. Un bus d'événements global non typé rendrait le
cheminement plus difficile à suivre ; les demandes hôtes restent explicites.

## D6 — Garder les instantanés et introduire des transactions

L'historique existant offre une base simple. Le premier changement porte sur la
délimitation des actions et sur le suivi des mutations, plutôt que sur des commandes
inverses pour chaque opération. Les instantanés consomment de la mémoire sur les
gros documents ; une optimisation sera décidée sur des mesures représentatives.

## D7 — Séparer l'enveloppe documentaire des contenus métier

Chaque type de document porte une identité stable et sa version de modèle. Le
codec de sa notation possède ses migrations. Le coût est un adaptateur de lecture
des anciens `.opx` et des sessions. Les tests de compatibilité doivent précéder
le changement d'écriture. Voir [le format documentaire](documents-et-compatibilite.md).

## D8 — Découpler les calculs lourds de l'UI après le pilote

Les fonctions métier restent synchrones et testables ; un service adapté à chaque
plateforme gère leur exécution. Cela évite de placer l'asynchronisme dans tous les
modèles. Le pilote Petri utilise un thread natif et un Web Worker qui charge le
même Wasm que l’application. Il ne requiert pas de bibliothèque de tâches supplémentaire.

## Points d'évolution après le pilote

- Les contrats sont maintenant éprouvés sur UML, BPMN, Petri et Mining.
- La granularité des transactions pour le texte et les cellules de matrices.
- Les états de vues sont typés et sauvegardés séparément ; les identifiants des commandes sont explicites.
- Le pilote navigateur utilise un Web Worker ; étendre les tâches aux autres analyses selon les besoins.
- Le budget mémoire de l'historique sur des documents représentatifs.

Les optimisations supplémentaires, notamment le budget mémoire et les deltas
d'historique, restent guidées par des mesures. Aucun découpage supplémentaire en crates n'est requis.
