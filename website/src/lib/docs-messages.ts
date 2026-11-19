import type { Language } from "./messages";

const en = {
  title: "Documentation", intro: "From your first diagram to the details of every tool. Find practical guides, supported formats and the project reference in one place.",
  general: "Start here", tools: "Tools and workflows", development: "Project and development", records: "Plans and records",
  generalIntro: "Learn the workspace, keep your work and share it.", toolsIntro: "Choose a tool for step-by-step usage, supported features and limits.",
  developmentIntro: "Understand the implementation, contribute and maintain the project.", recordsIntro: "Read proposals and historical checks with their status and scope.",
  search: "Find a guide", placeholder: "Tool, format, saving, shortcuts…", results: "matching guides", empty: "No guide matches. Try another tool name or a broader term.",
  browse: "Browse documentation", contents: "On this page", source: "View source", home: "Home", previous: "Previous guide", next: "Next guide",
  english: "The guides are maintained in English. Navigation follows your selected language.", start: "Create your first diagram",
} as const;
type Copy = { [K in keyof typeof en]: string };
const fr: Copy = {
  title: "Documentation", intro: "Du premier diagramme aux détails de chaque outil. Retrouvez les guides pratiques, les formats pris en charge et la documentation du projet au même endroit.",
  general: "Bien démarrer", tools: "Outils et parcours", development: "Projet et développement", records: "Projets et historique",
  generalIntro: "Découvrez l’espace de travail, sauvegardez et partagez vos documents.", toolsIntro: "Choisissez un outil pour connaître son utilisation, ses fonctionnalités et ses limites.",
  developmentIntro: "Comprenez le fonctionnement du projet, contribuez et assurez sa maintenance.", recordsIntro: "Consultez les propositions et les vérifications historiques avec leur statut et leur portée.",
  search: "Rechercher un guide", placeholder: "Outil, format, sauvegarde, raccourcis…", results: "guides correspondants", empty: "Aucun guide ne correspond. Essayez le nom d’un outil ou un terme plus général.",
  browse: "Parcourir la documentation", contents: "Sur cette page", source: "Voir la source", home: "Accueil", previous: "Guide précédent", next: "Guide suivant",
  english: "Les guides sont maintenus en anglais. La navigation suit la langue que vous avez sélectionnée.", start: "Créer votre premier diagramme",
};
const es: Copy = {
  title: "Documentación", intro: "Del primer diagrama a los detalles de cada herramienta. Encuentra guías prácticas, formatos compatibles y la documentación del proyecto en un solo lugar.",
  general: "Primeros pasos", tools: "Herramientas y flujos", development: "Proyecto y desarrollo", records: "Planes e historial",
  generalIntro: "Conoce el espacio de trabajo, guarda y comparte tus documentos.", toolsIntro: "Elige una herramienta para conocer su uso, sus funciones y sus límites.",
  developmentIntro: "Comprende la implementación, contribuye y mantén el proyecto.", recordsIntro: "Consulta propuestas y verificaciones históricas con su estado y alcance.",
  search: "Buscar una guía", placeholder: "Herramienta, formato, guardado, atajos…", results: "guías coincidentes", empty: "No hay guías coincidentes. Prueba el nombre de una herramienta o un término más general.",
  browse: "Explorar la documentación", contents: "En esta página", source: "Ver código fuente", home: "Inicio", previous: "Guía anterior", next: "Guía siguiente",
  english: "Las guías se mantienen en inglés. La navegación sigue el idioma seleccionado.", start: "Crear tu primer diagrama",
};
export const docsMessages: Record<Language, Copy> = { en, fr, es };
