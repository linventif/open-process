import { expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { appUrl, galleryImage, modeFromQuery, readColorMode } from "../src/lib/color-mode";

test("retired appearance preferences retain only their color mode", () => {
  for (const theme of ["classic", "cloud", "lilac", "blossom", "lagoon", "midnight", "graphite", "workshop", "unknown"]) {
    expect(readColorMode(JSON.stringify({ theme, mode: "dark" }))).toBe("dark");
    expect(readColorMode(JSON.stringify({ theme, mode: "light" }))).toBe("light");
  }
  for (const raw of [null, "bad json", "null", "[]", "{}", '{"mode":"unknown"}']) expect(readColorMode(raw)).toBe("light");
  expect(readColorMode("dark")).toBe("dark");
  expect(readColorMode("light")).toBe("light");
});

test("app links and explicit mode URLs cannot select retired themes", () => {
  const url = new URL(appUrl("/app/?theme=cloud&existing=yes#editor", "http://localhost:4321", "dark"));
  expect(url.origin).toBe("http://localhost:4321");
  expect(url.pathname).toBe("/app/");
  expect(url.searchParams.has("theme")).toBe(false);
  expect(url.searchParams.get("mode")).toBe("dark");
  expect(url.searchParams.get("existing")).toBe("yes");
  expect(url.hash).toBe("#editor");
  expect(modeFromQuery("?theme=cloud", "light")).toBe("light");
  expect(modeFromQuery("?theme=midnight&mode=dark", "light")).toBe("dark");
  expect(modeFromQuery("?mode=invalid", "dark")).toBe("dark");
  expect(galleryImage("../../private", "dark")).toBe("/assets/classic/dark/draw.webp");
});

test("localized pages keep Classic and its light/dark gallery without theme controls", () => {
  const dist = new URL("../dist/", import.meta.url);
  for (const route of ["", "fr/", "es/", "terms/", "privacy/", "fr/terms/", "fr/privacy/", "es/terms/", "es/privacy/"]) {
    const html = readFileSync(new URL(`${route}index.html`, dist), "utf8");
    expect(html).not.toContain('id="appearance-theme"');
    expect(html).not.toContain('class="appearance-bar"');
    expect(html).not.toContain('data-theme=');
    expect(html).toContain('id="color-mode-toggle"');
  }
  expect(existsSync(new URL("assets/themes/", dist))).toBe(false);
  for (const mode of ["light", "dark"] as const) {
    for (const view of ["draw", "database", "uml", "bpmn", "petri", "mining", "mermaid", "split"]) {
      expect(existsSync(new URL(galleryImage(view, mode).slice(1), dist))).toBe(true);
    }
  }
});
