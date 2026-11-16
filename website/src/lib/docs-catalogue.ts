import { localizedPath, type Language } from "./messages";
import { repository } from "./site";

export const docGroups = ["general", "tools", "development", "records"] as const;
export type DocGroup = typeof docGroups[number];
export interface DocEntry {
  slug: string;
  source: string;
  group: DocGroup;
  title: string;
  description: string;
  keywords?: string;
}

// Paths identify the maintained repository sources; the site does not copy their prose.
export const documentation: readonly DocEntry[] = [
  { slug: "getting-started", source: "docs/getting-started.md", group: "general", title: "Getting started", description: "Open the app, create a document, save your work and export it.", keywords: "débuter démarrer premiers pas empezar inicio" },
  { slug: "interface", source: "docs/interface.md", group: "general", title: "Interface and controls", description: "Navigation, panels, canvas, zoom, shortcuts and exam mode.", keywords: "raccourcis clavier examen atajos lienzo" },
  { slug: "data-and-storage", source: "docs/data-and-storage.md", group: "general", title: "Saving and local data", description: "Automatic copies, browser storage, backups, privacy and deletion.", keywords: "sauvegarde enregistrer stockage données guardar almacenamiento" },
  { slug: "export-formats", source: "docs/export-formats.md", group: "general", title: "Imports and exports", description: "File formats, Mermaid projections and what each export preserves.", keywords: "importer exporter formats archivos" },
  { slug: "documents-and-compatibility", source: "docs/documents-et-compatibilite.md", group: "general", title: "Documents and compatibility", description: "The .opx format, migration, restoration and recovery behavior.", keywords: "fichiers compatibilité récupération recuperación" },
  { slug: "troubleshooting", source: "docs/troubleshooting.md", group: "general", title: "Troubleshooting", description: "Recover a document, understand an unavailable result and report a problem.", keywords: "aide problème erreur dépannage ayuda error" },
  { slug: "free-drawing", source: "docs/dessin-libre.md", group: "tools", title: "Free drawing", description: "Shapes, attached arrows, text, freehand strokes, clipboard and SVG.", keywords: "dessin libre dibujo whiteboard" },
  { slug: "database", source: "docs/base-de-donnees-ddl.md", group: "tools", title: "Relational databases", description: "Tables, keys, relations, indexes and SQL for three database dialects.", keywords: "bases de données bases de datos PostgreSQL MySQL SQLite" },
  { slug: "uml-classes", source: "docs/uml-classes.md", group: "tools", title: "UML classes", description: "Classes, interfaces, members, relationships and Mermaid conversion." },
  { slug: "bpmn", source: "docs/bpmn.md", group: "tools", title: "BPMN processes", description: "Tasks, events, gateways, pools, lanes, validation and conversions.", keywords: "processus procesos" },
  { slug: "uml-use-cases", source: "docs/uml-use-cases.md", group: "tools", title: "UML use cases", description: "Actors, system boundaries, associations, include and extend.", keywords: "cas utilisation casos uso" },
  { slug: "petri-nets", source: "docs/petri-nets.md", group: "tools", title: "Petri nets", description: "Token game, matrices, invariants, reachability and workflow soundness.", keywords: "réseaux jetons matrices redes fichas" },
  { slug: "process-discovery", source: "docs/process-discovery.md", group: "tools", title: "Process discovery", description: "Import event logs, follow Alpha, check exercises and replay traces.", keywords: "découverte journaux événements minería registros trazas" },
  { slug: "bpmn-evaluation", source: "docs/bpmn-evaluation.md", group: "tools", title: "BPMN costs and durations", description: "Trace costs and expected durations for supported XOR/AND processes.", keywords: "coûts durées évaluation costes duración CA TA" },
  { slug: "dmn", source: "docs/dmn.md", group: "tools", title: "DMN decision tables", description: "Typed columns, rules, hit policies, JSON inputs and evaluation.", keywords: "décision décisions tablas decisión" },
  { slug: "epc", source: "docs/epc.md", group: "tools", title: "EPC diagrams", description: "Events, functions, logical connectors and structural validation.", keywords: "événements fonctions eventos funciones" },
  { slug: "uml-activities", source: "docs/uml-activities.md", group: "tools", title: "UML activities", description: "Actions, decisions, forks, joins, guards and control flows.", keywords: "activités actividades" },
  { slug: "mermaid", source: "docs/mermaid.md", group: "tools", title: "Mermaid diagrams", description: "Source and Builder, family coverage, previews and native conversions." },
  { slug: "notation-coverage", source: "docs/notations-ajoutees.md", group: "tools", title: "DMN, EPC and activity coverage", description: "Detailed supported subsets and implementation limits." },
  { slug: "overview", source: "README.md", group: "development", title: "Project overview", description: "Features, installation, builds, releases and community resources." },
  { slug: "architecture", source: "docs/architecture-actuelle.md", group: "development", title: "Current architecture", description: "Typed models, editor modules, registry, persistence and background jobs." },
  { slug: "target-architecture", source: "docs/architecture-cible.md", group: "development", title: "Target architecture", description: "Approved boundaries and design principles." },
  { slug: "decisions", source: "docs/decisions.md", group: "development", title: "Architecture decisions", description: "The rationale behind module registration, state, history and saving." },
  { slug: "adding-a-notation", source: "docs/ajouter-une-notation.md", group: "development", title: "Adding a notation", description: "Declare a model, session, commands, views and formats." },
  { slug: "translations", source: "docs/traductions.md", group: "development", title: "Translations", description: "Message catalogues, stable keys, language fallback and checks." },
  { slug: "visual-identity", source: "docs/identite-graphique.md", group: "development", title: "Visual identity", description: "Classic styling, color modes, design tokens and gallery captures." },
  { slug: "icons", source: "docs/icone.md", group: "development", title: "Icons and desktop packages", description: "Master artwork, generated icons and platform packaging." },
  { slug: "website", source: "website/README.md", group: "development", title: "Website development", description: "Astro, Bun, documentation publishing and Cloudflare Pages." },
  { slug: "contributing", source: "CONTRIBUTING.md", group: "development", title: "Contributing", description: "Set up a development environment, run checks and submit changes." },
  { slug: "support", source: "SUPPORT.md", group: "development", title: "Support", description: "Questions, bug reports, feature requests and useful reproduction details." },
  { slug: "governance", source: "GOVERNANCE.md", group: "development", title: "Governance", description: "Project decisions, reviews and release responsibilities." },
  { slug: "security", source: "SECURITY.md", group: "development", title: "Security", description: "Private vulnerability reporting and the supported update scope." },
  { slug: "code-of-conduct", source: "CODE_OF_CONDUCT.md", group: "development", title: "Code of conduct", description: "Community expectations and reporting channels." },
  { slug: "repository-rules", source: "AGENTS.md", group: "development", title: "Repository rules", description: "Architecture constraints, language policy and required checks." },
  { slug: "roadmap", source: "docs/parite-looping-bizagi.md", group: "records", title: "Product goals and coverage", description: "Implemented, partial and proposed Looping/Bizagi-inspired capabilities." },
  { slug: "diagram-catalogue", source: "docs/catalogue-diagrammes.md", group: "records", title: "Proposed diagram catalogue", description: "Possible future families, separate from the available tools." },
  { slug: "migration", source: "docs/migration.md", group: "records", title: "Migration record", description: "Completed architecture steps and their verification scope." },
  { slug: "verification", source: "docs/verification.md", group: "records", title: "Verification history", description: "Historical local checks and their practical limits." },
  { slug: "changelog", source: "CHANGELOG.md", group: "records", title: "Changelog", description: "Curated changes and published build history." },
  { slug: "documentation-index", source: "docs/README.md", group: "development", title: "Repository documentation index", description: "The maintained reading guide for all repository documentation." },
];

