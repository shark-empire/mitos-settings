//! The main window: a `StackSidebar` (the standard GTK pattern for exactly
//! this "categories on the left, settings on the right" shape — the same
//! widget pairing GNOME Settings itself uses) driving a `Stack` with one
//! page per category. Every page comes from `category_page::build`, which
//! walks the schema generically — nothing here is hand-authored per
//! category, so a new category added anywhere in `mitos_settings::categories`
//! shows up automatically next time this is rebuilt.

use crate::category_page;
use gtk::prelude::*;
use mitos_settings::categories;
use mitos_settings::settings::manager::SettingsManager;
use std::cell::RefCell;
use std::rc::Rc;

pub fn build(app: &gtk::Application, manager: Rc<RefCell<SettingsManager>>) {
    let stack = gtk::Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);

    for cat in categories::all() {
        let page = category_page::build(cat.as_ref(), &manager);
        stack.add_titled(&page, Some(cat.id()), cat.name());
    }

    let sidebar = gtk::StackSidebar::new();
    sidebar.set_stack(&stack);
    sidebar.set_width_request(200);

    let content = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    content.append(&sidebar);
    content.append(&stack);

    let window = gtk::ApplicationWindow::new(app);
    window.set_title(Some("MITOS Settings"));
    window.set_default_width(920);
    window.set_default_height(640);
    window.set_child(Some(&content));
    window.present();
}
