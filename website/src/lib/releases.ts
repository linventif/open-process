import { repository } from "./site";

export const releasesUrl = `${repository}/releases`;
export const assetNames = {
  windows: "open-process-x86_64-pc-windows-msvc.exe",
  linux: "open-process-x86_64-unknown-linux-gnu",
  "mac-arm": "open-process-aarch64-apple-darwin",
  "mac-intel": "open-process-x86_64-apple-darwin",
} as const;
export type Platform = keyof typeof assetNames;
export interface ReleaseAsset { name: string; browser_download_url: string }
export interface Release { tag_name: string; html_url: string; assets: ReleaseAsset[] }

export function safeGithubUrl(value: unknown): string | null {
  if (typeof value !== "string") return null;
  try {
    const url = new URL(value);
    return url.origin === "https://github.com" && !url.username && !url.password
      && url.pathname.startsWith("/linventif/open-process/releases/") ? url.href : null;
  } catch { return null; }
}

export function parseRelease(value: unknown): Release | null {
  if (!value || typeof value !== "object") return null;
  const data = value as Partial<Release>;
  if (typeof data.tag_name !== "string" || !safeGithubUrl(data.html_url) || !Array.isArray(data.assets)) return null;
  return {
    tag_name: data.tag_name,
    html_url: safeGithubUrl(data.html_url)!,
    assets: data.assets.filter((asset): asset is ReleaseAsset => Boolean(
      asset && typeof asset.name === "string" && safeGithubUrl(asset.browser_download_url),
    )),
  };
}

export function downloadUrl(release: Release | null, platform: Platform): string {
  const asset = release?.assets.find((item) => item.name === assetNames[platform]);
  return safeGithubUrl(asset?.browser_download_url) ?? releasesUrl;
}
