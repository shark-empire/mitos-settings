# mitos-settings-gui

The graphical front-end for MITOS settings — so a user who's never opened
a terminal never has to. A `StackSidebar` of the 27 categories on the
left, a form on the right, one row per setting, generated straight from
the schema, plus a search box that looks across every category at once.
Same `SettingsManager` as the CLI and daemon underneath it — this crate
is purely presentation.

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
3. **`src/window.rs`'s search box**: `gtk::SearchEntry`, its
   `connect_search_changed` signal, and `.text()` via the `Editable`
   trait it shares with `Entry` (already proven elsewhere in this file).
   The one genuinely new property is `SearchEntry::set_placeholder_text`
   — `Entry` definitely has it, `SearchEntry` almost certainly re-exposes
   it, but that's the one line to check first if this specific file
   doesn't build.
4. **`src/widgets.rs`'s reset button**: `Box::insert_child_after`, used
   to swap a freshly-rebuilt control into a row in place of the old one.
   Standard, symmetrical with `append`/`prepend`/`remove` (all already
   proven elsewhere in this file), but less-exercised in typical gtk4-rs
   sample code than those three.
5. Everything else (`Box::new`, `.append()`, `Adjustment::new`,
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

- ~~**Live refresh.**~~ Done: `window.rs` now polls both `SettingsManager::events`
  (in-process — covers a hypothetical second window) and a new
  `IpcClient::subscribe` connection (cross-process — the daemon pushes over
  the IPC socket now, see the root README's "Known gaps") every 500ms on the
  main loop, rebuilding the category stack when either has something
  pending. Deliberately polling rather than pushing straight from the
  background thread into GTK's main loop: that would need picking a
  specific cross-thread channel type (`glib::MainContext::channel` or
  whatever its current replacement is), which is exactly the kind of
  version-specific API this crate can't verify without a compiler, so
  `gtk::glib::source::timeout_add_local` — a plainer, longer-stable call —
  was worth the small latency trade-off instead. That call itself is still
  a best-effort guess at this crate's pinned gtk4-rs version's exact
  spelling; see the comment above it in `window.rs`.
- **String lists** (`applications.startup_applications`,
  `language.keyboard_layouts`) are edited as plain comma-separated text,
  not a proper add/remove list widget.
- **No live theming from `appearance.*`.** The window uses your system's
  default GTK4 theme; it doesn't apply `theme.mode`/`appearance.accent_color`
  to itself. A little on-the-nose for a settings app not to reflect its
  own settings, but avoiding GTK CSS-provider APIs kept this first pass
  smaller.
- ~~**`SettingSpec::dangerous`'s staged-then-confirm flow**~~ Done: all
  five `build_*` control functions in `widgets.rs` now share the same
  stage-then-Apply shape `build_switch` originated.
