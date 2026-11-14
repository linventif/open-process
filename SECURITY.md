# Security policy

## Versions receiving fixes

Security fixes target the current `main` branch and are distributed through new
[releases](https://github.com/linventif/open-process/releases). Older releases
do not have a separate backport or long-term support policy. Include the exact
release tag or commit in a report, since several builds can share a Cargo version.

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
