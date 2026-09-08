//! Change Password dialog for the Users settings page.

use gtk::prelude::*;
use gtk::{Button, Dialog, Entry, Label, Orientation, ResponseType};
use std::rc::Rc;

pub fn show(parent: &gtk::Window, username: &str) {
    let dialog = Dialog::builder()
        .title("Change Password")
        .transient_for(parent)
        .modal(true)
        .build();

    let content = dialog.content_area();
    content.set_spacing(12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.set_orientation(Orientation::Vertical);

    // Header
    let label = Label::builder()
        .label(&format!("Changing password for: {}", username))
        .build();
    label.add_css_class("heading");
    content.append(&label);

    // New Password field
    let new_password = Entry::builder()
        .placeholder_text("New Password")
        .visibility(false) // Hide the password
        .build();
    content.append(&new_password);

    // Confirm Password field
    let confirm_password = Entry::builder()
        .placeholder_text("Confirm Password")
        .visibility(false)
        .build();
    content.append(&confirm_password);

    // Error label (hidden by default)
    let error_label = Label::builder().label("").visible(false).build();
    error_label.add_css_class("error");
    content.append(&error_label);

    // Buttons
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Change Password", ResponseType::Accept);

    let username_owned = username.to_string();
    let new_password_clone = new_password.clone();
    let confirm_password_clone = confirm_password.clone();
    let error_label_clone = error_label.clone();

    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Accept {
            let new_pass = new_password_clone.text().to_string();
            let confirm_pass = confirm_password_clone.text().to_string();

            // Validate passwords match
            if new_pass != confirm_pass {
                error_label_clone.set_label("Passwords do not match");
                error_label_clone.set_visible(true);
                return;
            }

            // Validate password strength
            if new_pass.len() < 8 {
                error_label_clone.set_label("Password must be at least 8 characters");
                error_label_clone.set_visible(true);
                return;
            }

            // Send IPC request to the daemon
            // This is where you'd call your IPC client
            match mitos_settings::ipc::client::change_password(&username_owned, &new_pass) {
                Ok(()) => {
                    dialog.close();
                    // Show success toast
                    show_success_toast("Password changed successfully");
                }
                Err(e) => {
                    error_label_clone.set_label(&e);
                    error_label_clone.set_visible(true);
                }
            }
        } else {
            dialog.close();
        }
    });

    dialog.show();
}

fn show_success_toast(message: &str) {
    // Use D-Bus notifications or your preferred toast system
    // This is a placeholder - implement based on your notification system
    println!("{}", message);
}
