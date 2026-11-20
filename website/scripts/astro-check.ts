// Type-checks `.astro` components. `astro check` refuses TypeScript 7, the
// project's compiler (`tsc`, which checks every `.ts` file). Astro's TypeScript 7
// route, @astrojs/ts-content-mapper, needs TypeScript 7.1, so until then the same
// checker engine runs on the TypeScript 6 kept for it as `typescript-6`. Its
// Volar dependencies load `typescript` themselves (peer `<7`), so this script
// alone resolves that name to TypeScript 6.
import Module, { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const typescript = createRequire(import.meta.url).resolve("typescript-6");
const resolver = Module as unknown as { _resolveFilename: (request: string, ...rest: unknown[]) => string };
const resolve = resolver._resolveFilename;
resolver._resolveFilename = function (request, ...rest) {
  return resolve.call(this, request === "typescript" ? typescript : request, ...rest);
};
const { AstroCheck } = await import("@astrojs/language-server");

const root = fileURLToPath(new URL("..", import.meta.url));
const checker = new AstroCheck(root, typescript, undefined);
const result = await checker.lint({ logErrors: { level: "hint" } });
console.info(
  `Result (${result.fileChecked} files): ${result.errors} errors, ${result.warnings} warnings, ${result.hints} hints`,
);
process.exit(result.errors > 0 ? 1 : 0);
