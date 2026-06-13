//! Safe wrappers around Windows APIs.
//!
//! Unsafe blocks are intentionally kept in this module tree and immediately
//! wrapped by owned or validated Rust types.

#![allow(unsafe_code)]

pub mod com;
pub mod console_capture;
pub mod dynamic_library;
pub mod error;
pub mod filesystem;
pub mod filetime;
pub mod guid;
pub mod handle;
pub mod registry;
pub mod runtime;
pub mod security;
pub mod security_descriptor;
pub mod services;
pub mod string_conversion;
