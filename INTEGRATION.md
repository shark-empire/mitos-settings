# Integrating with mitos-settings

This is the contract other MITOS repos should read before talking to
mitos-settings. It covers every path in, every path out, which mechanism
to use for what, and — at the bottom — the specific things I don't know
yet about your other projects that would let this go deeper.

If you're just orienting yourself in this repo, start with
[`docs/architecture.md`](docs/architecture.md) instead; this file is
specifically about the *outside* edges.

## The four ways in

mitos-settings exposes the same underlying `SettingsManager` through four
different doors. Pick based on what you're building.

| Mechanism | Best for | Language | Read/write |
|---|---|---|---|
| [Rust library](#1-rust-library-same-workspacegit-dependency) | Another Rust MITOS component | Rust only | Both |
| [CLI](#2-cli) | Scripts, install steps, shell hooks | Any | Both |
| [IPC daemon (Unix socket)](#3-ipc-daemon-unix-socket) | A long-running process that needs live read/write | Any (it's a text protocol) | Both |
| [JSON export](#4-json-export) | Docs generation, discovery, one-off tooling | Any | Read-only |

Plus one thing mitos-settings *writes for* other components to just read
passively — [home.conf](#5-homeconf-passive-file-watch) — and one thing it
*calls out to* — [the D-Bus file picker](#6-d-bus-calls-mitos-settings-makes).

---

## 1. Rust library (same workspace/git dependency)

`mitos-settings` is built as both a binary and a library crate
(`mitos_settings`, see `Cargo.toml`). If your component is also Rust, you
can depend on it directly and skip IPC entirely:

```toml
# in your Cargo.toml
[dependencies]
mitos-settings = { path = "../mitos-settings" }   # or git = "..."
```

```rust
use mitos_settings::settings::manager::{Mode, SettingsManager};
use mitos_settings::hardware; // read-only /proc, /sys introspection
use mitos_settings::services; // live system mutation helpers

let manager = SettingsManager::load(Mode::Standalone)?;
let brightness = manager.get("display.brightness")?;
```

This gets you the full `Schema`, live validation, and the `hardware`/
`services` modules (CPU/memory/battery/network introspection, volume/
brightness/wifi control, etc.) without shelling out to the CLI or opening
a socket. See `docs/settings-api.md`.

**I don't yet know if your other components are Rust, or if this repo
layout is a Cargo workspace, separate repos, or git submodules** — that
changes whether `path = "../mitos-settings"` even makes sense. See
[Open questions](#open-questions) below.

---

## 2. CLI

```sh
mitos-settings get <key>
mitos-settings set <key> <value>
mitos-settings list [category] [--json]
mitos-settings reset <key> | --all
mitos-settings schema          # full schema as JSON
mitos-settings pick-wallpaper  # D-Bus file picker -> sets wallpaper.desktop_path
```

Exit code 0 on success, 1 on failure, error message on stderr. Good for
install scripts (`mitos-installer` setting initial defaults),
one-off shell hooks, or anything that doesn't want to hold a socket open.
Full reference: `docs/settings-api.md`.

---

## 3. IPC daemon (Unix socket)

For a long-running process (a login manager, a system tray applet) that
wants live read/write without shelling out per call. The daemon
(`mitos-settings --daemon`) listens on `/run/mitos-settings/daemon.sock`
and speaks a deliberately simple **line-oriented text protocol** — no
binary framing, no schema negotiation, so it's trivial to implement from
any language with a Unix socket API (C, Python, Go, whatever).

**Requests** (one line each):
```
GET <key>
SET <key> <kind>:<payload>
LIST [<category>]
RESET <key>
RESET --all
PING
WHOAMI
```

**Responses:**
```
OK <message>              # single-line success
ERR <message>              # failure, human-readable reason
OK                          # multi-line: header...
DATA <key>=<kind>:<payload>  # ...one row per setting...
END                          # ...terminator (used by LIST)
```

Value encoding is `<kind>:<payload>` where kind is `bool`/`int`/`float`/
`str`/`strlist` — e.g. `SET sound.volume int:65`, `SET theme.mode str:dark`.
Full grammar and the exact escaping rules: `src/ipc/protocol.rs`'s module
doc comment, and `docs/security.md`.

**Auth:** every connection is identity-checked via `SO_PEERCRED` — the
daemon knows the real, kernel-verified uid of whoever connected, and
checks it against each setting's required privilege level. You don't need
to do anything special to get this; it's automatic per-connection. If
you're running as an unprivileged user and try to write an Admin/Root
setting, you'll get `ERR ... requires admin privileges ...` back — same
as the CLI does when it can't escalate.

**Socket permissions:** `0660` on the socket, `0710` on its parent
directory — reachable by root and whatever group owns it (see
`docs/security.md`). If your component needs to reach this socket and
isn't root, it needs to be in that admin group.

---

## 4. JSON export

Two flavors, both stable/deterministic output (sorted by key):

```sh
mitos-settings schema        # every setting's key, type, default, privilege, constraints
mitos-settings list --json   # current values, all categories
mitos-settings list sound --json   # current values, one category
```

This is the integration point for `mitos-docs` (auto-generate a settings
reference — every setting's `label`/`description`/`kind`/`default`/
`privilege`/`choices`/`range`/`format` is in `schema` output) or any tool
that wants to discover what's configurable without depending on Rust at
all.

Example `schema` entry:
```json
{
  "key": "appearance.accent_color",
  "category": "appearance",
  "label": "Accent color",
  "description": "Highlight color used for selections, links, toggles, and the MITOS shell. Hex: #RGB, #RRGGBB, or #RRGGBBAA",
  "kind": "str",
  "default": "#4d9eff",
  "privilege": "user",
  "read_only": false,
  "format": "hex_color"
}
```

**Not yet included:** each category's `live_info()` (disk usage, paired
Bluetooth devices, kernel version, ...) — that's presentation-layer data
computed on the fly, not part of the stored schema/values. If
`mitos-docs` or another tool wants that too, say so and it's a small
addition to `src/settings/json.rs`.

---

## 5. home.conf (passive file watch)

`mitos-gui` and `mitos-file-manager` don't call into mitos-settings at
all — they watch `~/.config/mitos/home.conf` via inotify and re-read it
whenever it changes. mitos-settings owns writing this file; nothing else
should. Full contract, including the exact key list and format:
[`docs/home-conf.md`](docs/home-conf.md).

This is the right model for **any other component that just needs to
react to a handful of settings changing** and doesn't need to *ask*
mitos-settings anything — cheaper than holding an IPC connection open,
and language-agnostic (any inotify-capable language can watch a text
file). If you're building something like that, the pattern in
`docs/home-conf.md` — a small, hand-authored-format projection file,
regenerated atomically on every relevant change — is meant to be
copy-pasteable for a second file if you need a different subset of
settings than what mitos-gui/mitos-file-manager get.

---

## 6. D-Bus calls mitos-settings makes

mitos-settings is a D-Bus **client** in one place: `mitos-settings
pick-wallpaper` calls `OpenFile` on a session-bus service named
`org.mitos.FilePicker` (see `src/services/dbus.rs`). It assumes:

- Object path: `/org/mitos/FilePicker`
- Interface: `org.mitos.FilePicker` (same as the bus name)
- Method: `OpenFile`, no arguments, returns a single string (the chosen
  path), or an empty result if cancelled

**These are assumptions, not confirmed against a real service** — I
don't know what actually implements `org.mitos.FilePicker` yet (probably
`mitos-file-manager`?). If the real interface differs, the three
constants at the top of `src/services/dbus.rs` are the only things that
need to change.

mitos-settings does **not** currently expose its own D-Bus service (no
signals emitted on setting changes). If some future component wants D-Bus
signals instead of watching `home.conf` or holding an IPC connection open,
that's a real feature to add, not just a docs update — see
[On dependencies](#on-dependencies) below.

---

## Paths reference

Everything in one place:

| What | Path |
|---|---|
| User settings store (mitos-settings' own format) | `$XDG_CONFIG_HOME/mitos-settings/settings.conf` |
| System settings store (Admin/Root-level settings) | `/etc/mitos-settings/settings.conf` |
| Daemon socket | `/run/mitos-settings/daemon.sock` |
| Shared desktop-shell config (mitos-gui, mitos-file-manager) | `$XDG_CONFIG_HOME/mitos/home.conf` |
| Log directory (referenced by `developer.*` settings, not yet written to) | `/var/log/mitos` |

`$XDG_CONFIG_HOME` falls back to `~/.config` if unset, per usual XDG
convention (see `src/config/paths.rs`).

## On dependencies

This project is currently **zero external dependencies** — persistence,
the IPC protocol, JSON export, and CLI parsing are all hand-rolled on
`std`. That was originally a necessity (I couldn't compile/verify a
dependency in the sandbox I built this in) as much as a choice, but it's
turned into a real property worth preserving deliberately in most places:
the formats are simple enough that hand-rolled code is easy to audit, and
nothing you build against this crate needs to worry about a `mitos-settings`
dependency dragging in a large tree of its own.

That said, here's where a real dependency would be a genuine improvement,
if you want it:

- **`zbus`** (pure-Rust D-Bus, no `libdbus` C dependency) — would replace
  `src/services/dbus.rs` shelling out to `gdbus`/`dbus-send` with a typed,
  properly-erroring D-Bus client. Also the prerequisite if you want
  mitos-settings to expose its *own* D-Bus service (signals on setting
  changes) instead of/alongside `home.conf`.
- **`serde` + `serde_json`** — would replace the hand-rolled encoder in
  `src/settings/json.rs`. Only worth it if that file's shape needs to get
  more complex than "flat objects with strings/numbers/bools" — right now
  hand-rolled is genuinely fine and I can reason about its correctness by
  hand, which matters given I can't currently run `cargo test` myself to
  check.
- **`libc`** — would replace the hand-written `SO_PEERCRED` FFI in
  `src/ipc/permissions.rs` with per-architecture-correct constants. Only
  matters if MITOS ever targets something other than x86/x86_64/arm/aarch64.
- **`clap`** — would improve CLI help text/parsing robustness over the
  hand-rolled argv matching in `src/cli/mod.rs`. Low priority; the current
  CLI surface is small enough that this hasn't been a real pain point.

**I haven't added any of these.** Every change I've made so far, I could
reason through by hand and I had the existing test suite to catch obvious
regressions. A new dependency is a bigger leap of faith without being able
to actually run `cargo build` — so: say the word on any of the above and
I'll add it, but I'd rather you make that call than have me guess.

## Open questions

Things that would let me wire the next layer deeper, instead of leaving
it documented-but-unbuilt like the D-Bus assumptions above:

1. **Are your other MITOS components Rust?** Determines whether "depend
   on the `mitos_settings` crate directly" (§1) is actually available to
   them, or whether the CLI/IPC/JSON surfaces (§2–4) are the only options.
2. **Repo layout** — one monorepo, a Cargo workspace across separate
   repos, or fully independent repos? Affects whether `path = "../..."`
   dependencies make sense or whether this needs to be a git dependency
   (or eventually, a real internal registry).
3. **`mitos-network`** exists as its own top-level component, and
   `categories::network`/`services::network`/`hardware::network` already
   exist here too. Does `mitos-network` **own** network configuration
   (meaning this repo's network category should become a thin client of
   whatever `mitos-network` exposes), or is `mitos-network` something
   lower-level (a stack/driver manager) that sits *underneath* what's
   here? Right now `services::network` shells out to `nmcli`/`ip`
   directly — worth knowing before that grows further in a direction
   that conflicts with what `mitos-network` is doing.
4. **`mitos-update`** — same question. `services::updates` currently does
   generic apt/dnf/pacman detection as an explicit placeholder (per our
   last conversation) since `mitos-pkg` isn't designed yet. Is
   `mitos-update` going to be the user-facing frontend for `mitos-pkg`
   that this should shell out to, once it exists?
5. **`mitos-login`** — a login manager plausibly wants
   `users.auto_login`, `users.guest_account_enabled`,
   `security.require_password_immediately`, and
   `users.require_password_on_wake` at boot, likely before the daemon is
   even running. It can already read `/etc/mitos-settings/settings.conf`
   directly (root can read anything, and those are all Admin-level
   settings stored there) or link the crate directly if it's Rust — but
   if it wants a purpose-built minimal file (like `home.conf`, but for
   login-relevant settings) instead, that's easy to add once I know the
   shape it actually needs.
