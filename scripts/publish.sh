#!/usr/bin/env bash
set -euo pipefail

CRATES=(
    "light-instruction-decoder-derive"
    "light-instruction-decoder"
)

get_workspace_version() {
    # Extract version from [workspace.package] section in root Cargo.toml
    sed -n '/\[workspace\.package\]/,/\[/p' Cargo.toml | grep '^version' | head -1 | sed 's/.*"\(.*\)".*/\1/'
}

get_tag_version() {
    local ref="${GITHUB_REF:-}"
    if [ -z "$ref" ]; then
        echo "ERROR: GITHUB_REF is not set" >&2
        exit 1
    fi
    echo "${ref#refs/tags/v}"
}

validate() {
    local tag_version
    tag_version="$(get_tag_version)"
    echo "Tag version: $tag_version"

    local workspace_version
    workspace_version="$(get_workspace_version)"
    echo "Workspace version: $workspace_version"

    if [ "$tag_version" != "$workspace_version" ]; then
        echo "ERROR: Tag version ($tag_version) does not match workspace version ($workspace_version)" >&2
        exit 1
    fi

    echo "Version match: $tag_version"
    echo ""

    echo "Dry-run publishing derive crate..."
    cargo publish -p light-instruction-decoder-derive --dry-run

    echo "Dry-run publishing main library..."
    cargo publish -p light-instruction-decoder --dry-run

    echo "Validation passed."
}

publish() {
    local tag_version
    tag_version="$(get_tag_version)"

    echo "Publishing light-instruction-decoder-derive v${tag_version}..."
    cargo publish -p light-instruction-decoder-derive

    echo "Waiting for crates.io index to propagate..."
    for i in $(seq 1 30); do
        if cargo search light-instruction-decoder-derive 2>/dev/null | grep -q "$tag_version"; then
            echo "Derive crate is available on crates.io."
            break
        fi
        if [ "$i" -eq 30 ]; then
            echo "WARNING: Timed out waiting for derive crate to appear on crates.io. Attempting publish anyway."
        fi
        sleep 10
    done

    echo "Publishing light-instruction-decoder v${tag_version}..."
    cargo publish -p light-instruction-decoder

    echo "Published successfully."
}

case "${1:-}" in
    --validate)
        validate
        ;;
    --publish)
        publish
        ;;
    *)
        echo "Usage: $0 [--validate|--publish]" >&2
        exit 1
        ;;
esac
