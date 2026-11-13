# Instructions pour travailler sur Open Process

## Documentation et statut de l'architecture

- Commencer par [docs/README.md](docs/README.md).
- [L'architecture actuelle](docs/architecture-actuelle.md) décrit l'implémentation.
- [La cible](docs/architecture-cible.md) et les [décisions](docs/decisions.md) sont
  validées. Le [suivi de migration](docs/migration.md) précise les étapes réalisées.
- Pour une extension, suivre [le guide d'ajout](docs/ajouter-une-notation.md).
- Les nouvelles notations déclarent leurs fabriques, commandes, vues, imports et
  exports dans leur module, puis s'enregistrent dans `composition.rs`. Le shell
  consomme le registre injecté ; ne pas réintroduire un catalogue d'éditeurs par enum.
- Les éditeurs graphiques réutilisent `op-app/src/editor/layout.rs` pour la barre
  de configuration, l'inspecteur, ses onglets et le canvas. Ne pas recopier les
  marges, hauteurs, seuils responsive ou widgets de zoom dans chaque notation.
  Les modules fournissent les options et contenus métier via les emplacements prévus.
- Garder les modèles et historiques typés. Les états de vues ne dupliquent pas le
  document. Les sondes `Any` actuelles sont réservées aux tests d'interface.
- Distinguer dans la documentation les comportements implémentés, les propositions
  et les validations réellement exécutées. Après une évolution, actualiser les pages concernées.

## Repères actuels

- `crates/op-core/` : modèles typés, règles métier, analyses, conversions,
  import/export et géométrie ; indépendant d'egui et d'eframe.
- `crates/op-app/` : application egui/eframe, éditeurs BPMN, Petri, UML et Mining,
  accueil, docking, historique, fichiers et persistance native/web.
- `website/` : vitrine Astro/TypeScript/Bun et assemblage du site avec l'application
  WebAssembly sous `/app/`. Voir [son README](website/README.md).
- Versions Rust et dépendances : fichiers `Cargo.toml` du workspace et des crates.

## Règles de contribution

- Examiner `git status` avant d'éditer et préserver le travail déjà présent,
  y compris les fichiers non suivis. Limiter les changements au périmètre demandé.
- Garder les règles et algorithmes métier dans `op-core`, sans dépendance à l'UI,
  aux dialogues système ou au stockage du navigateur.
- Conserver les types et invariants propres aux notations. Mutualiser un mécanisme
  lorsque plusieurs usages concrets justifient son interface.
- Préserver les données `.opx`, les imports/exports existants et la restauration
  des sessions. Documenter et tester toute modification de schéma avec des fichiers antérieurs.
- Vérifier les conséquences native et WebAssembly ; isoler les API spécifiques
  à une plateforme. Un futur calcul long dans le navigateur doit rendre la main à l'UI.
- Utiliser les mécanismes de traduction existants pour les textes de l'interface.
- Ne pas ajouter une dépendance ou extraire une crate pour un simple déplacement
  de fichiers. Expliquer la frontière ou le besoin que cela apporte.

## Vérifications selon le changement

Exécuter les vérifications pertinentes et indiquer celles qui n'ont pas pu l'être.
Pour un changement documentaire seul, vérifier les liens locaux et les différences ;
une compilation complète n'est pas nécessaire.

```sh
# Modèles, règles, formats et algorithmes
cargo test --locked -p op-core

# Application et tests d'interface (comme en CI, exécution séquentielle)
cargo test --locked --workspace -- --test-threads=1

# Compilation native et vérification de la cible web, si elle est installée
cargo check --locked -p op-app
cargo check --locked -p op-app --target wasm32-unknown-unknown

# Espaces et erreurs de patch
git diff --check
```

Pour le site, utiliser les commandes de [website/README.md](website/README.md).
La [CI](.github/workflows/ci.yml) vérifie les tests sur trois OS, la compilation
native release et le build du site avec l'application web. Ne pas présenter une
vérification locale partielle comme une réussite de toute cette matrice.
