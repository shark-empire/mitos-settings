//! Builds one category's page generically from the schema — every setting
//! in a category gets a row with the widget appropriate to its
//! `ValueKind`/constraints (see `widgets::build_setting_row`), plus
//! read-only rows for anything the category exposes via `live_info()`.
//! Nothing here is hand-authored per category.

use crate::widgets;
use gtk::prelude::*;
use mitos_settings::categories::Category;
use mitos_settings::settings::manager::SettingsManager;
use std::cell::RefCell;
use std::rc::Rc;

pub fn build(category: &dyn Category, manager: &Rc<RefCell<SettingsManager>>) -> gtk::ScrolledWindow {
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);

    // Clone the specs out and drop the borrow immediately, rather than
    // holding `manager.borrow()` across widget construction -- keeps this
    // function obviously free of any risk of a RefCell double-borrow, even
    // though in practice the borrow would end before any callback could
    // fire anyway.
    let specs: Vec<_> = {
        let m = manager.borrow();
        m.schema().by_category(category.id()).cloned().collect()
    };

    for spec in &specs {
        list.append(&widgets::build_setting_row(spec, manager));
    }

    for (label, value) in category.live_info() {
        list.append(&widgets::build_info_row(label, &value));
    }

    let scrolled = gtk::ScrolledWindow::new();
    scrolled.set_child(Some(&list));
    scrolled.set_vexpand(true);
    scrolled.set_hexpand(true);
    scrolled
}
