//! Turns a `Schema` into the value map a brand-new install (or a `reset
//! --all`) should start from.

use crate::settings::schema::Schema;
use crate::settings::value::Value;
use std::collections::HashMap;

pub fn default_values(schema: &Schema) -> HashMap<String, Value> {
    schema.all().map(|spec| (spec.key.to_string(), spec.default.clone())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::categories;

    #[test]
    fn defaults_cover_every_registered_setting() {
        let mut schema = Schema::new();
        categories::register_all(&mut schema);
        let defaults = default_values(&schema);
        assert_eq!(defaults.len(), schema.len());
    }
}
