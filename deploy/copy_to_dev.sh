#!/bin/sh
if [ -z "$XEN_HOST" ]; then
	echo "XEN_HOST env var is not set"
	exit 1
fi

scp ../target/x86_64-unknown-linux-musl/release/xenbakd root@${XEN_HOST}:/tmp
scp ../apps/xenbakd/config.toml root@${XEN_HOST}:/tmp
