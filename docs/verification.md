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
