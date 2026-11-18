#![forbid(unsafe_code)]
//! Language server protocol implementation for HTMX 2.0.

pub mod handler;
pub mod server;

pub use server::{ExitKind, Server};
