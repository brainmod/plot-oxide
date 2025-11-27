pub mod source;
pub mod stats;

// Re-export key types for convenience
#[allow(unused_imports)]
pub use source::{DataSource, DataError};
#[allow(unused_imports)]
pub use stats::Stats;
