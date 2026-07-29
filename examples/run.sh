#! /usr/bin/env bash
cd $1 && cargo build --target wasm32-unknown-unknown && cp target/wasm32-unknown-unknown/debug/*.wasm ~/.local/share/wasm_os/boot.wasm
cd ~/projects/wasm_os/ && cargo run
