//! Entry point for the MITOS Settings graphical app. A thin GTK4 shell over
//! `mitos_settings::settings::manager::SettingsManager` — every widget
//! reads from and writes through the exact same schema, validation, and
//! privilege-escalation logic the CLI and the daemon use. Nothing about
//! settings semantics lives in this crate; it's purely presentation.
//!
//! See `../../INTEGRATION.md` for how this fits the rest of MITOS, and
//! this crate's `README.md` for what's not yet handled (live refresh when
//! another process changes a setting, search) and which specific API calls
//! are best-effort guesses worth checking first if this doesn't compile
//! clean.

mod category_page;
mod widgets;
mod window;

use gtk::prelude::*;
use mitos_settings::settings::manager::{Mode, SettingsManager};
use std::cell::RefCell;
use std::rc::Rc;

const APP_ID: &str = "org.mitos.Settings";

fn main() {
    let app = gtk::Application::builder().application_id(APP_ID).build();

    app.connect_activate(|app| {
        let manager = match SettingsManager::load(Mode::Standalone) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("mitos-settings-gui: could not load settings: {e}");
                std::process::exit(1);
            }
        };
        window::build(app, Rc::new(RefCell::new(manager)));
    });

    app.run();
}
