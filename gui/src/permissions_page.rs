//! The "Application permissions" list: mitos-service's rulebook,
//! browsable and editable from Settings. Hooked into
//! `category_page::build` for the `privacy` category, the same way
//! that module already special-cases Users into actionable rows
//! instead of generic schema-driven ones -- a grant isn't a scalar
//! setting either.
//!
//! **Every apply, revoke, and hash computation goes through a
//! background thread** (`run_in_background`, below). A dangerous
//! capability's grant doesn't resolve until mitos-service hears back
//! from a real mitos-session elevation prompt, which can legitimately
//! take up to a couple of minutes (see
//! `mitos_settings::ipc::IpcClient::set_grant`'s doc comment). Calling
//! that directly from a button's click handler would freeze this
//! entire window for as long as someone takes to read the prompt and
//! type their password. `run_in_background` uses a spawned
//! `std::thread` plus a plain `mpsc` channel polled by a short-lived
//! `glib::timeout_add_local` (not `async-channel` + `spawn_future_local`,
//! only because this crate can't verify that API surface against the
//! exact gtk4-rs version pinned in `gui/Cargo.toml` without a compiler
//! on hand -- `timeout_add_local`/`ControlFlow` have been stable for
//! far longer and are the safer bet here).
//!
//! **What identifies an app here is its binary's SHA-256 hash, nothing
//! else** -- mitos-service has no concept of an app name (see
//! `mitos_settings::grants::model`'s doc comment on `Sha256Hex`).
//! There's no picker of "installed apps" anywhere in MITOS yet to
//! offer instead; adding a grant here means typing or pasting a hash,
//! or a file path to hash on the spot (`service_client::compute_sha256`,
//! which shells out to the system `sha256sum` -- see that function's
//! own doc comment for why).

