#!/bin/sh
set -e # Exit immediately if a command exits with a non-zero status

# Define a function for rollback
rollback() {
	echo "Rolling back changes..."
	# Restore backed up files
	ssh "${SSH_USER}"@"${XEN_HOST}" "sudo cp /usr/bin/xenbakd_backup /usr/bin/xenbakd"
	ssh "${SSH_USER}"@"${XEN_HOST}" "sudo cp /etc/systemd/system/xenbakd.service_backup /etc/systemd/system/xenbakd.service"
	ssh "${SSH_USER}"@"${XEN_HOST}" "sudo cp /etc/xenbakd/config.toml_backup /etc/xenbakd/config.toml"
	# Reload systemd daemon and restart xenbakd service
	ssh "${SSH_USER}"@"${XEN_HOST}" "sudo systemctl daemon-reload && sudo systemctl restart xenbakd"
	exit 1 # Exit with error status
}

# Check if XEN_HOST environment variable is set
if [ -z "$XEN_HOST" ]; then
	echo "XEN_HOST env var is not set"
	exit 1
fi

# Check if SSH_USER environment variable is set
if [ -z "$SSH_USER" ]; then
	echo "SSH_USER env var is not set"
	exit 1
fi

# Stop xenbakd service on remote host
ssh "${SSH_USER}"@"${XEN_HOST}" "sudo systemctl stop xenbakd"

# Backup existing files on remote host
ssh "${SSH_USER}"@"${XEN_HOST}" "sudo cp /usr/bin/xenbakd /usr/bin/xenbakd_backup"
ssh "${SSH_USER}"@"${XEN_HOST}" "sudo cp /etc/systemd/system/xenbakd.service /etc/systemd/system/xenbakd.service_backup"
ssh "${SSH_USER}"@"${XEN_HOST}" "sudo cp /etc/xenbakd/config.toml /etc/xenbakd/config.toml_backup"

# Copy xenbakd binary to remote host
scp ../target/x86_64-unknown-linux-musl/release/xenbakd "${SSH_USER}"@"${XEN_HOST}":/usr/bin/xenbakd || rollback

# Copy systemd service file to remote host
scp ../deploy/xenbakd.service "${SSH_USER}"@"${XEN_HOST}":/etc/systemd/system/xenbakd.service || rollback

# Create directories on remote host
ssh "${SSH_USER}"@"${XEN_HOST}" "sudo mkdir -p /etc/xenbakd && sudo mkdir -p /var/log/xenbakd" || rollback

# Copy config file to remote host
scp ../apps/xenbakd/config.toml "${SSH_USER}"@"${XEN_HOST}":/etc/xenbakd/config.toml || rollback

# Reload systemd daemon and restart xenbakd service
ssh "${SSH_USER}"@"${XEN_HOST}" "sudo systemctl daemon-reload && sudo systemctl restart xenbakd" || rollback

echo "Deployment successful"
