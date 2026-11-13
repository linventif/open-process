import type { APIRoute } from "astro";
import { isPreview } from "../lib/deployment";

export const GET: APIRoute = ({ site }) => new Response(
  isPreview
    ? "User-agent: *\nDisallow: /\n"
    : `User-agent: *\nAllow: /\n\nSitemap: ${new URL("/sitemap-index.xml", site)}\n`,
  { headers: { "Content-Type": "text/plain; charset=utf-8" } },
);
