//! Heatmap core: all persistence, date and statistics logic, independent of any UI.
pub mod dates;
pub mod db;
pub mod capture;
pub mod error;
pub mod git;
pub mod insights;
pub mod io;
pub mod model;
pub mod stats;

pub use db::Db;
pub use error::{Error, Result};
pub use model::*;
pub use stats::Stats;
