# Command Usage

Run every command from a directory containing `.spawnbx.yml`.

| Command | Behavior |
| --- | --- |
| `spawnbx` | Ensures the container is running, configures it, and attaches the selected shell. |
| `spawnbx update [packages...]` | Configures the environment, upgrades Nix packages, reconciles configured packages, and exits. Named packages must already be installed. |
| `spawnbx stop` | Stops the derived container without starting or configuring it. |
| `spawnbx remove` | Removes the container and retains `.spawnbx/home`. |
| `spawnbx recreate` | Removes, recreates or starts, configures, and attaches the environment. |

`--shell`, `--packages`, and `--save` must appear before a subcommand. `--packages` uses comma-separated values.
