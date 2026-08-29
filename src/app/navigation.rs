//! A small breadcrumb stack over the category tree, so the interactive app
//! can print something like `Settings > Display` and support `back`.

use crate::categories::{self, Category};

pub struct Navigation {
    /// Category ids, outermost first. In this single-level Settings tree
    /// it's at most one entry deep, but the stack shape leaves room for
    /// drilling into a subitem later without reworking callers.
    breadcrumbs: Vec<String>,
}

impl Navigation {
    pub fn new() -> Self {
        Navigation { breadcrumbs: Vec::new() }
    }

    pub fn push(&mut self, category_id: &str) {
        self.breadcrumbs.push(category_id.to_string());
    }

    pub fn pop(&mut self) -> Option<String> {
        self.breadcrumbs.pop()
    }

    pub fn is_at_root(&self) -> bool {
        self.breadcrumbs.is_empty()
    }

    pub fn current_category(&self) -> Option<Box<dyn Category>> {
        self.breadcrumbs.last().and_then(|id| categories::find(id))
    }

    /// Renders the breadcrumb trail, e.g. "Settings > Display".
    pub fn path_string(&self) -> String {
        let mut path = String::from("Settings");
        for id in &self.breadcrumbs {
            if let Some(cat) = categories::find(id) {
                path.push_str(" > ");
                path.push_str(cat.name());
            }
        }
        path
    }
}

impl Default for Navigation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_pop_track_breadcrumbs() {
        let mut nav = Navigation::new();
        assert!(nav.is_at_root());
        nav.push("display");
        assert!(!nav.is_at_root());
        assert_eq!(nav.path_string(), "Settings > Display");
        assert_eq!(nav.pop(), Some("display".to_string()));
        assert!(nav.is_at_root());
    }
}
