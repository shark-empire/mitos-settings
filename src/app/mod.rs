//! The interactive front-end: `application` runs the text navigator loop,
//! `navigation` tracks where in the category tree you are, and `state`
//! holds the rest of the transient UI state.

pub mod application;
pub mod navigation;
pub mod state;

pub use application::Application;
