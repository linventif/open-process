import {
  appearanceFromQuery, appearanceStorageKey, appUrl, cssTokens, defaultAppearance,
  presetFor, presets, readAppearance, type Appearance,
} from "../lib/appearance";

const root = document.documentElement;
const theme = document.querySelector<HTMLSelectElement>("#appearance-theme");
const mode = document.querySelector<HTMLSelectElement>("#appearance-mode");
const caption = document.querySelector<HTMLElement>("#appearance-caption");
let stored = { ...defaultAppearance };
try { stored = readAppearance(localStorage.getItem(appearanceStorageKey)); } catch { /* Storage may be unavailable. */ }
let current = appearanceFromQuery(location.search, stored);

function apply(value: Appearance, persist = true) {
  current = value;
  const preset = presetFor(value.theme);
  root.dataset.theme = preset.id;
  root.dataset.mode = value.mode;
  root.dataset.layout = preset.layout;
  root.style.colorScheme = value.mode;
  for (const [name, token] of Object.entries(cssTokens(value))) root.style.setProperty(name, token);
  if (theme) theme.value = preset.id;
  if (mode) mode.value = value.mode;
  if (caption && theme) caption.textContent = theme.selectedOptions[0]?.dataset.description ?? "";
  document.querySelectorAll<HTMLButtonElement>("[data-theme-choice]").forEach(button => {
    button.setAttribute("aria-pressed", String(button.dataset.themeChoice === preset.id));
  });
  document.querySelectorAll<HTMLImageElement>(".brand img").forEach(img => {
    const light = value.mode === "dark" || (preset.id === "workshop" && !!img.closest(".site-header"));
    img.src = `/assets/logo${light ? "-light" : ""}.svg`;
  });
  document.querySelectorAll<HTMLAnchorElement>('a[href*="/app/"]').forEach(link => {
    if (new URL(link.href).pathname === "/app/") link.href = appUrl(link.href, location.origin, value);
  });
  document.querySelector<HTMLMetaElement>('meta[name="theme-color"]')?.setAttribute("content", cssTokens(value)["--paper"]!);
  document.dispatchEvent(new CustomEvent("open-process-appearance", { detail: value }));
  if (persist) {
    try { localStorage.setItem(appearanceStorageKey, JSON.stringify(value)); } catch { /* The preview still works without storage. */ }
    const url = new URL(location.href);
    if (url.searchParams.has("theme") || url.searchParams.has("mode")) {
      url.searchParams.set("theme", value.theme);
      url.searchParams.set("mode", value.mode);
      history.replaceState(null, "", url);
    }
  }
}

function choose(id: string) {
  const preset = presetFor(id);
  apply({ theme: preset.id, mode: preset.dark_default ? "dark" : current.mode });
}

theme?.addEventListener("change", () => choose(theme.value));
mode?.addEventListener("change", () => apply({ ...current, mode: mode.value === "dark" ? "dark" : "light" }));
document.querySelectorAll<HTMLButtonElement>("[data-theme-choice]").forEach(button => {
  const preset = presetFor(button.dataset.themeChoice);
  button.style.setProperty("--swatch", `rgb(${preset.light.accent.join(", ")})`);
  button.addEventListener("click", () => choose(preset.id));
});
document.querySelector("#appearance-reset")?.addEventListener("click", () => apply({ ...defaultAppearance }));
window.addEventListener("storage", event => {
  if (event.key === appearanceStorageKey) apply(readAppearance(event.newValue), false);
});
apply(current, false);
if (new URLSearchParams(location.search).has("theme") || new URLSearchParams(location.search).has("mode")) {
  try { localStorage.setItem(appearanceStorageKey, JSON.stringify(current)); } catch { /* Optional browser preference. */ }
}

// Gallery code can load before or after this module, so it also reads the root attributes.
export { presets };
