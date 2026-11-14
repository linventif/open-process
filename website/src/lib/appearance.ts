import catalogue from "../../../crates/op-app/assets/appearance.json";

export const presets = catalogue;
export const appearanceStorageKey = "open-process-site-appearance-v1";
export type ColorMode = "light" | "dark";
export interface Appearance { theme: string; mode: ColorMode }
export const defaultAppearance: Appearance = { theme: "classic", mode: "light" };

export function presetFor(id: unknown) {
  return presets.find(preset => preset.id === id) ?? presets[0]!;
}

export function readAppearance(raw: string | null): Appearance {
  try {
    const value: unknown = JSON.parse(raw ?? "null");
    if (!value || typeof value !== "object") return { ...defaultAppearance };
    const saved = value as Record<string, unknown>;
    return { theme: presetFor(saved.theme).id, mode: saved.mode === "dark" ? "dark" : "light" };
  } catch {
    return { ...defaultAppearance };
  }
}

export function appearanceFromQuery(search: string, fallback: Appearance): Appearance {
  const query = new URLSearchParams(search);
  return {
    theme: query.has("theme") ? presetFor(query.get("theme")).id : fallback.theme,
    mode: query.get("mode") === "dark" ? "dark" : query.get("mode") === "light" ? "light" : fallback.mode,
  };
}

export function appUrl(href: string, origin: string, appearance: Appearance): string {
  const target = new URL(href, origin);
  target.searchParams.set("theme", presetFor(appearance.theme).id);
  target.searchParams.set("mode", appearance.mode);
  return target.href;
}

export function galleryImage(view: string, appearance: Appearance): string {
  const allowed = ["draw", "database", "uml", "bpmn", "petri", "mining", "mermaid", "split"];
  const name = allowed.includes(view) ? view : "draw";
  return `/assets/themes/${presetFor(appearance.theme).id}/${appearance.mode}/${name}.webp`;
}

export function cssTokens(appearance: Appearance): Record<string, string> {
  const preset = presetFor(appearance.theme);
  const p = appearance.mode === "light" ? preset.site_light ?? preset.light : preset.dark;
  const rgb = (value: number[]) => `rgb(${value.join(", ")})`;
  return {
    "--paper": rgb(p.background), "--white": rgb(p.surface), "--ink": rgb(p.text),
    "--muted": rgb(p.muted), "--line": rgb(p.border), "--accent": rgb(p.accent),
    "--on-accent": rgb(p.on_accent),
    "--brand": preset.id === "classic" && appearance.mode === "light" ? "#e39b2e" : rgb(p.accent),
    "--radius": `${preset.control_radius}px`, "--card-radius": `${preset.card_radius}px`,
    "--button-radius": `${preset.button_radius}px`,
    "--frame-shadow": preset.shadow_blur === 0 ? "3px 3px 0 var(--line)" : `0 16px ${preset.shadow_blur + 12}px color-mix(in srgb, var(--ink) 10%, transparent)`,
    "--diagram-fill": `color-mix(in srgb, ${rgb(p.accent)} 12%, ${rgb(p.surface)})`,
    "--dark-bg": rgb(preset.dark.background), "--dark-surface": rgb(preset.dark.surface),
    "--dark-text": rgb(preset.dark.text), "--dark-muted": rgb(preset.dark.muted),
    "--dark-accent": rgb(preset.dark.accent),
    "--heading-font": preset.heading === "serif" ? "Georgia, 'Times New Roman', serif" : preset.heading === "mono" ? "var(--mono)" : "var(--sans)",
  };
}
