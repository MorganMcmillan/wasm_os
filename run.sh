#!/bin/bash
rustc boot.rs --target wasm32-unknown-unknown && cargo run
