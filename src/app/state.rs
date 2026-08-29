//! Transient UI state for the interactive text navigator. Nothing here is
//! persisted — it resets every time `mitos-settings` (with no subcommand)
//! is launched.

#[derive(Debug, Default)]
pub struct AppState {
    /// Index into `categories::all()` for the currently open category, if
    /// any (`None` means the top-level category list is showing).
    pub current_category: Option<usize>,
    pub search: String,
    pub quit: bool,
}

impl AppState {
    pub fn new() -> Self {
        AppState::default()
    }

    pub fn open_category(&mut self, index: usize) {
        self.current_category = Some(index);
    }

    pub fn close_category(&mut self) {
        self.current_category = None;
    }
}
