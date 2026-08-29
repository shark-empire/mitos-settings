//! Carries older on-disk config formats forward to `CURRENT_VERSION`. Each
//! step is small and self-contained; add a new `migrate_vN_to_vN+1`
//! function and call it from `migrate` when you next change the key layout.

use super::loader::{RawDocument, CURRENT_VERSION};

pub fn migrate(mut doc: RawDocument) -> RawDocument {
    if doc.version < 2 {
        migrate_v1_to_v2(&mut doc);
    }
    doc.version = CURRENT_VERSION;
    doc
}

/// v1 stored Wi-Fi under the bare key `wifi.enabled`; v2 namespaces every
/// network setting under `network.*` to match the category layout used by
/// `categories::network`.
fn migrate_v1_to_v2(doc: &mut RawDocument) {
    if let Some(v) = doc.entries.remove("wifi.enabled") {
        doc.entries.insert("network.wifi_enabled".to_string(), v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn renames_legacy_wifi_key() {
        let mut entries = BTreeMap::new();
        entries.insert("wifi.enabled".to_string(), "bool:true".to_string());
        let doc = RawDocument { version: 1, entries };

        let migrated = migrate(doc);

        assert_eq!(migrated.version, CURRENT_VERSION);
        assert!(!migrated.entries.contains_key("wifi.enabled"));
        assert_eq!(migrated.entries.get("network.wifi_enabled").unwrap(), "bool:true");
    }

    #[test]
    fn current_version_doc_is_left_untouched() {
        let mut entries = BTreeMap::new();
        entries.insert("display.brightness".to_string(), "int:80".to_string());
        let doc = RawDocument { version: CURRENT_VERSION, entries: entries.clone() };
        let migrated = migrate(doc);
        assert_eq!(migrated.entries, entries);
    }
}
