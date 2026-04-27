#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod executor;
pub mod run;

pub use executor::TokioCommandExecutor;
pub use run::run_effect;
