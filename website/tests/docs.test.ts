import { expect, test } from "bun:test";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { documentation, documentationUrl, docsPath, toolGuides } from "../src/lib/docs-catalogue";
import { languages, localizedPath } from "../src/lib/messages";

const dist = new URL("../dist/", import.meta.url);
const checkout = new URL("../../", import.meta.url);
const read = (route: string) => readFileSync(new URL(`${route.replace(/^\//, "")}index.html`, dist), "utf8");

async function inspect(html: string) {
  const ids = new Set<string>();
  const links: string[] = [];
  const labelled: string[] = [];
  let h1 = 0;
  await new HTMLRewriter()
    .on("[id]", { element: element => { ids.add(element.getAttribute("id")!); } })
    .on("[aria-labelledby]", { element: element => { labelled.push(...element.getAttribute("aria-labelledby")!.split(/\s+/)); } })
    .on("a[href], img[src], script[src], link[href]", { element: element => { links.push(element.getAttribute("href") ?? element.getAttribute("src")!); } })
    .on("h1", { element: () => { h1++; } })
    .transform(new Response(html)).text();
  return { ids, links, labelled, h1 };
}

test("the catalogue publishes every maintained guide once and covers every landing tool", () => {
  expect(new Set(documentation.map(doc => doc.slug)).size).toBe(documentation.length);
  expect(new Set(documentation.map(doc => doc.source)).size).toBe(documentation.length);
  for (const file of readdirSync(new URL("docs/", checkout)).filter(file => file.endsWith(".md"))) {
    expect(documentation.some(doc => doc.source === `docs/${file}`)).toBe(true);
  }
  for (const doc of documentation) expect(existsSync(new URL(doc.source, checkout))).toBe(true);
  expect(Object.keys(toolGuides)).toHaveLength(12);
  for (const slug of Object.values(toolGuides)) expect(documentation.some(doc => doc.slug === slug && doc.group === "tools")).toBe(true);
});

test("repository links retain fragments, assets and the reader's language", () => {
  expect(documentationUrl("dessin-libre.md#attached-lines-and-arrows", "docs/README.md", "fr")).toBe("/fr/docs/free-drawing/#attached-lines-and-arrows");
  expect(documentationUrl("../README.md#automatic-releases", "docs/getting-started.md", "es")).toBe("/es/docs/overview/#automatic-releases");
  expect(documentationUrl("../website/README.md", "docs/traductions.md", "en")).toBe("/docs/website/");
  expect(documentationUrl("https://github.com/linventif/open-process/blob/main/docs/mermaid.md#implementation-and-limits", "README.md", "fr")).toBe("/fr/docs/mermaid/#implementation-and-limits");
  expect(documentationUrl("diagrams/interface-zones.svg", "docs/interface.md", "fr")).toBe("/docs-assets/diagrams/interface-zones.svg");
  expect(documentationUrl("examples/shop.sqlite.sql", "docs/base-de-donnees-ddl.md", "en")).toBe("/docs-assets/examples/shop.sqlite.sql");
  expect(documentationUrl("../crates/op-core/src/lib.rs#L1", "docs/README.md", "en")).toBe("https://github.com/linventif/open-process/blob/main/crates/op-core/src/lib.rs#L1");
  for (const href of ["#usage", "/app/", "https://example.org/guide", "mailto:contact@example.org"]) {
    expect(documentationUrl(href, "docs/README.md", "fr")).toBe(href);
  }
});

test("all tool cards open their dedicated guide in the selected language", () => {
  for (const lang of languages) {
    const html = read(localizedPath(lang));
    for (const [tool, slug] of Object.entries(toolGuides)) {
      const card = html.match(new RegExp(`<article[^>]*data-diagram="${tool.replaceAll(".", "\\.")}"[^>]*>(.*?)</article>`, "s"))?.[1];
      expect(card).toBeDefined();
      expect(card).toContain(`href="${docsPath(lang, slug)}"`);
      expect(card).not.toContain("github.com");
    }
    expect(html).toContain(`href="${docsPath(lang)}"`);
    expect(html).not.toContain('href="#learn"');
  }
});

test("documentation pages render without client JavaScript and keep article-aware language metadata", () => {
  const sitemap = readFileSync(new URL("sitemap-0.xml", dist), "utf8");
  for (const lang of languages) {
    for (const doc of [undefined, ...documentation]) {
      const route = docsPath(lang, doc?.slug);
      const html = read(route);
      expect(html).toContain(`lang="${lang}"`);
      expect(html).toContain(`rel="canonical" href="https://open-process.linv.dev${route}"`);
      expect(html).toContain('"@type":"WebPage"');
      expect(sitemap).toContain(`https://open-process.linv.dev${route}`);
      expect(html).not.toContain("undefined");
      expect(html).not.toMatch(/\sstyle="/);
      for (const other of languages) {
        const alternate = docsPath(other, doc?.slug);
        expect(html).toContain(`hreflang="${other}" href="https://open-process.linv.dev${alternate}"`);
        expect(html).toContain(`href="${alternate}" hreflang="${other}"`);
      }
      if (doc) {
        expect(html).toContain('<article class="doc-content" lang="en">');
        expect(html).toContain(`href="${route}" aria-current="page"`);
        expect(html).toContain(`https://github.com/linventif/open-process/blob/main/${doc.source}`);
        expect(html).toContain('class="docs-inline-toc"');
      }
      if (lang !== "en") expect(html).toContain('class="docs-language-note"');
    }
  }
});

test("every local documentation link, asset, section anchor and accessible heading target resolves", async () => {
  const parsed = new Map<string, Awaited<ReturnType<typeof inspect>>>();
  for (const lang of languages) {
    for (const doc of [undefined, ...documentation]) {
      const route = docsPath(lang, doc?.slug);
      const page = await inspect(read(route));
      expect(page.h1).toBe(1);
      for (const id of page.labelled) expect(page.ids.has(id)).toBe(true);
      parsed.set(route, page);
    }
  }
  for (const [route, page] of parsed) {
    for (const link of page.links) {
      const url = new URL(link, `https://open-process.linv.dev${route}`);
      if (url.origin !== "https://open-process.linv.dev" || url.pathname.startsWith("/app/")) continue;
      const path = decodeURIComponent(url.pathname).slice(1);
      expect(existsSync(new URL(path.endsWith("/") ? `${path}index.html` : path, dist))).toBe(true);
      if (url.hash && parsed.has(url.pathname)) {
        expect(parsed.get(url.pathname)!.ids.has(decodeURIComponent(url.hash.slice(1)))).toBe(true);
      }
    }
  }
});

test("documentation publishes its examples and applies the presentation CSP", () => {
  for (const asset of ["diagrams/interface-zones.svg", "examples/shop.sqlite.sql", "examples/shop.mysql.sql", "examples/shop.postgresql.sql"]) {
    expect(readFileSync(new URL(`docs-assets/${asset}`, dist), "utf8")).toBe(readFileSync(new URL(`docs/${asset}`, checkout), "utf8"));
  }
  const headers = readFileSync(new URL("_headers", dist), "utf8");
  for (const route of ["/docs/*", "/fr/docs/*", "/es/docs/*"]) {
    const policy = headers.split(`\n${route}\n`)[1]?.split("\n\n")[0];
    expect(policy).toContain("script-src 'self'");
    expect(policy).toContain("style-src 'self'");
    expect(policy).not.toContain("unsafe-inline");
  }
});
