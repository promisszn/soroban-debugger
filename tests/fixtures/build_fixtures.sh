#!/bin/bash
set -e
WASM_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/wasm"
CONTRACTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/contracts"
WORKSPACE_TARGET_DIR="${CONTRACTS_DIR}/target/wasm32-unknown-unknown/release"
mkdir -p "$WASM_DIR"
for dir in "${CONTRACTS_DIR}"/*/; do
    if [ -f "${dir}Cargo.toml" ]; then
        name=$(basename "$dir")
        package_name=$(sed -n 's/^name = "\(.*\)"/\1/p' "${dir}Cargo.toml" | head -n 1)
        if [ -z "${package_name}" ]; then
            echo "Failed to determine package name for ${name}"
            exit 1
        fi
        (cd "$dir" && cargo build --release --target wasm32-unknown-unknown)
        cp "${WORKSPACE_TARGET_DIR}/${package_name//-/_}.wasm" "$WASM_DIR/${name}.wasm"
    fi
done
