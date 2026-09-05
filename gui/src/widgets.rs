//! Builds the right control for a setting based on its `ValueKind` and
//! constraints — the schema-driven part that means this crate never
//! hand-authors a widget per setting. Every control writes through
//! `SettingsManager::set`, so validation, range/choice checks, and
//! privilege escalation (forwarding to the daemon over IPC when this
//! process isn't privileged enough) all come for free from the core crate.
//!
//! **API-risk note:** every GTK call here was written from memory of
//! well-established gtk4-rs patterns, without a compiler to check against
//! (see this crate's README.md). If something doesn't compile, the signal
//! names and getter methods (`connect_active_notify`, `is_active`,
//! `connect_selected_notify`, `DropDown::from_strings`) are the most likely
//! spots to need a small adjustment for whatever gtk4-rs version actually
//! resolves — the overall structure (one function per `ValueKind`, notify
//! signals over signals-with-return-values) should hold regardless.

use gtk::prelude::*;
use mitos_settings::permissions::PrivilegeLevel;
use mitos_settings::settings::manager::{SettingsError, SettingsManager};
use mitos_settings::settings::schema::{SettingSpec, ValueKind};
use mitos_settings::settings::value::Value;
use std::cell::RefCell;
use std::rc::Rc;

/// One row: label (+ description as a tooltip) on the left, the
/// value-appropriate control on the right. Read-only settings get a plain
/// dimmed label instead of an editable control.
pub fn build_setting_row(spec: &SettingSpec, manager: &Rc<RefCell<SettingsManager>>) -> gtk::Widget {
    let row = row_box();

    let label_text =
        if spec.privilege > PrivilegeLevel::User { format!("{} (admin)", spec.label) } else { spec.label.to_string() };
    let label = gtk::Label::new(Some(label_text.as_str()));
    label.set_halign(gtk::Align::Start);
    label.set_hexpand(true);
    label.set_tooltip_text(Some(spec.description));
    row.append(&label);

    if spec.read_only {
        let value_text = current_value_text(spec, manager);
        let value_label = gtk::Label::new(Some(value_text.as_str()));
        value_label.add_css_class("dim-label");
        row.append(&value_label);
    } else {
        row.append(&build_control(spec, manager));
    }

    row.upcast::<gtk::Widget>()
}

/// A read-only row for `Category::live_info()` output — no backing
/// setting, just a label/value pair.
pub fn build_info_row(label_text: &str, value_text: &str) -> gtk::Widget {
    let row = row_box();

    let label = gtk::Label::new(Some(label_text));
    label.set_halign(gtk::Align::Start);
    label.set_hexpand(true);
    row.append(&label);

    let value = gtk::Label::new(Some(value_text));
    value.add_css_class("dim-label");
    row.append(&value);

    row.upcast::<gtk::Widget>()
}

fn row_box() -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.set_margin_top(6);
    row.set_margin_bottom(6);
    row.set_margin_start(12);
    row.set_margin_end(12);
    row
}

fn current_value_text(spec: &SettingSpec, manager: &Rc<RefCell<SettingsManager>>) -> String {
    manager.borrow().get(spec.key).map(|v| v.to_string()).unwrap_or_default()
}

fn build_control(spec: &SettingSpec, manager: &Rc<RefCell<SettingsManager>>) -> gtk::Widget {
    match spec.kind {
        ValueKind::Bool => build_switch(spec, manager),
        ValueKind::Str if spec.choices.is_some() => build_dropdown(spec, manager),
        ValueKind::Int => build_int_spin(spec, manager),
        ValueKind::Float => build_float_spin(spec, manager),
        ValueKind::Str | ValueKind::StrList => build_entry(spec, manager),
    }
}

fn build_switch(spec: &SettingSpec, manager: &Rc<RefCell<SettingsManager>>) -> gtk::Widget {
    let current = manager.borrow().get(spec.key).ok().and_then(|v| v.as_bool()).unwrap_or(false);

    let switch = gtk::Switch::new();
    switch.set_active(current);
    switch.set_valign(gtk::Align::Center);

    let key = spec.key;
    let manager = Rc::clone(manager);
    switch.connect_active_notify(move |switch| {
        let result = manager.borrow_mut().set(key, Value::Bool(switch.is_active()));
        report_result(switch.upcast_ref::<gtk::Widget>(), result);
    });

    switch.upcast::<gtk::Widget>()
}

