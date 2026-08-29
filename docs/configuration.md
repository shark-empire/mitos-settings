# Configuration

## File format

Settings are stored as plain `key=value` text files — no external
serialization crate is used (see `docs/architecture.md` for why). Example:

```
__version__=2
display.brightness=int:80
network.wifi_enabled=bool:true
theme.mode=str:dark
language.keyboard_layouts=strlist:us
```

Every value is stored as `<kind>:<payload>`:

| kind      | payload                                                    |
|-----------|-------------------------------------------------------------|
| `bool`    | `true` or `false`                                            |
| `int`     | a base-10 integer                                             |
| `float`   | a base-10 float                                               |
| `str`     | raw text, with `\` and newlines escaped as `\\` / `\n`         |
| `strlist` | items joined by the ASCII unit separator (0x1F), never typed  |

Lines starting with `#` are comments; blank lines are ignored. The first
line is always `__version__=N` (see Migrations below).

Writes are atomic: `config::writer` writes to a `.tmp` file in the same
directory and renames it over the real path, so a crash or power loss
mid-write can never leave a half-written config file.

## Paths

| Store  | Path                              | Writable by                     |
|--------|------------------------------------|----------------------------------|
| User   | `$XDG_CONFIG_HOME/mitos-settings/settings.conf` (falls back to `~/.config/mitos-settings/settings.conf`) | the signed-in user |
| System | `/etc/mitos-settings/settings.conf` | root / the daemon               |

A setting's `SettingSpec.privilege` determines which store it lives in:
anything above `User` privilege is persisted to the system store, never
the user one — see `settings::manager::SettingsManager::persist`.

The daemon listens on a fixed Unix socket, `/run/mitos-settings/daemon.sock`
(see `docs/security.md` for its permissions).

## Migrations

Bumping the on-disk format means: increment `config::loader::CURRENT_VERSION`,
then add a `migrate_vN_to_vN+1` step in `config::migration::migrate`. Existing
files at the old version are transformed in memory the next time they're
loaded (`settings::persistence::Store::load` always calls `migrate` after
`loader::load`). The file on disk isn't rewritten until something calls
`Store::save` again — reading an old file never silently mutates it.

`config::migration` currently ships one example migration (v1 → v2:
renaming the legacy bare `wifi.enabled` key to `network.wifi_enabled`) to
show the pattern; see its tests for a worked example.
