# Current Container Flow

`spawnbx` loads `.spawnbx.yml`, combines the command-line shell and package options, and optionally saves the resulting settings.

For attach, update, and recreate, it inspects the derived Docker container. A running container is reused, a stopped one is started, and a missing one is created with the project and `.spawnbx/home` mounts. It then configures the mapped user and reconciles packages in `/nix/var/nix/profiles/default`.

`update` exits after package work. The default and `recreate` commands attach the requested shell. `remove` removes the container, while `stop` only attempts runtime cleanup. Desktop and host integrations are not part of this flow.
