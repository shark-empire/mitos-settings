use crate::settings::json;
use crate::settings::manager::SettingsManager;

/// Dumps the full schema (every setting's key, type, default, privilege,
/// and constraints) as JSON — the "what can be configured" counterpart to
/// `list --json`'s "what is configured right now". See INTEGRATION.md.
pub fn execute(manager: &SettingsManager) -> Result<String, String> {
    Ok(json::schema_to_json(manager.schema()))
}
