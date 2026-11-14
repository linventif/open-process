# Open Process documentation

**Status: English is the default documentation language. The modular architecture
is implemented; individual guides distinguish available features from proposals.**

Start with [getting started](getting-started.md) to use the application,
[CONTRIBUTING.md](../CONTRIBUTING.md) to contribute, or
[the current architecture](architecture-actuelle.md) to explore the code.
For questions, bugs and suggestions, see [SUPPORT.md](../SUPPORT.md).

## Reading guide

| Document | Purpose |
| --- | --- |
| [Getting started](getting-started.md) | Create, save and export your first document. |
| [Data and storage](data-and-storage.md) | Local sessions, backups, network requests and deletion. |
| [Current architecture](architecture-actuelle.md) | Actual implementation, modules and entry points. |
| [Target architecture](architecture-cible.md) | Approved design principles, responsibilities and dependencies. |
| [Architecture decisions](decisions.md) | Accepted tradeoffs and alternatives. |
| [Documents and compatibility](documents-et-compatibilite.md) | File storage, migrations and preservation of existing data. |
| [Migration record](migration.md) | Completed architecture migration steps. |
| [Interface zones](interface.md) | Shared UI vocabulary, annotated SVG and common layout components. |
| [Translations](traductions.md) | Edit interface copy and add stable message keys. |
| [Looping and Bizagi Modeler goals](parite-looping-bizagi.md) | Feature coverage, gaps and proposed delivery stages. |
| [Proposed diagram catalogue](catalogue-diagrammes.md) | Possible future model families after the product priorities. |
| [DMN, EPC and UML activities](notations-ajoutees.md) | Implemented features and limits of the initial integrations. |
| [Database design and DDL](base-de-donnees-ddl.md) | Tables and SQL generation for PostgreSQL, MySQL/MariaDB and SQLite. |
| [Free drawing](dessin-libre.md) | Shapes, arrows, notes and freehand strokes on desktop and web. |
| [Mermaid editor](mermaid.md) | Source editing, preview, templates, source/SVG export and UML class conversions. |
| [Application icon](icone.md) | Master artwork, generated formats and platform integration. |
| [Adding a notation](ajouter-une-notation.md) | Implement and register a notation module. |
| [Verification record](verification.md) | Historical local checks, outcomes and their limits. |
| [Contributing](../CONTRIBUTING.md) | Set up a development environment, check changes and submit a PR. |
| [Repository rules](../AGENTS.md) | Architecture constraints, language policy and relevant checks. |
| [Governance](../GOVERNANCE.md) | Decisions, reviews and releases. |
| [Changelog](../CHANGELOG.md) | Curated changes and published builds. |

## Architecture overview

Open Process is a modular monolith: one native/WebAssembly application, with
modules for each notation. The workspace keeps two Rust crates, `op-core` for
typed models and domain operations and `op-app` for the UI and platform services.

Each module owns its model, rules, editor and format declarations. A statically
composed registry supplies document types, commands, views and formats to the
shell. Graphical editors share canvas mechanisms and layout; text and table
editors use the same document lifecycle without pretending to be canvases.

The app writes `.opx` v2, reads older files and restores legacy sessions through
tested migrations. Historical verification results are records of the checkout
at each change, rather than a current certification of the entire project.

## Language and stable links

Project documentation and original interface copy use English. The application
starts in English without a saved preference; French and Spanish remain
available. Contributions in French are welcome. Existing documentation filenames
are retained so links continue to work, even where their names originated in French.
