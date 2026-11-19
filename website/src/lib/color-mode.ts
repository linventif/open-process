export type ColorMode = "light" | "dark";
// Only explicit choices are saved in v2; v1 also saved automatic defaults.
export const colorModeStorageKey = "open-process-color-mode-v2";
export const legacyColorModeStorageKey = "open-process-color-mode-v1";
export const legacyAppearanceStorageKey = "open-process-site-appearance-v1";

/** Missing or invalid preferences follow the system. */
export function readColorMode(raw: string | null): ColorMode | null {
  return raw === "dark" || raw === "light" ? raw : null;
}

export function modeFromQuery(search: string, fallback: ColorMode | null): ColorMode | null {
  return readColorMode(new URLSearchParams(search).get("mode")) ?? fallback;
}

export function appUrl(href: string, origin: string, mode: ColorMode): string {
  const url = new URL(href, origin);
  url.searchParams.delete("theme");
  url.searchParams.set("mode", mode);
  return url.href;
}

export function galleryImage(view: string, mode: ColorMode): string {
  const allowed = ["draw", "database", "uml", "bpmn", "petri", "mining", "mermaid", "split"];
  return `/assets/classic/${mode}/${allowed.includes(view) ? view : "draw"}.webp`;
}
