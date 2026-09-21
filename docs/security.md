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

## The daemon's trust model

Every connection to `ipc::server::IpcServer`'s Unix socket is
authenticated two ways, layered:

1. **Socket file permissions.** `IpcServer::bind` sets the socket to mode
   `0660` and its parent directory to `0710` (owner: full access; group:
   can open the socket by name but not list the directory; everyone else:
   nothing). This is the first gate — if you can't reach the socket at
   all, nothing past this matters.
2. **Per-connection identity via `SO_PEERCRED`.** `ipc::permissions::peer_credentials`
   asks the kernel who is *actually* on the other end of each connection
   — a real, unspoofable uid, not just "did they reach the socket."
   `ipc::server::handle` resolves this into an `AuthContext` (via
   `permissions::context_for_uid`) before dispatching the request, and
   `SettingsManager::set_for_peer`/`reset_for_peer` check *that* identity
   against the setting's required `PrivilegeLevel` — not the daemon's own
   (root) identity, which is a different question entirely (see below).

This means a non-admin user who can reach the socket can still only apply
`User`-level settings; the daemon now actually distinguishes between
different connecting users, rather than trusting anyone who gets that far.

**Known limitation:** `SO_PEERCRED`'s numeric value (`ipc::permissions::SO_PEERCRED`)
is hardcoded rather than pulled from a per-architecture table like the
`libc` crate maintains. The hardcoded value is correct for the
architectures this project actually targets — x86, x86_64, arm, aarch64,
i.e. real desktop/laptop hardware — but would need updating to port to
something like MIPS or SPARC.

## Two different questions: "is the daemon capable" vs. "is the peer allowed"

These compose, and it's worth being explicit that they're separate checks:

- `ipc::permissions::ensure_daemon_may_apply` asks "can the daemon *process
  itself* even do this" — a `Root`-level setting is refused unless the
  daemon is actually running as uid 0, independent of who's asking.
- `SettingsManager::set_for_peer` asks "is *this specific connecting user*
  allowed to do this" — checked against their real, `SO_PEERCRED`-verified
  identity.

Both have to pass. Neither is a substitute for the other.

## How privilege is determined

`permissions::authorization::current_context` (the daemon's own identity)
and `context_for_uid` (an arbitrary peer's identity, resolved from a
`SO_PEERCRED` uid) both shell out to the `id` coreutil rather than calling
`getuid(2)`/`getgroups(2)` through an FFI crate — `SO_PEERCRED` itself is
the one place this project uses raw FFI, and only for that one syscall.
A caller is `Admin` if they're uid 0, or a member of
`sudo`/`wheel`/`admin`/`mitos-admin`.

## Permission grants

Worth calling out on its own because it crosses into two *other*
projects' trust models, not just this one's: this daemon no longer
talks to mitos-session at all for grants (an earlier version did — see
git history if curious); it talks only to mitos-service, and
mitos-service is the one that talks to mitos-session.

**What this daemon can and can't do.** `grants::service_client` sends
`GRANT <sha256> <capability> <allow|deny> <scope> <uid>` to
mitos-service's control socket, with `<uid>` always the *connecting
peer's own* (`SO_PEERCRED`-resolved, same as every other privileged
write here) — never a value the request itself supplies, so a person
can only ever grant something against their own mitos-session session
through this client, never someone else's. What happens after that is
entirely mitos-service's decision and mitos-session's verification;
this daemon has no way to skip, weaken, or observe that check — it
just relays mitos-service's eventual answer (`granted` / `denied: ...`
/ `error: ...`) back to whoever asked.

**What "granted" does and doesn't establish.** A successful `GRANT`
for a `dangerous`/`critical` capability means the person at the
keyboard typed their account's real password, verified by
mitos-session over real PAM. It says nothing about whether flipping
*this particular* grant was wise — that judgment call (is `root_shell`
actually appropriate for this app?) is left entirely to whoever's
looking at the toggle, the same way typing your password for `sudo`
doesn't mean the command you're about to run is a good idea.

**Current simplification, inherited from mitos-service rather than
introduced here: grants are system-wide, not per-user.**
mitos-service's rulebook keys each grant by `(sha256, capability)`
alone, with no uid anywhere in the record — every account on the
machine shares one grant per app. For MITOS's apparent
single-primary-user desktop focus this is a reasonable starting point,
but it's a real gap on a genuinely multi-user machine (user A denying
an app's camera access would also deny it for user B). Not something
this project can fix unilaterally — it would need mitos-service's own
rulebook to grow a uid dimension.

**What identifies an app: a binary hash, not a name — and this daemon
trusts whatever hash a caller supplies.** Neither this daemon nor
mitos-service independently verifies that a `sha256` actually
corresponds to the app the person granting it thinks they're granting
it for; see mitos-service's own `SECURITY.md` for the same limitation
stated on that side. `grants::service_client::compute_sha256` (used by
this daemon's own GUI) at least computes the hash from a real file
rather than trusting a pasted string blindly, but a person can still
paste an arbitrary hash they got from anywhere.

## What's deliberately *not* implemented

- **Applying system updates.** `services::updates::check_pending` is
  read-only by design. Actually upgrading packages is privileged,
  potentially disruptive (can restart services, need a reboot, fail
  halfway through), and deserves its own confirmation flow — not a bare
  `set updates.foo true`. (This is doubly true for MITOS specifically,
  since it's getting its own package manager rather than
  apt/dnf/pacman — `services::updates::PackageManager` will need a real
  rework once that exists, not just a new enum variant.)
- **Account creation/deletion.** `services::accounts::list` is read-only.
  Creating or removing accounts is out of scope for a settings applet.
- **Escalation UI for settings other than grants.** `permissions::privileged::run_as_root`
  will try `pkexec` then non-interactive `sudo` for an ordinary
  `Admin`/`Root`-level setting, but this project ships no graphical
  polkit agent for that path — a session/desktop-environment concern
  this settings daemon doesn't take on generally. Permission grants
  specifically are the one place a real password prompt exists today
  (via mitos-session, see above) precisely because that flow was
  already built and mitos-session already runs the real thing; it
  hasn't been generalized to cover every other privileged setting in
  this project.
- **Auditing who changed what.** `SO_PEERCRED` resolution means the
  daemon *could* log "uid 1000 (amy) changed network.proxy_mode" today,
  but nothing currently writes such a log anywhere durable for ordinary
  settings. Grant changes are a partial exception: mitos-session logs
  every elevation request/response it handles on this daemon's behalf
  (see that project's `logging::audit_log`), but a grant change that
  *didn't* need elevating leaves no record at all on either side.

## Reporting a real vulnerability

This is a reference implementation. The trust model above is real and
enforced, not aspirational — but it hasn't been through any kind of
external review, so treat it as "reasonable defaults for a from-scratch
OS," not "audited."
