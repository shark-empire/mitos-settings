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

- **27 settings categories** (`src/categories/`), matching the full
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
  `schema`/`history`/`import`/`grants`/`pick-wallpaper`), an interactive
  text navigator, and the daemon's IPC server — all three are thin shells
  over the same `SettingsManager`.

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
│   ├── categories/      the 27 settings categories
│   ├── cli/             get / set / list / reset / schema / history / import / grants / pick-wallpaper
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

## Known gaps

Called out explicitly so nobody re-discovers these by surprise:

- **File-level locking across processes.** `SettingsManager::persist`
  reads the whole store, changes one value, and writes it all back — two
  processes doing that for different keys at nearly the same moment can
  silently lose one change. `config::file_lock` is a ready,
  dependency-free advisory lock (create a `.lock` file atomically; steal
  it if the PID recorded inside it is dead) that isn't wired into
  `persist` yet. Integration point: wrap `persist`'s body in
  `config::file_lock::acquire(&lock_path, Duration::from_secs(2))?`,
  where `lock_path` is `store.path().with_extension("lock")` — and map
  the `Err(String)` it can return into a new `SettingsError` variant
  (remembering every existing `match` on `SettingsError` then needs that
  arm too).
- **Cross-process live reload.** `notifications::EventBus` is real and
  tested, but in-process only — nothing outside the process that made a
  change hears about it, and the GUI doesn't subscribe to its own bus
  either. A real fix needs the daemon to *push* change notifications to
  connected clients over the IPC socket instead of only ever replying to
  requests it received — a protocol change, not just a client-side
  subscription.
- **Multi-key transactions over IPC.** `SettingsManager::import_values`
  validates a whole batch before applying any of it, but only for the
  local, in-process path — a batch containing a daemon-forwarded,
  Admin-privilege key can still apply some entries and fail partway
  through, the same as any other single `set` call already can. A real
  fix needs a new `Request`/`Response` pair the daemon applies atomically
  server-side.
- **`SettingSpec::dangerous`** only changes behavior in
  `gui/src/widgets.rs`'s `build_switch` — the other four control-building
  functions don't have a staged-apply path yet. That's fine today since
  every setting marked `dangerous` so far is a `Bool`, but worth knowing
  before marking a dropdown/spinner/entry-backed setting dangerous too.

## License

MIT — see [`LICENSE`](LICENSE).
