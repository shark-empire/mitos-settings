# Settings API

## As a library

```rust
use mitos_settings::settings::manager::{Mode, SettingsManager};
use mitos_settings::settings::value::Value;

let mut manager = SettingsManager::load(Mode::Standalone)?;

// Read
let brightness = manager.get("display.brightness")?;
println!("{brightness}");

// Write (validated against the setting's schema automatically)
manager.set("display.brightness", Value::Int(70))?;

// Reset to default
manager.reset("display.brightness")?;

// Inspect what's available
for spec in manager.schema().by_category("display") {
    println!("{} ({}) = default {}", spec.key, spec.kind, spec.default);
}
```

`SettingsManager::load` reads from the real, well-known config paths (see
`docs/configuration.md`). Tests and embedders that want isolated storage
should use `SettingsManager::with_stores` instead, passing
`settings::persistence::Store::at(some_path)` for both the user and system
stores.

## Key naming

Every setting's key is `<category>.<name>`, e.g. `display.brightness`,
`network.wifi_enabled`, `sound.volume`. Run `mitos-settings list` to see
every category, or `mitos-settings list <category>` to see one category's
keys and current values.

## CLI

```text
mitos-settings get <key>              Print the current value of a setting
mitos-settings set <key> <value>      Change a setting
mitos-settings list [category]        List categories, or one category's settings
mitos-settings reset <key> | --all    Restore a setting (or everything) to default
mitos-settings pick-wallpaper         Open the MITOS file picker and set the wallpaper
mitos-settings --daemon               Run as the privileged settings daemon
mitos-settings                        Open the interactive text navigator
```

Booleans accept `true`/`false`/`on`/`off`/`yes`/`no`/`1`/`0`. Lists
(`StrList` settings, like `applications.startup_applications`) are
comma-separated: `mitos-settings set applications.startup_applications
foo.desktop,bar.desktop`.

```text
$ mitos-settings get display.brightness
80

$ mitos-settings set sound.volume 65
Updated sound.volume.

$ mitos-settings set network.proxy_mode automatic
mitos-settings: 'network.proxy_mode' requires admin privileges (you
currently have user); re-run with sudo, or make sure the mitos-settings
daemon is running
```

## Errors

`SettingsManager` methods return `Result<_, SettingsError>`, whose
variants explain exactly what went wrong: `UnknownKey`, `Invalid` (a
validation failure — wrong type, out of range, not an allowed choice, or
the key is read-only), `PermissionDenied`, `Io`, or `Daemon` (the IPC
round-trip to the privileged daemon failed or the daemon itself rejected
the write). All of them implement `Display` with a message suitable for
showing directly to a user.
