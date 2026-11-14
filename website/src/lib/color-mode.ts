export type ColorMode = "light" | "dark";
export const colorModeStorageKey = "open-process-color-mode-v1";
export const legacyAppearanceStorageKey = "open-process-site-appearance-v1";

/** Accept the former preference only to preserve its light/dark choice. */
export function readColorMode(raw: string | null): ColorMode {
  if (raw === "dark" || raw === "light") return raw;
  try {
    const value: unknown = JSON.parse(raw ?? "null");
    return value && typeof value === "object" && "mode" in value && value.mode === "dark" ? "dark" : "light";
  } catch {
    return "light";
  }
}

export function modeFromQuery(search: string, fallback: ColorMode): ColorMode {
  const mode = new URLSearchParams(search).get("mode");
  return mode === "dark" || mode === "light" ? mode : fallback;
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
