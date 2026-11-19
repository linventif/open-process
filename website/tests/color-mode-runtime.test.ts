import { expect, test } from "bun:test";
import { runInNewContext } from "node:vm";
import { colorModeStorageKey, legacyAppearanceStorageKey, legacyColorModeStorageKey } from "../src/lib/color-mode";

const build = await Bun.build({
  entrypoints: [new URL("../src/scripts/color-mode.ts", import.meta.url).pathname],
  target: "browser",
  format: "iife",
});
if (!build.success) throw new Error(build.logs.join("\n"));
const script = await build.outputs[0]!.text();

/** Run the shipped event handlers against browser APIs without adding a DOM dependency. */
function page({ dark = false, stored = new Map<string, string>(), search = "", blocked = false } = {}) {
  const mediaEvents = new EventTarget();
  const media = { matches: dark, addEventListener: mediaEvents.addEventListener.bind(mediaEvents) };
  const windowEvents = new EventTarget();
  const documentEvents = new EventTarget();
  const toggle = new EventTarget();
  const attributes = new Map<string, string>();
  const button = Object.assign(toggle, {
    dataset: { lightLabel: "Switch to light", darkLabel: "Switch to dark" },
    setAttribute: (key: string, value: string) => attributes.set(key, value),
    title: "",
  });
  const root = { dataset: {} as Record<string, string>, style: { colorScheme: "" } };
  const logo = { src: "" };
  const link = { href: "http://localhost:4321/app/?theme=cloud&existing=yes#editor" };
  let url = new URL(`http://localhost:4321/fr/${search}`);
  let writes = 0;
  let updates = 0;
  documentEvents.addEventListener("open-process-color-mode", () => updates++);
  const checkStorage = () => { if (blocked) throw new Error("Storage disabled"); };
  runInNewContext(script, {
    URL, URLSearchParams, Event,
    window: { matchMedia: () => media, addEventListener: windowEvents.addEventListener.bind(windowEvents) },
    document: {
      documentElement: root,
      querySelector: (selector: string) => selector === "#color-mode-toggle" ? button : null,
      querySelectorAll: (selector: string) => selector === ".brand img" ? [logo] : [link],
      dispatchEvent: documentEvents.dispatchEvent.bind(documentEvents),
    },
    location: { get href() { return url.href; }, get search() { return url.search; }, get origin() { return url.origin; } },
    history: { replaceState: (_state: unknown, _title: string, next: URL) => { url = new URL(next); } },
    localStorage: {
      getItem: (key: string) => { checkStorage(); return stored.get(key) ?? null; },
      setItem: (key: string, value: string) => { checkStorage(); stored.set(key, value); writes++; },
      removeItem: (key: string) => { checkStorage(); stored.delete(key); },
    },
  });
  return {
    root, stored, logo, link, attributes,
    get url() { return url; },
    get writes() { return writes; },
    get updates() { return updates; },
    click: () => toggle.dispatchEvent(new Event("click")),
    system(dark: boolean) {
      media.matches = dark;
      mediaEvents.dispatchEvent(new Event("change"));
    },
    storage(value: string | null, key: string | null = colorModeStorageKey) {
      if (key === null) stored.clear();
      else if (value === null) stored.delete(key);
      else stored.set(key, value);
      windowEvents.dispatchEvent(Object.assign(new Event("storage"), { key, newValue: value }));
    },
  };
}

test("automatic mode follows the system on load and live changes without saving a choice", () => {
  for (const dark of [false, true]) {
    const p = page({ dark });
    expect(p.root.dataset.mode).toBe(dark ? "dark" : "light");
    p.system(!dark);
    const mode = dark ? "light" : "dark";
    expect(p.root.dataset.mode).toBe(mode);
    expect(p.root.style.colorScheme).toBe(mode);
    expect(p.logo.src).toBe(`/assets/logo${mode === "dark" ? "-light" : ""}.svg`);
    expect(new URL(p.link.href).searchParams.get("mode")).toBe(mode);
    expect(new URL(p.link.href).searchParams.has("theme")).toBe(false);
    expect(p.updates).toBe(2);
    expect(p.stored.size).toBe(0);
    expect(p.writes).toBe(0);
  }
});

test("old automatically saved defaults cannot pin the site to light mode", () => {
  const stored = new Map([
    [legacyColorModeStorageKey, "light"],
    [legacyAppearanceStorageKey, JSON.stringify({ theme: "classic", mode: "light" })],
  ]);
  const p = page({ dark: true, stored });
  expect(p.root.dataset.mode).toBe("dark");
  expect(stored.size).toBe(0);
  p.system(false);
  expect(p.root.dataset.mode).toBe("light");
  expect(p.writes).toBe(0);
});

test("an explicit button choice persists across reloads and takes priority over the system", () => {
  const p = page();
  p.click();
  expect(p.root.dataset.mode).toBe("dark");
  expect(p.attributes.get("aria-label")).toBe("Switch to light");
  expect(p.stored.get(colorModeStorageKey)).toBe("dark");
  p.system(true);
  p.system(false);
  expect(p.root.dataset.mode).toBe("dark");
  const reloaded = page({ stored: p.stored });
  expect(reloaded.root.dataset.mode).toBe("dark");
  expect(reloaded.writes).toBe(0);
});

test("cross-tab preference changes and clearing storage resume system tracking", () => {
  const p = page({ dark: true });
  p.storage("light");
  expect(p.root.dataset.mode).toBe("light");
  p.storage(null);
  expect(p.root.dataset.mode).toBe("dark");
  p.system(false);
  expect(p.root.dataset.mode).toBe("light");
  p.storage("dark");
  p.storage(null, null);
  expect(p.root.dataset.mode).toBe("light");
  p.storage("dark", "unrelated-setting");
  expect(p.root.dataset.mode).toBe("light");
  p.storage("malformed");
  p.system(true);
  expect(p.root.dataset.mode).toBe("dark");
  expect(p.writes).toBe(0);
});

test("explicit URLs override saved/system modes while malformed values use the system", () => {
  const p = page({ search: "?mode=dark&theme=cloud&existing=yes#showcase" });
  expect(p.root.dataset.mode).toBe("dark");
  expect(p.stored.get(colorModeStorageKey)).toBe("dark");
  expect(p.url.searchParams.has("theme")).toBe(false);
  expect(p.url.searchParams.get("existing")).toBe("yes");
  expect(p.url.hash).toBe("#showcase");
  p.click();
  expect(p.url.searchParams.get("mode")).toBe("light");
  p.storage("dark");
  p.system(true);
  expect(p.root.dataset.mode).toBe("light");
  expect(page({ dark: true, search: "?mode=invalid", stored: new Map([[colorModeStorageKey, "invalid"]]) }).root.dataset.mode).toBe("dark");
});

test("blocked storage still permits live system updates and a manual choice for this page", () => {
  const p = page({ dark: true, blocked: true });
  expect(p.root.dataset.mode).toBe("dark");
  p.system(false);
  expect(p.root.dataset.mode).toBe("light");
  p.click();
  p.system(true);
  p.system(false);
  expect(p.root.dataset.mode).toBe("dark");
});
