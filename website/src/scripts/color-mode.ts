import { appUrl, colorModeStorageKey, legacyAppearanceStorageKey, legacyColorModeStorageKey, modeFromQuery, readColorMode, type ColorMode } from "../lib/color-mode";

const root = document.documentElement;
const toggle = document.querySelector<HTMLButtonElement>("#color-mode-toggle");
const systemTheme = window.matchMedia("(prefers-color-scheme: dark)");
const systemMode = (): ColorMode => systemTheme.matches ? "dark" : "light";
let saved: ColorMode | null = null;
try {
  saved = readColorMode(localStorage.getItem(colorModeStorageKey));
  // Retire ambiguous preferences that could have been written without a user action.
  localStorage.removeItem(legacyColorModeStorageKey);
  localStorage.removeItem(legacyAppearanceStorageKey);
} catch { /* Browser storage is optional. */ }
const queryMode = modeFromQuery(location.search, null);
let preference = queryMode ?? saved;
let current = preference ?? systemMode();

function apply(mode: ColorMode) {
  current = mode;
  root.dataset.mode = mode;
  root.style.colorScheme = mode;
  if (toggle) {
    const label = mode === "dark" ? toggle.dataset.lightLabel! : toggle.dataset.darkLabel!;
    toggle.setAttribute("aria-label", label);
    toggle.title = label;
  }
  document.querySelectorAll<HTMLImageElement>(".brand img").forEach(img => {
    img.src = `/assets/logo${mode === "dark" ? "-light" : ""}.svg`;
  });
  document.querySelectorAll<HTMLAnchorElement>('a[href*="/app/"]').forEach(link => {
    if (new URL(link.href).pathname === "/app/") link.href = appUrl(link.href, location.origin, mode);
  });
  document.querySelector<HTMLMetaElement>('meta[name="theme-color"]')?.setAttribute("content", getComputedStyle(root).getPropertyValue("--paper").trim());
  document.dispatchEvent(new Event("open-process-color-mode"));
}

function choose(mode: ColorMode) {
  preference = mode;
  apply(mode);
  try {
    localStorage.setItem(colorModeStorageKey, mode);
  } catch { /* A manual choice still works for this page without storage. */ }
}

toggle?.addEventListener("click", () => {
  choose(current === "light" ? "dark" : "light");
  const url = new URL(location.href);
  if (url.searchParams.has("mode")) {
    url.searchParams.set("mode", current);
    history.replaceState(null, "", url);
  }
});
systemTheme.addEventListener("change", () => {
  if (preference === null) apply(systemMode());
});
window.addEventListener("storage", event => {
  if (event.key === colorModeStorageKey || event.key === null) {
    preference = modeFromQuery(location.search, readColorMode(event.newValue));
    apply(preference ?? systemMode());
  }
});

const url = new URL(location.href);
if (url.searchParams.has("theme")) {
  url.searchParams.delete("theme");
  history.replaceState(null, "", url);
}
if (queryMode) choose(queryMode);
else apply(current);
