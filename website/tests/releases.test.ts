import { describe, expect, test } from "bun:test";
import { assetNames, downloadUrl, parseRelease, releasesUrl, safeGithubUrl } from "../src/lib/releases";
import { getSiteUrl, cloudApp } from "../src/lib/site";

const release = {
  tag_name: "main-42-abcdef0",
  html_url: `${releasesUrl}/tag/main-42-abcdef0`,
  assets: Object.values(assetNames).map((name) => ({ name, browser_download_url: `${releasesUrl}/download/main-42-abcdef0/${name}` })),
};

describe("release downloads", () => {
  test("selects each architecture's binary and handles missing assets", () => {
    const parsed = parseRelease(release)!;
    for (const platform of Object.keys(assetNames) as (keyof typeof assetNames)[]) {
      expect(downloadUrl(parsed, platform)).toEndWith(assetNames[platform]);
      expect(downloadUrl(null, platform)).toBe(releasesUrl);
    }
    expect(downloadUrl({ ...parsed, assets: parsed.assets.slice(0, 1) }, "linux")).toBe(releasesUrl);
  });

  test("rejects malformed metadata and untrusted download destinations", () => {
    for (const value of [null, {}, { ...release, assets: null }, { ...release, html_url: "https://example.org/release" }]) {
      expect(parseRelease(value)).toBeNull();
    }
    for (const url of ["javascript:alert(1)", "http://github.com/linventif/open-process/releases/tag/v1", "https://github.com/another/repo/releases/tag/v1", "https://github.com@evil.example/linventif/open-process/releases/tag/v1", "https://user:secret@github.com/linventif/open-process/releases/tag/v1"]) {
      expect(safeGithubUrl(url)).toBeNull();
    }
    const parsed = parseRelease({ ...release, assets: [null, {}, { name: assetNames.linux, browser_download_url: "https://evil.example/file" }] });
    expect(parsed?.assets).toEqual([]);
    expect(downloadUrl(parsed, "linux")).toBe(releasesUrl);
  });
});

test("canonical origin and cloud app share the intended domain", () => {
  expect(getSiteUrl(undefined)).toBe("https://open-process.linv.dev");
  expect(getSiteUrl("https://open-process.linv.dev/")).toBe("https://open-process.linv.dev");
  expect(cloudApp).toBe("https://open-process.linv.dev/app/");
  for (const url of ["http://example.org", "https://example.org/path", "https://example.org/?preview=1", "https://user:secret@example.org"]) {
    expect(() => getSiteUrl(url)).toThrow();
  }
});
