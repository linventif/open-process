# Security policy

## Versions receiving fixes

Security fixes target the current `main` branch and are distributed through new
[releases](https://github.com/linventif/open-process/releases). Older releases
do not have a separate backport or long-term support policy. Include the exact
release tag or commit in a report, since several builds can share a Cargo version.

## Dependency audit exception

CI checks the complete website lockfile through `bun run audit` in `website/`.
Registry failures and all advisories block CI, except the following temporary,
reviewed build-tool exception:

| Advisory | Installed packages | Review and expiry |
| --- | --- | --- |
| [GHSA-fc5q-39xc-vjpf](https://github.com/advisories/GHSA-fc5q-39xc-vjpf) | `picomatch` 4.0.7 and 2.3.2, through Astro/Vite and development/checking tools | Reviewed 17 November 2026; expires 1 December 2026 at 00:00 UTC. |

The advisory describes denial of service from attacker-controlled repeated
extglob patterns. As reviewed on 17 November, the advisory lists no patched
version and npm's latest release is 4.0.7. The current site uses Astro's static
output; glob patterns come from repository sources and tooling configuration,
not visitor input or imported diagrams. `picomatch` is not included in the
shipped browser JavaScript or native Rust application. This limits exposure for
this deployment but does not fix the vulnerable dependency. Re-review this
assessment if the hosting model or glob inputs change.

[audit.ts](website/scripts/audit.ts) applies only this GHSA exception before its
deadline and prints its reason. Its JSON output retains the unfiltered report;
new advisories and network failures still fail the command. At the deadline it
automatically stops ignoring this advisory. Raw `bun audit` continues to report
it. Remove the exception and update all affected dependency paths when a patched
release becomes available; do not extend the deadline without a fresh review.

## Report a vulnerability privately

Email [contact@open-process.linv.dev](mailto:contact@open-process.linv.dev) with
the subject `Open Process security report`. Do not publish exploit details,
sensitive models or credentials in an issue or pull request.

If GitHub private vulnerability reporting is enabled for the repository, you can
also use **Security → Report a vulnerability**. Availability of that button
depends on the repository's GitHub settings.

Include:

- The affected release/commit, operating system and browser if relevant.
- The affected component, prerequisites and potential impact.
- Minimal reproduction steps or a proof of concept using synthetic data.
- Any mitigation you have identified and how we may contact you.

Reports are reviewed according to maintainer availability. We do not promise a
response or fix deadline, and there is no bounty program. We aim to coordinate
disclosure with the reporter after a fix or mitigation is available. Please
discuss publication timing through the private report.

For ordinary crashes, incorrect analysis results or import failures without a
security impact, use the [bug report form](https://github.com/linventif/open-process/issues/new?template=bug_report.yml).