fn build_dropdown(spec: &SettingSpec, manager: &Rc<RefCell<SettingsManager>>) -> gtk::Widget {
    let choices = spec.choices.unwrap_or(&[]);
    let current =
        manager.borrow().get(spec.key).ok().and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_default();
    let current_index = choices.iter().position(|c| *c == current).unwrap_or(0) as u32;

    let dropdown = gtk::DropDown::from_strings(choices);
    dropdown.set_selected(current_index);

    let key = spec.key;
    let manager = Rc::clone(manager);
    let choices_owned: Vec<String> = choices.iter().map(|c| c.to_string()).collect();
    dropdown.connect_selected_notify(move |dropdown| {
        let Some(choice) = choices_owned.get(dropdown.selected() as usize) else { return };
        let result = manager.borrow_mut().set(key, Value::Str(choice.clone()));
        report_result(dropdown.upcast_ref::<gtk::Widget>(), result);
    });

    dropdown.upcast::<gtk::Widget>()
}

fn build_int_spin(spec: &SettingSpec, manager: &Rc<RefCell<SettingsManager>>) -> gtk::Widget {
    let (lo, hi) = spec.range.unwrap_or((0.0, 1_000_000.0));
    let current = manager.borrow().get(spec.key).ok().and_then(|v| v.as_int()).unwrap_or(0);

    let adjustment = gtk::Adjustment::new(current as f64, lo, hi, 1.0, 10.0, 0.0);
    let spin = gtk::SpinButton::new(Some(&adjustment), 1.0, 0);

    let key = spec.key;
    let manager = Rc::clone(manager);
    spin.connect_value_changed(move |spin| {
        let result = manager.borrow_mut().set(key, Value::Int(spin.value() as i64));
        report_result(spin.upcast_ref::<gtk::Widget>(), result);
    });

    spin.upcast::<gtk::Widget>()
}

fn build_float_spin(spec: &SettingSpec, manager: &Rc<RefCell<SettingsManager>>) -> gtk::Widget {
    let (lo, hi) = spec.range.unwrap_or((0.0, 1000.0));
    let current = manager.borrow().get(spec.key).ok().and_then(|v| v.as_float()).unwrap_or(0.0);

    let adjustment = gtk::Adjustment::new(current, lo, hi, 0.01, 0.1, 0.0);
    let spin = gtk::SpinButton::new(Some(&adjustment), 0.01, 2);

    let key = spec.key;
    let manager = Rc::clone(manager);
    spin.connect_value_changed(move |spin| {
        let result = manager.borrow_mut().set(key, Value::Float(spin.value()));
        report_result(spin.upcast_ref::<gtk::Widget>(), result);
    });

    spin.upcast::<gtk::Widget>()
}

/// Used for plain strings, hex colors, and string lists alike (v1: a
/// string list is just edited as comma-separated text, reusing
/// `Value::parse`'s existing splitting logic — no dedicated list-editor
/// widget yet, see README.md).
fn build_entry(spec: &SettingSpec, manager: &Rc<RefCell<SettingsManager>>) -> gtk::Widget {
    let current = manager.borrow().get(spec.key).map(|v| v.to_string()).unwrap_or_default();

    let entry = gtk::Entry::new();
    entry.set_text(&current);
    entry.set_width_chars(24);
    if spec.format.is_some() {
        entry.set_placeholder_text(Some("#RRGGBB"));
    }

    let key = spec.key;
    let kind = spec.kind;
    let manager = Rc::clone(manager);
    entry.connect_activate(move |entry| {
        let text = entry.text();
        match Value::parse(kind, &text) {
            Ok(value) => {
                let result = manager.borrow_mut().set(key, value);
                report_result(entry.upcast_ref::<gtk::Widget>(), result);
            }
            Err(reason) => {
                entry.add_css_class("error");
                entry.set_tooltip_text(Some(reason.as_str()));
            }
        }
    });
    entry.connect_changed(|entry| entry.remove_css_class("error"));

    entry.upcast::<gtk::Widget>()
}

/// Applies (or clears) the "error" CSS styling GTK4 themes recognize, and
/// surfaces the reason as a tooltip. This is how validation failures *and*
/// permission failures both show up -- `SettingsManager::set` already
/// distinguishes them (`SettingsError::Invalid` vs `PermissionDenied`),
/// this just displays whichever message it returns.
fn report_result(widget: &gtk::Widget, result: Result<(), SettingsError>) {
    match result {
        Ok(()) => {
            widget.remove_css_class("error");
            widget.set_tooltip_text(None);
        }
        Err(e) => {
            widget.add_css_class("error");
            let message = e.to_string();
            widget.set_tooltip_text(Some(message.as_str()));
        }
    }
}