use gtk::glib;
use gtk::prelude::*;
use mitos_settings::grants::{Decision, Grant, Risk, Scope};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// Runs `work` on a spawned thread, then calls `on_done` with its
/// result back on the GTK main loop once it finishes -- see this
/// module's doc comment for why, and why this particular mechanism.
fn run_in_background<T, F, D>(work: F, on_done: D)
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
    D: FnOnce(T) + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(work());
    });
    let mut on_done = Some(on_done);
    glib::source::timeout_add_local(Duration::from_millis(150), move || match rx.try_recv() {
        Ok(result) => {
            if let Some(f) = on_done.take() {
                f(result);
            }
            glib::ControlFlow::Break
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
        Err(std::sync::mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
    });
}

pub fn build() -> gtk::Widget {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let list_container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    list_container.set_margin_top(12);

    container.append(&build_add_grant_section(list_container.clone()));
    refresh_grant_list(&list_container);
    container.append(&list_container);

    container.upcast()
}

/// Clears and repopulates `list_container` from a fresh `list_grants()`
/// call -- the one thing every mutating action in this file (grant,
/// revoke) does once it succeeds, so the list never shows a state the
/// daemon didn't actually confirm.
fn refresh_grant_list(list_container: &gtk::Box) {
    while let Some(child) = list_container.first_child() {
        list_container.remove(&child);
    }

    match mitos_settings::ipc::IpcClient::list_grants() {
        Ok(grants) if grants.is_empty() => {
            let label = gtk::Label::new(Some("No grants recorded yet."));
            label.add_css_class("dim-label");
            label.set_margin_top(12);
            label.set_margin_bottom(12);
            list_container.append(&label);
        }
        Ok(grants) => {
            let list = gtk::ListBox::new();
            list.set_selection_mode(gtk::SelectionMode::None);
            for grant in grants {
                list.append(&build_grant_row(grant, list_container.clone()));
            }
            list_container.append(&list);
        }
        Err(e) => {
            let label = gtk::Label::new(Some(&format!(
                "Couldn't load permission grants: {e}\n(Is mitos-service running?)"
            )));
            label.add_css_class("dim-label");
            label.set_margin_top(12);
            label.set_margin_bottom(12);
            label.set_justify(gtk::Justification::Center);
            list_container.append(&label);
        }
    }
}

fn short_hash(sha256: &str) -> String {
    format!("{}…", sha256.get(..12).unwrap_or(sha256))
}

fn risk_badge(risk: Risk) -> Option<(&'static str, &'static str)> {
    // (text, css class) -- Low/Moderate get no badge: mitos-service
    // applies those without elevating, so nothing here needed extra
    // attention to grant them either.
    match risk {
        Risk::Low | Risk::Moderate => None,
        Risk::Dangerous => Some(("Requires password", "warning")),
        Risk::Critical => Some(("Requires password · Critical", "error")),
    }
}

fn build_grant_row(grant: Grant, list_container: gtk::Box) -> gtk::Widget {
    let container = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    container.set_margin_top(8);
    container.set_margin_bottom(8);
    container.set_margin_start(12);
    container.set_margin_end(12);

    let text_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text_box.set_hexpand(true);
    text_box.set_valign(gtk::Align::Center);

    let title = gtk::Label::new(Some(&format!(
        "{}  ·  {}",
        short_hash(&grant.sha256),
        grant.capability
    )));
    title.set_halign(gtk::Align::Start);
    title.add_css_class("monospace");
    text_box.append(&title);

    if let Some((text, css_class)) = risk_badge(grant.risk) {
        let badge = gtk::Label::new(Some(text));
        badge.set_halign(gtk::Align::Start);
        badge.add_css_class("caption");
        badge.add_css_class(css_class);
        text_box.append(&badge);
    }
    container.append(&text_box);

    let spinner = gtk::Spinner::new();
    spinner.set_visible(false);
    container.append(&spinner);

    let error_label = gtk::Label::new(None);
    error_label.add_css_class("error");
    error_label.add_css_class("caption");
    error_label.set_visible(false);
    error_label.set_max_width_chars(24);
    error_label.set_wrap(true);
    container.append(&error_label);

    let decision_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    decision_box.add_css_class("linked");
    let allow_btn = gtk::ToggleButton::builder().label("Allow").build();
    let deny_btn = gtk::ToggleButton::builder().label("Deny").build();
    deny_btn.set_group(Some(&allow_btn));
    match grant.decision {
        Decision::Allow => allow_btn.set_active(true),
        Decision::Deny => deny_btn.set_active(true),
    }
    decision_box.append(&allow_btn);
    decision_box.append(&deny_btn);
    container.append(&decision_box);

    let scope_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    scope_box.add_css_class("linked");
    let once_btn = gtk::ToggleButton::builder().label("Once").build();
    let session_btn = gtk::ToggleButton::builder().label("Session").build();
    let always_btn = gtk::ToggleButton::builder().label("Always").build();
    session_btn.set_group(Some(&once_btn));
    always_btn.set_group(Some(&once_btn));
    match grant.scope {
        Scope::Once => once_btn.set_active(true),
        Scope::Session => session_btn.set_active(true),
        Scope::Always => always_btn.set_active(true),
    }
    scope_box.append(&once_btn);
    scope_box.append(&session_btn);
    scope_box.append(&always_btn);
    container.append(&scope_box);

    let revoke_btn = gtk::Button::with_label("Revoke");
    revoke_btn.add_css_class("destructive-action");
    container.append(&revoke_btn);

    // Tracks the last decision/scope mitos-service actually confirmed,
    // so a declined/failed change has something correct to revert to,
    // and guards against the `set_active(true)` a revert performs
    // re-triggering these same handlers and starting a second request.
    let confirmed_decision = Rc::new(Cell::new(grant.decision));
    let confirmed_scope = Rc::new(Cell::new(grant.scope));
    let suppress_signal = Rc::new(Cell::new(false));
    let busy = Rc::new(Cell::new(false));
    let sha256 = Rc::new(grant.sha256.clone());
    let capability = Rc::new(grant.capability.clone());

    let set_busy = {
        let allow_btn = allow_btn.clone();
        let deny_btn = deny_btn.clone();
        let once_btn = once_btn.clone();
        let session_btn = session_btn.clone();
        let always_btn = always_btn.clone();
        let revoke_btn = revoke_btn.clone();
        let spinner = spinner.clone();
        move |is_busy: bool| {
            allow_btn.set_sensitive(!is_busy);
            deny_btn.set_sensitive(!is_busy);
            once_btn.set_sensitive(!is_busy);
            session_btn.set_sensitive(!is_busy);
            always_btn.set_sensitive(!is_busy);
            revoke_btn.set_sensitive(!is_busy);
            spinner.set_visible(is_busy);
            if is_busy {
                spinner.start();
            } else {
                spinner.stop();
            }
        }
    };

    // Decision toggles.
    for (btn, decision) in [(&allow_btn, Decision::Allow), (&deny_btn, Decision::Deny)] {
        let suppress_signal = Rc::clone(&suppress_signal);
        let busy = Rc::clone(&busy);
        let confirmed_decision = Rc::clone(&confirmed_decision);
        let confirmed_scope = Rc::clone(&confirmed_scope);
        let sha256 = Rc::clone(&sha256);
        let capability = Rc::clone(&capability);
        let error_label = error_label.clone();
        let allow_btn = allow_btn.clone();
        let deny_btn = deny_btn.clone();
        let set_busy = set_busy.clone();
        btn.connect_toggled(move |b| {
            if suppress_signal.get()
                || !b.is_active()
                || busy.get()
                || decision == confirmed_decision.get()
            {
                return;
            }
            error_label.set_visible(false);
            busy.set(true);
            set_busy(true);
            let scope = confirmed_scope.get();
            let sha = (*sha256).clone();
            let cap = (*capability).clone();
            let confirmed_decision2 = Rc::clone(&confirmed_decision);
            let suppress_signal2 = Rc::clone(&suppress_signal);
            let busy2 = Rc::clone(&busy);
            let set_busy2 = set_busy.clone();
            let allow_btn2 = allow_btn.clone();
            let deny_btn2 = deny_btn.clone();
            let error_label2 = error_label.clone();
            run_in_background(
                move || mitos_settings::ipc::IpcClient::set_grant(&sha, &cap, decision, scope),
                move |result| {
                    busy2.set(false);
                    set_busy2(false);
                    match result {
                        Ok(()) => confirmed_decision2.set(decision),
                        Err(e) => {
                            error_label2.set_label(&e);
                            error_label2.set_visible(true);
                            suppress_signal2.set(true);
                            match confirmed_decision2.get() {
                                Decision::Allow => allow_btn2.set_active(true),
                                Decision::Deny => deny_btn2.set_active(true),
                            }
                            suppress_signal2.set(false);
                        }
                    }
                },
            );
        });
    }

    // Scope toggles -- structurally identical to the decision ones
    // above, just against `Scope` instead of `Decision`.
    for (btn, scope) in [
        (&once_btn, Scope::Once),
        (&session_btn, Scope::Session),
        (&always_btn, Scope::Always),
    ] {
        let suppress_signal = Rc::clone(&suppress_signal);
        let busy = Rc::clone(&busy);
        let confirmed_decision = Rc::clone(&confirmed_decision);
        let confirmed_scope = Rc::clone(&confirmed_scope);
        let sha256 = Rc::clone(&sha256);
        let capability = Rc::clone(&capability);
        let error_label = error_label.clone();
        let once_btn = once_btn.clone();
        let session_btn = session_btn.clone();
        let always_btn = always_btn.clone();
        let set_busy = set_busy.clone();
        btn.connect_toggled(move |b| {
            if suppress_signal.get()
                || !b.is_active()
                || busy.get()
                || scope == confirmed_scope.get()
            {
                return;
            }
            error_label.set_visible(false);
            busy.set(true);
            set_busy(true);
            let decision = confirmed_decision.get();
            let sha = (*sha256).clone();
            let cap = (*capability).clone();
            let confirmed_scope2 = Rc::clone(&confirmed_scope);
            let suppress_signal2 = Rc::clone(&suppress_signal);
            let busy2 = Rc::clone(&busy);
            let set_busy2 = set_busy.clone();
            let once_btn2 = once_btn.clone();
            let session_btn2 = session_btn.clone();
            let always_btn2 = always_btn.clone();
            let error_label2 = error_label.clone();
            run_in_background(
                move || mitos_settings::ipc::IpcClient::set_grant(&sha, &cap, decision, scope),
                move |result| {
                    busy2.set(false);
                    set_busy2(false);
                    match result {
                        Ok(()) => confirmed_scope2.set(scope),
                        Err(e) => {
                            error_label2.set_label(&e);
                            error_label2.set_visible(true);
                            suppress_signal2.set(true);
                            match confirmed_scope2.get() {
                                Scope::Once => once_btn2.set_active(true),
                                Scope::Session => session_btn2.set_active(true),
                                Scope::Always => always_btn2.set_active(true),
                            }
                            suppress_signal2.set(false);
                        }
                    }
                },
            );
        });
    }

    // Revoke -- on success, the whole row is gone (there's no "revoked"
    // decision/scope to revert to), so this refreshes the entire list
    // rather than trying to reset this one row's buttons to anything.
    {
        let busy = Rc::clone(&busy);
        let sha256 = Rc::clone(&sha256);
        let capability = Rc::clone(&capability);
        let error_label = error_label.clone();
        let set_busy = set_busy.clone();
        revoke_btn.connect_clicked(move |_| {
            if busy.get() {
                return;
            }
            error_label.set_visible(false);
            busy.set(true);
            set_busy(true);
            let sha = (*sha256).clone();
            let cap = (*capability).clone();
            let list_container = list_container.clone();
            let busy2 = Rc::clone(&busy);
            let error_label2 = error_label.clone();
            run_in_background(
                move || mitos_settings::ipc::IpcClient::revoke_grant(&sha, &cap),
                move |result| {
                    busy2.set(false);
                    match result {
                        Ok(()) => refresh_grant_list(&list_container),
                        Err(e) => {
                            error_label2.set_label(&e);
                            error_label2.set_visible(true);
                        }
                    }
                },
            );
        });
    }

    container.upcast()
}

fn build_add_grant_section(list_container: gtk::Box) -> gtk::Widget {
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 6);
    outer.set_margin_top(12);
    outer.set_margin_bottom(12);
    outer.set_margin_start(12);
    outer.set_margin_end(12);

    let heading = gtk::Label::new(Some("Add a permission grant"));
    heading.add_css_class("heading");
    heading.set_halign(gtk::Align::Start);
    outer.append(&heading);

    let hint = gtk::Label::new(Some(
        "mitos-service identifies an app by its binary's SHA-256 hash, not a name. \
         Paste a hash below, or a file path to hash it on the spot.",
    ));
    hint.add_css_class("dim-label");
    hint.add_css_class("caption");
    hint.set_halign(gtk::Align::Start);
    hint.set_wrap(true);
    outer.append(&hint);

    let hash_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let hash_entry = gtk::Entry::new();
    hash_entry.set_placeholder_text(Some("SHA-256 hash, or a file path to hash"));
    hash_entry.set_hexpand(true);
    let hash_btn = gtk::Button::with_label("Use path");
    hash_row.append(&hash_entry);
    hash_row.append(&hash_btn);
    outer.append(&hash_row);

    let capability_entry = gtk::Entry::new();
    capability_entry.set_placeholder_text(Some("capability, e.g. camera, raw_disk, root_shell"));
    outer.append(&capability_entry);

    let decision_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    decision_box.add_css_class("linked");
    let allow_btn = gtk::ToggleButton::builder().label("Allow").build();
    let deny_btn = gtk::ToggleButton::builder().label("Deny").build();
    deny_btn.set_group(Some(&allow_btn));
    allow_btn.set_active(true);
    decision_box.append(&allow_btn);
    decision_box.append(&deny_btn);

    let scope_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    scope_box.add_css_class("linked");
    let once_btn = gtk::ToggleButton::builder().label("Once").build();
    let session_btn = gtk::ToggleButton::builder().label("Session").build();
    let always_btn = gtk::ToggleButton::builder().label("Always").build();
    session_btn.set_group(Some(&once_btn));
    always_btn.set_group(Some(&once_btn));
    always_btn.set_active(true);
    scope_box.append(&once_btn);
    scope_box.append(&session_btn);
    scope_box.append(&always_btn);

    let toggles_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    toggles_row.append(&decision_box);
    toggles_row.append(&scope_box);
    outer.append(&toggles_row);

    let status_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let spinner = gtk::Spinner::new();
    spinner.set_visible(false);
    let status_label = gtk::Label::new(None);
    status_label.add_css_class("caption");
    status_label.set_visible(false);
    status_label.set_wrap(true);
    status_label.set_hexpand(true);
    status_label.set_halign(gtk::Align::Start);
    let submit_btn = gtk::Button::with_label("Grant");
    submit_btn.add_css_class("suggested-action");
    status_row.append(&spinner);
    status_row.append(&status_label);
    status_row.append(&submit_btn);
    outer.append(&status_row);

    let busy = Rc::new(Cell::new(false));

    // "Use path": if the entry doesn't already look like a 64-hex-char
    // hash, treat it as a path and replace it with that file's hash.
    {
        let hash_entry = hash_entry.clone();
        let status_label = status_label.clone();
        let spinner = spinner.clone();
        let busy = Rc::clone(&busy);
        hash_btn.connect_clicked(move |_| {
            if busy.get() {
                return;
            }
            let text = hash_entry.text().to_string();
            if text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit()) {
                status_label.set_label("That already looks like a hash.");
                status_label.set_visible(true);
                return;
            }
            busy.set(true);
            spinner.set_visible(true);
            spinner.start();
            status_label.set_visible(false);
            let path = std::path::PathBuf::from(text);
            let hash_entry2 = hash_entry.clone();
            let status_label2 = status_label.clone();
            let spinner2 = spinner.clone();
            let busy2 = Rc::clone(&busy);
            run_in_background(
                move || mitos_settings::grants::service_client::compute_sha256(&path),
                move |result| {
                    busy2.set(false);
                    spinner2.stop();
                    spinner2.set_visible(false);
                    match result {
                        Ok(hash) => hash_entry2.set_text(&hash),
                        Err(e) => {
                            status_label2.set_label(&e);
                            status_label2.set_visible(true);
                        }
                    }
                },
            );
        });
    }

    {
        let hash_entry = hash_entry.clone();
        let capability_entry = capability_entry.clone();
        let status_label = status_label.clone();
        let spinner = spinner.clone();
        let submit_btn2 = submit_btn.clone();
        let busy = Rc::clone(&busy);
        let list_container = list_container.clone();
        submit_btn.connect_clicked(move |_| {
            if busy.get() {
                return;
            }
            let sha256 = hash_entry.text().to_string();
            let capability = capability_entry.text().to_string();
            if sha256.len() != 64 || !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
                status_label.set_label("Not a valid SHA-256 hash (64 hex characters).");
                status_label.set_visible(true);
                return;
            }
            if capability.is_empty() {
                status_label.set_label("Enter a capability name.");
                status_label.set_visible(true);
                return;
            }
            let decision = if allow_btn.is_active() {
                Decision::Allow
            } else {
                Decision::Deny
            };
            let scope = if once_btn.is_active() {
                Scope::Once
            } else if session_btn.is_active() {
                Scope::Session
            } else {
                Scope::Always
            };

            busy.set(true);
            submit_btn2.set_sensitive(false);
            spinner.set_visible(true);
            spinner.start();
            status_label.set_visible(false);

            let hash_entry2 = hash_entry.clone();
            let capability_entry2 = capability_entry.clone();
            let status_label2 = status_label.clone();
            let spinner2 = spinner.clone();
            let submit_btn3 = submit_btn2.clone();
            let busy2 = Rc::clone(&busy);
            let list_container = list_container.clone();
            run_in_background(
                move || {
                    mitos_settings::ipc::IpcClient::set_grant(&sha256, &capability, decision, scope)
                },
                move |result| {
                    busy2.set(false);
                    submit_btn3.set_sensitive(true);
                    spinner2.stop();
                    spinner2.set_visible(false);
                    match result {
                        Ok(()) => {
                            hash_entry2.set_text("");
                            capability_entry2.set_text("");
                            status_label2.set_label("Granted.");
                            status_label2.set_visible(true);
                            refresh_grant_list(&list_container);
                        }
                        Err(e) => {
                            status_label2.set_label(&e);
                            status_label2.set_visible(true);
                        }
                    }
                },
            );
        });
    }

    outer.upcast()
}
