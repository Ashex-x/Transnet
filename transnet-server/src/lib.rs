//! JSON HTTP gateway for the Transnet translation backend.
//!
//! The crate owns API routing and backend forwarding. It intentionally does not serve a WebUI or
//! static files; browser applications must be deployed independently.

mod backend;
mod config;
mod error;
mod model;
mod routes;

pub use backend::BackendClient;
pub use config::ServerConfig;
pub use error::TransnetError;
pub use model::*;
pub use routes::{create_router, AppState};
