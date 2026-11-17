// Temporary exception for a build dependency with no published fix; see SECURITY.md.
export const picomatchAdvisory = "GHSA-fc5q-39xc-vjpf";
export const picomatchExceptionExpires = "2026-12-01T00:00:00Z";

export function auditArguments(now: Date): string[] {
  // JSON keeps the full, unfiltered advisory report visible, including exceptions.
  const args = ["audit", "--json"];
  if (now.getTime() < Date.parse(picomatchExceptionExpires)) {
    args.push("--ignore", picomatchAdvisory);
  }
  return args;
}

if (import.meta.main) {
  const args = auditArguments(new Date());
  if (args.includes("--ignore")) {
    console.warn(
      `Temporary build-tool exception: ${picomatchAdvisory} remains unpatched. ` +
      `Expires ${picomatchExceptionExpires}; see SECURITY.md. All other advisories still fail the audit.`,
    );
  }
  const result = Bun.spawnSync([process.execPath, ...args], { stdout: "inherit", stderr: "inherit" });
  process.exit(result.exitCode);
}
