# Contributing

Thanks for helping out! Issues and pull requests are welcome.

## Development

```sh
cargo test --workspace                       # offline; live Discord API tests are #[ignore]d
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

- The macro compile-error tests compare against `crates/macros/tests/ui/*.stderr`, which include
  snippets of the standard library's source: install it with `rustup component add rust-src`.
  After changing a macro error, regenerate them with
  `TRYBUILD=overwrite cargo test -p composure_macros --test commands` and review the diff.
- Every public item needs docs (`#![warn(missing_docs)]`). Preview the docs as docs.rs builds
  them with `RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --no-deps --all-features -p composure --open`.
- The user guide is an [mdBook](https://rust-lang.github.io/mdBook/) in `book/`, published to
  GitHub Pages from `main`. Preview it with `mdbook serve book --open`; its examples run as
  doctests in `cargo test` (`crates/composure/src/guide.rs`). Link to API items with docs.rs
  URLs, and to other chapters with relative `.md` links.
- Design decisions are recorded in `docs/adr/`, terms in `CONTEXT.md`.

## Change files

Releases are managed with [knope](https://knope.tech) and change files, the same format as
[changesets](https://github.com/changesets/changesets). Every pull request that users would notice
adds one to `.changeset/`; CI checks for it.

```sh
cargo install knope
knope document-change
```

Each published crate has its own version: `composure`, `composure_macros`, `composure_commands`,
`composure_sync`, `composure_adapter_cloudflare` and `composure_adapter_tokio`. A change file names
the crates it changes and the kind of change for each. `knope document-change` asks for them and a
summary, and writes a file like `.changeset/add_thread_endpoints.md`:

```markdown
---
composure: minor
composure_sync: patch
---

# Add the thread Endpoints

Optional details in Markdown, shown under the summary in each crate's changelog.
```

| Type | When | Version bump while on 0.x |
|---|---|---|
| `major` | A breaking change | `0.1.4` to `0.2.0` |
| `minor` | A new feature | `0.1.4` to `0.1.5` |
| `patch` | A fix | `0.1.4` to `0.1.5` |

Name every crate whose published code changes; one that isn't named keeps its version and isn't
republished. A crate that needs something new from another (a new `composure` item used by an
Adapter) is released with a requirement on the other's new version, so name both.

A breaking change to a crate breaks the crates that depend on it too, since their API uses its
types: they need `major` as well (for `composure_macros`, that is every other crate; for
`composure`, all but `composure_macros`). knope doesn't do this itself; CI runs
`scripts/check-change-files.sh`, which checks it and that every name is a published crate.

Commit the file with your change. A pull request that needs no release note (docs, CI, tests,
refactors) gets the `no changeset` label instead.

## Releases

Everything after merging is automated by `.github/workflows/release.yml`:

1. On every push to `main` with change files, knope opens (or updates) the **release PR** on the
   `release` branch: it bumps the version of every crate a change file names (in its
   `Cargo.toml` and in the root `[workspace.dependencies]`, so the other crates require it),
   updates `Cargo.lock`, writes the entries to that crate's `CHANGELOG.md`, and deletes the
   consumed change files.
2. Merging the release PR publishes every crate whose version has no `<crate>/v<version>` tag to
   crates.io in dependency order, then tags each one and creates its GitHub release with its
   changelog entry. crates.io and docs.rs pick it up from there.

To release, merge the release PR. Nothing else is published until then, so change files can pile
up across several PRs.

Releases up to 0.1.3 moved every crate together and are tagged `v<version>`; their notes are in
the root `CHANGELOG.md`.

### Maintainer setup

- **`CARGO_REGISTRY_TOKEN`** secret: a crates.io token with the `publish-new` and
  `publish-update` scopes.
- **Settings → Actions → General → Workflow permissions**: allow GitHub Actions to create pull
  requests.
- **`RELEASE_TOKEN`** secret (recommended): a fine-grained token with contents and pull requests
  write access on this repository. GitHub does not run CI on pull requests opened with the
  default `GITHUB_TOKEN`, so without it the release PR has no checks.
- The publish job runs in the **`release`** environment: add required reviewers there to approve
  each release by hand.
- A **`no changeset`** label for pull requests without release notes.
