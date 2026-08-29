# Translations

This project's user-facing strings (category names, setting labels,
descriptions) are currently hardcoded English `&'static str`s in
`src/categories/*.rs` — there's no i18n layer wired up yet. This directory
is where locale files would live once there is one.

The suggested format follows the same dependency-free philosophy as the
rest of the project: flat `key=value` files, one per locale, keyed by
`<category>.<field>.<setting_key>`. `en.lang` is a starting example — see
a handful of real entries below. A real i18n pass would need to generate
this file's keys from `Schema` (`spec.key`, `spec.label`,
`spec.description`) rather than hand-maintaining it, and swap
`SettingSpec::label`/`description` from `&'static str` to a lookup against
the active locale.
