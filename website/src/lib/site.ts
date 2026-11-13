export const repository = "https://github.com/linventif/open-process";
export const cloudApp = "https://open-process.linv.dev/app/";
export const presentationSite = "https://open-process.linv.dev";

/** Keep canonicals on the production origin, including on preview deployments. */
export function getSiteUrl(value: string | undefined): string {
  const url = new URL(value || presentationSite);
  if (url.protocol !== "https:" || url.username || url.password || url.pathname !== "/" || url.search || url.hash) {
    throw new Error("SITE_URL must be an HTTPS origin, for example https://your-project.pages.dev (without a path, query or fragment).");
  }
  return url.origin;
}
