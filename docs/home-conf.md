# home.conf: talking to mitos-gui and mitos-file-manager

`mitos-settings` is the source of truth for settings, but `mitos-gui` (the
compositor) and `mitos-file-manager` don't talk to it over IPC to get
their configuration — they watch a plain file, `~/.config/mitos/home.conf`,
via inotify, and re-read it whenever it changes. This document is the
contract between the three programs.

## Why a separate file from mitos-settings' own store

`settings::persistence` stores *every* setting, typed and namespaced
(`kind:payload`, `category.key`), because it needs to round-trip
losslessly and support 100+ settings across every category.
`home.conf` is a much smaller, hand-authored-looking projection of just
the settings a desktop shell and file manager need, in the plain format
they expect — no type tags, no unrelated settings. Think of it as a
public API with its own stability contract, versus mitos-settings'
internal storage format, which is free to change.

**mitos-settings owns writing this file. Nothing else should write to it.**

## Format

Flat `key = value`, one per line, `#` for comments:

```
# --- Visual Theme ---
theme_mode = dark
accent_color = #4d9eff
glass_opacity = 0.72
panel_radius = 18.0

# --- Wallpaper ---
wallpaper = /home/user/Pictures/mitos-bg.png

# --- Shell Layout ---
top_bar = true
top_bar_height = 38.0
dock = true
dock_height = 72.0
launcher = true

# --- File Manager Preferences ---
show_hidden_files = false
enable_thumbnails = true
thumbnail_max_mb = 50
```

| Key                    | Type            | Backing `mitos-settings` key                       |
|-------------------------|-----------------|------------------------------------------------------|
| `theme_mode`            | `light`/`dark`  | `theme.mode` (see note below)                        |
| `accent_color`          | hex color       | `appearance.accent_color`                             |
| `glass_opacity`         | float 0.0–1.0   | `appearance.glass_opacity`                             |
| `panel_radius`          | float           | `appearance.panel_radius`                              |
| `wallpaper`              | absolute path   | `wallpaper.desktop_path`                                |
| `top_bar`                | bool            | `appearance.top_bar_enabled`                             |
| `top_bar_height`         | float           | `appearance.top_bar_height`                              |
| `dock`                   | bool            | `appearance.dock_enabled`                                 |
| `dock_height`             | float           | `appearance.dock_height`                                   |
| `launcher`                | bool            | `appearance.launcher_enabled`                                |
| `show_hidden_files`        | bool            | `applications.file_manager_show_hidden`                       |
| `enable_thumbnails`         | bool            | `applications.file_manager_thumbnails_enabled`                  |
| `thumbnail_max_mb`           | int             | `applications.file_manager_thumbnail_max_mb`                      |

**`theme_mode` note:** `theme.mode` in mitos-settings' own schema has a
third value, `"system"`, that home.conf's contract doesn't understand.
`services::home_conf` currently resolves `"system"` to `"dark"` — there's
no real "what does the system want" signal to read at this layer (that's
arguably a session/compositor concern). If mitos-gui wants to own that
resolution instead (e.g. based on time of day), say so and this can be
changed to just pass `"system"` through.

## How updates happen

No IPC, no D-Bus, no signal. `services::home_conf::sync_if_relevant` runs
inside `SettingsManager::set`, every time — it checks whether the key that
changed is one of the ones in the table above, and if so, regenerates the
whole file from the manager's current values. Writing is atomic (temp file
+ rename), so a watcher never sees a half-written file. mitos-gui and
mitos-file-manager just need to watch `~/.config/mitos/home.conf` with
inotify (`IN_CLOSE_WRITE` or `IN_MOVED_TO`, since it arrives via rename)
and re-parse it on change.

The directory (`~/.config/mitos/`) is created automatically if it doesn't
exist yet — nothing else needs to pre-create it.

## Wallpaper picking

Rather than mitos-settings shipping its own file browser, picking a
wallpaper goes through the MITOS file picker over D-Bus:
`mitos-settings pick-wallpaper` calls `OpenFile` on the
`org.mitos.FilePicker` session-bus service and sets
`wallpaper.desktop_path` (which then flows into `home.conf` as usual) to
whatever path comes back.

This is implemented in `services::dbus` by shelling out to `gdbus`
(falling back to `dbus-send`) rather than a D-Bus client crate — see that
file's doc comment for the exact assumptions about the file picker's
object path and interface name, which should be confirmed once
`mitos-gui`/`mitos-file-manager` actually define that service.

## Adding a new home.conf key

1. Register the underlying setting in the right `categories/*.rs` file, as
   normal.
2. Add it to `RELEVANT_KEYS` in `src/services/home_conf.rs`.
3. Add the line-writing logic to `write_home_conf` in the same file.
4. Update the table above.
