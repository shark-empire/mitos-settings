//! Low-level, format-agnostic config file I/O: where files live (`paths`),
//! how to read/write the plain-text format (`loader`/`writer`), and how to
//! carry old files forward (`migration`). `settings::persistence` builds on
//! top of this to add `Value` typing.

pub mod loader;
pub mod migration;
pub mod paths;
pub mod writer;