export function docsPath(lang: Language, slug?: string): string {
  return localizedPath(lang, slug ? `docs/${slug}` : "docs");
}

/** Resolve repository-relative Markdown links while keeping the reader's route language. */
export function documentationUrl(href: string, source: string, lang: Language): string {
  const repositoryPrefix = `${repository}/blob/main/`;
  const inRepository = href.startsWith(repositoryPrefix);
  if (!inRepository && /^(?:[a-z][a-z\d+.-]*:|\/|#)/i.test(href)) return href;
  const url = inRepository
    ? new URL(href.slice(repositoryPrefix.length), "https://repository.invalid/")
    : new URL(href, `https://repository.invalid/${source}`);
  const path = decodeURIComponent(url.pathname.slice(1));
  const suffix = url.search + url.hash;
  const entry = documentation.find(doc => doc.source === path);
  if (entry) return docsPath(lang, entry.slug) + suffix;
  if (/^docs\/(?:diagrams\/[^/]+\.svg|examples\/[^/]+\.sql)$/.test(path)) {
    return `/docs-assets/${path.slice(5)}${suffix}`;
  }
  if (path === "crates/op-app/assets/icons/open-process.svg") return "/assets/logo.svg";
  return `${repository}/blob/main/${url.pathname.slice(1)}${suffix}`;
}

export const toolGuides: Record<string, string> = {
  whiteboard: "free-drawing", "database.schema": "database", "uml.class": "uml-classes",
  bpmn: "bpmn", "uml.use-case": "uml-use-cases", petri: "petri-nets",
  "event-log": "process-discovery", "bpmn.evaluation": "bpmn-evaluation", "dmn.table": "dmn",
  epc: "epc", "uml.activity": "uml-activities", mermaid: "mermaid",
};
