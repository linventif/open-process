# Contributing to Open Process

Contributions are welcome: bug reports, translations, documentation, sample
models, accessibility improvements and code. You can open issues and pull
requests in English or French. English is the default language for project
documentation, new source comments and original interface copy. Please follow
our [Code of Conduct](CODE_OF_CONDUCT.md).

## Find a place to start

- Read the [README](README.md) and [getting started guide](docs/getting-started.md).
- Search [existing issues](https://github.com/linventif/open-process/issues) before
  reporting a bug or proposing a feature. Small documentation and translation
  fixes can go straight to a pull request.
- For a new notation, dependency, format change or substantial feature, open an
  issue first to discuss the use case and scope.
- For translations, follow [the translation guide](docs/traductions.md).
- For usage questions, see [SUPPORT.md](SUPPORT.md). Report vulnerabilities using
  [SECURITY.md](SECURITY.md), rather than a public issue.

## Set up the project

Install Rust with Cargo and a native C/C++ toolchain. The minimum Rust version is
declared in [Cargo.toml](Cargo.toml); use a stable toolchain meeting that version.
On Debian/Ubuntu, the CI installs `libxkbcommon-dev` and `libgtk-3-dev`.
Other Linux distributions need the equivalent development libraries.

Fork the repository on GitHub, then clone your fork and create a branch:

```sh
git clone https://github.com/YOUR-USERNAME/open-process.git
cd open-process
git switch -c describe-your-change
cargo run --locked -p op-app
```

To develop the browser app, install the WebAssembly target and Trunk:

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked trunk --version 0.22.1
cd crates/op-app
trunk serve
```

Open the URL printed by Trunk. For the Astro website, use the versions and commands
in [website/README.md](website/README.md).

## Follow the project structure

Start with [docs/README.md](docs/README.md), the
[current architecture](docs/architecture-actuelle.md) and [AGENTS.md](AGENTS.md).
Documentation is maintained in English. Existing documentation filenames remain
unchanged to preserve links. Keep French and Spanish interface translations and
respect saved language preferences.

- `op-core` owns typed models, algorithms and formats, independently of the UI.
- `op-app` owns editors, platform integration and the shared graphical layout.
- `website/` owns the presentation site and assembly with the browser app.

Follow the [notation extension guide](docs/ajouter-une-notation.md) for new editors.
Preserve existing `.opx` files, imports/exports and session restoration. Format
changes need documented migrations and tests with older files. Use the existing
translation mechanisms for interface text. Check both native and browser effects.
Keep changes focused and preserve unrelated work in your checkout.

## Verify your change

Run the checks relevant to what you changed, from the repository root:

```sh
# Core models, algorithms and formats
cargo test --locked -p op-core

# Application and UI tests: run sequentially, as in CI
cargo test --locked --workspace -- --test-threads=1

# Native and browser compilation (install the wasm target first)
cargo check --locked -p op-app
cargo check --locked -p op-app --target wasm32-unknown-unknown

# Whitespace and patch errors
git diff --check
```

Add regression tests for behavior changes where useful. For UI changes, describe
manual checks and include screenshots when they help review. Format the Rust
files you touched using rustfmt; avoid unrelated formatting changes.

For documentation-only changes, check local links and the diff; a Rust build is
unnecessary. Translation checks are documented in the translation guide. Website
checks are in its README. Packaging changes also need
`python3 -m unittest discover -s scripts -p 'test_*.py'` (use `python` if that is
your Python 3 command).

The [CI workflow](.github/workflows/ci.yml) runs tests and release builds on
Linux, Windows and macOS, plus the full website/WebAssembly build and Bun tests.
Report the checks you actually ran and any you could not run; a partial local
check does not cover the whole CI matrix.

## Submit a pull request

Keep one coherent change per PR. Explain the problem, resulting behavior and
validation, link any related issue, and update the affected documentation.
Describe any compatibility or native/browser limitations. Draft PRs are welcome
for work that needs early feedback.

Maintainers review contributions and may ask for changes before merging. See
[GOVERNANCE.md](GOVERNANCE.md) for how project decisions are handled. Review times
depend on availability; no response deadline is guaranteed.

## Contribution license

Unless explicitly agreed otherwise, contributions submitted for inclusion must
be available under the project's existing dual license:
[MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE), at the recipient's option.
Only submit work you have the right to contribute. Identify third-party code or
assets and preserve their license and attribution notices. Report any different
licensing requirements before submitting the material.
