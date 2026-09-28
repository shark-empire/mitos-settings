//! The interactive front-end. There's no GUI toolkit in this
//! dependency-free project, so this is a small text navigator over stdin —
//! but it exercises the exact same `SettingsManager` API the CLI and the
//! daemon use, which is the point: every front-end in this project is a
//! thin shell over the same core.

use crate::app::navigation::Navigation;
use crate::app::state::AppState;
use crate::categories::{self, Category};
use crate::permissions::PrivilegeLevel;
use crate::settings::manager::{Mode, SettingsManager};
use crate::settings::value::Value;
use std::io::{self, Write};

pub struct Application {
    manager: SettingsManager,
    nav: Navigation,
    state: AppState,
}

impl Application {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let manager = SettingsManager::load(Mode::Standalone)?;
        Ok(Application {
            manager,
            nav: Navigation::new(),
            state: AppState::new(),
        })
    }

    pub fn run(&mut self) {
        println!("MITOS Settings — interactive mode. Type 'help' for commands, 'quit' to exit.");
        println!("(Prefer a graphical app? Run mitos-settings-gui instead.)");
        while !self.state.quit {
            self.print_screen();
            let Some(line) = read_line("\n> ") else { break };
            self.handle_command(line.trim());
        }
    }

    fn print_screen(&self) {
        println!("\n{}", self.nav.path_string());
        if !self.state.search.is_empty() {
            self.print_search_results();
            return;
        }
        match self.nav.current_category() {
            None => {
                for (i, cat) in categories::all().iter().enumerate() {
                    let subitems = cat.subitems();
                    let hint = if subitems.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", subitems.join(", "))
                    };
                    println!("  {:>2}. {}{}", i + 1, cat.name(), hint);
                }
                println!("\nType a number to open a category, 'search <query>', or 'quit'.");
            }
            Some(cat) => {
                for spec in self.manager.schema().by_category(cat.id()) {
                    let value = self
                        .manager
                        .get(spec.key)
                        .map(|v| v.to_string())
                        .unwrap_or_default();
                    println!("  {:<32} {}{}", spec.label, value, tags_for(spec));
                }
                for (label, value) in cat.live_info() {
                    println!("  {label:<32} {value} (live)");
                }
                println!("\nCommands: set <key> <value>  |  reset <key>  |  back  |  quit");
            }
        }
    }

    /// Every setting whose label, description, or category name
    /// contains the current query (case-insensitive) -- the same match
    /// rule `gui/src/window.rs`'s search uses, so the two front-ends
    /// agree on what a query matches. Results show their category
    /// (there's no other indication which one a hit came from, unlike
    /// browsing) and are `set`/`reset`-able directly, without first
    /// navigating into that category.
    fn print_search_results(&self) {
        let query = self.state.search.to_lowercase();
        let hits: Vec<_> = self
            .manager
            .schema()
            .all()
            .filter(|s| {
                s.label.to_lowercase().contains(query.as_str())
                    || s.description.to_lowercase().contains(query.as_str())
                    || s.category.to_lowercase().contains(query.as_str())
            })
            .collect();

        let count = hits.len();
        let noun = if count == 1 { "match" } else { "matches" };
        println!("Search: \"{}\" ({count} {noun})", self.state.search);
        if hits.is_empty() {
            println!("  No settings match. 'back' to clear the search.");
        }
        for spec in &hits {
            let value = self
                .manager
                .get(spec.key)
                .map(|v| v.to_string())
                .unwrap_or_default();
            println!(
                "  {:<32} {} ({}){}",
                spec.label,
                value,
                spec.category,
                tags_for(spec)
            );
        }
        println!("\nCommands: set <key> <value>  |  reset <key>  |  back  |  quit");
    }

    fn handle_command(&mut self, input: &str) {
        if input.is_empty() {
            return;
        }
        if input.eq_ignore_ascii_case("quit") || input.eq_ignore_ascii_case("exit") {
            self.state.quit = true;
            return;
        }
        if input.eq_ignore_ascii_case("back") {
            if !self.state.search.is_empty() {
                self.state.search.clear();
            } else {
                self.nav.pop();
                self.state.close_category();
            }
            return;
        }
        if input.eq_ignore_ascii_case("help") {
            print_help();
            return;
        }

        if self.nav.is_at_root() && self.state.search.is_empty() {
            self.handle_root_command(input);
            return;
        }

        // Reached with either a category open or search results showing
        // -- both display settings by their (globally unique) key, so
        // set/reset work identically in either state.
        let mut parts = input.splitn(3, ' ');
        match parts.next() {
            Some("set") => match (parts.next(), parts.next()) {
                (Some(key), Some(raw_value)) => self.apply_set(key, raw_value),
                _ => println!("usage: set <key> <value>"),
            },
            Some("reset") => match parts.next() {
                Some(key) => match self.manager.reset(key) {
                    Ok(()) => println!("Reset {key} to its default."),
                    Err(e) => println!("Could not reset {key}: {e}"),
                },
                None => println!("usage: reset <key>"),
            },
            _ => println!("Unrecognized command '{input}'. Type 'help'."),
        }
    }

    fn handle_root_command(&mut self, input: &str) {
        if let Some(query) = input.strip_prefix("search ") {
            let query = query.trim();
            if query.is_empty() {
                println!("usage: search <query>");
            } else {
                self.state.search = query.to_string();
            }
            return;
        }
        match input.parse::<usize>() {
            Ok(index) if index >= 1 && index <= categories::all().len() => {
                let id = categories::all()[index - 1].id();
                self.nav.push(id);
                self.state.open_category(index - 1);
            }
            Ok(_) => println!("No such category number."),
            Err(_) => println!("Unrecognized command '{input}'. Type 'help'."),
        }
    }

    fn apply_set(&mut self, key: &str, raw_value: &str) {
        let Some(spec) = self.manager.schema().get(key) else {
            println!("Unknown setting '{key}'.");
            return;
        };
        let kind = spec.kind;
        match Value::parse(kind, raw_value) {
            Ok(value) => match self.manager.set(key, value) {
                Ok(()) => println!("Updated {key}."),
                Err(e) => println!("Could not update {key}: {e}"),
            },
            Err(e) => println!("Invalid value for {key}: {e}"),
        }
    }
}

/// The `[read-only]`/`[admin]`/`[restart]` suffix for one setting row,
/// shared by the category view and search results so the two forms of
/// browsing tag things identically. `read-only` and `admin` stay
/// mutually exclusive (a setting is shown as one or the other, matching
/// this project's existing convention -- a read-only setting's
/// privilege level isn't really the interesting fact about it);
/// `restart` is independent and can stack with either.
fn tags_for(spec: &crate::settings::schema::SettingSpec) -> String {
    let mut tags = String::new();
    if spec.read_only {
        tags.push_str(" [read-only]");
    } else if spec.privilege > PrivilegeLevel::User {
        tags.push_str(" [admin]");
    }
    if spec.requires_restart {
        tags.push_str(" [restart]");
    }
    tags
}

fn print_help() {
    println!("Commands:");
    println!("  <number>          open a category (from the top-level list)");
    println!("  search <query>    find settings by name, description, or category");
    println!("  set <key> <val>   change a setting (within a category or search results)");
    println!("  reset <key>       restore a setting to its default");
    println!("  back              return to the category list (or clear a search)");
    println!("  quit              exit");
}

fn read_line(prompt: &str) -> Option<String> {
    print!("{prompt}");
    io::stdout().flush().ok()?;
    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        Ok(0) => None, // EOF (e.g. piped input ran out)
        Ok(_) => Some(line),
        Err(_) => None,
    }
}
