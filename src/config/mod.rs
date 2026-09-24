//! Low-level, format-agnostic config file I/O: where files live (`paths`),
//! how to read/write the plain-text format (`loader`/`writer`), how to
//! carry old files forward (`migration`), and a cross-process advisory
//! lock (`file_lock`). `settings::persistence` builds on top of this to
//! add `Value` typing.

pub mod file_lock;
pub mod loader;
pub mod migration;
pub mod paths;
pub mod writer;
