import type { MarkdownInstance } from "astro";
import { documentationUrl, type DocEntry } from "./docs-catalogue";
import type { Language } from "./messages";

const modules = import.meta.glob<MarkdownInstance<Record<string, unknown>>>(
  ["../../../docs/*.md", "../../../*.md", "../../README.md"], { eager: true },
);
const sources = new Map(Object.entries(modules).map(([path, document]) => [
  path.startsWith("../../../") ? path.slice("../../../".length) : "website/README.md",
  document,
]));

export function docSource(entry: DocEntry) {
  const document = sources.get(entry.source);
  if (!document) throw new Error(`Documentation source missing: ${entry.source}`);
  return document;
}

export async function docHtml(entry: DocEntry, lang: Language): Promise<string> {
  const html = await docSource(entry).compiledContent();
  return html.replace(/\b(href|src)="([^"]*)"/g, (_, attribute: string, value: string) => {
    const href = documentationUrl(value.replaceAll("&amp;", "&"), entry.source, lang);
    return `${attribute}="${href.replaceAll("&", "&amp;").replaceAll('"', "&quot;")}"`;
  });
}
