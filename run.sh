#!/bin/bash
cp crates/boot/target/wasm32-unknown-unknown/debug/boot.wasm boot.wasm
cp crates/child/target/wasm32-unknown-unknown/debug/child.wasm child.wasm
cargo run
