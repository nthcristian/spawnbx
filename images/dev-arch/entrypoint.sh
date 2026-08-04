#!/bin/sh

echo "Starting entrypoint..."

sudo chown -R dev:dev /home/dev

mkdir -p /workspace
sudo chown -R dev:dev /workspace

echo "Starting sshd daemon..."
sudo /usr/bin/sshd

echo "Starting shell as dev user..."
sudo chsh -s "$SHELL" dev
exec "$SHELL"
