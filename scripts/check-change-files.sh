#!/usr/bin/env bash
# Checks the pending change files (.changeset/*.md) against knope.toml's packages: each names
# published crates (`composure: minor`), and a breaking (`major`) change to a crate needs one for
# every crate that depends on it, directly or not. Those use its types in their own API, and knope
# doesn't bump dependents: crates.io would keep only versions of them built against the old one,
# so users upgrading the crate would get two copies of it. Run by CI on every pull request.
set -euo pipefail
cd "$(dirname "$0")/.."

# Published crate -> its directory, from `[packages.<crate>]` and `changelog = "<dir>/CHANGELOG.md"`
declare -A dir
while read -r crate changelog; do
    dir[$crate]=${changelog%/CHANGELOG.md}
done < <(tr -d '\r' < knope.toml | awk -F'"' '
    /^\[packages\./ { crate = $0; gsub(/^\[packages\.|\]$/, "", crate) }
    /^changelog = / && crate != "" { print crate, $2 }')

# Published crate -> the published crates that depend on it (normal dependencies, any target)
declare -A dependents
for crate in "${!dir[@]}"; do
    while read -r dep; do
        if [ -n "${dir[$dep]+x}" ]; then dependents[$dep]+=" $crate"; fi
    done < <(tr -d '\r' < "${dir[$crate]}/Cargo.toml" | awk '
        /^\[/ { deps = /dependencies\]$/ && !/(dev|build)-dependencies\]$/; next }
        deps && /^[A-Za-z0-9_-]+ *[.=]/ { sub(/ *[.=].*/, ""); print }')
done

errors=0
fail() {
    echo "error: $*" >&2
    errors=1
}

# Crates with a breaking change, from the change files' front matter
declare -A major
for file in .changeset/*.md; do
    [ -e "$file" ] || continue
    while IFS=: read -r key value; do
        key=$(echo "$key" | tr -d ' "'"'")
        value=$(echo "$value" | tr -d ' "'"'")
        if [ -z "${dir[$key]+x}" ]; then
            fail "$file names \`$key\`, which is not a published crate: use one of ${!dir[*]}"
        elif [ "$value" = major ]; then
            major[$key]=1
        fi
    done < <(tr -d '\r' < "$file" | awk 'NR == 1 && /^---/ { inside = 1; next } inside && /^---/ { exit } inside && NF')
done

for crate in "${!major[@]}"; do
    queue=(${dependents[$crate]:-})
    declare -A seen=()
    while [ ${#queue[@]} -gt 0 ]; do
        dependent=${queue[0]}
        queue=("${queue[@]:1}")
        [ -z "${seen[$dependent]+x}" ] || continue
        seen[$dependent]=1
        if [ -z "${major[$dependent]+x}" ]; then
            fail "\`$crate\` has a breaking change, so \`$dependent\` (which depends on it) needs \`$dependent: major\` in a change file"
        fi
        queue+=(${dependents[$dependent]:-})
    done
    unset seen
done

if [ "$errors" = 0 ]; then
    echo "Change files are consistent"
fi
exit "$errors"
