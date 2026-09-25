//! The main window: a `StackSidebar` (the standard GTK pattern for exactly
//! this "categories on the left, settings on the right" shape — the same
//! widget pairing GNOME Settings itself uses) driving a `Stack` with one
//! page per category. Every page comes from `category_page::build`, which
//! walks the schema generically — nothing here is hand-authored per
//! category, so a new category added anywhere in `mitos_settings::categories`
//! shows up automatically next time this is rebuilt.
//!
//! Above the sidebar sits a `SearchEntry` that searches every category at
//! once, not just whichever one happens to be open — matching against each
//! setting's label/description/category name straight from the schema
//! (`SettingsManager::schema`), and reusing `widgets::build_setting_row`
//! for each hit so a setting found this way is exactly as live and
//! editable as it is on its own category page (including read-only rows
//! rendering the same dimmed-label way). An empty query just hides the
//! results view again; the normal sidebar-driven `stack` underneath it
//! never goes away.

use crate::category_page;
use crate::widgets;
use gtk::prelude::*;
use mitos_settings::categories;
use mitos_settings::settings::manager::SettingsManager;
use mitos_settings::settings::schema::SettingSpec;
use std::cell::RefCell;
use std::rc::Rc;

pub fn build(app: &gtk::Application, manager: Rc<RefCell<SettingsManager>>) {
    let stack = gtk::Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    rebuild_categories(&stack, &manager);

    let sidebar = gtk::StackSidebar::new();
    sidebar.set_stack(&stack);
    sidebar.set_width_request(200);
    // `search_entry` above takes its own (non-expanding) height, so
    // `sidebar` needs to explicitly claim the rest of `sidebar_box`'s
    // vertical space instead of shrinking to its own natural size.
    sidebar.set_vexpand(true);

    let search_entry = gtk::SearchEntry::new();
    // `SearchEntry` carries its own `placeholder-text` property in GTK4
    // (separate from `Entry`'s) -- if this specific setter turns out not
    // to be in whatever gtk4-rs version resolves, it's safe to just
    // delete this one line, same "check this first" spirit as the API-risk
    // note in `widgets.rs`.
    search_entry.set_placeholder_text(Some("Search settings"));
    search_entry.set_margin_top(8);
    search_entry.set_margin_bottom(4);
    search_entry.set_margin_start(8);
    search_entry.set_margin_end(8);

    let sidebar_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    sidebar_box.append(&search_entry);
    sidebar_box.append(&sidebar);

    // A plain vertical `Box` rather than a `ListBox` -- these rows are
    // never selected or activated, just displayed, and clearing/rebuilding
    // a `Box` via `first_child()`/`remove()` is the exact pattern
    // `permissions_page::refresh_grant_list` already uses successfully.
    let results_list = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let results_scrolled = gtk::ScrolledWindow::new();
    results_scrolled.set_child(Some(&results_list));
    results_scrolled.set_hexpand(true);
    results_scrolled.set_vexpand(true);
    // Hidden until there's a query -- see the `search-changed` handler
    // below. An invisible widget takes no space in `content`'s layout,
    // so this doesn't leave a gap next to `stack`.
    results_scrolled.set_visible(false);

    {
        let manager = Rc::clone(&manager);
        let stack = stack.clone();
        let results_list = results_list.clone();
        let results_scrolled = results_scrolled.clone();
        search_entry.connect_search_changed(move |entry| {
            let query = entry.text().to_string();
            let query = query.trim();

            while let Some(child) = results_list.first_child() {
                results_list.remove(&child);
            }

            if query.is_empty() {
                results_scrolled.set_visible(false);
                stack.set_visible(true);
                return;
            }
            stack.set_visible(false);
            results_scrolled.set_visible(true);

            // Collect owned matches first and drop the borrow before
            // building any widgets -- `widgets::build_setting_row` takes
            // its own `.borrow()` of `manager` (for the current value,
            // and later a `.borrow_mut()` from whatever control it
            // builds once the person actually uses it), so ending this
            // borrow here first avoids relying on `RefCell` tolerating
            // nested immutable borrows. Same shape `category_page::build`
            // already uses for its own per-category row list.
            let needle = query.to_lowercase();
            let matches: Vec<(&'static str, SettingSpec)> = {
                let manager_ref = manager.borrow();
                let schema = manager_ref.schema();
                schema
                    .all()
                    .filter(|spec| {
                        format!("{} {} {}", spec.label, spec.description, spec.category)
                            .to_lowercase()
                            .contains(&needle)
                    })
                    .map(|spec| {
                        let category_name = schema
                            .category(spec.category)
                            .map(|c| c.name)
                            .unwrap_or(spec.category);
                        (category_name, spec.clone())
                    })
                    .collect()
            };

            if matches.is_empty() {
                let empty = gtk::Label::new(Some("No matching settings."));
                empty.add_css_class("dim-label");
                empty.set_margin_top(24);
                empty.set_margin_bottom(24);
                results_list.append(&empty);
            } else {
                for (category_name, spec) in &matches {
                    let entry_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
                    let breadcrumb = gtk::Label::new(Some(*category_name));
                    breadcrumb.add_css_class("caption");
                    breadcrumb.add_css_class("dim-label");
                    breadcrumb.set_halign(gtk::Align::Start);
                    breadcrumb.set_margin_start(12);
                    breadcrumb.set_margin_top(6);
                    entry_box.append(&breadcrumb);
                    entry_box.append(&widgets::build_setting_row(spec, &manager));
                    results_list.append(&entry_box);
                }
            }
        });
    }

    let content = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    content.append(&sidebar_box);
    content.append(&stack);
    content.append(&results_scrolled);

    let window = gtk::ApplicationWindow::new(app);
    window.set_title(Some("MITOS Settings"));
    window.set_default_width(920);
    window.set_default_height(640);
    window.set_child(Some(&content));

    // Live reload: reflect a setting changed by *another* process (the
    // CLI, another instance of this GUI, mitos-service) without the
    // person closing and reopening the window. `manager`'s own
    // `EventBus` additionally covers a hypothetical second window in
    // this same process; `IpcClient::subscribe` covers everything
    // outside it -- see README's "Known gaps". Either signal just means
    // "something changed"; both drive the same rebuild rather than
    // trying to patch individual widgets in place, reusing the exact
    // `rebuild_categories` the initial build above already went
    // through (the same "clear and rebuild" shape `permissions_page`
    // already uses elsewhere in this crate).
    //
    // Polls both receivers every 500ms on the main loop instead of
    // pushing straight from the background thread that reads them --
    // GTK widgets can't be touched off the main thread, and this
    // sidesteps needing to pick a specific cross-thread channel type
    // that's liable to have moved between gtk4-rs versions. The
    // `timeout_add_local`/`ControlFlow` call below is itself a
    // best-effort guess at this crate's pinned gtk4-rs version's exact
    // API (same spirit as the other "check this first" notes in this
    // crate) -- if it comes back as unresolved, `glib::timeout_add_local`
    // (crate root, no `gtk::` prefix) is the other likely spelling, and
    // an older version would want `glib::Continue(true)` as the closure's
    // return value instead of `gtk::glib::ControlFlow::Continue`.
    {
        let events_rx = manager.borrow().events.subscribe();
        let daemon_rx = mitos_settings::ipc::IpcClient::subscribe(
            &mitos_settings::config::paths::daemon_socket_path(),
        )
        .ok();
        let manager = Rc::clone(&manager);
        let stack = stack.clone();
        gtk::glib::source::timeout_add_local(std::time::Duration::from_millis(500), move || {
            let mut changed = false;
            while events_rx.try_recv().is_ok() {
                changed = true;
            }
            if let Some(rx) = &daemon_rx {
                while rx.try_recv().is_ok() {
                    changed = true;
                }
            }
            if changed {
                rebuild_categories(&stack, &manager);
            }
            gtk::glib::ControlFlow::Continue
        });
    }

    window.present();
}

/// (Re)populates `stack` with one page per category, straight from the
/// schema -- used both for the initial build and, unchanged, for every
/// live-reload refresh (see `build`'s live-reload block below), so a
/// refresh can never drift from what a fresh launch would show.
/// Preserves whichever page was visible before the rebuild, if any.
fn rebuild_categories(stack: &gtk::Stack, manager: &Rc<RefCell<SettingsManager>>) {
    let previous = stack.visible_child_name();

    while let Some(child) = stack.first_child() {
        stack.remove(&child);
    }

    for cat in categories::all() {
        let page = category_page::build(cat.as_ref(), manager);
        stack.add_titled(&page, Some(cat.id()), cat.name());
    }

    if let Some(name) = previous {
        stack.set_visible_child_name(&name);
    }
}
