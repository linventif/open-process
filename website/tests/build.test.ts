import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";

const dist = fileURLToPath(new URL("../dist/", import.meta.url));
const read = (path: string) => readFileSync(`${dist}${path}`, "utf8");

test("both languages are rendered with crawlable content and complete SEO metadata", () => {
  for (const [file, lang, route, heading] of [["index.html", "en", "/", "Every process."], ["fr/index.html", "fr", "/fr/", "Vos processus."]]) {
    const html = read(file!);
    expect(html).toContain(`lang="${lang}"`);
    expect(html).toContain(heading!);
    expect(html).toContain(`rel="canonical" href="https://open-process.linv.dev${route}"`);
    expect(html).toContain('hreflang="en" href="https://open-process.linv.dev/"');
    expect(html).toContain('hreflang="fr" href="https://open-process.linv.dev/fr/"');
    expect(html).toContain('hreflang="x-default"');
    expect(html).toContain('href="/app/"');
    expect(html).toContain('name="twitter:card" content="summary_large_image"');
    expect(html).toContain('property="og:image" content="https://open-process.linv.dev/assets/bpmn.webp"');
    const json = html.match(/<script type="application\/ld\+json">(.*?)<\/script>/)?.[1];
    const structured = JSON.parse(json!);
    expect(structured["@type"]).toBe("SoftwareApplication");
    expect(structured.inLanguage).toBe(lang);
    expect(structured.installUrl).toBe("https://open-process.linv.dev/app/");
    expect(structured.offers.price).toBe("0");
    expect((html.match(/<h1\b/g) ?? []).length).toBe(1);
    expect(html).not.toContain('<script type="module">');
    expect(html).not.toContain("undefined");
  }
});

test("robots and sitemap use the production domain and exclude the app and error page", () => {
  expect(read("robots.txt")).toContain("Sitemap: https://open-process.linv.dev/sitemap-index.xml");
  const sitemap = read("sitemap-0.xml");
  expect(sitemap).toContain("https://open-process.linv.dev/fr/");
  expect(sitemap).toContain("https://open-process.linv.dev/");
  expect(sitemap).not.toContain("/404");
  expect(sitemap).not.toContain("/app");
  expect(read("404.html")).toContain('name="robots" content="noindex, nofollow"');
});

test("the shared Cloudflare output contains the app with all assets under /app/", () => {
  const html = read("app/index.html");
  expect(html).toContain("the_canvas_id");
  const files = readdirSync(`${dist}app`);
  expect(files.some((name) => name.endsWith(".wasm"))).toBe(true);
  expect(files.some((name) => name.endsWith(".js"))).toBe(true);
  for (const match of html.matchAll(/(?:from\s*|new URL\(|href=)["'](\/app\/[^"']+)["']/g)) {
    expect(files).toContain(match[1]!.split("/").pop()!);
  }
  const headers = read("_headers");
  expect(headers).toContain("/app/*");
  expect(headers).toContain("'wasm-unsafe-eval'");
  expect(read("_redirects")).toContain("/app /app/ 301");
});
