//! Domain models and pure rules. No database or GUI dependencies.
pub mod diagram;
pub mod model;
pub mod query;
pub use diagram::*;
pub use model::*;
pub use query::*;
