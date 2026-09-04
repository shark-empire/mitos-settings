//! Everything to do with the settings data model itself: the type a value
//! can hold (`value`), what settings exist (`schema`), what they start at
//! (`defaults`), whether a candidate value is acceptable (`validation`),
//! how it's saved (`persistence`), and the orchestrator that ties all of
//! that together (`manager`).

pub mod defaults;
pub mod json;
pub mod manager;
pub mod persistence;
pub mod schema;
pub mod validation;
pub mod value;
