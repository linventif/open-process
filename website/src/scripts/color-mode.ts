import { appUrl, colorModeStorageKey, legacyAppearanceStorageKey, modeFromQuery, readColorMode, type ColorMode } from "../lib/color-mode";

const root = document.documentElement;
const toggle = document.querySelector<HTMLButtonElement>("#color-mode-toggle");
let saved: ColorMode = "light";
try {
  saved = readColorMode(localStorage.getItem(colorModeStorageKey) ?? localStorage.getItem(legacyAppearanceStorageKey));
} catch { /* Browser storage is optional. */ }
let current = modeFromQuery(location.search, saved);

function apply(mode: ColorMode, persist = true) {
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
  if (persist) {
    try {
      localStorage.setItem(colorModeStorageKey, mode);
      localStorage.removeItem(legacyAppearanceStorageKey);
    } catch { /* Mode switching still works without storage. */ }
  }
}

toggle?.addEventListener("click", () => {
  apply(current === "light" ? "dark" : "light");
  const url = new URL(location.href);
  if (url.searchParams.has("mode")) {
    url.searchParams.set("mode", current);
    history.replaceState(null, "", url);
  }
});
window.addEventListener("storage", event => {
  if (event.key === colorModeStorageKey) apply(readColorMode(event.newValue), false);
});

const url = new URL(location.href);
if (url.searchParams.has("theme")) {
  url.searchParams.delete("theme");
  history.replaceState(null, "", url);
}
apply(current);
