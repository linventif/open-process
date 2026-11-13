# Icône d’Open Process

**Statut : intégration implémentée à la demande de l’utilisateur.**

Le dessin maître est [open-process.svg](../crates/op-app/assets/icons/open-process.svg),
un vrai SVG composé de cercles et de chemins, sans image embarquée ni fond.
Il reprend le motif O → P en bleu nuit et ambre. Les espaces entre les traits et
autour du symbole sont transparents. L’ancienne proposition raster reste archivée
dans [source.png](../crates/op-app/assets/icons/source.png), avec les
[prompts de génération](../crates/op-app/assets/icons/source-prompt.txt), mais ne
sert plus à produire les icônes.

## Utilisations implémentées

| Usage | Intégration |
| --- | --- |
| Fenêtre native et barre des tâches | PNG embarqué dans [main.rs](../crates/op-app/src/main.rs), via `ViewportBuilder::with_icon` |
| Barre de menus, accueil et fenêtre À propos | [brand.rs](../crates/op-app/src/brand.rs), PNG transparent dérivé du SVG, traits clairs en thème sombre |
| Exécutable Windows dans l’Explorateur | ICO multirésolution compilé par [build.rs](../crates/op-app/build.rs) et lié au binaire |
| Finder et Dock macOS | ICNS placé dans `Open Process.app/Contents/Resources`, référencé par `Info.plist` |
| Menu d’applications Linux | SVG redimensionnable, PNG et fichier `.desktop` installés dans le compte courant |
| Site, pages FR/EN et erreur 404 | Logo SVG transparent, favicon SVG adapté au thème du navigateur, repli ICO/PNG et icône Apple |
| Application WebAssembly | Favicons et manifeste avec icônes 192/512 px, chemins relatifs compatibles avec `/app/` et Trunk à la racine |
| Présentation du dépôt | Logo dans le [README](../README.md) |

Le manifeste web définit l’icône et l’ouverture autonome dans les navigateurs qui
le prennent en charge. Il n’ajoute pas de fonctionnement hors ligne.

## Régénération des formats

Avec Python 3, Pillow et un moteur SVG (CairoSVG, ou les modules système Rsvg,
PyGObject et Pycairo) installés, depuis la racine du dépôt :

```sh
python3 scripts/generate-icons.py
```

Le script rend le SVG sur un fond transparent et produit les PNG, ICO et ICNS dans
`crates/op-app/assets/icons/`, puis synchronise les copies dans `website/public/`.
Il produit aussi la variante claire des traits pour le thème sombre d’egui.
Il ne relance pas de génération par IA. Ces formats sont conservés dans le dépôt :
les outils de conversion ne sont nécessaires ni pour compiler Rust, ni pour construire le site en CI.
Aucune dépendance Rust supplémentaire n’a été ajoutée.

Le build Windows MSVC utilise `rc.exe`, trouvé sur `PATH` ou dans le Windows SDK.
La variable `RC` peut préciser son chemin. Pour une cible Windows GNU, `WINDRES`
peut désigner le compilateur de ressources. La compilation échoue explicitement
si ce compilateur est absent, afin de ne pas publier un exécutable sans icône.

## Paquets natifs

Le workflow de release appelle [package_release.py](../scripts/package_release.py)
avec Python 3.11 ou ultérieur. Exemple après compilation :

```sh
python3 scripts/package_release.py --target x86_64-unknown-linux-gnu --binary target/release/open-process --output out
```

Le script conserve les exécutables seuls et ajoute les archives Linux/macOS.
Le site préfère ces archives et conserve un repli vers les anciennes releases.
Sur macOS, extraire l’archive puis déplacer `Open Process.app` dans Applications ;
lancer le binaire seul avec `cargo run` ne crée pas de paquet Finder.

Sur Linux, extraire l’archive et lancer `sh install.sh`. Le script installe le
binaire dans `~/.local/bin`, le lanceur dans `~/.local/share/applications` et le
SVG et PNG dans le thème d’icônes utilisateur. `XDG_DATA_HOME` est respecté et
`OP_INSTALL_PREFIX` permet de remplacer le préfixe `~/.local`.

## Validation locale

Le SVG ne contient que des chemins et des cercles, sans fond ni image embarquée.
La transparence des PNG, ICO et ICNS a été vérifiée par lecture des pixels alpha.
La génération des formats, leur lecture et le favicon 32 px ont été vérifiés.
Les deux tests Python, avec un binaire factice, couvrent les quatre formats de distribution,
les permissions dans les archives, les métadonnées du paquet macOS et
l’installation Linux dans un dossier temporaire. Les tests de téléchargement
du site vérifient aussi le choix des archives et le repli vers les anciens binaires.

Les contrôles `cargo check --locked -p op-app` natif et WebAssembly passent.
Le site Astro et l’assemblage Trunk ont produit le dossier web complet :
les neuf tests Bun passent, y compris la présence de l’app, l’identité de son
icône et celle du site, et les chemins du manifeste sous `/app/`. Un build HTML
isolé avec Trunk vérifie aussi les chemins à la racine. Le rendu de l’icône dans
l’accueil et la barre de menus a été inspecté en thèmes clair et sombre.
Les liens documentaires locaux, le fichier `.desktop`, la syntaxe de l’installeur
et `git diff --check` ont aussi été vérifiés.

Après le passage au SVG transparent, `cargo test --locked --workspace -- --test-threads=1`
passe : 23 tests de l’application et 49 tests du cœur. Les précédents échecs de la
refonte parallèle ne sont plus présents dans cet état du dossier de travail.
Ces vérifications locales ne remplacent pas la matrice complète de CI.
L’affichage Windows et macOS et la publication des paquets restent à vérifier
sur leurs plateformes.
