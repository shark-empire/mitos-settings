use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};
use std::process::Command;

pub struct PrintersCategory;

impl Category for PrintersCategory {
    fn id(&self) -> &'static str {
        "printers"
    }
    fn name(&self) -> &'static str {
        "Printers"
    }
    fn icon(&self) -> &'static str {
        "printer"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Printers", "Default printer", "Printing options"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(SettingSpec::new(
            "printers.default_printer",
            "printers",
            "Default printer",
            "CUPS queue name used when no printer is explicitly chosen",
            ValueKind::Str,
            Value::Str(String::new()),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "printers.default_paper_size",
                "printers",
                "Default paper size",
                "Paper size used unless a document specifies otherwise",
                ValueKind::Str,
                Value::Str("A4".into()),
                PrivilegeLevel::User,
            )
            .choices(&["A4", "Letter", "Legal", "A3"]),
        );

        schema.register(
            SettingSpec::new(
                "printers.print_quality",
                "printers",
                "Print quality",
                "Default print quality/speed tradeoff",
                ValueKind::Str,
                Value::Str("normal".into()),
                PrivilegeLevel::User,
            )
            .choices(&["draft", "normal", "high"]),
        );
    }

    /// This is the one category that talks to `lpstat` directly rather than
    /// through a `services::` module, because the given project layout
    /// doesn't include a dedicated printers service or hardware file.
    fn live_info(&self) -> Vec<(&'static str, String)> {
        let Ok(output) = Command::new("lpstat").arg("-p").output() else {
            return vec![("printers", "CUPS not available (lpstat not found)".to_string())];
        };
        let text = String::from_utf8_lossy(&output.stdout);
        let printers: Vec<(&'static str, String)> =
            text.lines().filter(|l| l.starts_with("printer ")).map(|l| ("printer", l.to_string())).collect();
        if printers.is_empty() {
            vec![("printers", "no printers configured".to_string())]
        } else {
            printers
        }
    }
}
