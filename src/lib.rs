//! Library part of the molecular engine.
//!
//! The original grid engine lives in `main.rs` and its modules and is left untouched.
//! `sim` is the continuous engine: text chemistry with wildcards, sphere/spring
//! physics, molecule extraction/insertion. It is used by the `acsim` (SDL viewer)
//! and `acrun` (headless) binaries.

pub mod sim;
