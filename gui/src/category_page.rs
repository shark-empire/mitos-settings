//! Builds one category's page generically from the schema — every setting
//! in a category gets a row with the widget appropriate to its
//! `ValueKind`/constraints (see `widgets::build_setting_row`), plus
//! read-only rows for anything the category exposes via `live_info()`.
//! Nothing here is hand-authored per category, with one deliberate
//! exception: the Users page turns its account list into actionable rows
//! with a "Change Password" button, because changing a password is an
//! *action*, not a stored setting.

use crate::widgets;
use gtk::prelude::*;
use mitos_settings::categories::Category;
use mitos_settings::settings::manager::SettingsManager;
use std::cell::RefCell;
use std::rc::Rc;

pub fn build(
    category: &dyn Category,
    manager: &Rc<RefCell<SettingsManager>>,
) -> gtk::ScrolledWindow {
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);

    let specs: Vec<_> = {
        let m = manager.borrow();
        m.schema().by_category(category.id()).cloned().collect()
    };

    for spec in &specs {
        list.append(&widgets::build_setting_row(spec, manager));
    }

    if category.id() == "users" {
        // Actionable account rows *instead of* the generic read-only
        // live_info rows, so the account list doesn't appear twice.
        for account in mitos_settings::services::accounts::list() {
            list.append(&build_account_row(&account.username, account.uid));
        }
    } else {
        for (label, value) in category.live_info() {
            list.append(&widgets::build_info_row(label, &value));
        }
    }

    let scrolled = gtk::ScrolledWindow::new();
    scrolled.set_child(Some(&list));
    scrolled.set_vexpand(true);
    scrolled.set_hexpand(true);
    scrolled
}

/// One account row: "username (uid N)" on the left, a Change Password
/// button on the right that opens the dialog against the daemon.
fn build_account_row(username: &str, uid: u32) -> gtk::Widget {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.set_margin_top(6);
    row.set_margin_bottom(6);
    row.set_margin_start(12);
    row.set_margin_end(12);

    let label = gtk::Label::new(Some(&format!("{username} (uid {uid})")));
    label.set_halign(gtk::Align::Start);
    label.set_hexpand(true);
    row.append(&label);

    let button = gtk::Button::builder().label("Change Password…").build();
    let username = username.to_string();
    button.connect_clicked(move |button| {
        // If your gtk4-rs version lacks `and_downcast`, use:
        // button.root().and_then(|r| r.downcast::<gtk::Window>().ok())
        let Some(window) = button.root().and_downcast::<gtk::Window>() else {
            return;
        };
        crate::dialogs::change_password::show(&window, &username);
    });
    row.append(&button);

    row.upcast::<gtk::Widget>()
}
