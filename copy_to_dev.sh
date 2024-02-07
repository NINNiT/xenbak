#!/bin/sh
scp target/x86_64-unknown-linux-musl/release/xenbakd root@192.168.100.2:/tmp
scp config.toml root@192.168.100.2:/tmp
