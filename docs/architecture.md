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

## Permission grants

Separate from the `settings::manager` stack described above: `grants/`
is per-app permission grants (camera, microphone, root access, ...),
matching the original MITOS permission design's description of this
project's job: "see every app, see its permissions, and flip toggles."
It's deliberately *not* built on `SettingSpec`/`Schema` — those assume
a fixed, compile-time-known set of keys, and which apps exist (and
what they ask for) is dynamic.

**This module holds no state of its own.** An earlier version of this
feature kept its own local rulebook (a file this daemon wrote and
read, with its own guessed risk-classification table), built before
mitos-service existed as a reachable project. Once it did, that local
copy was removed entirely — mitos-service is the single rulebook every
MITOS component looks to (see that project's own README), and
`grants/` is now just enough to talk to it (`service_client`) and hold
the shape of what it says back (`model`). If mitos-service is
unreachable, permission grants simply don't work right now — a clear
error surfaced up through `ipc::server` and the GUI, not a silent
local fallback that could quietly drift from the real rulebook.

Two things worth knowing about how a `SetGrant`/`ListGrants`/
`RevokeGrant` request actually flows through `ipc::server`:

- **None of them touch `SettingsManager`'s mutex.** `IpcServer::run`
  only holds `Arc<Mutex<SettingsManager>>` — grants requests never
  lock it at all (`ipc::server::dispatch` routes them to
  `grants::service_client` directly, before ever reaching
  `dispatch_settings`). This matters because a dangerous `SetGrant` can
  legitimately block for up to a couple of minutes waiting on a real
  mitos-session elevation prompt (relayed through mitos-service); if
  that wait held the same lock ordinary settings reads/writes use, one
  pending permission prompt would freeze brightness/volume/every other
  setting on the machine for as long as it took someone to type a
  password elsewhere.
- **The uid mitos-session ends up asking to verify is always the
  connecting peer's own** (`SO_PEERCRED`, the same identity every
  other privileged write in this protocol is authorized against) —
  never a value the request itself supplies. See
  `protocol::Request::SetGrant`'s doc comment.

`grants::service_client` is a real client of mitos-service's real
control socket, speaking its actual plain-text protocol (see that
project's `ipc.rs`) — no serialization dependency needed for it,
unlike the equivalent piece of mitos-gui or mitos-session's own
protocol, which is why this crate stays fully dependency-free even
with mitos-service wired in (see "Dependencies" below).

## Dependencies

The core crate builds against `std` alone for everything it does — no
clap, no tokio, no serde, and (as of the mitos-service integration
above) still nothing beyond that: mitos-service's control-socket
protocol is plain newline-delimited text on both ends, so talking to
it needed no serialization dependency the way talking to
mitos-session's bincode wire protocol would have. That's a genuine
design choice for something that runs as root as part of the base OS,
not just a demo constraint.

If you're extending this for a real product, the seams where a
dependency would plug in cleanly are:

- `settings::value` / `settings::persistence` → serde + a real format
- `cli::mod` → clap or a similar arg parser
- `ipc::protocol` → a real RPC framing (or just JSON lines)
