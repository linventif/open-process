# Open Process — site Astro, TypeScript et Bun

Un seul projet Cloudflare Pages sert la vitrine et l’application :

- <https://open-process.linv.dev/> : présentation anglaise, générée en HTML par Astro.
- <https://open-process.linv.dev/fr/> : présentation française, avec sa propre URL.
- `/es/` : présentation espagnole, générée avec les mêmes composants et traductions typées.
- <https://open-process.linv.dev/app/> : application Rust compilée en WebAssembly.

## Développement

Installer Bun 1.4.0 et Node.js 24, puis :

```sh
cd website
bun install --frozen-lockfile
bun run dev
```

Ouvrir <http://localhost:4321>. Le serveur Astro recharge les pages modifiées.
Pour développer l’application Rust en parallèle, utiliser `trunk serve` dans
`crates/op-app/` ; l’application complète est disponible dans l’aperçu de production.

```sh
bun run build
bun test
bun run preview
```

Le build vérifie les types TypeScript et Astro, génère la vitrine, puis compile
l’application avec Trunk dans `dist/app/`. Il nécessite Rust et la cible
`wasm32-unknown-unknown`. Sur Linux x64, le script installe Rust et Trunk 0.22.1
s’ils manquent, dans le dossier ignoré `.tools/`, sans modifier le profil shell.
Sur Windows ou macOS, installer Rust et Trunk avant le premier build.

`bun run build:site` et `bun run build:app` permettent de reconstruire chaque
partie séparément ; le premier recrée `dist/`, il faut donc le lancer avant le second.
L’aperçu sert le dossier final : l’anglais à `/`, le français à `/fr/`, l’espagnol
à `/es/` et l’app à `/app/`.

## Cloudflare Pages

Connecter le dépôt `linventif/open-process` à un projet **Pages** :

| Paramètre | Valeur |
| --- | --- |
| Branche de production | `main` |
| Framework | Astro |
| Répertoire racine | `website` |
| Commande de build | `bun install --frozen-lockfile && bun run build` |
| Répertoire de sortie | `dist` |
| Variable `BUN_VERSION` | `1.4.0` |
| Variable `NODE_VERSION` | `24` |
| Domaine personnalisé | `open-process.linv.dev` |

Le domaine se configure dans **Custom domains** du projet Pages. Le build contient
les deux parties : aucun deuxième projet ni proxy n’est nécessaire pour `/app/`.
La première compilation Rust télécharge la chaîne d’outils et les dépendances.

Le domaine de production est configuré dans `src/lib/site.ts`. La variable
optionnelle `SITE_URL` peut le remplacer par une autre origine HTTPS. Les previews
Cloudflare gardent les URL canoniques de production et déclarent `noindex` et un
`robots.txt` qui interdit l’indexation. Le dossier `dist/` peut aussi être envoyé
par **Direct Upload** après `bun run build`.

Voir les documentations officielles [Astro sur Cloudflare](https://docs.astro.build/en/guides/deploy/cloudflare/)
et [configuration des builds Pages](https://developers.cloudflare.com/pages/configuration/build-configuration/).

## SEO et contenu

- HTML anglais, français et espagnol généré au build : titres, descriptions, contenu et liens
  restent lisibles sans JavaScript.
- URL canoniques, `hreflang` anglais/français/espagnol/x-default, Open Graph et cartes Twitter.
- Données structurées `SoftwareApplication` avec licence, gratuité et lien vers l’app.
- `sitemap-index.xml`, sitemap localisé et `robots.txt` générés au build ; l’app et la
  page 404 sont exclus du sitemap, et `/app/` porte un en-tête `noindex`.
- CSS et TypeScript regroupés avec noms de fichiers versionnés, captures WebP,
  dimensions d’images explicites, police Inter locale et cache des assets.
- Navigation clavier, onglets de captures et agrandissement conservés. Un menu
  déroulant de langue (`details`/`summary`) affiche EN, FR ou ES et propose les
  trois langues avec leur nom natif et la langue active cochée. Ses liens
  fonctionnent sans JavaScript ; avec JavaScript, Échap ou un clic extérieur
  referme le menu.

Les traductions sont dans `src/lib/messages.ts`, le contenu dans
`src/components/Landing.astro`, les métadonnées dans `src/layouts/Page.astro` et les
interactions dans `src/scripts/site.ts`. `public/assets/` conserve le logo, les
captures et les licences des polices. `_headers` applique des politiques séparées
à la vitrine et à l’application, qui utilise WebAssembly et des scripts générés.

Le logo SVG transparent et les favicons de `public/icons/` dérivent du dessin
vectoriel commun à l’application native et web. Voir [l’icône du logiciel](../docs/icone.md)
pour régénérer tous les formats sans modifier le dessin.

## Téléchargements et CI/CD

Une requête publique à l’API GitHub charge la dernière release stable. Les liens
se transforment en téléchargements directs si le binaire existe, sinon ils restent
dirigés vers les releases. Aucun jeton n’est exposé dans le navigateur.

| Système | Asset |
| --- | --- |
| Windows x64 | `open-process-x86_64-pc-windows-msvc.exe` |
| Linux x64 | `open-process-x86_64-unknown-linux-gnu.tar.gz` |
| macOS Apple Silicon | `open-process-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `open-process-x86_64-apple-darwin.tar.gz` |

Les archives macOS contiennent `Open Process.app` avec son icône Finder/Dock.
L’archive Linux fournit le binaire, le lanceur `.desktop`, l’icône et `install.sh`
pour installer l’application dans le compte courant. Les exécutables seuls
restent publiés ; les liens du site les utilisent en repli lorsqu’une ancienne
release n’a pas d’archive. L’icône Windows est embarquée dans le `.exe` au build.

GitHub Actions vérifie le build Pages complet et les tests Bun avant de publier
les releases natives. Chaque push sur `main` publie une release
`main-<numéro du run>-<SHA court>` après réussite des tests et des quatre builds.
Les tags `v*` restent pris en charge. Cloudflare Pages construit et déploie le site
avec son intégration Git ; l’ancien déploiement GitHub Pages est remplacé.

Les tests Bun vérifient les liens de releases, les métadonnées des trois langues,
le sitemap et la présence de l’app dans le dossier final. Les tests sur le dossier
final se lancent après le build. La validation visuelle couvre aussi les onglets,
la navigation FR/EN, le dialogue de capture, les téléchargements et le mobile.
L’ajout de l’espagnol et des liens EN/FR/ES est vérifié par le build et les tests
HTML ; aucune nouvelle validation visuelle dans un navigateur n’a été exécutée.
