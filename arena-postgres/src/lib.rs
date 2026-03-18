pub mod postgres_dependency;
pub mod builder;

pub use crate::postgres_dependency::PostgresDependency;
pub use postgres_dependency::postgres_container_impl::PostgresImpl;