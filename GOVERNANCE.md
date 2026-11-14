# Project governance

Open Process is maintained through the
[linventif/open-process repository](https://github.com/linventif/open-process).
The repository owner has final responsibility for project direction, access and
releases; maintainers with repository permissions review and merge contributions.
The project does not currently define elected roles or a formal voting process.

## Decisions and contributions

Small fixes can be proposed directly as pull requests. Discuss substantial
features, new notations, dependencies and compatibility changes in an issue
before implementation. Maintainers decide whether a proposal fits the scope and
can be maintained, using the documented architecture and compatibility rules.
An issue or roadmap entry is not a delivery commitment.

Record architectural decisions in [docs/decisions.md](docs/decisions.md) and
update the implementation and migration documentation when appropriate.
Contribution requirements are in [CONTRIBUTING.md](CONTRIBUTING.md).

## Releases and continuity

The existing release workflow publishes builds after CI passes on `main`, and
also accepts `v*` tags. See the [release process](README.md#automatic-releases)
and [changelog](CHANGELOG.md). Changes to release policy or maintainer roles
should be documented here when adopted. Maintenance and reviews depend on
availability; the project has no fixed delivery schedule.

Use [SUPPORT.md](SUPPORT.md) for help and private contact, and
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for community concerns.
