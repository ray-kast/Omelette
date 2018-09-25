#!/usr/bin/env sh
diesel migration redo || diesel migration run
cargo run --release $@