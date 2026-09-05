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
        Ok(Application { manager, nav: Navigation::new(), state: AppState::new() })
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
        match self.nav.current_category() {
            None => {
                for (i, cat) in categories::all().iter().enumerate() {
                    println!("  {:>2}. {}", i + 1, cat.name());
                }
                println!("\nType a number to open a category, or 'quit'.");
            }
            Some(cat) => {
                for spec in self.manager.schema().by_category(cat.id()) {
                    let value = self.manager.get(spec.key).map(|v| v.to_string()).unwrap_or_default();
                    let tag = if spec.read_only {
                        " [read-only]"
                    } else if spec.privilege > PrivilegeLevel::User {
                        " [admin]"
                    } else {
                        ""
                    };
                    println!("  {:<32} {}{}", spec.label, value, tag);
                }
                for (label, value) in cat.live_info() {
                    println!("  {label:<32} {value} (live)");
                }
                println!("\nCommands: set <key> <value>  |  reset <key>  |  back  |  quit");
            }
        }
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
            self.nav.pop();
            self.state.close_category();
            return;
        }
        if input.eq_ignore_ascii_case("help") {
            print_help();
            return;
        }

        if self.nav.is_at_root() {
            self.handle_root_command(input);
            return;
        }

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

fn print_help() {
    println!("Commands:");
    println!("  <number>          open a category (from the top-level list)");
    println!("  set <key> <val>   change a setting within the open category");
    println!("  reset <key>       restore a setting to its default");
    println!("  back              return to the category list");
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
