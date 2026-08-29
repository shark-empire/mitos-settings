# Architecture

MITOS Settings is split into layers. Each layer only depends on the ones
below it — nothing in `hardware` knows `categories` exists, nothing in
`categories` knows `cli` exists, and so on. That makes it possible to add a
new front-end (a GUI, say) without touching anything below `app`/`cli`.

```
 ┌─────────────┐  ┌─────────────┐  ┌──────────────┐
 │   cli/      │  │   app/      │  │  ipc/server  │  <- front-ends / daemon
 └──────┬──────┘  └──────┬──────┘  └──────┬───────┘
        └────────────────┼─────────────────┘
                          ▼
                 ┌──────────────────┐
                 │ settings::manager│  <- the one place everything meets
                 └────────┬─────────┘
        ┌─────────────────┼─────────────────┬───────────────┐
        ▼                 ▼                 ▼               ▼
 ┌─────────────┐  ┌───────────────┐  ┌────────────┐  ┌──────────────┐
 │  schema/    │  │ persistence/  │  │ permissions│  │ notifications│
 │  validation │  │ config/*      │  │            │  │              │
 └─────────────┘  └───────────────┘  └────────────┘  └──────────────┘

 ┌──────────────┐        ┌───────────────┐
 │ categories/  │──uses──▶ services/     │──uses──▶ hardware/ + platform/
 │ (26 files)   │        │ (mutates      │        │ (read-only /proc,
 │              │        │  live system) │        │  /sys, shells out)
 └──────────────┘        └───────────────┘        └───────────────────┘
```

## The core data model

- **`settings::value::Value`** — the five kinds a setting can hold
  (bool/int/float/str/strlist), plus parsing (from CLI text) and encoding
  (to/from the on-disk and IPC wire format).
- **`settings::schema::Schema`** — a registry of `SettingSpec`s (key, type,
  default, privilege level, valid range/choices) and `CategoryMeta`s (the
  26 navigator entries). Built once at startup by
  `categories::register_all`.
- **`settings::manager::SettingsManager`** — the orchestrator. Every
  front-end (`cli`, `app`, `ipc::server`) talks to one of these, never
  directly to persistence, validation, or services.

## Categories vs. services vs. hardware

These three are easy to conflate, so here's the boundary:

- **`hardware/`** is read-only. It parses `/proc` and `/sys` and returns
  plain structs. It never shells out to mutate anything.
- **`services/`** is where mutation happens — flipping Wi-Fi on, changing
  volume, setting the timezone — usually by shelling out to a standard
  Linux tool (`nmcli`, `amixer`, `timedatectl`, ...). It also has read
  helpers that go beyond what `hardware` can see (paired Bluetooth
  devices, pending package updates).
- **`categories/`** is the presentation layer over both: each file
  registers its `SettingSpec`s into the schema, and optionally implements
  `live_info()` to surface read-only data from `hardware`/`services` (disk
  usage, kernel version, paired devices, ...) that isn't itself a stored
  setting.

`services::apply()` is the single dispatch point `SettingsManager::set`
calls after a value is validated and persisted — see its doc comment in
`src/services/mod.rs` for the full list of wired-up keys. Persistence
always succeeds or fails independently of whether the live system could be
updated: a setting "sticks" even if, say, `amixer` isn't installed.

## Privilege and the daemon

Every `SettingSpec` carries a `PrivilegeLevel` (`User`/`Admin`/`Root`).
`SettingsManager::set` checks the caller's own level (`permissions::AuthContext`)
against it:

- Sufficient privilege → write proceeds directly.
- Insufficient, and we're a normal CLI/app process (`Mode::Standalone`) →
  the write is forwarded over `ipc::client` to the privileged daemon.
- Insufficient, and we *are* the daemon (`Mode::DaemonAuthority`) → the
  write is rejected outright; there's nowhere further to escalate to.

See `docs/security.md` for what the daemon's socket permissions do and
don't guarantee.

## Why no external crates

The whole crate builds against `std` alone — no serde, no clap, no tokio.
For a project this shape (a settings store, a tiny IPC protocol, a CLI)
that's a genuine design choice, not just a demo constraint: it keeps the
dependency tree at zero, which matters for something that runs as root as
part of the base OS. If you're extending this for a real product, the
seams where a dependency would plug in cleanly are:

- `settings::value` / `settings::persistence` → serde + a real format
- `cli::mod` → clap or a similar arg parser
- `ipc::protocol` → a real RPC framing (or just JSON lines)
