# mitos-power IPC reference

Two files copied verbatim from mitos-power's own `src/ipc/` (as of the
`mitos-power-1.zip` shared during this integration work), so the schema
`src/power/client.rs` is built against sits next to the code that depends
on it:

- `messages.rs` -- per-method param structs and the `method_names`
  tiers (`PUBLIC` / `SESSION` / `PRIVILEGED`).
- `protocol.rs` -- the wire framing: newline-delimited JSON,
  `{"kind":"request","id":..,"method":..,"params":..}` in,
  `{"kind":"response","id":..,"ok":..,"result"|"error":..}` out. Not the
  same shape as mitos-network's (`../mitos-network-reference/`), which is
  length-prefixed with externally-tagged enums.

Things that only became clear from the source, not from method names, and
are easy to get wrong again if this snapshot goes stale:

- `SetIdleTimeout` sets the **display-off** threshold, not a suspend
  timeout (`IdleDetector::set_off_timeout` -> `DisplayTimeouts.off_after`).
  Suspend has its own `suspend_after`/`suspend_on_idle`, config-file-only.
- A timeout of `0` means "instantly idle", not "never".
- `SetProfile` replaces the whole timeout set with the profile's own, and
  `SetIdleTimeout` isn't persisted -- so it must be re-pushed after a
  profile change (`services::reapply_dependents`).
- Profile names on the wire are `performance` / `balanced` / `powersave`
  (no hyphen, no trailing "r"); `GetPowerState` reports the profile as
  `{"kind":"power_saver"}` (adjacently tagged) -- two different
  vocabularies inside mitos-power itself.
- Lid action, low/critical battery thresholds and actions, and suspend-on-
  idle are all config-file-only in mitos-power; there is no IPC method to
  set any of them.

This is a snapshot: if mitos-power's schema changes, re-check
`power::client` against the real file.
