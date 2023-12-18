#!/bin/bash
set -eu


FOLDER_NAME=${PWD##*/}
CRATE_NAME=$FOLDER_NAME # assume crate name is the same as the folder name
# EXAMPLE_NAME="demo"

rustup target add wasm32-unknown-unknown

# Release:
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/${CRATE_NAME}.wasm web/

# # Debug:
# cargo build --example ${EXAMPLE_NAME} --target wasm32-unknown-unknown
# cp target/wasm32-unknown-unknown/debug/examples/${EXAMPLE_NAME}.wasm docs/

# brew install wabt # to get wasm-strip
wasm-strip web/${CRATE_NAME}.wasm