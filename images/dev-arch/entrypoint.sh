#!/bin/sh

echo "Starting entrypoint..."

sudo chown -R dev:dev /home/dev

mkdir -p /workspace
sudo chown -R dev:dev /workspace

echo "Initializing sshd daemon..."
sudo /usr/bin/sshd

echo "Staring shell as dev user..."
exec "$SHELL"
