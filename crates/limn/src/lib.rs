//! limn: a native UI library for Rust. An application describes its window
//! as a tree of elements; limn lays it out, keeps the state of every control,
//! and produces drawing commands for a backend to paint.
//!
//! This crate is both halves under one name: the engine, [`limn_core`], and
//! the design that ships with it, [`limn_components`]. An application with a
//! design of its own depends on `limn-core` alone.

pub use limn_components::*;
pub use limn_core::*;
