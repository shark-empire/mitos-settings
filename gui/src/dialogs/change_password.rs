//! Change Password dialog for the Users settings page.

use gtk::prelude::*;
use gtk::{Dialog, Entry, Label, ResponseType};

pub fn show(parent: &gtk::Window, username: &str) {
    let dialog = Dialog::builder()
        .title("Set Password")
        .transient_for(parent)
        .modal(true)
        .build();

    let content = dialog.content_area();
    content.set_spacing(12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);

    let header = Label::new(Some(&format!("Set a password for “{username}”")));
    header.add_css_class("title-4");
    content.append(&header);

    let hint = Label::new(Some(
        "A password is required before the lock screen can be used.",
    ));
    hint.add_css_class("dim-label");
    content.append(&hint);

    let new_password = Entry::builder()
        .placeholder_text("New password")
        .visibility(false)
        .build();
    content.append(&new_password);

    let confirm_password = Entry::builder()
        .placeholder_text("Confirm password")
        .visibility(false)
        .build();
    content.append(&confirm_password);

    let error_label = Label::new(None);
    error_label.add_css_class("error");
    error_label.set_visible(false);
    content.append(&error_label);

    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Save", ResponseType::Accept);
    dialog.set_default_response(Some(ResponseType::Accept));

    let username = username.to_string();
    dialog.connect_response(move |dialog, response| {
        if response != ResponseType::Accept {
            dialog.close();
            return;
        }

        let new_pass = new_password.text().to_string();
        let confirm_pass = confirm_password.text().to_string();

        let problem = if new_pass != confirm_pass {
            Some("Passwords do not match.")
        } else if new_pass.len() < 8 {
            Some("Password must be at least 8 characters.")
        } else {
            None
        };

        if let Some(problem) = problem {
            error_label.set_label(problem);
            error_label.set_visible(true);
            return; // keep the dialog open so they can fix it
        }

        match mitos_settings::ipc::client::change_password(&username, &new_pass) {
            Ok(()) => dialog.close(),
            Err(e) => {
                error_label.set_label(&e);
                error_label.set_visible(true);
            }
        }
    });

    dialog.present();
}

fn show_success_toast(message: &str) {
    // Use D-Bus notifications or your preferred toast system
    // This is a placeholder - implement based on your notification system
    println!("{}", message);
}
