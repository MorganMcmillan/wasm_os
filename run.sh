#!/bin/bash
cp crates/boot/target/wasm32-unknown-unknown/debug/boot.wasm boot.wasm && cargo run
