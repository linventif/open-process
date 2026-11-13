# Open Process — site vitrine

Site statique autonome, sans framework, installation ni compilation. Le dossier
`website/` est directement publiable sur Cloudflare Pages. L’application Rust et sa
version WebAssembly restent dans `crates/op-app/` ; ce dossier présente le produit.

## Aperçu local

Depuis la racine du dépôt :

```sh
python3 -m http.server 8080 --directory website
```

Ouvrir <http://localhost:8080>. `index.html` fonctionne aussi directement, mais un
serveur permet de vérifier le même comportement qu’en hébergement.

## Cloudflare Pages

Créer un projet **Pages**, connecter le dépôt `linventif/open-process` et utiliser :

| Paramètre | Valeur |
| --- | --- |
| Branche de production | `main` |
| Framework | Aucun / None |
| Répertoire racine | Racine du dépôt (laisser vide) |
| Commande de build | `exit 0` |
| Répertoire de sortie | `website` |

Le [guide officiel Cloudflare pour les sites HTML statiques](https://developers.cloudflare.com/pages/framework-guides/deploy-anything/)
décrit ce déploiement. Pour un projet Pages **Direct Upload**, envoyer simplement
le contenu de `website/` (avec `index.html` à la racine de l’envoi).

`_headers` fournit les en-têtes HTTP ; `404.html` évite le comportement de repli
SPA de Pages sur les chemins inexistants. Aucune variable d’environnement,
fonction serveur ou clé API n’est nécessaire.

## Contenu et liens

- `index.html` : contenu anglais utilisable sans JavaScript, métadonnées et liens.
- `styles.css` : mise en page responsive, polices Inter locales, mouvement réduit.
- `script.js` : traduction FR/EN, préférence locale, onglets de captures,
  agrandissement et résolution de la dernière release publique.
- `assets/` : logo SVG **1C · Lien fléché**, choisi dans la session Claude de création
  du logo, et vraies captures de l’application, converties en WebP.
- `assets/fonts/Inter-LICENSE.txt` : licence SIL OFL des polices distribuées.

L’anglais est la langue par défaut, comme dans l’application. Le bouton FR/EN
mémorise le choix si le navigateur autorise le stockage local. Les captures
montrent l’interface anglaise ; le site traduit leurs légendes et descriptions.

Les liens GitHub ciblent `https://github.com/linventif/open-process`. Une seule
requête publique vers l’API GitHub recherche la dernière release stable. Les
boutons proposent un téléchargement direct **uniquement si le fichier existe**
dans les assets de cette release. Sinon, ils mènent à la liste des versions.
L’absence de release et les erreurs réseau ou limitations d’API ont un message
dédié ; aucun jeton GitHub n’est utilisé.

Les noms de fichiers correspondent au workflow `.github/workflows/release.yml` :

| Système | Asset |
| --- | --- |
| Windows x64 | `open-process-x86_64-pc-windows-msvc.exe` |
| Linux x64 | `open-process-x86_64-unknown-linux-gnu` |
| macOS Apple Silicon | `open-process-aarch64-apple-darwin` |
| macOS Intel | `open-process-x86_64-apple-darwin` |

Le bouton « Ouvrir la version cloud » mène à
<https://open-process.linv.dev>. La session de l’application est conservée
localement dans le navigateur. L’hébergement du site vitrine n’installe pas
automatiquement l’application WebAssembly.

Les nouveaux binaires sont construits par GitHub Actions quand un tag `v*` est
poussé. Sur Linux et macOS, il peut être nécessaire d’autoriser l’exécution du
binaire (`chmod +x`). Les bibliothèques système et l’outil de compilation sont
décrits dans le README principal.

## Vérifications rapides

```sh
node --check website/script.js
git diff --check
```

Vérifier aussi les onglets, l’agrandissement et sa fermeture avec Échap, le choix
FR/EN après rechargement, les deux architectures macOS et la mise en page mobile.
