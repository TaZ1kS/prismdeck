use std::sync::RwLock;

use crate::prism::types::PrismSnapshot;

/// In-memory state for the current session. Persistent settings arrive in M6.
#[derive(Default)]
pub struct AppState {
    pub snapshot: RwLock<PrismSnapshot>,
}
