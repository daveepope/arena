pub(crate) const MODULE: &str = "arena-temporal";

pub mod builder;
pub mod temporal_dependency;

pub use crate::temporal_dependency::TemporalDependency;
pub use crate::temporal_dependency::TemporalImpl;
