//! General-purpose utilities inspired by the C++ `vlr-util` library.
//!
//! The Rust version favors standard-library types and idioms over one-to-one
//! ports. Modules are added as useful behavior is migrated.

pub mod cache;
pub mod conversion;
pub mod data;
pub mod display;
pub mod filesystem;
pub mod logging;
pub mod network;
pub mod numeric;
pub mod options;
pub mod retry;
pub mod scope;
pub mod shared_registry;
pub mod strings;
pub mod text;
pub mod threading;

#[cfg(windows)]
pub mod windows;

pub mod ffi;
