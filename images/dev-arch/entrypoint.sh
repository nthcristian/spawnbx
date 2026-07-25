#!/bin/sh

echo "Starting entrypoint..."

sudo chown -R dev:dev /home/dev

mkdir -p /workspace
sudo chown -R dev:dev /workspace

echo "Staring shell as dev user..."
exec "$SHELL"
