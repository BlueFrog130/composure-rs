#!/usr/bin/env sh
# Re-vendors Discord's OpenAPI spec (crates/composure/spec) at the latest discord-api-spec commit,
# or at the commit given as $1. Then run the spec check:
#   cargo test -p composure --features full --lib rest::spec -- --nocapture
set -eu
dir="$(cd "$(dirname "$0")/.." && pwd)/crates/composure/spec"
sha="${1:-$(curl -sSL https://api.github.com/repos/discord/discord-api-spec/commits/main | sed -n 's/^  "sha": "\([0-9a-f]*\)".*/\1/p' | head -n1)}"
curl -sSLf -o "$dir/openapi.json" "https://raw.githubusercontent.com/discord/discord-api-spec/$sha/specs/openapi.json"
echo "$sha" > "$dir/PINNED"
echo "spec pinned at $sha"
