export const isPreview = process.env.CF_PAGES === "1" && process.env.CF_PAGES_BRANCH !== "main";
