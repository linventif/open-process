# Catalogue de diagrammes à développer

**Propositions produit, classées après confirmation de la priorité Looping et
Bizagi Modeler.** Le statut du logiciel se trouve dans
[les notations ajoutées](notations-ajoutees.md) et dans
[la matrice de couverture](parite-looping-bizagi.md).

L’objectif est de proposer des outils qui s’enchaînent : concevoir des données,
décrire un processus, préciser ses décisions, simuler et publier. Les familles
ci-dessous sont des recommandations pour Open Process ; elles ne sont pas toutes
des fonctions de Looping ou de Bizagi.

## Priorité immédiate : données et processus

| Type / vue | Utilité dans Open Process | Statut / ordre |
| --- | --- | --- |
| MCD Merise | Entités, associations, cardinalités et règles conceptuelles. | Prochain chantier Looping. |
| MLD relationnel | Tables, clés et dépendances dérivées du MCD. | Après le MCD. |
| MPD / ERD physique | Types propres au SGBD, index et contraintes. | Première version relationnelle intégrée avec [génération DDL](base-de-donnees-ddl.md). |
| Dictionnaire de données | Vue structurée des attributs, domaines, définitions et usages. | Avec le MCD ; ce n’est pas un canvas. |
| MOF / diagramme de flux Merise | Acteurs et échanges d’information. | Parité Looping. |
| Cartographie des processus | Organisation et accès aux processus d’un projet. | Parité Modeler. |
| Chaîne de valeur | Activités stratégiques, principales et de support. | Parité Modeler. |
| Matrice RACI | Responsabilités sur les activités et processus. | Parité Modeler ; vue tabulaire. |
| Tableau de scénarios et résultats | Comparaison de variantes de simulation. | Parité Modeler ; vue tabulaire. |
| DMN : table de décision | Règles exécutables avec entrées/sorties typées. | Première version intégrée. |
| DMN : DRD | Dépendances entre décisions, connaissances et données. | Extension après la table initiale. |
| EPC | Chaînes d’événements, fonctions et opérateurs logiques. | Première version intégrée. |
| UML activité | Actions, choix, fusion et parallélisme. | Première version intégrée. |

MCD, MLD et MPD doivent partager les services de transformation et de provenance,
tout en conservant leurs modèles et invariants. RACI, dictionnaire et résultats
réutilisent le shell sans recevoir de faux contrôles de canvas.

## Extensions utiles après les fonctions prioritaires

Un [dessin libre](dessin-libre.md) est maintenant disponible pour les esquisses,
formes, notes et tracés à main levée. Il ne remplace pas les modèles typés des
familles ci-dessous.

| Type | Usage proposé | Dépendance ou particularité |
| --- | --- | --- |
| UML séquence | Décrire les messages entre acteurs et composants. | Layout temporel, lignes de vie, fragments et activations. |
| UML états | Décrire le cycle de vie d’une commande ou d’un objet. | Hiérarchie, transitions, gardes et actions. |
| UML composants | Montrer les contrats et dépendances du logiciel. | Interfaces fournies/requises. |
| UML déploiement | Relier applications, nœuds et environnements. | Ressources d’infrastructure. |
| UML packages | Organiser les domaines et leurs dépendances. | Références vers les éléments existants. |
| UML objets | Illustrer des instances et leurs liens. | Lien avec les classes typées. |
| UML communication | Montrer les échanges numérotés sur un réseau d’objets. | Peut partager un modèle d’interaction avec la séquence. |
| UML structure composite | Décrire les parties et ports d’un composant. | Conteneurs et connexions typées. |
| UML timing | Suivre les états et contraintes temporelles. | Axe temporel spécifique. |
| Vue globale d’interaction | Composer plusieurs interactions. | Références et navigation entre diagrammes. |
| Profil UML | Définir stéréotypes et propriétés d’un domaine. | Métamodèle et validation à cadrer. |
| Organigramme | Relier équipes, rôles et responsabilités. | Réutiliser le répertoire de ressources de Modeler. |
| DFD | Décrire transformations et circulation des données. | Décomposition par niveau et magasins de données. |
| MCT Merise | Décrire événements et opérations conceptuelles. | Proposition complémentaire ; ne pas l’attribuer à Looping sans preuve. |
| MOT Merise | Affecter opérations, acteurs, lieux et temporalité. | Dépend du répertoire d’acteurs. |
| Flowchart | Décrire un algorithme ou une procédure simple. | Petite notation distincte de BPMN et de l’activité UML. |
| C4 contexte | Montrer personnes, système et systèmes voisins. | Références communes aux autres niveaux C4. |
| C4 conteneurs | Décrire applications, services et stockages. | Hiérarchie et liens typés. |
| C4 composants | Détailler les composants d’un conteneur. | Navigation vers UML et déploiement. |
| C4 dynamique | Expliquer un scénario entre composants. | Échanges ordonnés. |
| C4 déploiement | Placer les conteneurs dans les environnements. | Liens avec la vue de déploiement. |
| Mind map | Cadrer un problème et organiser des idées. | Arbre, pliage et placement automatique. |
| WBS | Décomposer un livrable en lots de travail. | Hiérarchie et identifiants de lots. |
| Gantt | Planifier tâches, dépendances et jalons. | Axe temporel ; un éditeur de graphe libre ne suffit pas. |
| PERT / CPM | Calculer dépendances et chemin critique. | Algorithmes et hypothèses de durée explicites. |
| Ishikawa | Explorer les causes d’un problème. | Arbre de causes et catégories. |
| Customer journey | Décrire étapes, contacts, ressentis et difficultés. | Tableau/matrice à plusieurs dimensions. |
| Story map | Organiser les usages et incréments produit. | Références vers les cas d’utilisation. |
| Kanban | Suivre les éléments de travail d’un projet. | Vue opérationnelle ; ne pas confondre avec un diagramme UML. |

## Horizon Visual Paradigm

Après la parité prioritaire, étudier ArchiMate, SysML, CMMN, SoaML, BMM,
cartographie de capacités, exigences et traçabilité, modèles d’API, wireframes
et parcours d’écrans. Il faudra aussi comparer la génération/rétroconception de
code, les transformations entre modèles, les analyses d’impact et le référentiel
partagé. Ces capacités ne se résument pas à un catalogue de symboles.

Le [catalogue officiel Visual Paradigm](https://www.visual-paradigm.com/features/)
confirme la diversité des domaines à couvrir. Il sert ici à préparer une direction,
sans annoncer une parité ni retenir dès maintenant toutes ses fonctions.

## Conditions d’entrée d’une nouvelle notation

Chaque type doit apporter un cas d’usage, un modèle typé, des règles de validation,
une fabrique enregistrée, au moins un exemple, la sauvegarde `.opx`, l’historique,
les traductions et un format d’échange pertinent. Un éditeur graphique réutilise
`DiagramLayout` ; un axe temporel ou une table possède son layout de contenu
adapté dans le même shell. Suivre [le guide d’ajout](ajouter-une-notation.md).
