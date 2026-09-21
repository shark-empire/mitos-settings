# MITOS Settings

A system settings manager and privileged daemon for MITOS. The core
(this crate) is written in Rust with **zero external dependencies** —
persistence, IPC, and CLI parsing are all hand-rolled on top of `std`,
and that now includes talking to mitos-service (`grants/`), since its
control-socket protocol is plain text, not a binary wire format
needing a serialization crate. See `docs/architecture.md` for the full
reasoning, and where a real dependency would plug in if you're
extending this for production.

There's also a graphical settings app (`gui/`, GTK4) for anyone who
shouldn't have to touch a terminal — see [`gui/README.md`](gui/README.md).
It's a separate workspace member specifically so GTK4 never becomes a
dependency of the root-owned daemon binary.

**Building another MITOS component that needs to talk to this one?**
Start with [`INTEGRATION.md`](INTEGRATION.md) — it covers every way in
(Rust library, CLI, IPC daemon, JSON export, `home.conf`) and exactly
what's still undecided.

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
  settings get forwarded automatically rather than just failing. Every
  connection is authenticated via `SO_PEERCRED` — the daemon checks the
  real, connecting peer's privilege, not just whether they reached the
  socket. See `docs/security.md`.
- **Real system integration** (`src/hardware/`, `src/services/`): reads
  `/proc` and `/sys` directly, and shells out to standard Linux tools
  (`nmcli`, `amixer`, `timedatectl`, ...) to actually apply changes — with
  every write logged rather than fatal if the tool isn't installed.
- **Live sync to the rest of the desktop** (`src/services/home_conf.rs`):
  appearance/theme/wallpaper/shell-layout changes are projected out to
  `~/.config/mitos/home.conf`, which `mitos-gui` and `mitos-file-manager`
  watch via inotify — no IPC needed on their end. See `docs/home-conf.md`.
- **Per-app permission grants** (`src/grants/`, "Application permissions"
  in Settings → Privacy): which apps — identified by their binary's
  SHA-256 hash, mitos-service tracks no name — may use which
  capabilities (camera, root access, ...), each Allow/Deny with a
  scope of Once/Session/Always. This daemon holds none of that state
  itself: every read and write is a live call to mitos-service, the
  single rulebook every MITOS component looks to, and changing a
  capability mitos-service classifies as dangerous genuinely blocks on
  a real mitos-session elevation prompt (relayed through
  mitos-service) — not a local check. See the `grants` module's own
  doc comment.
- **Three front-ends over one core**: a CLI (`get`/`set`/`list`/`reset`/
  `pick-wallpaper`), an interactive text navigator, and the daemon's IPC
  server — all three are thin shells over the same `SettingsManager`.

## Building

```sh
cargo build --release
cargo test
```

No network access is required to build — see `Cargo.toml`.

## Layout

```
mitos-settings/           core crate: daemon, CLI, library (zero deps)
├── src/
│   ├── app/            interactive text navigator
│   ├── categories/      the 26 settings categories
│   ├── cli/             get / set / list / reset / schema / pick-wallpaper
│   ├── config/           low-level config file I/O, paths, migrations
│   ├── hardware/         read-only /proc, /sys introspection
│   ├── ipc/               daemon protocol, client, server
│   ├── notifications/     in-process pub/sub event bus
│   ├── permissions/        privilege levels and identity
│   ├── platform/            OS-specific command execution helpers
│   ├── services/           live system mutation (volume, wifi, timezone, ...)
│   └── settings/            data model: Value, Schema, SettingsManager, JSON export
└── gui/                    workspace member: GTK4 graphical settings app
```

Full docs in `docs/`:
[`architecture.md`](docs/architecture.md) ·
[`settings-api.md`](docs/settings-api.md) ·
[`configuration.md`](docs/configuration.md) ·
[`home-conf.md`](docs/home-conf.md) ·
[`security.md`](docs/security.md) ·
[`developers.md`](docs/developers.md)

## License

MIT — see [`LICENSE`](LICENSE).
