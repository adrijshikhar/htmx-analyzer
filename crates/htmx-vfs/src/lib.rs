#![forbid(unsafe_code)]
//! Virtual File System and rope-based document management for HTMX Analyzer.

pub mod document;
pub mod store;

pub use document::Document;
pub use store::DocumentStore;
