import { expect, test } from "bun:test";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";

const dist = fileURLToPath(new URL("../dist/", import.meta.url));
const read = (path: string) => readFileSync(`${dist}${path}`, "utf8");

test("all three languages are rendered with crawlable content and complete SEO metadata", () => {
  for (const [file, lang, route, heading] of [["index.html", "en", "/", "Every process."], ["fr/index.html", "fr", "/fr/", "Vos processus."], ["es/index.html", "es", "/es/", "Tus procesos."]]) {
    const html = read(file!);
    expect(html).toContain(`lang="${lang}"`);
    expect(html).toContain(heading!);
    expect(html).toContain(`rel="canonical" href="https://open-process.linv.dev${route}"`);
    expect(html).toContain('hreflang="en" href="https://open-process.linv.dev/"');
    expect(html).toContain('hreflang="fr" href="https://open-process.linv.dev/fr/"');
    expect(html).toContain('hreflang="es" href="https://open-process.linv.dev/es/"');
    expect(html).toContain('hreflang="x-default"');
    expect(html).toContain('href="/app/"');
    expect(html).toContain('name="twitter:card" content="summary_large_image"');
    expect(html).toContain('property="og:image" content="https://open-process.linv.dev/assets/bpmn.webp"');
    const locale = { en: "en_GB", fr: "fr_FR", es: "es_ES" }[lang!];
    expect(html).toContain(`property="og:locale" content="${locale}"`);
    const json = html.match(/<script type="application\/ld\+json">(.*?)<\/script>/)?.[1];
    const structured = JSON.parse(json!);
    expect(structured["@type"]).toBe("SoftwareApplication");
    expect(structured.inLanguage).toBe(lang);
    expect(structured.installUrl).toBe("https://open-process.linv.dev/app/");
    expect(structured.offers.price).toBe("0");
    expect((html.match(/<h1\b/g) ?? []).length).toBe(1);
    expect(html).not.toContain('<script type="module">');
    expect(html).not.toContain("undefined");
    const dropdown = html.match(/<details\b[^>]*id="language-dropdown"[^>]*>(.*?)<\/details>/s)?.[1];
    expect(dropdown).toBeDefined();
    expect(dropdown).toContain(`<span>${lang!.toUpperCase()}</span>`);
    expect((dropdown!.match(/aria-current="page"/g) ?? []).length).toBe(1);
    expect(dropdown).toContain(`href="${route}" hreflang="${lang}" lang="${lang}" aria-current="page"`);
    for (const [otherLang, otherRoute, name] of [["en", "/", "English"], ["fr", "/fr/", "Français"], ["es", "/es/", "Español"]]) {
      expect(dropdown).toContain(`href="${otherRoute}" hreflang="${otherLang}" lang="${otherLang}"`);
      expect(dropdown).toContain(`<span>${name}</span>`);
    }
  }
});

test("robots and sitemap use the production domain and exclude the app and error page", () => {
  expect(read("robots.txt")).toContain("Sitemap: https://open-process.linv.dev/sitemap-index.xml");
  const sitemap = read("sitemap-0.xml");
  expect(sitemap).toContain("https://open-process.linv.dev/fr/");
  expect(sitemap).toContain("https://open-process.linv.dev/es/");
  expect(sitemap).toContain('hreflang="es"');
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
    expect(existsSync(`${dist}${match[1]!.slice(1)}`)).toBe(true);
  }
  const headers = read("_headers");
  expect(headers).toContain("/app/*");
  expect(headers).toContain("'wasm-unsafe-eval'");
  expect(read("_redirects")).toContain("/app /app/ 301");
});

test("all website pages ship the approved logo and browser icons", () => {
  for (const page of ["index.html", "fr/index.html", "es/index.html", "404.html"]) {
    const html = read(page);
    expect(html).toContain('/assets/logo.svg');
    expect(html).toContain('/icons/favicon.png');
    expect(html).toContain('/icons/apple-touch-icon.png');
    expect(html).toContain('/icons/favicon.svg');
  }
  expect(existsSync(`${dist}assets/logo.svg`)).toBe(true);
  expect(existsSync(`${dist}icons/favicon.png`)).toBe(true);
  expect(existsSync(`${dist}icons/apple-touch-icon.png`)).toBe(true);
});

test("the standalone app ships the same icon and resolvable install metadata", () => {
  expect(readFileSync(`${dist}assets/logo.png`)).toEqual(readFileSync(`${dist}app/web/icon-512.png`));
  const html = read("app/index.html");
  expect(html).toContain('href="web/site.webmanifest"');
  expect(html).toContain('href="web/favicon.png"');
  const manifest = JSON.parse(read("app/web/site.webmanifest"));
  const base = new URL("https://example.org/app/web/site.webmanifest");
  expect(new URL(manifest.start_url, base).pathname).toBe("/app/");
  expect(new URL(manifest.scope, base).pathname).toBe("/app/");
  for (const icon of manifest.icons) {
    const path = new URL(icon.src, base).pathname.slice(1);
    expect(existsSync(`${dist}${path}`)).toBe(true);
  }
});
