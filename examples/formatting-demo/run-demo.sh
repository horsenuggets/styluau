#!/usr/bin/env bash
set -euo pipefail

# Format the canonical examples in place using this checkout's formatter.
# Run from any directory. Ignored examples and configuration files remain unchanged.
DEMO_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$DEMO_DIR/../.." && pwd)

cd "$DEMO_DIR"
sources=()
while IFS= read -r source; do
    [[ -n "$source" ]] || continue
    sources+=("$source")
done < cases.txt

printf '%s\n' 'Formatting examples with the local StyLuau source.'

cargo run --quiet --locked --all-features --manifest-path "$REPO_ROOT/Cargo.toml" \
    --bin styluau -- --verify --respect-ignores "${sources[@]}"

printf '%s\n' 'Done. The formatted examples are in src/.'
