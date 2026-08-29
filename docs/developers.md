# Developer guide

## Building

```sh
cargo build
cargo test
cargo run -- list
```

No external crates — `cargo build` needs nothing beyond a stable Rust
toolchain and never touches the network.

## Adding a new setting to an existing category

1. Open the relevant file in `src/categories/`.
2. Add a `schema.register(SettingSpec::new(...))` call in `register()`,
   picking a key of the form `<category>.<name>`.
3. If it needs live enforcement of the change (not just persistence), add
   a match arm in `services::apply` (`src/services/mod.rs`) that calls
   into a `services::*` function — write one if it doesn't exist yet.
4. If you want the app/CLI to show something read-only alongside it (a
   live device list, a computed status), add or extend `live_info()` on
   that category.

That's it — persistence, validation, the CLI, the interactive app, and IPC
all pick it up automatically because they all go through `Schema` and
`SettingsManager`, never a hardcoded key list.

## Adding a whole new category

1. Create `src/categories/your_category.rs` following the shape of any
   existing one: a unit struct implementing `Category`.
2. Add `pub mod your_category;` to `src/categories/mod.rs`.
3. Add `Box::new(your_category::YourCategory),` to `categories::all()`, in
   the position you want it to appear in the navigator.

`categories::mod.rs` has a test (`every_category_has_a_unique_id`) that
will catch a copy-pasted `id()`.

## Testing approach

- **Unit tests** live inside each module (`#[cfg(test)] mod tests` at the
  bottom of the file) and test that module in isolation — e.g.
  `settings::value`'s tests only check `Value` encode/decode, with no
  schema or manager involved.
- **Integration tests** (`tests/*.rs`) exercise the public API end to end,
  including one (`tests/validation.rs::every_writable_settings_default_passes_its_own_validation`)
  that walks every registered setting and checks its own default doesn't
  violate its own range/choices constraint — the cheapest way to catch a
  typo'd `SettingSpec`.
- Tests that need a `SettingsManager` use
  `SettingsManager::with_stores(mode, Store::at(tmp1), Store::at(tmp2))`
  with a unique temp directory per test, rather than pointing at the real
  `~/.config`/`/etc` paths or mutating `$HOME`. This is what makes it safe
  to run the whole suite with default (parallel, multi-threaded) `cargo
  test` — nothing shares mutable global state.
- `tests/ipc.rs` spins up a real `IpcServer` on a throwaway Unix socket
  in a background thread and talks to it with a real `IpcClient` — no
  mocking of the protocol layer.

## Project layout

See `docs/architecture.md` for the module dependency diagram. The short
version: `hardware` (read-only) → `services` (mutates) → `categories`
(registers settings, exposes live info) → `settings::manager` (the one
orchestrator) → `cli` / `app` / `ipc::server` (front-ends).
