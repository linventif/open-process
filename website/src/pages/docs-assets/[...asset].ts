import type { APIRoute } from "astro";

export function getStaticPaths() {
  const assets = import.meta.glob<string>("../../../../docs/{diagrams/*.svg,examples/*.sql}", {
    eager: true, query: "?raw", import: "default",
  });
  return Object.entries(assets).map(([path, content]) => ({
    params: { asset: path.split("/docs/")[1] }, props: { content },
  }));
}

export const GET: APIRoute = ({ params, props }) => new Response(props.content, {
  headers: {
    "Content-Type": params.asset?.endsWith(".svg") ? "image/svg+xml; charset=utf-8" : "text/plain; charset=utf-8",
  },
});
