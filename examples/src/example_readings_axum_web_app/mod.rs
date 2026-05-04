//! Example Axum web app: bootstrap, HTTP router, shared state, and OAuth.

pub mod app;
pub mod oauth;
pub mod readings;
pub mod router;
pub mod state;

pub use app::ExampleAxumWebApp;
