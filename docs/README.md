# Architecture d'Open Process

**Statut : direction validée, architecture modulaire implémentée.**

L'objectif est de pouvoir ajouter de nombreuses familles de diagrammes sans
répartir leur intégration dans toute l'application. Ces pages décrivent la cible, les décisions et les étapes réalisées.
Commencer par l'architecture actuelle pour retrouver l'implémentation.

La migration a intégré les éditeurs UML et l'écran d'accueil déjà présents dans
le dossier de travail, en préservant les autres modifications locales.

## Parcours de lecture

| Document | À quoi il sert |
| --- | --- |
| [Architecture cible](architecture-cible.md) | Comprendre l'organisation souhaitée, les responsabilités et les dépendances. |
| [Décisions retenues](decisions.md) | Retrouver les compromis et alternatives considérés. |
| [Documents et compatibilité](documents-et-compatibilite.md) | Examiner le stockage, les migrations et la protection des fichiers existants. |
| [Plan de migration](migration.md) | Consulter les étapes réalisées et leurs résultats. |
| [Architecture actuelle](architecture-actuelle.md) | Retrouver l’implémentation dans le code. |
| [Zones de l’interface](interface.md) | Nommer les zones avec un layout SVG et retrouver les composants communs. |
| [Traductions](traductions.md) | Modifier les textes et ajouter des messages avec des clés stables. |
| [Cible Looping et Bizagi Modeler](parite-looping-bizagi.md) | Comparer les fonctions documentées, mesurer les écarts et ordonner les chantiers. |
| [Catalogue de diagrammes proposé](catalogue-diagrammes.md) | Choisir les prochaines familles de modèles après les priorités produit. |
| [DMN, EPC et UML activité](notations-ajoutees.md) | Retrouver les fonctions et limites des trois nouvelles intégrations. |
| [Base de données et DDL](base-de-donnees-ddl.md) | Concevoir des tables et générer le SQL PostgreSQL, MySQL/MariaDB et SQLite. |
| [Dessin libre](dessin-libre.md) | Dessiner formes, flèches, notes et tracés à main levée, en natif et sur le web. |
| [Mermaid editor](mermaid.md) | Source editing, preview, templates and source/SVG export. |
| [Icône du logiciel](icone.md) | Retrouver les formats et l’intégration de l’identité visuelle. |
| [Ajout d’une notation](ajouter-une-notation.md) | Implémenter un module et l’enregistrer. |
| [Vérifications](verification.md) | Retrouver les tests et contrôles exécutés. |
| [Instructions de contribution](../AGENTS.md) | Savoir comment intervenir dès maintenant dans le dépôt. |

## Architecture en quelques lignes

Conserver Rust, egui/eframe et les deux crates actuelles au début. Organiser le
produit comme un **monolithe modulaire** : une application unique, des modules
par notation, et des contrats communs pour leur intégration.

Chaque module possède ses modèles typés, ses règles et son interface. Un registre
statique fournit les types de documents, les commandes, les vues et les formats
disponibles. Le canvas propose des outils réutilisables aux éditeurs graphiques.
Les documents textuels et tabulaires peuvent utiliser les mêmes services applicatifs.

Les fichiers existants restent lisibles ; l'application écrit désormais les `.opx` v2
et restaure les anciennes sessions, avec des migrations testées.

## Pour poursuivre

1. Consulter [le suivi de migration](migration.md) et [les vérifications](verification.md).
2. Suivre [le guide d'ajout d'une notation](ajouter-une-notation.md) pour la prochaine famille de documents.
3. Conserver les tests de compatibilité lors de chaque évolution du stockage.

Les noms de dossiers, d'interfaces et d'identifiants montrés dans la proposition
sont des repères de conception. Les signatures effectivement utilisées sont documentées par le guide d'ajout.
Le pilote de calculs utilise directement la dépendance Wasm déjà présente transitivement.
