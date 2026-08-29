# MITOS Settings

A system settings manager and privileged daemon for MITOS, written in
Rust with **zero external dependencies** — persistence, IPC, and CLI
parsing are all hand-rolled on top of `std`. See `docs/architecture.md`
for why, and where a real dependency would plug in if you're extending
this for production.

```
$ mitos-settings list
appearance       Appearance
theme            Theme
wallpaper        Wallpaper
display          Display
...

$ mitos-settings get display.brightness
80

$ mitos-settings set sound.volume 65
Updated sound.volume.

$ mitos-settings
MITOS Settings — interactive mode. Type 'help' for commands, 'quit' to exit.

Settings
   1. Appearance
   2. Theme
   3. Wallpaper
   4. Display
   ...
```

## What's here

- **26 settings categories** (`src/categories/`), matching the full
  Personalization → About MITOS tree — each one a real `Category`
  implementation registering typed, validated settings.
- **A typed settings core** (`src/settings/`): a small `Value` enum, a
  `Schema` registry, range/choice validation, and atomic on-disk
  persistence with a real migration example.
- **A privileged daemon** (`src/ipc/`, `mitos-settings --daemon`) reached
  over a Unix socket, so unprivileged writes to `Admin`/`Root`-level
  settings get forwarded automatically rather than just failing.
- **Real system integration** (`src/hardware/`, `src/services/`): reads
  `/proc` and `/sys` directly, and shells out to standard Linux tools
  (`nmcli`, `amixer`, `timedatectl`, ...) to actually apply changes — with
  every write logged rather than fatal if the tool isn't installed.
- **Three front-ends over one core**: a CLI (`get`/`set`/`list`/`reset`),
  an interactive text navigator, and the daemon's IPC server — all three
  are thin shells over the same `SettingsManager`.

## Building

```sh
cargo build --release
cargo test
```

No network access is required to build — see `Cargo.toml`.

## Layout

```
src/
├── app/            interactive text navigator
├── categories/      the 26 settings categories
├── cli/             get / set / list / reset subcommands
├── config/          low-level config file I/O, paths, migrations
├── hardware/        read-only /proc, /sys introspection
├── ipc/             daemon protocol, client, server
├── notifications/   in-process pub/sub event bus
├── permissions/      privilege levels and identity
├── platform/         OS-specific command execution helpers
├── services/        live system mutation (volume, wifi, timezone, ...)
└── settings/         the core data model: Value, Schema, SettingsManager
```

Full docs in `docs/`:
[`architecture.md`](docs/architecture.md) ·
[`settings-api.md`](docs/settings-api.md) ·
[`configuration.md`](docs/configuration.md) ·
[`security.md`](docs/security.md) ·
[`developers.md`](docs/developers.md)

## License

MIT — see [`LICENSE`](LICENSE).
