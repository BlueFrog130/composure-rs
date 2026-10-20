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
- Every public item needs docs (`#![warn(missing_docs)]`). The user guide lives in
  `crates/composure/src/guide/`; its examples run as doctests. Preview the docs as docs.rs builds
  them with `RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --no-deps --all-features -p composure --open`.
- Design decisions are recorded in `docs/adr/`, terms in `CONTEXT.md`.

## Change files

Releases are managed with [knope](https://knope.tech) and change files, the same format as
[changesets](https://github.com/changesets/changesets). Every pull request that users would notice
adds one to `.changeset/`; CI checks for it.

```sh
cargo install knope
knope document-change
```

`knope document-change` asks for the kind of change and a summary, and writes a file like
`.changeset/add_thread_endpoints.md`:

```markdown
---
default: minor
---

# Add the thread Endpoints

Optional details in Markdown, shown under the summary in the changelog.
```

| Type | When | Version bump while on 0.x |
|---|---|---|
| `major` | A breaking change | `0.1.4` to `0.2.0` |
| `minor` | A new feature | `0.1.4` to `0.1.5` |
| `patch` | A fix | `0.1.4` to `0.1.5` |

`default` is the package: all the published crates share one version. Commit the file with your
change. A pull request that needs no release note (docs, CI, tests, refactors) gets the
`no changeset` label instead.

## Releases

Everything after merging is automated by `.github/workflows/release.yml`:

1. On every push to `main` with change files, knope opens (or updates) the **release PR** on the
   `release` branch: it bumps the version of every crate, updates `Cargo.lock`, writes the
   entries to `CHANGELOG.md`, and deletes the consumed change files.
2. Merging the release PR publishes every crate to crates.io in dependency order, then tags
   `v<version>` and creates the GitHub release with the changelog entry. crates.io and docs.rs
   pick it up from there.

To release, merge the release PR. Nothing else is published until then, so change files can pile
up across several PRs.

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
