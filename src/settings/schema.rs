//! The schema is the single source of truth for what settings exist: their
//! type, default, privilege requirement, and validation constraints. It's
//! built once at startup by `categories::register_all` and then treated as
//! read-only for the rest of the process's life.

use crate::permissions::PrivilegeLevel;
use crate::settings::value::{Value, ValueKind};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueFormat {
    /// A CSS-style hex color: `#RGB`, `#RRGGBB`, or `#RRGGBBAA`.
    HexColor,
}

#[derive(Debug, Clone)]
pub struct SettingSpec {
    pub key: &'static str,
    pub category: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub kind: ValueKind,
    pub default: Value,
    pub privilege: PrivilegeLevel,
    pub choices: Option<&'static [&'static str]>,
    pub range: Option<(f64, f64)>,
    pub format: Option<ValueFormat>,
    /// Read-only settings are informational — populated live from
    /// `hardware`/`services` rather than stored — and `SettingsManager::set`
    /// rejects writes to them.
    pub read_only: bool,
}

impl SettingSpec {
    pub fn new(
        key: &'static str,
        category: &'static str,
        label: &'static str,
        description: &'static str,
        kind: ValueKind,
        default: Value,
        privilege: PrivilegeLevel,
    ) -> Self {
        SettingSpec {
            key,
            category,
            label,
            description,
            kind,
            default,
            privilege,
            choices: None,
            range: None,
            format: None,
            read_only: false,
        }
    }

    pub fn choices(mut self, choices: &'static [&'static str]) -> Self {
        self.choices = Some(choices);
        self
    }

    pub fn range(mut self, lo: f64, hi: f64) -> Self {
        self.range = Some((lo, hi));
        self
    }

    pub fn format(mut self, format: ValueFormat) -> Self {
        self.format = Some(format);
        self
    }

    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }
}

#[derive(Debug, Clone)]
pub struct CategoryMeta {
    pub id: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub subitems: &'static [&'static str],
}

#[derive(Debug, Default)]
pub struct Schema {
    specs: BTreeMap<&'static str, SettingSpec>,
    categories: Vec<CategoryMeta>,
}

impl Schema {
    pub fn new() -> Self {
        Schema { specs: BTreeMap::new(), categories: Vec::new() }
    }

    pub fn register(&mut self, spec: SettingSpec) {
        self.specs.insert(spec.key, spec);
    }

    pub fn register_category(&mut self, meta: CategoryMeta) {
        self.categories.push(meta);
    }

    pub fn get(&self, key: &str) -> Option<&SettingSpec> {
        self.specs.get(key)
    }

    pub fn all(&self) -> impl Iterator<Item = &SettingSpec> {
        self.specs.values()
    }

    pub fn by_category<'a>(&'a self, category: &'a str) -> impl Iterator<Item = &'a SettingSpec> {
        self.specs.values().filter(move |s| s.category == category)
    }

    pub fn categories(&self) -> &[CategoryMeta] {
        &self.categories
    }

    pub fn category(&self, id: &str) -> Option<&CategoryMeta> {
        self.categories.iter().find(|c| c.id == id)
    }

    pub fn len(&self) -> usize {
        self.specs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_lookup() {
        let mut schema = Schema::new();
        schema.register(SettingSpec::new(
            "display.brightness",
            "display",
            "Brightness",
            "Screen brightness",
            ValueKind::Int,
            Value::Int(80),
            PrivilegeLevel::User,
        ).range(0.0, 100.0));

        let spec = schema.get("display.brightness").unwrap();
        assert_eq!(spec.category, "display");
        assert_eq!(spec.range, Some((0.0, 100.0)));
        assert!(schema.get("nonexistent").is_none());
    }
}
