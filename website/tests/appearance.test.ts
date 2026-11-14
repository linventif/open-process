import { expect, test } from "bun:test";
import { readFileSync, existsSync } from "node:fs";
import {
  appearanceFromQuery, appUrl, cssTokens, defaultAppearance, galleryImage, presets, readAppearance,
} from "../src/lib/appearance";

test("appearance restoration rejects malformed and unknown values", () => {
  for (const value of [null, "bad json", "null", "[]", "{}", '{"theme":"future","mode":"invalid"}']) {
    expect(readAppearance(value)).toEqual(defaultAppearance);
  }
  for (const preset of presets) {
    for (const mode of ["light", "dark"] as const) {
      const value = { theme: preset.id, mode };
      expect(readAppearance(JSON.stringify(value))).toEqual(value);
      expect(cssTokens(value)["--paper"]).toMatch(/^rgb\(/);
    }
  }
});

test("explicit theme links preserve the local app destination and select only registered themes", () => {
  const current = { theme: "lilac", mode: "dark" as const };
  const link = new URL(appUrl("/app/?existing=yes#editor", "http://localhost:4321", current));
  expect(link.origin).toBe("http://localhost:4321");
  expect(link.pathname).toBe("/app/");
  expect(link.searchParams.get("theme")).toBe("lilac");
  expect(link.searchParams.get("mode")).toBe("dark");
  expect(link.searchParams.get("existing")).toBe("yes");
  expect(link.hash).toBe("#editor");
  expect(appearanceFromQuery("?theme=cloud&mode=light", current)).toEqual({ theme: "cloud", mode: "light" });
  expect(appearanceFromQuery("?theme=invalid&mode=invalid", current)).toEqual({ theme: "classic", mode: "dark" });
  expect(appearanceFromQuery("", current)).toEqual(current);
  expect(galleryImage("../../private", current)).toBe("/assets/themes/lilac/dark/draw.webp");
});

test("each appearance and color mode ships real gallery images and translated selectors", () => {
  const dist = new URL("../dist/", import.meta.url);
  for (const language of ["", "fr/", "es/"]) {
    const html = readFileSync(new URL(`${language}index.html`, dist), "utf8");
    expect(html).toContain('id="appearance-theme"');
    expect(html).toContain('id="appearance-mode"');
    for (const preset of presets) expect(html).toContain(`value="${preset.id}"`);
  }
  expect(new Set(presets.map(preset => preset.id)).size).toBe(8);
  for (const preset of presets) {
    for (const mode of ["light", "dark"] as const) {
      for (const view of ["draw", "database", "uml", "bpmn", "petri", "mining", "mermaid", "split"]) {
        expect(existsSync(new URL(galleryImage(view, { theme: preset.id, mode }).slice(1), dist))).toBe(true);
      }
    }
  }
});
