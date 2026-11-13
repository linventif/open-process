# Traductions de l’application

**Statut : clés stables implémentées pour les trois langues.**

Les messages de l’application Rust utilisent un identifiant indépendant du texte
affiché. Les trois catalogues se trouvent dans `crates/op-core/src/i18n/` :

- [en.rs](../crates/op-core/src/i18n/en.rs) contient les textes anglais.
- [fr.rs](../crates/op-core/src/i18n/fr.rs) contient les traductions françaises.
- [es.rs](../crates/op-core/src/i18n/es.rs) contient les traductions espagnoles.

## Modifier un texte

Pour corriger l’anglais, modifier uniquement la valeur dans `en.rs` :

```rust
("drawing.choose_tool", "Choose a drawing tool, then drag on the canvas."),
```

La clé `drawing.choose_tool` reste identique dans le code et les autres langues.
Une correction de formulation ne nécessite pas de la renommer. Le même principe
s’applique aux valeurs françaises et espagnoles. Si le sens change, vérifier que
les traductions expriment toujours le même message.

## Ajouter un message

Choisir une clé descriptive avec un espace de noms : `file.save`,
`drawing.choose_tool`, `bpmn.validation.start_event`. Utiliser des lettres ASCII
minuscules, des chiffres et des tirets bas, avec des points entre les parties.
Les clés restent des identifiants explicites ; elles ne sont pas calculées depuis
les textes anglais. Les catalogues sont triés par clé.

Déclarer la clé dans les trois catalogues, puis l’utiliser avec :

```rust
tr("drawing.choose_tool");
trf("file.open_failed", &[&file_name, &error]);
```

Le texte anglais de `file.open_failed` est `Cannot open {}: {}`. Conserver les
mêmes paramètres et sauts de ligne dans les traductions. Les paramètres
positionnels `{0}`, `{1}`, etc. permettent de changer leur ordre.

Pour les palettes, commandes, vues et descripteurs du registre, conserver la clé
jusqu’au rendu :

```rust
label: key("file.save"),
// Lors du rendu :
ui.button(tr(command.label));
```

`key` marque une clé pour le contrôle des usages ; cette fonction ne traduit pas.
Ne pas remplacer par une clé les noms et textes saisis par l’utilisateur, les
identifiants métier ni les valeurs de formats importés/exportés. Les documents
et sessions existants conservent leurs données et leur schéma.

## Repli et vérification

`tr` et `trf` utilisent la langue active. `translate(lang, key)` choisit une langue
explicitement. Une traduction absente se replie sur la valeur anglaise ; si la clé
est aussi absente du catalogue anglais, elle est retournée telle quelle pour
rendre l’erreur visible.

Les tests vérifient l’unicité et la syntaxe des clés, la présence des mêmes clés
dans chaque catalogue, les paramètres, les sauts de ligne et l’utilisation de
chaque message. La couverture des polices embarquées inclut les trois langues.
Le contrôle des usages exclut le module de tests après normalisation des fins
de ligne LF et CRLF, pour reconnaître la même frontière sur Linux et Windows.
Un test de régression couvre les deux formats de source.

```sh
cargo test --locked -p op-core i18n
cargo test --locked -p op-app i18n -- --test-threads=1
```

La [vitrine Astro](../website/README.md) utilise déjà des clés indépendantes des
textes dans `website/src/lib/messages.ts` ; ses catalogues restent distincts de
ceux de l’application Rust.
