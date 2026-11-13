import { downloadUrl, parseRelease, releasesUrl, type Platform, type Release } from "../lib/releases";

function element<T extends HTMLElement>(selector: string): T {
  const result = document.querySelector<T>(selector);
  if (!result) throw new Error(`Missing page element: ${selector}`);
  return result;
}

const tabs = [...document.querySelectorAll<HTMLButtonElement>("[data-view]")];
const image = element<HTMLImageElement>("#preview-image");
const dialog = element<HTMLDialogElement>("#preview-dialog");
const macArchitecture = element<HTMLSelectElement>("#mac-architecture");
const languageDropdown = element<HTMLDetailsElement>("#language-dropdown");
document.addEventListener("click", (event) => {
  if (event.target instanceof Node && !languageDropdown.contains(event.target)) {
    languageDropdown.open = false;
  }
});
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && languageDropdown.open) {
    languageDropdown.open = false;
    languageDropdown.querySelector("summary")?.focus();
  }
});
let view = 0;

function updatePreview() {
  const tab = tabs[view]!;
  const src = `/assets/${tab.dataset.view}.webp`;
  const caption = tab.dataset.caption ?? "";
  const alt = tab.dataset.alt ?? "";
  image.src = src;
  image.alt = alt;
  element("#preview-caption").textContent = caption;
  element("#preview-number").textContent = `0${view + 1} / 04`;
  element("#preview-panel").setAttribute("aria-labelledby", tab.id);
  element<HTMLImageElement>("#dialog-image").src = src;
  element<HTMLImageElement>("#dialog-image").alt = alt;
  element("#dialog-caption").textContent = caption;
  tabs.forEach((item, i) => {
    item.setAttribute("aria-selected", String(view === i));
    item.tabIndex = view === i ? 0 : -1;
  });
}

tabs.forEach((tab, index) => {
  tab.addEventListener("click", () => { view = index; updatePreview(); });
  tab.addEventListener("keydown", (event) => {
    if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    view = event.key === "Home" ? 0 : event.key === "End" ? tabs.length - 1
      : (view + (event.key === "ArrowRight" ? 1 : -1) + tabs.length) % tabs.length;
    updatePreview();
    tabs[view]!.focus();
  });
});

element("#expand-preview").addEventListener("click", () => dialog.showModal());
element("#close-preview").addEventListener("click", () => dialog.close());
dialog.addEventListener("click", (event) => { if (event.target === dialog) dialog.close(); });

type ReleaseState = "initial" | "none" | "error" | "available";
type ReleaseCopy = Record<ReleaseState | "download" | "viewReleases", string>;
const copy: ReleaseCopy = JSON.parse(element("[data-release-copy]").dataset.releaseCopy!);
let state: ReleaseState = "initial";
let release: Release | null = null;

function updateDownloads() {
  element("#release-status").textContent = copy[state].replace("{version}", release?.tag_name ?? "");
  element<HTMLAnchorElement>("#release-link").href = release?.html_url ?? releasesUrl;
  document.querySelectorAll<HTMLAnchorElement>(".download-link").forEach((link) => {
    const platform = (link.dataset.platform === "mac" ? macArchitecture.value : link.dataset.platform) as Platform;
    const url = downloadUrl(release, platform);
    link.href = url;
    element<HTMLSpanElement>(`[data-platform="${link.dataset.platform}"] [data-download-label]`).textContent = copy[url === releasesUrl ? "viewReleases" : "download"];
    if (link.lastElementChild) link.lastElementChild.textContent = url === releasesUrl ? "↗" : "↓";
  });
}

macArchitecture.addEventListener("change", updateDownloads);
updatePreview();

async function loadRelease() {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 8000);
  try {
    const response = await fetch("https://api.github.com/repos/linventif/open-process/releases/latest", {
      headers: { Accept: "application/vnd.github+json" }, signal: controller.signal,
    });
    if (response.status === 404) {
      state = "none";
    } else {
      if (!response.ok) throw new Error("Release unavailable");
      release = parseRelease(await response.json());
      if (!release) throw new Error("Invalid release metadata");
      state = "available";
    }
  } catch {
    state = "error";
  } finally {
    clearTimeout(timeout);
    updateDownloads();
  }
}

void loadRelease();
