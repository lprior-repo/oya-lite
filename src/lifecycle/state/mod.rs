#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod persist;
pub mod state_db;

pub use persist::{load_state, persist_state};
pub use state_db::StateDb;
