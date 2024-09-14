<h1 align="center">xenbak</h1>

<p align="center">
  <img 
    src="banner.jpg" 
    alt="Project Logo" 
    width="80%" 
</p>

<p align="center">
  <a href="#about">About</a> •
  <a href="#features">Features</a> •
  <a href="#usage">Usage</a> •
  <a href="#installation">Installation</a> •
  <a href="#building">Building</a>
  <a href="#configuration">Configuration</a> •
</p>

---

## About

`xenbak` is a backup utility for Xenserver/XCP-Ng hypervisors written in rust.

## Features

- configuration using TOML
- can run as a daemon or as a one-shot command
- xen-hosts, storage-handlers and backup-jobs can be combined freely
- 100% safe rust
- MUSL builds available (self contained binary, can run directly on xen host with no system lib dependencies)
- filter VMs by tags (include/exclude)
- multiple storage backends (local-storage, borg-storage)
- multiple compression algorithms for backups (zstd, gzip, borg algorithms, ...)
- multiple alert handlers (mail, healthchecks.io)
- uses the xapi CLI client (`xe`) to interact with local and remote XAPI hosts

## Dependencies

Docker images come with all dependencies included. Manual installation requires the following dependencies.

### Required

- `xe` (XAPI CLI client, pre-installed on Xenserver/XCP-NG hosts)
  - `stunnel` (for remote hosts, pre-installed on Xenserver/XCP-NG hosts)

### Optional

- `borg` (for borg storage backend)

### xe installation

#### rpm package

Build artifacts from XCP-NGs build-server are available [here](https://koji.xcp-ng.org/rpminfo?rpmID=15542)

## Installation

### Binary

1. Download the a release from the [releases page]() and extract the binary to a location in your PATH.

```bash
tar -xvf xenbakd-v1.2.0.tar.gz
mv xenbak /usr/bin
```

2. Create a TOML configuration file (config.toml) and adjust it to your needs. You can use the provided [example config](#example-config) as a starting point.

```bash
mkdir -p /etc/xenbak && $EDITOR /etc/xenbak/config.toml
```

3.1 Run xenbak in deamon-mode (runs scheduled jobs)

```bash
xenbakd --config /etc/xenbak/config.toml daemon
```

3.2.1 Create a systemd service file (xenbakd.service) and adjust it to your needs.

```bash
$EDITOR /etc/systemd/system/xenbakd.service
```

```systemd
[Unit]
Description=xenbakd is a backup daemon for Xen / XCP-NG hosts
After=network-online.target

[Service]
Type=simple
ExecStart=/usr/bin/xenbakd --config /etc/xenbakd/config.toml daemon
# ExecStart=/bin/bash -c '/usr/bin/xenbakd --config /etc/xenbakd/config.toml daemon > /path/to/log/xenbakd.log'
Restart=always

[Install]
WantedBy=multi-user.target
```

3.2.2 Enable and start the service

```bash
systemctl enable --now xenbakd
```

### Docker

```bash
# run as daemon
docker run -d --name xenbakd -v /path/to/config.toml:/etc/config.toml -v path/to/storage:/mnt/storage ghcr.io/xenbak/xenbakd:0.2.0
# run once / custom CMD
docker run -d --name xenbakd -v /path/to/config.toml:/etc/config.toml -v path/to/storage:/mnt/storage ghcr.io/xenbak/xenbakd:0.2.0 xenbakd --config /etc/config.toml run --j job1,job2
```

## Usage

```text
❯ xenbakd --help

__  _____ _ __ | |__   __ _| | ____| |
\ \/ / _ \ '_ \| '_ \ / _` | |/ / _` |
 >  <  __/ | | | |_) | (_| |   < (_| |
/_/\_\___|_| |_|_.__/ \__,_|_|\_\__,_|

A backup daemon for Xen hypervisors

Usage: xenbakd --config <CONFIG> <COMMAND>

Commands:
  daemon  Starts the xenbakd daemon
  run     Runs jobs once
  help    Print this message or the help of the given subcommand(s)

Options:
  -c, --config <CONFIG>  Sets a custom config file
  -h, --help             Print help
  -V, --version          Print version

```

Daemon mode

```bash
xenbakd --config /etc/xenbak/config.toml daemon
```

Run specified jobs once and exit

```bash
xenbakd --config /etc/xenbak/config.toml run --jobs job1,job2
```

## Building

#### Install toolchain

```bash
rustup target add x86_64-unknown-linux-musl
```

#### Build the MUSL binary

```bash
 RUSTFLAGS='-C link-arg=-s' cargo build --release --target x86_64-unknown-linux-musl
```

#### Docker Image (needs above step)

```bash
docker build -t xenbakd:dev --file deploy/docker/Dockerfile .
```

## Configuration

```toml
[general]
log_level = "info" # debug, info, trace, warn, error

[monitoring.mail]
enabled = true
smtp_server = "192.168.100.164"
smtp_port = 1025
smtp_user = ""
smtp_password = ""
smtp_from = "xenbak@localhost"
smtp_to = ["asdf@test.test"]

[monitoring.healthchecks]
enabled = true
api_key = "VkSpHYVtXfkQRuhojpeUrKAwBexF-oTq"
server = "http://192.168.100.164:8000"
grace = 7200
max_retry = 5

[[xen]]
enabled = true
name = "xen1"
username = "root"
server = "192.168.100.2"
password = "asdfasdf"
port = 443

[[xen]]
enabled = true
name = "xen2"
username = "root"
server = "192.168.100.3"
password = "asdfasdf"
port = 443

[[storage.local]]
enabled = true
name = "local"              # name of the storage handler
path = "/mnt/storage/local" # path to the local storage directory
compression = "zstd"        # gzip, zstd or none
retention = 3               # keep the last N backups

[[storage.borg]]
enabled = true
name = "borg"                                                  # name of the storage handler
binary_path = "/usr/bin/borg"                                  # path to the borg binary
temp_dir = "/mnt/storage/tmp"                                  # borg needs a temporary directory to store the backup before it is uploaded to the repository
repository = "/mnt/storage/borgrepo"                           # path to the borg repository (can be local or remote)
encryption = "none"                                            # repokey-blake2, repokey, keyfile-blake2, keyfile, none
compression = "zstd"                                           # all of the borg compression algorithms
retention = { daily = 7, weekly = 1, monthly = 1, yearly = 1 } # Number of backups to keep
#ssh_key_path = ""                                              # (optional) path to the ssh key for remote borg repository, ignored on local

[[jobs]]
enabled = true
name = "test"
schedule = "0 */4 * * * *"
tag_filter = ["xenbak-daily"]          # Only backup VMs with the given tags
tag_filter_exclude = ["xenbak-exclude"] # Exclude VMs with the given tags
concurrency = 3                  # Number of concurrent backups
storages = ["local"]             # Storage to use for the backup
xen_hosts = ["xen1", "xen2"]     # Xen hosts to backup
use_existing_snapshot = true     # Use an existing snapshots instead of creating a new one, if available (default: false)
use_existing_snapshot_age = 3600 # Define the maximum age of an existing snapshot in seconds (default: 3600)
```
