//! The only module that may reach the C library.

/// Name of the binding crate; no other file may name it.
pub const BINDING_CRATE: &str = "lz4-sys";
