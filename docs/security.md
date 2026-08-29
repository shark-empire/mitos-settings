# Security

This document is meant to be read honestly: it says what the current
implementation actually guarantees, and calls out where that falls short
of what a production system would need.

## Privilege levels

Every setting has a `PrivilegeLevel`: `User`, `Admin`, or `Root`. These are
enforced in exactly one place, `SettingsManager::set` — there is no
alternate write path that skips the check.

- **`User`** — anything that only affects the signed-in user's own
session (volume, wallpaper, keyboard layout, ...).
- **`Admin`** — anything that affects the whole machine or opens a
network-facing service (proxy settings, firewall, file/screen sharing,
timezone, system language, ...).
- **`Root`** — nothing ships at this level today; it's reserved for
settings where "administrator" isn't a strong enough guarantee. The
daemon refuses to apply a `Root`-level write unless it's actually
running as uid 0 (`ipc::permissions::ensure_daemon_may_apply`), even
though no code path currently produces one.

## How privilege is determined

`permissions::authorization::current_context` shells out to the `id`
coreutil (`id -u`, `id -un`, `id -Gn`) rather than calling `getuid(2)`
through an FFI crate. This keeps the project free of `unsafe` code and
external dependencies. A caller is `Admin` if they're uid 0, or a member
of `sudo`/`wheel`/`admin`/`mitos-admin`.

## The daemon's trust model — read this before deploying

`ipc::server::IpcServer` does **not** check the identity of who's
connected to its Unix socket via `SO_PEERCRED` or any other per-connection
credential check. The access control is entirely the socket file's Unix
permissions: `IpcServer::bind` sets it to mode `0660`. In practice this
means:

- Anyone who can't read/write `/run/mitos-settings/daemon.sock` can't
reach the daemon at all — normal filesystem permissions handle that.
- Anyone who **can** reach the socket (root, or a member of whatever
group owns it) can ask the daemon to apply **any** setting up to
`Root` level, without the daemon distinguishing between different
connecting users.

For a single-admin-group deployment (one trusted `mitos-admin` group, no
untrusted multi-user access to that group) this is a reasonable, honestly
simple boundary. It is **not** sufficient if you need to distinguish
between multiple different admin users' authority, or need an audit trail
of *which* user changed a system setting.

**Hardening path**, if you need that: add a per-connection check using
`SO_PEERCRED` (Linux-specific; needs either a small amount of `unsafe`
FFI or a crate like `libc`/`nix`) to recover the connecting process's real
uid, and use that — not just "did they reach the socket" — as the
authorization input to `ipc::permissions::ensure_daemon_may_apply`.

## What's deliberately *not* implemented

- **Applying system updates.** `services::updates::check_pending` is
read-only by design. Actually upgrading packages is privileged,
potentially disruptive (can restart services, need a reboot, fail
halfway through), and deserves its own confirmation flow — not a bare
`set updates.foo true`.
- **Account creation/deletion.** `services::accounts::list` is read-only.
Creating or removing accounts is out of scope for a settings applet.
- **Escalation UI.** `permissions::privileged::run_as_root` will try
`pkexec` then non-interactive `sudo`, but this project ships no
graphical polkit agent — that's a session/desktop-environment concern,
not a settings-daemon concern.

## Reporting a real vulnerability

This is a reference implementation, not a hardened production daemon.
If you're adapting it for real deployment, treat the "daemon trust model"
section above as the first thing to fix, not an edge case.
