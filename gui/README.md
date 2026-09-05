# mitos-settings-gui

The graphical front-end for MITOS settings — so a user who's never opened
a terminal never has to. A `StackSidebar` of the 26 categories on the
left, a form on the right, one row per setting, generated straight from
the schema. Same `SettingsManager` as the CLI and daemon underneath it —
this crate is purely presentation.

```
$ cargo build --release -p mitos-settings-gui
$ ./target/release/mitos-settings-gui
```

## Read this before building

**I wrote this without a working Rust compiler or network access in the
sandbox I built it in — see the root README for the full explanation.**
Everything else in this repo, I could reason through carefully and check
against the existing test suite. GTK4 bindings are a much bigger, more
version-sensitive API surface than anything else here, and I have no way
to check this compiles clean on a first try. Treat this crate as the
least-verified part of the whole project.

If it doesn't build, here's where to look first, roughly in order of how
likely I think each one is to need a tweak:

1. **The `gtk4` version pin in `Cargo.toml`** (`version = "0.9"`) — this is
   a guess, not a checked-available version. Run `cargo add gtk4` in this
   directory to let Cargo resolve whatever's actually current, or check
   what your system's GTK4 dev libraries need (gtk4-rs versions track
   minimum GTK4 C library versions).
2. **Signal/getter names in `src/widgets.rs`** — specifically
   `connect_active_notify`/`is_active` (Switch), `connect_selected_notify`
   (DropDown), `DropDown::from_strings`. I picked the "notify::property"
   style signals deliberately over signals-that-return-a-value (like
   `state-set`) because their signature (`Fn(&Self)`, no return value) has
   stayed more stable across gtk-rs versions — but exact method names can
   still drift.
3. Everything else (`Box::new`, `.append()`, `Adjustment::new`,
   `SpinButton::new`, `ApplicationWindow`, `.set_*` property setters,
   `.upcast()`/`.upcast_ref()`, `add_css_class`/`remove_css_class`) is
   foundational, long-stable GTK4 API that's shown the same way in
   essentially every gtk4-rs tutorial — lower risk, but not zero.

None of these are structural — the "walk the schema, build a row per
setting, write through `SettingsManager::set`" shape is right regardless
of what specific method names need fixing. If you (or I, next session) hit
a compile error, paste it back and it should be a fast, localized fix, not
a rewrite.

## System requirements

GTK4 development headers, needed to build (not just run) any GTK4 app:

```sh
# Debian/Ubuntu-family
apt install libgtk-4-dev

# Fedora-family
dnf install gtk4-devel

# Arch-family
pacman -S gtk4
```

Since `mitos-file-manager` is already a GTK4 app per its own spec (see
`../docs/home-conf.md`), whatever base MITOS install includes GTK4 for
that already covers this.

## Install

```sh
install -Dm755 target/release/mitos-settings-gui /usr/bin/mitos-settings-gui
install -Dm644 mitos-settings-gui.desktop /usr/share/applications/mitos-settings-gui.desktop
```

Once installed, "Settings" shows up in the MITOS launcher/dock like any
other app — no CLI knowledge needed anywhere in that path.

## What's not handled yet (v1 scope)

- **Live refresh.** If another process (the CLI, another instance of this
  GUI, the daemon) changes a setting while this window is open, it won't
  update on its own — you'd need to restart the app to see it. `SettingsManager::events`
  (an `EventBus`) already exists for exactly this, but wiring an
  `mpsc::Receiver` into GTK's main loop safely (via `glib::MainContext`)
  is its own chunk of GTK-specific API risk, so it's deliberately left out
  of this first pass rather than compounding the risk in one shot.
- **String lists** (`applications.startup_applications`,
  `language.keyboard_layouts`) are edited as plain comma-separated text,
  not a proper add/remove list widget.
- **No search.** With 122 settings across 26 categories, a search bar
  (`GtkSearchEntry` filtering the sidebar/rows) would help — straightforward
  to add once the base UI is confirmed working.
- **No live theming from `appearance.*`.** The window uses your system's
  default GTK4 theme; it doesn't apply `theme.mode`/`appearance.accent_color`
  to itself. A little on-the-nose for a settings app not to reflect its
  own settings, but avoiding GTK CSS-provider APIs kept this first pass
  smaller.
