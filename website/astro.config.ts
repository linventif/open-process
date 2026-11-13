import { defineConfig } from "astro/config";
import sitemap from "@astrojs/sitemap";
import { getSiteUrl } from "./src/lib/site";

export default defineConfig({
  site: getSiteUrl(process.env.SITE_URL),
  output: "static",
  trailingSlash: "always",
  // Keep executable scripts in cacheable files, compatible with the Pages CSP.
  vite: { build: { assetsInlineLimit: 0 } },
  i18n: {
    defaultLocale: "en",
    locales: ["en", "fr", "es"],
    routing: { prefixDefaultLocale: false },
  },
  integrations: [sitemap({
    filter: (page) => !page.endsWith("/404/"),
    i18n: { defaultLocale: "en", locales: { en: "en", fr: "fr", es: "es" } },
  })],
});
