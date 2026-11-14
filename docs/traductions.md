# Application translations

**Status: stable message keys are implemented for English, French and Spanish.**

English is the default application language and the source and fallback catalogue.
A saved language preference takes precedence at startup. Project documentation
and new original interface copy use English; French and Spanish remain supported.

Rust messages use identifiers independent of displayed text. The catalogues are
in `crates/op-core/src/i18n/`:

- [en.rs](../crates/op-core/src/i18n/en.rs): original English copy.
- [fr.rs](../crates/op-core/src/i18n/fr.rs): French translations.
- [es.rs](../crates/op-core/src/i18n/es.rs): Spanish translations.

## Edit existing copy

To correct English, change only the value in `en.rs`:

```rust
("drawing.choose_tool", "Choose a drawing tool, then drag on the canvas."),
```

Keep `drawing.choose_tool` unchanged in source and the other catalogues. A wording
correction does not require renaming the key. The same rule applies to translated
values. If the meaning changes, review the other languages for consistency.

## Add a message

Choose a descriptive, namespaced key such as `file.save`, `drawing.choose_tool`
or `bpmn.validation.start_event`. Use lowercase ASCII letters, digits and
underscores, with periods between components. Keys are explicit identifiers,
not computed from English text. Keep catalogues sorted by key.

Declare the same key in all three catalogues and use it through:

```rust
tr("drawing.choose_tool");
trf("file.open_failed", &[&file_name, &error]);
```

The English value of `file.open_failed` is `Cannot open {}: {}`. Preserve argument
slots and line breaks in translations. Positional placeholders such as `{0}` and
`{1}` allow translations to reorder arguments.

For palettes, commands, views and registry descriptors, keep the key until rendering:

```rust
label: key("file.save"),
// At render time:
ui.button(tr(command.label));
```

`key` marks a key for usage checks; it does not translate. Do not translate
user-entered names/text, domain identifiers or imported/exported format values.
Existing documents and session schemas remain unchanged.

## Fallback and validation

`tr` and `trf` use the active language. `translate(lang, key)` chooses explicitly.
A missing translation falls back to English; a key missing from English is
returned unchanged to make the error visible.

Tests check unique and valid keys, catalogue parity, placeholders, line breaks
and message usage. Embedded font checks cover all three languages. Source usage
checks normalize LF/CRLF before excluding test modules; a regression test covers
both source formats.

```sh
cargo test --locked -p op-core i18n
cargo test --locked -p op-app i18n -- --test-threads=1
```

The language dropdown and persisted preference use the existing
`open-process-lang` key. `Lang::default()` is English; loading without a saved
preference uses that default. Serialized French and Spanish preferences remain valid.

The [Astro website](../website/README.md) has separate typed catalogues in
`website/src/lib/messages.ts`. Its root route is English, with `/fr/` and `/es/`
for translations; changing the documentation language does not remove those routes.
