#!/usr/bin/env bash
# Generates each cargo-generate template (templates/*) the way users do, points its Composure
# dependencies at this workspace (the templates require the released version, which may not be
# on crates.io yet), then tests it on the host and, for Cloudflare, checks the wasm build. Run by
# CI so the templates can't drift from the crates. Needs cargo-generate.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && (pwd -W 2>/dev/null || pwd))
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT
# One target directory for both, so dependencies build once
export CARGO_TARGET_DIR="$root/target/templates"

# Published crate -> its directory in the workspace
crates="composure:crates/composure composure_macros:crates/macros composure_commands:crates/commands composure_sync:crates/sync composure_adapter_cloudflare:crates/adapters/cloudflare composure_adapter_tokio:crates/adapters/tokio"

for template in cloudflare tokio; do
    echo "::group::templates/$template"
    cargo generate --path "$root/templates/$template" --name "ci-$template" \
        --destination "$out" --silent --vcs none --define application=1100146565504565411 --define guild=798662131062931547
    project="$out/ci-$template"

    # `name = "0.2"` and `name = { version = "0.2", .. }` become path dependencies
    for entry in $crates; do
        crate=${entry%%:*}
        path="$root/${entry#*:}"
        sed -i -E \
            -e "s#^($crate = )\"[^\"]+\"#\\1{ path = \"$path\" }#" \
            -e "s#^($crate = \\{ )version = \"[^\"]+\"#\\1path = \"$path\"#" \
            "$project/Cargo.toml"
    done
    if grep -E '^composure[a-z_]* = .*version' "$project/Cargo.toml"; then
        echo "::error::templates/$template: a Composure dependency in a form this script can't point at the workspace"
        exit 1
    fi

    (cd "$project" && cargo test)
    if [ "$template" = cloudflare ]; then
        (cd "$project" && cargo check --lib --target wasm32-unknown-unknown)
    fi
    echo "::endgroup::"
done
