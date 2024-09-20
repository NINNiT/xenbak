#!/bin/sh
RUSTFLAGS='-C link-arg=-s' cargo build --release --target x86_64-unknown-linux-musl
